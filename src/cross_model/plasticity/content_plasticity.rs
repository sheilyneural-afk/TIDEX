//! Evidence-bound content drift controller.
//!
//! This module tracks measured representation similarity. It never edits model
//! representations; its output is only a bounded control signal for governance.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContentPlasticityState {
    pub representation_sha256: String,
    pub adaptation_pressure: f64,
    pub measured_similarity: f64,
    pub evidence_sha256: String,
    pub last_update: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentPlasticityMatrix {
    pub fact_confidence: HashMap<String, f64>,
    pub source_trust: HashMap<String, f64>,
    pub cross_reference_graph: HashMap<String, Vec<String>>,
    pub temporal_decay: HashMap<String, f64>,
    pub bcm_threshold_content: f64,
    pub bcm_rate_content: f64,
    pub eligibility_lambda_content: f64,
    pub eligibility_decay_content: f64,
    pub min_consolidation_confidence: f64,
    pub consolidation_rate: f64,
}

impl Default for ContentPlasticityMatrix {
    fn default() -> Self {
        Self {
            fact_confidence: HashMap::new(),
            source_trust: HashMap::new(),
            cross_reference_graph: HashMap::new(),
            temporal_decay: HashMap::new(),
            bcm_threshold_content: 0.5,
            bcm_rate_content: 0.01,
            eligibility_lambda_content: 0.95,
            eligibility_decay_content: 0.99,
            min_consolidation_confidence: 0.7,
            consolidation_rate: 0.1,
        }
    }
}

impl ContentPlasticityMatrix {
    fn validate_fact_id(fact_id: &str) -> Result<(), String> {
        if fact_id.trim().is_empty() {
            return Err("content_plasticity_fact_id_invalid".into());
        }
        Ok(())
    }

    pub fn update_fact_confidence(
        &mut self,
        fact_id: &str,
        activity: f64,
        verification_confidence: f64,
    ) -> Result<f64, String> {
        Self::validate_fact_id(fact_id)?;
        if !activity.is_finite() || !verification_confidence.is_finite() {
            return Err("content_plasticity_activity_must_be_finite".into());
        }
        let current = self.fact_confidence.get(fact_id).copied().unwrap_or(0.0);
        let bounded_activity = activity.clamp(0.0, 1.0);
        let bounded_verification = verification_confidence.clamp(0.0, 1.0);
        let bcm_drive = bounded_activity * (bounded_activity - self.bcm_threshold_content);
        let delta = bcm_drive * self.bcm_rate_content * bounded_verification;
        let bootstrap = if current == 0.0 && bounded_activity >= self.bcm_threshold_content {
            bounded_activity * bounded_verification * self.bcm_rate_content
        } else {
            0.0
        };
        let new_confidence = (current + delta + bootstrap).clamp(0.0, 1.0);
        self.fact_confidence
            .insert(fact_id.to_string(), new_confidence);
        Ok(new_confidence)
    }

    pub fn update_source_trust(
        &mut self,
        source_id: &str,
        verification_success: bool,
        cross_reference_count: usize,
    ) -> Result<f64, String> {
        if source_id.trim().is_empty() {
            return Err("content_plasticity_source_id_invalid".into());
        }
        let current = self.source_trust.get(source_id).copied().unwrap_or(0.5);
        let verification_delta = if verification_success { 0.1 } else { -0.2 };
        let cross_reference_bonus = (cross_reference_count as f64 / 50.0_f64).clamp(0.0, 1.0) * 0.1;
        let new_trust = (current + verification_delta + cross_reference_bonus).clamp(0.0, 1.0);
        self.source_trust.insert(source_id.to_string(), new_trust);
        Ok(new_trust)
    }

    pub fn add_cross_reference(
        &mut self,
        fact_id: &str,
        reference_fact_id: &str,
    ) -> Result<(), String> {
        Self::validate_fact_id(fact_id)?;
        if reference_fact_id.trim().is_empty() {
            return Err("content_plasticity_reference_id_invalid".into());
        }
        let refs = self
            .cross_reference_graph
            .entry(fact_id.to_string())
            .or_default();
        if refs.iter().any(|item| item == reference_fact_id) {
            return Ok(());
        }
        if refs.len() >= 50 {
            return Err("content_plasticity_cross_reference_limit_exceeded".into());
        }
        refs.push(reference_fact_id.to_string());
        Ok(())
    }

