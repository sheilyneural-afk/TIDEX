//! Deterministic readiness gates over measured evidence.
//!
//! These gates do not authorize production activation. They only decide whether
//! a cross-model candidate has enough evidence to be handed to CEREBRO3's core
//! promotion authority.

use super::{DomainFitnessResult, EvidenceType, ValidationResult};
use crate::cross_model::models::{sha256_hex, CapabilityMetadata};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateType {
    EvidenceCompleteness,
    BehavioralPerformance,
    Preservation,
    IndependentReplay,
    DomainFitness,
    CoreAuthorizationBoundary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PromotionGateResult {
    pub gate_type: GateType,
    pub passed: bool,
    pub measured_value: Option<f64>,
    pub threshold: Option<f64>,
    pub evidence_sha256: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PromotionGateConfig {
    pub minimum_behavioral_score: f64,
    pub minimum_preservation_score: f64,
    pub minimum_replay_score: f64,
    pub require_domain_fitness: bool,
}

impl Default for PromotionGateConfig {
    fn default() -> Self {
        Self {
            minimum_behavioral_score: 0.55,
            minimum_preservation_score: 0.98,
            minimum_replay_score: 0.98,
            require_domain_fitness: true,
        }
    }
}

impl PromotionGateConfig {
    fn validate(&self) -> Result<(), String> {
        for value in [
            self.minimum_behavioral_score,
            self.minimum_preservation_score,
            self.minimum_replay_score,
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err("promotion_gate_config_invalid".into());
            }
        }
        Ok(())
    }
}

pub struct PromotionGates {
    config: PromotionGateConfig,
}

impl PromotionGates {
    pub fn new(config: PromotionGateConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn run_all_gates(
        &self,
        capability: &CapabilityMetadata,
        validation: &ValidationResult,
        domain_fitness: Option<&DomainFitnessResult>,
    ) -> Result<BTreeMap<GateType, PromotionGateResult>, String> {
        self.config.validate()?;
        capability.validate()?;
        if validation.capability_name != capability.name {
            return Err("promotion_validation_capability_mismatch".into());
        }
        let evidence = |kind: EvidenceType| {
            validation
                .evidence_by_type
                .get(&kind)
                .is_some_and(|count| *count > 0)
        };
        let mut gates = BTreeMap::new();
        gates.insert(
            GateType::EvidenceCompleteness,
            gate(
                GateType::EvidenceCompleteness,
                validation.passed,
                Some(validation.minimum_observed_score),
                None,
                &validation.evidence_sha256,
                if validation.passed {
                    "validated_evidence_complete"
                } else {
                    "validated_evidence_incomplete"
                },
            ),
        );
        gates.insert(
            GateType::BehavioralPerformance,
            gate(
                GateType::BehavioralPerformance,
                evidence(EvidenceType::BehavioralPerformance)
                    && validation.minimum_observed_score >= self.config.minimum_behavioral_score,
                Some(validation.minimum_observed_score),
                Some(self.config.minimum_behavioral_score),
                &validation.evidence_sha256,
                "behavioral_evidence_threshold",
            ),
        );
        let preservation_score = minimum_score_for(validation, EvidenceType::Preservation);
        gates.insert(
            GateType::Preservation,
            gate(
                GateType::Preservation,
                evidence(EvidenceType::Preservation)
                    && preservation_score
                        .is_some_and(|score| score >= self.config.minimum_preservation_score),
                preservation_score,
                Some(self.config.minimum_preservation_score),
                &validation.evidence_sha256,
                "preservation_evidence_threshold",
            ),
        );
        let replay_score = minimum_score_for(validation, EvidenceType::IndependentReplay);
        gates.insert(
            GateType::IndependentReplay,
            gate(
                GateType::IndependentReplay,
                evidence(EvidenceType::IndependentReplay)
                    && replay_score.is_some_and(|score| score >= self.config.minimum_replay_score),
                replay_score,
                Some(self.config.minimum_replay_score),
                &validation.evidence_sha256,
                "independent_replay_threshold",
            ),
        );
        let (domain_passed, domain_value, domain_source, domain_reason) = match domain_fitness {
            Some(result) => (
                result.passed,
                Some(result.delta),
                result.evidence_sha256.as_str(),
                "measured_domain_fitness",
            ),
            None if self.config.require_domain_fitness => (
                false,
                None,
                capability.evidence_sha256.as_str(),
                "domain_fitness_missing_required",
            ),
            None => (
                true,
                None,
                capability.evidence_sha256.as_str(),
                "domain_fitness_not_required_by_policy",
            ),
        };
        gates.insert(
            GateType::DomainFitness,
            gate(
                GateType::DomainFitness,
                domain_passed,
                domain_value,
                None,
                domain_source,
                domain_reason,
            ),
        );
        // Deliberately false: this layer is not the production authority.
        gates.insert(
            GateType::CoreAuthorizationBoundary,
            gate(
                GateType::CoreAuthorizationBoundary,
                false,
                None,
                None,
                &capability.evidence_sha256,
                "requires_universal_promotion_gate_and_adapter_bank_authorization",
            ),
        );
        Ok(gates)
    }

    pub fn readiness_passed(&self, results: &BTreeMap<GateType, PromotionGateResult>) -> bool {
        results
            .iter()
            .all(|(kind, result)| *kind == GateType::CoreAuthorizationBoundary || result.passed)
    }
}

fn minimum_score_for(validation: &ValidationResult, kind: EvidenceType) -> Option<f64> {
    // ValidationResult intentionally stores counts, not raw evidence. Its global
    // minimum is therefore a conservative lower bound for each present type.
    validation
        .evidence_by_type
        .contains_key(&kind)
        .then_some(validation.minimum_observed_score)
}

fn gate(
    gate_type: GateType,
    passed: bool,
    measured_value: Option<f64>,
    threshold: Option<f64>,
    source_sha256: &str,
    reason: &str,
) -> PromotionGateResult {
    let evidence_sha256 = sha256_hex(
        format!(
            "{gate_type:?}\0{passed}\0{measured_value:?}\0{threshold:?}\0{source_sha256}\0{reason}"
        )
        .as_bytes(),
    );
    PromotionGateResult {
        gate_type,
        passed,
        measured_value,
        threshold,
        evidence_sha256,
        reason: reason.into(),
    }
}
impl Default for PromotionGates {
    fn default() -> Self {
        Self::new(PromotionGateConfig::default()).expect("static promotion gate config")
    }
}
