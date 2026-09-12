//! Cross-model readiness reducer.
//!
//! This module never activates an adapter or claims production promotion.
//! A successful result means only that the candidate is ready to be submitted
//! to the core universal promotion and adapter-bank authorities.

use super::{
    DomainFitnessResult, EvidenceItem, EvidenceValidator, EvidenceValidatorConfig, GateType,
    PromotionGateConfig, PromotionGateResult, PromotionGates, ValidationResult,
};
use crate::cross_model::models::{sha256_hex, CapabilityMetadata};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionStage {
    EvidenceInsufficient,
    ReadyForCoreAuthorization,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PromotionResult {
    pub schema: String,
    pub capability_name: String,
    pub promotion_stage: PromotionStage,
    pub ready_for_core_authorization: bool,
    pub production_activated: bool,
    pub validation: ValidationResult,
    pub gate_results: BTreeMap<GateType, PromotionGateResult>,
    pub domain_fitness: Option<DomainFitnessResult>,
    pub evidence_sha256: String,
}

impl PromotionResult {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "tidex.cross_model.promotion_readiness/v1"
            || self.capability_name != self.validation.capability_name
            || self.production_activated
            || (self.ready_for_core_authorization
                != (self.promotion_stage == PromotionStage::ReadyForCoreAuthorization))
        {
            return Err("promotion_readiness_result_invalid".into());
        }
        if promotion_digest(self)? != self.evidence_sha256 {
            return Err("promotion_readiness_digest_mismatch".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct PromoterConfig {
    pub gate_config: PromotionGateConfig,
    pub evidence_config: EvidenceValidatorConfig,
}

pub struct Promoter {
    gates: PromotionGates,
    evidence_validator: EvidenceValidator,
    promotion_history: HashMap<String, Vec<PromotionResult>>,
}

impl Promoter {
    pub fn new(config: PromoterConfig) -> Result<Self, String> {
        Ok(Self {
            gates: PromotionGates::new(config.gate_config)?,
            evidence_validator: EvidenceValidator::new(config.evidence_config)?,
            promotion_history: HashMap::new(),
        })
    }

    pub fn add_evidence(
        &mut self,
        capability_name: &str,
        evidence: EvidenceItem,
    ) -> Result<(), String> {
        self.evidence_validator
            .add_evidence(capability_name, evidence)
    }

    pub fn evaluate(
        &mut self,
        capability: &CapabilityMetadata,
        domain_fitness: Option<DomainFitnessResult>,
    ) -> Result<PromotionResult, String> {
        capability.validate()?;
        let validation = self.evidence_validator.validate(capability)?;
        let gate_results =
            self.gates
                .run_all_gates(capability, &validation, domain_fitness.as_ref())?;
        let ready = self.gates.readiness_passed(&gate_results);
        let stage = if ready {
            PromotionStage::ReadyForCoreAuthorization
        } else if validation.passed {
            PromotionStage::Rejected
        } else {
            PromotionStage::EvidenceInsufficient
        };
        let mut result = PromotionResult {
            schema: "tidex.cross_model.promotion_readiness/v1".into(),
            capability_name: capability.name.clone(),
            promotion_stage: stage,
            ready_for_core_authorization: ready,
            production_activated: false,
            validation,
            gate_results,
            domain_fitness,
            evidence_sha256: String::new(),
        };
        result.evidence_sha256 = promotion_digest(&result)?;
        result.validate()?;
        self.promotion_history
            .entry(capability.name.clone())
            .or_default()
            .push(result.clone());
        Ok(result)
    }

    pub fn batch_evaluate(
        &mut self,
        capabilities: &[CapabilityMetadata],
    ) -> Result<Vec<PromotionResult>, String> {
        capabilities
            .iter()
            .map(|capability| self.evaluate(capability, None))
            .collect()
    }

    pub fn get_promotion_history(&self, capability_name: &str) -> Vec<&PromotionResult> {
        self.promotion_history
            .get(capability_name)
            .map(|rows| rows.iter().collect())
            .unwrap_or_default()
    }

    pub fn get_ready_capabilities(&self) -> Vec<String> {
        let mut values = self
            .promotion_history
            .iter()
            .filter_map(|(name, history)| {
                history
                    .last()
                    .filter(|result| result.ready_for_core_authorization)
                    .map(|_| name.clone())
            })
            .collect::<Vec<_>>();
        values.sort();
        values
    }

    pub fn get_promoted_capabilities(&self) -> Vec<String> {
        Vec::new()
    }

    pub fn reject(&mut self, capability_name: &str, reason: String) -> Result<(), String> {
        if reason.trim().is_empty() {
            return Err("promotion_rejection_reason_invalid".into());
        }
        let last = self
            .promotion_history
            .get_mut(capability_name)
            .and_then(|history| history.last_mut())
            .ok_or("promotion_history_missing")?;
        last.promotion_stage = PromotionStage::Rejected;
        last.ready_for_core_authorization = false;
        last.production_activated = false;
        last.evidence_sha256 = promotion_digest(last)?;
        Ok(())
    }

    pub fn get_statistics(&self) -> PromotionStatistics {
        let evaluated = self.promotion_history.values().map(Vec::len).sum();
        let ready = self
            .promotion_history
            .values()
            .flatten()
            .filter(|row| row.ready_for_core_authorization)
            .count();
        let rejected = self
            .promotion_history
            .values()
            .flatten()
            .filter(|row| row.promotion_stage == PromotionStage::Rejected)
            .count();
        PromotionStatistics {
            evaluated,
            ready_for_core_authorization: ready,
            rejected,
            production_activated: 0,
        }
    }

    pub fn clear_history(&mut self) {
        self.promotion_history.clear();
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PromotionStatistics {
    pub evaluated: usize,
    pub ready_for_core_authorization: usize,
    pub rejected: usize,
    pub production_activated: usize,
}

fn promotion_digest(result: &PromotionResult) -> Result<String, String> {
    let mut unsigned = result.clone();
    unsigned.evidence_sha256.clear();
    serde_json::to_vec(&unsigned)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("promotion_readiness_serialize:{error}"))
}
impl Default for Promoter {
    fn default() -> Self {
        Self::new(PromoterConfig::default()).expect("static promoter config")
    }
}
