//! Hash-bound promotion evidence validation.

use crate::cross_model::models::{sha256_hex, CapabilityMetadata};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    BehavioralPerformance,
    Counterfactual,
    InternalActivation,
    AlignmentValidation,
    WeightDelta,
    ShadowExecution,
    Preservation,
    IndependentReplay,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceItem {
    pub evidence_type: EvidenceType,
    pub source: String,
    pub source_sha256: String,
    pub score: f64,
    pub sample_size: usize,
    pub independence_group: String,
    pub timestamp: String,
}

impl EvidenceItem {
    pub fn validate(&self) -> Result<(), String> {
        if self.source.trim().is_empty()
            || self.source_sha256.len() != 64
            || !self.source_sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || !self.score.is_finite()
            || !(0.0..=1.0).contains(&self.score)
            || self.sample_size == 0
            || self.independence_group.trim().is_empty()
            || chrono::DateTime::parse_from_rfc3339(&self.timestamp).is_err()
        {
            return Err("promotion_evidence_item_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceValidatorConfig {
    pub minimum_items: usize,
    pub minimum_independent_groups: usize,
    pub minimum_score: f64,
    pub required_evidence_types: Vec<EvidenceType>,
}

impl Default for EvidenceValidatorConfig {
    fn default() -> Self {
        Self {
            minimum_items: 4,
            minimum_independent_groups: 2,
            minimum_score: 0.5,
            required_evidence_types: vec![
                EvidenceType::BehavioralPerformance,
                EvidenceType::ShadowExecution,
                EvidenceType::Preservation,
                EvidenceType::IndependentReplay,
            ],
        }
    }
}

impl EvidenceValidatorConfig {
    fn validate(&self) -> Result<(), String> {
        if self.minimum_items == 0
            || self.minimum_independent_groups == 0
            || !self.minimum_score.is_finite()
            || !(0.0..=1.0).contains(&self.minimum_score)
            || self.required_evidence_types.is_empty()
        {
            return Err("evidence_validator_config_invalid".into());
        }
        let unique = self.required_evidence_types.iter().collect::<BTreeSet<_>>();
        if unique.len() != self.required_evidence_types.len() {
            return Err("evidence_validator_required_type_duplicate".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ValidationResult {
    pub capability_name: String,
    pub passed: bool,
    pub total_evidence_count: usize,
    pub independent_group_count: usize,
    pub minimum_observed_score: f64,
    pub evidence_by_type: BTreeMap<EvidenceType, usize>,
    pub validation_issues: Vec<String>,
    pub evidence_sha256: String,
}

pub struct EvidenceValidator {
    config: EvidenceValidatorConfig,
    evidence_store: HashMap<String, Vec<EvidenceItem>>,
}

impl EvidenceValidator {
    pub fn new(config: EvidenceValidatorConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            evidence_store: HashMap::new(),
        })
    }

    pub fn add_evidence(
        &mut self,
        capability_name: &str,
        evidence: EvidenceItem,
    ) -> Result<(), String> {
        if capability_name.trim().is_empty() {
            return Err("evidence_capability_name_invalid".into());
        }
        evidence.validate()?;
        let items = self
            .evidence_store
            .entry(capability_name.into())
            .or_default();
        if items
            .iter()
            .any(|item| item.source_sha256 == evidence.source_sha256)
        {
            return Err("promotion_evidence_duplicate".into());
        }
        items.push(evidence);
        Ok(())
    }

    pub fn validate(&self, capability: &CapabilityMetadata) -> Result<ValidationResult, String> {
        capability.validate()?;
        let items = self
            .evidence_store
            .get(&capability.name)
            .cloned()
            .ok_or("promotion_evidence_missing")?;
        if items.is_empty() {
            return Err("promotion_evidence_empty".into());
        }
        let mut issues = Vec::new();
        for item in &items {
            item.validate()?;
        }
        let groups = items
            .iter()
            .map(|item| item.independence_group.as_str())
            .collect::<BTreeSet<_>>();
        let mut by_type = BTreeMap::new();
        for item in &items {
            *by_type.entry(item.evidence_type).or_insert(0usize) += 1;
        }
        let minimum_score = items
            .iter()
            .map(|item| item.score)
            .reduce(f64::min)
            .ok_or("promotion_evidence_empty")?;
        if items.len() < self.config.minimum_items {
            issues.push("insufficient_evidence_items".into());
        }
        if groups.len() < self.config.minimum_independent_groups {
            issues.push("insufficient_independent_groups".into());
        }
        if minimum_score < self.config.minimum_score {
            issues.push("minimum_evidence_score_below_policy".into());
        }
        for kind in &self.config.required_evidence_types {
            if !by_type.contains_key(kind) {
                issues.push(format!("missing_required_evidence:{kind:?}"));
            }
        }
        let passed = issues.is_empty();
        let evidence_sha256 = sha256_hex(
            &serde_json::to_vec(&(capability, &items, &issues))
                .map_err(|e| format!("validation_serialize:{e}"))?,
        );
        Ok(ValidationResult {
            capability_name: capability.name.clone(),
            passed,
            total_evidence_count: items.len(),
            independent_group_count: groups.len(),
            minimum_observed_score: minimum_score,
            evidence_by_type: by_type,
            validation_issues: issues,
            evidence_sha256,
        })
    }

    pub fn get_evidence(&self, capability_name: &str) -> Vec<&EvidenceItem> {
        self.evidence_store
            .get(capability_name)
            .map(|items| items.iter().collect())
            .unwrap_or_default()
    }

    pub fn get_evidence_by_type(
        &self,
        capability_name: &str,
        kind: EvidenceType,
    ) -> Vec<&EvidenceItem> {
        self.get_evidence(capability_name)
            .into_iter()
            .filter(|item| item.evidence_type == kind)
            .collect()
    }

    pub fn clear_evidence(&mut self, capability_name: &str) {
        self.evidence_store.remove(capability_name);
    }
    pub fn clear_all(&mut self) {
        self.evidence_store.clear();
    }
}
impl Default for EvidenceValidator {
    fn default() -> Self {
        Self::new(EvidenceValidatorConfig::default()).expect("static evidence config")
    }
}
