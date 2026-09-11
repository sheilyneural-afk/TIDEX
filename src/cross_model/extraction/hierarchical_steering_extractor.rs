//! Steering extraction from measured internal activations.

use crate::cross_model::models::{
    sha256_hex, CapabilityEvidenceKind, CapabilityMetadata, DType, Device, LLMModel, ModelAccess,
    SteeringVector, Tensor,
};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExtractionLevel {
    SingleLayer { layer: usize },
    LayerRange { start: usize, end_exclusive: usize },
    AllLayers,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QualityMetrics {
    pub norm: f64,
    pub sparsity: f64,
    pub mean_direction_cosine: f64,
    pub direction_persistence: f64,
    pub relative_dispersion: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HierarchicalComponent {
    pub layer_index: usize,
    pub vector: Tensor,
    pub quality: QualityMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExtractionResult {
    pub schema: String,
    pub steering_vector: SteeringVector,
    pub components: Vec<HierarchicalComponent>,
    pub positive_prompt_sha256: Vec<String>,
    pub negative_prompt_sha256: Vec<String>,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HierarchicalExtractorConfig {
    pub minimum_pairs: usize,
    pub maximum_pairs: usize,
    pub maximum_layers: usize,
    pub sparsity_epsilon: f64,
    pub minimum_direction_persistence: f64,
    pub minimum_mean_direction_cosine: f64,
}

impl Default for HierarchicalExtractorConfig {
    fn default() -> Self {
        Self {
            minimum_pairs: 4,
            maximum_pairs: 4096,
            maximum_layers: 256,
            sparsity_epsilon: 1e-8,
            minimum_direction_persistence: 0.75,
            minimum_mean_direction_cosine: 0.25,
        }
    }
}

impl HierarchicalExtractorConfig {
    fn validate(&self) -> Result<(), String> {
        if self.minimum_pairs < 2
            || self.maximum_pairs < self.minimum_pairs
            || self.maximum_layers == 0
            || !self.sparsity_epsilon.is_finite()
            || self.sparsity_epsilon < 0.0
            || !self.minimum_direction_persistence.is_finite()
            || !(0.0..=1.0).contains(&self.minimum_direction_persistence)
            || !self.minimum_mean_direction_cosine.is_finite()
            || !(-1.0..=1.0).contains(&self.minimum_mean_direction_cosine)
        {
            return Err("hierarchical_extractor_config_invalid".into());
        }
        Ok(())
    }
}

pub struct HierarchicalSteeringExtractor {
    config: HierarchicalExtractorConfig,
}

impl HierarchicalSteeringExtractor {
    pub fn new(config: HierarchicalExtractorConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn extract(
        &self,
        model: &dyn LLMModel,
        capability_name: &str,
        domain: &str,
        level: ExtractionLevel,
        positive_examples: &[String],
        negative_examples: &[String],
    ) -> Result<ExtractionResult, Box<dyn Error + Send + Sync>> {
        self.config.validate()?;
        if !model.supports(ModelAccess::InternalActivations) {
            return Err(format!("internal_activation_backend_required:{}", model.name()).into());
        }
        if capability_name.trim().is_empty()
            || domain.trim().is_empty()
            || positive_examples.len() != negative_examples.len()
            || positive_examples.len() < self.config.minimum_pairs
            || positive_examples.len() > self.config.maximum_pairs
            || positive_examples
                .iter()
                .any(|value| value.trim().is_empty())
            || negative_examples
                .iter()
                .any(|value| value.trim().is_empty())
        {
            return Err("steering_extraction_examples_invalid".into());
        }
        let layers = self.layers_for(model, &level)?;
        let mut components = Vec::with_capacity(layers.len());
        for layer in layers {
            components.push(self.extract_layer(
                model,
                layer,
                positive_examples,
                negative_examples,
            )?);
        }
        let best = components
            .iter()
            .filter(|component| {
                component.quality.direction_persistence >= self.config.minimum_direction_persistence
                    && component.quality.mean_direction_cosine
                        >= self.config.minimum_mean_direction_cosine
            })
            .max_by(|left, right| {
                quality_score(&left.quality)
                    .total_cmp(&quality_score(&right.quality))
                    .then_with(|| right.layer_index.cmp(&left.layer_index))
            })
            .ok_or("no_activation_layer_passed_quality_gate")?;

        let positive_prompt_sha256 = positive_examples
            .iter()
            .map(|value| sha256_hex(value.as_bytes()))
            .collect::<Vec<_>>();
        let negative_prompt_sha256 = negative_examples
            .iter()
            .map(|value| sha256_hex(value.as_bytes()))
            .collect::<Vec<_>>();
        let evidence_payload = serde_json::to_vec(&(
            model.config().runtime_metadata_sha256.as_str(),
            capability_name,
            domain,
            &positive_prompt_sha256,
            &negative_prompt_sha256,
            &components,
        ))?;
        let evidence_sha256 = sha256_hex(&evidence_payload);
        let confidence = ((best.quality.direction_persistence
            + ((best.quality.mean_direction_cosine + 1.0) / 2.0))
            / 2.0)
            .clamp(0.0, 1.0);
        let metadata = CapabilityMetadata {
            name: capability_name.into(),
            source_model: model.name().into(),
            source_layer: Some(best.layer_index),
            domain: domain.into(),
            confidence,
            evidence_kind: CapabilityEvidenceKind::InternalActivation,
            evidence_sha256: evidence_sha256.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let steering_vector = SteeringVector::new(best.vector.clone(), metadata, 1.0)?;
        Ok(ExtractionResult {
            schema: "cerebro.cross_model.steering_extraction/v1".into(),
            steering_vector,
            components,
            positive_prompt_sha256,
            negative_prompt_sha256,
            evidence_sha256,
        })
    }

    fn layers_for(
        &self,
        model: &dyn LLMModel,
        level: &ExtractionLevel,
    ) -> Result<Vec<usize>, String> {
        let count = model.num_layers();
        let layers = match level {
            ExtractionLevel::SingleLayer { layer } => vec![*layer],
            ExtractionLevel::LayerRange {
                start,
                end_exclusive,
            } => {
                if start >= end_exclusive {
                    return Err("extraction_layer_range_invalid".into());
                }
                (*start..*end_exclusive).collect()
            }
            ExtractionLevel::AllLayers => (0..count).collect(),
        };
        if layers.is_empty()
            || layers.len() > self.config.maximum_layers
            || layers.iter().any(|layer| *layer >= count)
        {
            return Err("extraction_layers_invalid".into());
        }
        Ok(layers)
    }

    fn extract_layer(
        &self,
        model: &dyn LLMModel,
        layer: usize,
        positives: &[String],
        negatives: &[String],
    ) -> Result<HierarchicalComponent, Box<dyn Error + Send + Sync>> {
        let mut pair_differences = Vec::with_capacity(positives.len());
        let mut dimension = None;
        for (positive, negative) in positives.iter().zip(negatives) {
            let pos = model.extract_layer_activations(layer, positive)?;
            let neg = model.extract_layer_activations(layer, negative)?;
            pos.validate()?;
            neg.validate()?;
            if pos.layer_index != layer
                || neg.layer_index != layer
                || pos.data.len() != neg.data.len()
            {
                return Err("activation_pair_binding_invalid".into());
            }
            if let Some(expected) = dimension {
                if pos.data.len() != expected {
                    return Err("activation_dimension_changed".into());
                }
            } else {
                dimension = Some(pos.data.len());
            }
            pair_differences.push(
                pos.data
                    .iter()
                    .zip(&neg.data)
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>(),
            );
        }
        let dimension = dimension.ok_or("activation_dimension_missing")?;
        if dimension == 0 {
            return Err("activation_dimension_zero".into());
        }
        let mut mean = vec![0.0; dimension];
        for row in &pair_differences {
            for (target, value) in mean.iter_mut().zip(row) {
                *target += *value / pair_differences.len() as f64;
            }
        }
        let vector = Tensor::new(mean, vec![dimension], Device::Cpu, DType::F64, layer);
        vector.validate()?;
        let norm = vector.l2_norm();
        if norm <= f64::EPSILON {
            return Err("activation_difference_degenerate".into());
        }
        let mut cosine_sum = 0.0;
        let mut persistent = 0usize;
        let mut squared_error = 0.0;
        for row in &pair_differences {
            let row_tensor =
                Tensor::new(row.clone(), vec![dimension], Device::Cpu, DType::F64, layer);
            let cosine = row_tensor
                .cosine_similarity(&vector)
                .ok_or("activation_pair_zero_norm")?;
            cosine_sum += cosine;
            if cosine > 0.0 {
                persistent += 1;
            }
            squared_error += row
                .iter()
                .zip(&vector.data)
                .map(|(value, mean)| (value - mean).powi(2))
                .sum::<f64>();
        }
        let mean_direction_cosine = cosine_sum / pair_differences.len() as f64;
        let direction_persistence = persistent as f64 / pair_differences.len() as f64;
        let rmse = (squared_error / (pair_differences.len() * dimension) as f64).sqrt();
        let relative_dispersion = rmse / (norm / (dimension as f64).sqrt());
        let sparsity = vector
            .data
            .iter()
            .filter(|value| value.abs() <= self.config.sparsity_epsilon)
            .count() as f64
            / dimension as f64;
        Ok(HierarchicalComponent {
            layer_index: layer,
            vector,
            quality: QualityMetrics {
                norm,
                sparsity,
                mean_direction_cosine,
                direction_persistence,
                relative_dispersion,
            },
        })
    }
}

fn quality_score(metrics: &QualityMetrics) -> f64 {
    metrics.direction_persistence * ((metrics.mean_direction_cosine + 1.0) / 2.0)
        / (1.0 + metrics.relative_dispersion)
}
impl Default for HierarchicalSteeringExtractor {
    fn default() -> Self {
        Self::new(HierarchicalExtractorConfig::default()).expect("static extractor config")
    }
}
