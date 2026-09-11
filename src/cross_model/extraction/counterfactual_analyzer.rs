//! Executed counterfactual analysis.
//!
//! The caller supplies the intervention and verifier. TIDE-X executes both
//! prompts on the same runtime and measures the change; it never invents model
//! output or semantic perturbations.

use crate::cross_model::discovery::ProbeVerifier;
use crate::cross_model::models::{sha256_hex, LLMModel, ModelAccess};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerturbationType {
    DeleteDeclaredSpan,
    ReplaceDeclaredLiteral,
    AppendDeclaredContext,
    PrependDeclaredContext,
    CallerDefined,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CounterfactualScenario {
    pub scenario_id: String,
    pub original_input: String,
    pub perturbed_input: String,
    pub perturbation_type: PerturbationType,
    pub verifier: ProbeVerifier,
}

impl CounterfactualScenario {
    fn validate(&self) -> Result<(), String> {
        if self.scenario_id.trim().is_empty()
            || self.original_input.trim().is_empty()
            || self.perturbed_input.trim().is_empty()
            || self.original_input == self.perturbed_input
            || self.original_input.len() > 1_048_576
            || self.perturbed_input.len() > 1_048_576
        {
            return Err("counterfactual_scenario_invalid".into());
        }
        self.verifier.validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LayerInterventionObservation {
    pub layer_index: usize,
    pub relative_activation_change: f64,
    pub cosine_similarity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CounterfactualResult {
    pub schema: String,
    pub scenario: CounterfactualScenario,
    pub model: String,
    pub original_response_sha256: String,
    pub perturbed_response_sha256: String,
    pub original_response: String,
    pub perturbed_response: String,
    pub original_score: f64,
    pub perturbed_score: f64,
    pub verified_effect: f64,
    pub layer_observations: Vec<LayerInterventionObservation>,
    pub evidence_sha256: String,
}

impl CounterfactualResult {
    pub fn validate(&self) -> Result<(), String> {
        self.scenario.validate()?;
        if self.schema != "cerebro.cross_model.counterfactual_result/v1"
            || self.model.trim().is_empty()
            || !(0.0..=1.0).contains(&self.original_score)
            || !(0.0..=1.0).contains(&self.perturbed_score)
            || !self.verified_effect.is_finite()
            || !(-1.0..=1.0).contains(&self.verified_effect)
            || self.original_response_sha256 != sha256_hex(self.original_response.as_bytes())
            || self.perturbed_response_sha256 != sha256_hex(self.perturbed_response.as_bytes())
            || self.layer_observations.iter().any(|row| {
                !row.relative_activation_change.is_finite()
                    || row.relative_activation_change < 0.0
                    || !row.cosine_similarity.is_finite()
                    || !(-1.0..=1.0).contains(&row.cosine_similarity)
            })
        {
            return Err("counterfactual_result_invalid".into());
        }
        if counterfactual_digest(self)? != self.evidence_sha256 {
            return Err("counterfactual_result_digest_mismatch".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CounterfactualAnalyzerConfig {
    pub maximum_scenarios: usize,
    pub maximum_activation_layers: usize,
}

impl Default for CounterfactualAnalyzerConfig {
    fn default() -> Self {
        Self {
            maximum_scenarios: 256,
            maximum_activation_layers: 64,
        }
    }
}

pub struct CounterfactualAnalyzer {
    config: CounterfactualAnalyzerConfig,
}

impl CounterfactualAnalyzer {
    pub fn new(config: CounterfactualAnalyzerConfig) -> Result<Self, String> {
        if config.maximum_scenarios == 0
            || config.maximum_scenarios > 100_000
            || config.maximum_activation_layers == 0
            || config.maximum_activation_layers > 4096
        {
            return Err("counterfactual_analyzer_config_invalid".into());
        }
        Ok(Self { config })
    }

    pub fn analyze(
        &self,
        model: &dyn LLMModel,
        scenarios: &[CounterfactualScenario],
        activation_layers: &[usize],
    ) -> Result<Vec<CounterfactualResult>, Box<dyn Error + Send + Sync>> {
        if scenarios.is_empty() || scenarios.len() > self.config.maximum_scenarios {
            return Err("counterfactual_scenario_count_invalid".into());
        }
        if activation_layers.len() > self.config.maximum_activation_layers
            || activation_layers
                .iter()
                .any(|layer| *layer >= model.num_layers())
        {
            return Err("counterfactual_activation_layers_invalid".into());
        }
        if !activation_layers.is_empty() && !model.supports(ModelAccess::InternalActivations) {
            return Err("counterfactual_internal_activation_backend_required".into());
        }
        scenarios
            .iter()
            .map(|scenario| self.analyze_one(model, scenario, activation_layers))
            .collect()
    }

    fn analyze_one(
        &self,
        model: &dyn LLMModel,
        scenario: &CounterfactualScenario,
        activation_layers: &[usize],
    ) -> Result<CounterfactualResult, Box<dyn Error + Send + Sync>> {
        scenario.validate()?;
        let original = model.generate(&scenario.original_input)?;
        let perturbed = model.generate(&scenario.perturbed_input)?;
        let original_score = scenario.verifier.score(&original.text)?;
        let perturbed_score = scenario.verifier.score(&perturbed.text)?;
        let mut layer_observations = Vec::with_capacity(activation_layers.len());
        for layer in activation_layers {
            let before = model.extract_layer_activations(*layer, &scenario.original_input)?;
            let after = model.extract_layer_activations(*layer, &scenario.perturbed_input)?;
            before.validate()?;
            after.validate()?;
            if before.layer_index != *layer
                || after.layer_index != *layer
                || before.data.len() != after.data.len()
            {
                return Err("counterfactual_activation_binding_invalid".into());
            }
            let difference_norm = before
                .data
                .iter()
                .zip(&after.data)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            let denominator = before.l2_norm().max(f64::EPSILON);
            let cosine_similarity = before
                .cosine_similarity(&after)
                .ok_or("counterfactual_activation_zero_norm")?;
            layer_observations.push(LayerInterventionObservation {
                layer_index: *layer,
                relative_activation_change: difference_norm / denominator,
                cosine_similarity,
            });
        }
        let mut result = CounterfactualResult {
            schema: "cerebro.cross_model.counterfactual_result/v1".into(),
            scenario: scenario.clone(),
            model: model.name().into(),
            original_response_sha256: original.response_sha256,
            perturbed_response_sha256: perturbed.response_sha256,
            original_response: original.text,
            perturbed_response: perturbed.text,
            original_score,
            perturbed_score,
            verified_effect: original_score - perturbed_score,
            layer_observations,
            evidence_sha256: String::new(),
        };
        result.evidence_sha256 = counterfactual_digest(&result)?;
        result.validate()?;
        Ok(result)
    }

    pub fn aggregate_results(
        &self,
        results: &[CounterfactualResult],
    ) -> Result<CounterfactualSummary, String> {
        if results.is_empty() {
            return Err("counterfactual_summary_empty".into());
        }
        for result in results {
            result.validate()?;
        }
        let mean_verified_effect =
            results.iter().map(|row| row.verified_effect).sum::<f64>() / results.len() as f64;
        let degradation_rate = results
            .iter()
            .filter(|row| row.verified_effect > 0.0)
            .count() as f64
            / results.len() as f64;
        let improvement_rate = results
            .iter()
            .filter(|row| row.verified_effect < 0.0)
            .count() as f64
            / results.len() as f64;
        let evidence_sha256 = sha256_hex(
            &serde_json::to_vec(results)
                .map_err(|e| format!("counterfactual_summary_serialize:{e}"))?,
        );
        Ok(CounterfactualSummary {
            total_scenarios: results.len(),
            mean_verified_effect,
            degradation_rate,
            improvement_rate,
            evidence_sha256,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CounterfactualSummary {
    pub total_scenarios: usize,
    pub mean_verified_effect: f64,
    pub degradation_rate: f64,
    pub improvement_rate: f64,
    pub evidence_sha256: String,
}

fn counterfactual_digest(result: &CounterfactualResult) -> Result<String, String> {
    let mut unsigned = result.clone();
    unsigned.evidence_sha256.clear();
    serde_json::to_vec(&unsigned)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("counterfactual_result_serialize:{error}"))
}
impl Default for CounterfactualAnalyzer {
    fn default() -> Self {
        Self::new(CounterfactualAnalyzerConfig::default()).expect("static counterfactual config")
    }
}