    pub fn consolidate_fact(
        &mut self,
        fact_id: &str,
        evidence_strength: f64,
    ) -> Result<f64, String> {
        Self::validate_fact_id(fact_id)?;
        if !evidence_strength.is_finite() {
            return Err("content_plasticity_evidence_strength_invalid".into());
        }
        let mut confidence = self.fact_confidence.get(fact_id).copied().unwrap_or(0.0);
        let decay = self.temporal_decay.get(fact_id).copied().unwrap_or(0.0);
        let similarity_bonus = self
            .cross_reference_graph
            .get(fact_id)
            .map(|refs| refs.len() as f64 / 50.0)
            .unwrap_or(0.0);
        confidence = (confidence * (1.0 - decay) + evidence_strength * self.consolidation_rate)
            .clamp(0.0, 1.0)
            + similarity_bonus * self.consolidation_rate;
        confidence = confidence.clamp(0.0, 1.0);
        self.temporal_decay
            .insert(fact_id.to_string(), (decay + self.eligibility_decay_content).min(0.99));
        self.fact_confidence.insert(fact_id.to_string(), confidence);
        Ok(confidence)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContentPlasticityConfig {
    pub similarity_threshold: f64,
    pub adaptation_rate: f64,
    pub maximum_pressure: f64,
}

impl Default for ContentPlasticityConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.8,
            adaptation_rate: 0.1,
            maximum_pressure: 1.0,
        }
    }
}

impl ContentPlasticityConfig {
    fn validate(&self) -> Result<(), String> {
        if !self.similarity_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.similarity_threshold)
            || !self.adaptation_rate.is_finite()
            || self.adaptation_rate < 0.0
            || !self.maximum_pressure.is_finite()
            || self.maximum_pressure <= 0.0
        {
            return Err("content_plasticity_config_invalid".into());
        }
        Ok(())
    }
}

pub struct ContentPlasticity {
    config: ContentPlasticityConfig,
    states: HashMap<String, ContentPlasticityState>,
    matrix: ContentPlasticityMatrix,
}

