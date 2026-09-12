//! Deterministic evidence-driven routing.
//!
//! Exploration is implemented as an uncertainty bonus from sample count, not
//! randomness. A model with no evidence is never silently assigned score 0.5.

use crate::cross_model::models::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RoutingObservation {
    pub model: String,
    pub score: f64,
    pub sample_size: usize,
    pub evidence_sha256: String,
}

impl RoutingObservation {
    fn validate(&self) -> Result<(), String> {
        if self.model.trim().is_empty()
            || !self.score.is_finite()
            || !(0.0..=1.0).contains(&self.score)
            || self.sample_size == 0
            || self.evidence_sha256.len() != 64
            || !self
                .evidence_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("routing_observation_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingPlasticityMatrix {
    pub weights: HashMap<String, HashMap<String, f64>>,
    pub learning_rate: f64,
    pub decay_rate: f64,
}

impl Default for RoutingPlasticityMatrix {
    fn default() -> Self {
        Self {
            weights: HashMap::new(),
            learning_rate: 0.01,
            decay_rate: 0.001,
        }
    }
}

impl RoutingPlasticityMatrix {
    fn validate_identity(value: &str) -> Result<(), String> {
        if value.trim().is_empty() || value.len() > 4096 {
            return Err("routing_plasticity_identity_invalid".into());
        }
        Ok(())
    }

    pub fn update_weight(
        &mut self,
        capability: &str,
        model: &str,
        correlation: f64,
    ) -> Result<f64, String> {
        Self::validate_identity(capability)?;
        Self::validate_identity(model)?;
        if !correlation.is_finite() {
            return Err("routing_plasticity_correlation_must_be_finite".into());
        }
        let delta = self.learning_rate * correlation - self.decay_rate;
        let row = self.weights.entry(capability.to_string()).or_default();
        let weight = row.entry(model.to_string()).or_insert(0.5);
        *weight = (*weight + delta).clamp(0.0, 1.0);
        Ok(*weight)
    }

    pub fn get_routing_strength(&self, capability: &str, model: &str) -> Result<f64, String> {
        Self::validate_identity(capability)?;
        Self::validate_identity(model)?;
        Ok(self
            .weights
            .get(capability)
            .and_then(|row| row.get(model))
            .copied()
            .unwrap_or(0.5))
    }

    pub fn stability_adjusted_score(
        &self,
        capability: &str,
        model: &str,
        measured_score: f64,
        uncertainty_bonus: f64,
        history_consistency: f64,
    ) -> Result<f64, String> {
        let matrix_strength = self
            .get_routing_strength(capability, model)?
            .clamp(0.0, 1.0);
        if !measured_score.is_finite()
            || !uncertainty_bonus.is_finite()
            || !history_consistency.is_finite()
        {
            return Err("routing_plasticity_score_inputs_must_be_finite".into());
        }
        let stability = history_consistency.clamp(0.0, 1.0);
        let structural_bonus = matrix_strength * 0.10 + stability * 0.05;
        Ok(measured_score + uncertainty_bonus + structural_bonus)
    }

    pub fn decay_tick(&mut self) -> Result<(), String> {
        if !self.decay_rate.is_finite() || !(0.0..=1.0).contains(&self.decay_rate) {
            return Err("routing_plasticity_decay_rate_must_be_within_0_1".into());
        }
        let factor = 1.0 - self.decay_rate;
        for row in self.weights.values_mut() {
            for weight in row.values_mut() {
                *weight *= factor;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RoutingDecision {
    pub target_model: String,
    pub capability: String,
    pub measured_score: f64,
    pub uncertainty_bonus: f64,
    pub routing_score: f64,
    pub evidence_sha256: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RoutingPlasticityConfig {
    pub uncertainty_weight: f64,
    pub minimum_measured_score: f64,
}

impl Default for RoutingPlasticityConfig {
    fn default() -> Self {
        Self {
            uncertainty_weight: 0.05,
            minimum_measured_score: 0.0,
        }
    }
}

impl RoutingPlasticityConfig {
    fn validate(&self) -> Result<(), String> {
        if !self.uncertainty_weight.is_finite()
            || self.uncertainty_weight < 0.0
            || !self.minimum_measured_score.is_finite()
            || !(0.0..=1.0).contains(&self.minimum_measured_score)
        {
            return Err("routing_config_invalid".into());
        }
        Ok(())
    }
}

pub struct RoutingPlasticity {
    config: RoutingPlasticityConfig,
    routing_history: HashMap<String, Vec<RoutingDecision>>,
    matrix: RoutingPlasticityMatrix,
}

impl RoutingPlasticity {
    pub fn new(config: RoutingPlasticityConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            routing_history: HashMap::new(),
            matrix: RoutingPlasticityMatrix::default(),
        })
    }

    pub fn route_capability(
        &mut self,
        capability: &str,
        observations: &[RoutingObservation],
    ) -> Result<RoutingDecision, String> {
        self.config.validate()?;
        if capability.trim().is_empty() || observations.is_empty() {
            return Err("routing_input_invalid".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for observation in observations {
            observation.validate()?;
            if !seen.insert(&observation.model) {
                return Err("routing_model_duplicate".into());
            }
        }
        let total_samples = observations
            .iter()
            .map(|row| row.sample_size)
            .sum::<usize>() as f64;
        let prior_history = self
            .routing_history
            .get(capability)
            .cloned()
            .unwrap_or_default();
        let mut scored = observations
            .iter()
            .filter(|row| row.score >= self.config.minimum_measured_score)
            .map(|row| {
                let uncertainty = self.config.uncertainty_weight
                    * ((total_samples.max(1.0).ln() / row.sample_size as f64).max(0.0)).sqrt();
                let history_consistency = if prior_history.is_empty() {
                    0.5
                } else {
                    let mean_gap = prior_history
                        .iter()
                        .map(|decision| (decision.routing_score - row.score).abs())
                        .sum::<f64>()
                        / prior_history.len() as f64;
                    (1.0 - mean_gap.clamp(0.0, 1.0)).clamp(0.0, 1.0)
                };
                let adjusted_score = self.matrix.stability_adjusted_score(
                    capability,
                    &row.model,
                    row.score,
                    uncertainty,
                    history_consistency,
                )?;
                Ok((row, uncertainty, adjusted_score))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let selected = scored
            .drain(..)
            .max_by(|left, right| {
                left.2
                    .total_cmp(&right.2)
                    .then_with(|| left.0.sample_size.cmp(&right.0.sample_size))
            })
            .ok_or("no_routing_candidate_passed_policy")?;
        let evidence_sha256 = sha256_hex(
            &serde_json::to_vec(&(capability, observations, selected.0.evidence_sha256.as_str()))
                .map_err(|error| format!("routing_serialize:{error}"))?,
        );
        let decision = RoutingDecision {
            target_model: selected.0.model.clone(),
            capability: capability.into(),
            measured_score: selected.0.score,
            uncertainty_bonus: selected.1,
            routing_score: selected.2,
            evidence_sha256,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.matrix
            .update_weight(capability, &selected.0.model, selected.0.score)?;
        self.routing_history
            .entry(capability.into())
            .or_default()
            .push(decision.clone());
        Ok(decision)
    }

    pub fn update_routing(
        &mut self,
        capability: &str,
        decision: RoutingDecision,
    ) -> Result<(), String> {
        if capability != decision.capability || capability.trim().is_empty() {
            return Err("routing_update_binding_invalid".into());
        }
        self.routing_history
            .entry(capability.into())
            .or_default()
            .push(decision);
        Ok(())
    }

    pub fn get_routing_history(&self, capability: &str) -> &[RoutingDecision] {
        self.routing_history
            .get(capability)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn get_all_routing(&self) -> Vec<&RoutingDecision> {
        self.routing_history
            .values()
            .flat_map(|rows| rows.iter())
            .collect()
    }

    pub fn clear_history(&mut self) {
        self.routing_history.clear();
    }

    pub fn config(&self) -> &RoutingPlasticityConfig {
        &self.config
    }

    pub fn set_matrix_learning_rate(&mut self, learning_rate: f64) -> Result<(), String> {
        if !learning_rate.is_finite() || learning_rate < 0.0 || learning_rate > 1.0 {
            return Err("routing_matrix_learning_rate_invalid".into());
        }
        self.matrix.learning_rate = learning_rate;
        Ok(())
    }

    pub fn export_history(&self) -> HashMap<String, Vec<RoutingDecision>> {
        self.routing_history.clone()
    }

    pub fn import_history(
        &mut self,
        history: HashMap<String, Vec<RoutingDecision>>,
    ) -> Result<(), String> {
        for (capability, decisions) in &history {
            if capability.trim().is_empty() {
                return Err("routing_history_import_invalid".into());
            }
            for decision in decisions {
                if decision.capability != *capability
                    || decision.target_model.trim().is_empty()
                    || !decision.measured_score.is_finite()
                    || !decision.routing_score.is_finite()
                    || decision.evidence_sha256.len() != 64
                    || !decision
                        .evidence_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit())
                    || chrono::DateTime::parse_from_rfc3339(&decision.timestamp).is_err()
                {
                    return Err("routing_history_import_invalid".into());
                }
            }
        }
        self.routing_history = history;
        Ok(())
    }

    pub fn export_matrix(&self) -> RoutingPlasticityMatrix {
        self.matrix.clone()
    }

    pub fn import_matrix(&mut self, matrix: RoutingPlasticityMatrix) -> Result<(), String> {
        if !matrix.learning_rate.is_finite()
            || matrix.learning_rate < 0.0
            || !matrix.decay_rate.is_finite()
            || !(0.0..=1.0).contains(&matrix.decay_rate)
        {
            return Err("routing_matrix_import_invalid".into());
        }
        for (capability, row) in &matrix.weights {
            RoutingPlasticityMatrix::validate_identity(capability)?;
            for (model, weight) in row {
                RoutingPlasticityMatrix::validate_identity(model)?;
                if !weight.is_finite() || !(0.0..=1.0).contains(weight) {
                    return Err("routing_matrix_import_invalid".into());
                }
            }
        }
        self.matrix = matrix;
        Ok(())
    }

    pub fn get_statistics(&self) -> RoutingStatistics {
        let total_decisions = self.routing_history.values().map(Vec::len).sum();
        let mut model_usage = HashMap::new();
        for decision in self.get_all_routing() {
            *model_usage
                .entry(decision.target_model.clone())
                .or_insert(0usize) += 1;
        }
        RoutingStatistics {
            total_decisions,
            unique_capabilities: self.routing_history.len(),
            model_usage,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RoutingStatistics {
    pub total_decisions: usize,
    pub unique_capabilities: usize,
    pub model_usage: HashMap<String, usize>,
}
impl Default for RoutingPlasticity {
    fn default() -> Self {
        Self::new(RoutingPlasticityConfig::default()).expect("static routing config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_matrix_adapts_to_positive_correlation() {
        let mut matrix = RoutingPlasticityMatrix::default();
        let updated = matrix
            .update_weight("capability-a", "model-b", 0.9)
            .expect("routing weight should update");
        let strength = matrix
            .get_routing_strength("capability-a", "model-b")
            .expect("routing strength should be readable");
        matrix.decay_tick().expect("decay should be valid");

        assert!(updated > 0.5);
        assert!(strength > 0.5);
        assert!(
            matrix
                .get_routing_strength("capability-a", "model-b")
                .unwrap()
                <= 1.0
        );
    }

    #[test]
    fn routing_capability_prefers_stability_weighted_model_when_scores_are_equal() {
        let mut routing = RoutingPlasticity::new(RoutingPlasticityConfig {
            uncertainty_weight: 0.0,
            minimum_measured_score: 0.0,
        })
        .expect("valid routing config");

        let mut matrix = RoutingPlasticityMatrix::default();
        matrix
            .update_weight("reasoning", "model-strong", 0.95)
            .expect("should strengthen the preferred model path");
        routing.matrix = matrix;

        let observations = vec![
            RoutingObservation {
                model: "model-weak".to_string(),
                score: 0.81,
                sample_size: 8,
                evidence_sha256: "a".repeat(64),
            },
            RoutingObservation {
                model: "model-strong".to_string(),
                score: 0.81,
                sample_size: 12,
                evidence_sha256: "b".repeat(64),
            },
        ];

        let decision = routing
            .route_capability("reasoning", &observations)
            .expect("routing should select a stable candidate");

        assert_eq!(decision.target_model, "model-strong");
        assert!(decision.routing_score > 0.8);
    }
}
