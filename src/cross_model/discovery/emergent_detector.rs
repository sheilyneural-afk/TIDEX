//! Emergence analysis over measured model-scale observations.
//!
//! No score is inferred from model names or prompt length. The detector only
//! consumes authenticated `ModelEvaluation` values produced by real inference.

use super::ModelEvaluation;
use crate::cross_model::models::sha256_hex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmergenceType {
    ConcentratedGain,
    GradualGain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScaleObservation {
    pub model: String,
    pub parameter_count: u64,
    pub evaluation: ModelEvaluation,
}

impl ScaleObservation {
    fn validate(&self) -> Result<(), String> {
        self.evaluation.validate()?;
        if self.model.trim().is_empty()
            || self.parameter_count == 0
            || self.model != self.evaluation.model
        {
            return Err("scale_observation_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EmergentCapability {
    pub schema: String,
    pub benchmark_id: String,
    pub emergence_type: EmergenceType,
    pub first_model: String,
    pub last_model: String,
    pub first_score: f64,
    pub last_score: f64,
    pub total_gain: f64,
    pub largest_adjacent_gain: f64,
    pub transition_after_model: String,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EmergentDetectorConfig {
    pub minimum_points: usize,
    pub minimum_total_gain: f64,
    pub concentrated_gain_fraction: f64,
}

impl Default for EmergentDetectorConfig {
    fn default() -> Self {
        Self {
            minimum_points: 3,
            minimum_total_gain: 0.15,
            concentrated_gain_fraction: 0.6,
        }
    }
}

impl EmergentDetectorConfig {
    fn validate(&self) -> Result<(), String> {
        if self.minimum_points < 3
            || self.minimum_points > 10_000
            || !self.minimum_total_gain.is_finite()
            || !(0.0..=1.0).contains(&self.minimum_total_gain)
            || !self.concentrated_gain_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.concentrated_gain_fraction)
        {
            return Err("emergent_detector_config_invalid".into());
        }
        Ok(())
    }
}

pub struct EmergentDetector {
    config: EmergentDetectorConfig,
}

impl EmergentDetector {
    pub fn new(config: EmergentDetectorConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn analyze_emergence(
        &self,
        observations: &[ScaleObservation],
    ) -> Result<Option<EmergentCapability>, String> {
        self.config.validate()?;
        if observations.len() < self.config.minimum_points {
            return Ok(None);
        }
        for observation in observations {
            observation.validate()?;
        }
        let benchmark_id = observations[0].evaluation.benchmark_id.clone();
        let benchmark_sha = observations[0].evaluation.benchmark_sha256.clone();
        if observations.iter().any(|row| {
            row.evaluation.benchmark_id != benchmark_id
                || row.evaluation.benchmark_sha256 != benchmark_sha
        }) {
            return Err("emergence_benchmark_mismatch".into());
        }
        let mut ordered = observations.to_vec();
        ordered.sort_by_key(|row| row.parameter_count);
        if ordered
            .windows(2)
            .any(|pair| pair[0].parameter_count == pair[1].parameter_count)
        {
            return Err("emergence_duplicate_scale".into());
        }
        let first = &ordered[0];
        let last = ordered.last().ok_or("emergence_empty")?;
        let total_gain = last.evaluation.weighted_score - first.evaluation.weighted_score;
        if total_gain < self.config.minimum_total_gain {
            return Ok(None);
        }

        let mut largest_gain = f64::NEG_INFINITY;
        let mut transition_index = 0usize;
        for (index, pair) in ordered.windows(2).enumerate() {
            let gain = pair[1].evaluation.weighted_score - pair[0].evaluation.weighted_score;
            if gain > largest_gain {
                largest_gain = gain;
                transition_index = index;
            }
        }
        if !largest_gain.is_finite() || largest_gain <= 0.0 {
            return Ok(None);
        }
        let emergence_type = if largest_gain / total_gain >= self.config.concentrated_gain_fraction
        {
            EmergenceType::ConcentratedGain
        } else {
            EmergenceType::GradualGain
        };
        let evidence_bytes =
            serde_json::to_vec(&ordered).map_err(|error| format!("emergence_serialize:{error}"))?;
        Ok(Some(EmergentCapability {
            schema: "cerebro.cross_model.emergent_capability/v1".into(),
            benchmark_id,
            emergence_type,
            first_model: first.model.clone(),
            last_model: last.model.clone(),
            first_score: first.evaluation.weighted_score,
            last_score: last.evaluation.weighted_score,
            total_gain,
            largest_adjacent_gain: largest_gain,
            transition_after_model: ordered[transition_index].model.clone(),
            evidence_sha256: sha256_hex(&evidence_bytes),
        }))
    }
}
impl Default for EmergentDetector {
    fn default() -> Self {
        Self::new(EmergentDetectorConfig::default()).expect("static emergence config")
    }
}