impl ContentPlasticity {
    pub fn new(config: ContentPlasticityConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            states: HashMap::new(),
            matrix: ContentPlasticityMatrix::default(),
        })
    }

    pub fn initialize_state(
        &mut self,
        capability_name: &str,
        representation_sha256: String,
        evidence_sha256: String,
    ) -> Result<(), String> {
        validate_sha(&representation_sha256)?;
        validate_sha(&evidence_sha256)?;
        if capability_name.trim().is_empty() || self.states.contains_key(capability_name) {
            return Err("content_plasticity_initialization_invalid".into());
        }
        self.states.insert(
            capability_name.into(),
            ContentPlasticityState {
                representation_sha256,
                adaptation_pressure: 0.0,
                measured_similarity: 1.0,
                evidence_sha256,
                last_update: chrono::Utc::now().to_rfc3339(),
            },
        );
        Ok(())
    }

    pub fn update_similarity(
        &mut self,
        capability_name: &str,
        representation_sha256: String,
        measured_similarity: f64,
        evidence_sha256: String,
    ) -> Result<f64, String> {
        validate_sha(&representation_sha256)?;
        validate_sha(&evidence_sha256)?;
        if !measured_similarity.is_finite() || !(-1.0..=1.0).contains(&measured_similarity) {
            return Err("content_similarity_invalid".into());
        }
        let state = self
            .states
            .get_mut(capability_name)
            .ok_or("content_plasticity_state_missing")?;
        state.measured_similarity = measured_similarity;
        let evidence_strength = (1.0 - measured_similarity.abs()).clamp(0.0, 1.0);
        let advanced_signal = self.matrix.update_fact_confidence(
            capability_name,
            measured_similarity.abs().clamp(0.0, 1.0),
            evidence_strength,
        )?;
        if measured_similarity < self.config.similarity_threshold {
            let deficit = (self.config.similarity_threshold - measured_similarity).max(0.0);
            state.adaptation_pressure = (state.adaptation_pressure
                + self.config.adaptation_rate * deficit
                + advanced_signal * self.config.adaptation_rate)
                .clamp(0.0, self.config.maximum_pressure);
        } else {
            state.adaptation_pressure = (state.adaptation_pressure
                * (1.0 - self.config.adaptation_rate)
                + advanced_signal * self.config.adaptation_rate * 0.5)
                .max(0.0)
                .min(self.config.maximum_pressure);
        }
        state.representation_sha256 = representation_sha256;
        state.evidence_sha256 = evidence_sha256;
        state.last_update = chrono::Utc::now().to_rfc3339();
        Ok(state.adaptation_pressure)
    }

    pub fn get_adaptation_level(&self, capability_name: &str) -> Option<f64> {
        self.states
            .get(capability_name)
            .map(|state| state.adaptation_pressure)
    }
    pub fn get_state(&self, capability_name: &str) -> Option<&ContentPlasticityState> {
        self.states.get(capability_name)
    }
    pub fn reset_state(&mut self, capability_name: &str) -> Result<(), String> {
        let state = self
            .states
            .get_mut(capability_name)
            .ok_or("content_plasticity_state_missing")?;
        state.adaptation_pressure = 0.0;
        Ok(())
    }
    pub fn clear_all(&mut self) {
        self.states.clear();
    }

    pub fn config(&self) -> &ContentPlasticityConfig {
        &self.config
    }

    pub fn set_adaptation_rate(&mut self, adaptation_rate: f64) -> Result<(), String> {
        let mut candidate = self.config.clone();
        candidate.adaptation_rate = adaptation_rate;
        candidate.validate()?;
        self.config = candidate;
        Ok(())
    }

    pub fn export_states(&self) -> HashMap<String, ContentPlasticityState> {
        self.states.clone()
    }

    pub fn import_states(
        &mut self,
        states: HashMap<String, ContentPlasticityState>,
    ) -> Result<(), String> {
        for (name, state) in &states {
            if name.trim().is_empty() {
                return Err("content_import_invalid".into());
            }
            validate_sha(&state.representation_sha256)?;
            validate_sha(&state.evidence_sha256)?;
            if !state.adaptation_pressure.is_finite()
                || state.adaptation_pressure < 0.0
                || state.adaptation_pressure > self.config.maximum_pressure
                || !state.measured_similarity.is_finite()
                || !(-1.0..=1.0).contains(&state.measured_similarity)
                || chrono::DateTime::parse_from_rfc3339(&state.last_update).is_err()
            {
                return Err("content_import_invalid".into());
            }
        }
        self.states = states;
        Ok(())
    }

    pub fn export_matrix(&self) -> ContentPlasticityMatrix {
        self.matrix.clone()
    }

    pub fn import_matrix(&mut self, matrix: ContentPlasticityMatrix) {
        self.matrix = matrix;
    }
}

fn validate_sha(value: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("content_plasticity_sha256_invalid".into());
    }
    Ok(())
}
impl Default for ContentPlasticity {
    fn default() -> Self {
        Self::new(ContentPlasticityConfig::default()).expect("static content config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advanced_content_plasticity_tracks_fact_confidence_and_source_trust() {
        let mut matrix = ContentPlasticityMatrix::default();
        let fact_confidence = matrix
            .update_fact_confidence("fact-alpha", 0.9, 0.8)
            .expect("fact confidence should update");
        let source_trust = matrix
            .update_source_trust("source-alpha", true, 8)
            .expect("source trust should update");
        let consolidated = matrix
            .consolidate_fact("fact-alpha", 0.85)
            .expect("fact should consolidate");

        assert!(fact_confidence > 0.0);
        assert!(source_trust > 0.5);
        assert!((0.0..=1.0).contains(&consolidated));
    }

    #[test]
    fn cross_reference_limit_fails_without_mutating_state() {
        let mut matrix = ContentPlasticityMatrix::default();
        for index in 0..50 {
            matrix
                .add_cross_reference("fact-alpha", &format!("ref-{index}"))
                .unwrap();
        }
        assert!(matrix
            .add_cross_reference("fact-alpha", "ref-overflow")
            .is_err());
        assert_eq!(matrix.cross_reference_graph["fact-alpha"].len(), 50);
        assert!(!matrix.cross_reference_graph["fact-alpha"]
            .iter()
            .any(|value| value == "ref-overflow"));
    }
}
