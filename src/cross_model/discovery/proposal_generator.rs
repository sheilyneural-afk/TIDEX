//! Governance proposals derived from verified discovery evidence.
//!
//! A behavioral gap is not itself a transferable artifact. The first allowed
//! proposal stage is therefore evidence acquisition, never automatic steering.

use super::{CapabilityGap, Priority, PriorityScore};
use crate::cross_model::models::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStage {
    AcquireInternalEvidence,
    CalibrateCrossModelAlignment,
    CompileWeightDelta,
    EvaluateShadow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResourceRequirements {
    pub maximum_compute_seconds: u64,
    pub maximum_memory_bytes: u64,
    pub maximum_storage_bytes: u64,
    pub maximum_parallelism: usize,
}

impl ResourceRequirements {
    fn validate(&self) -> Result<(), String> {
        if self.maximum_compute_seconds == 0
            || self.maximum_memory_bytes == 0
            || self.maximum_storage_bytes == 0
            || self.maximum_parallelism == 0
        {
            return Err("proposal_resource_budget_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransferProposal {
    pub schema: String,
    pub proposal_id: String,
    pub capability_name: String,
    pub source_model: String,
    pub target_model: String,
    pub stage: ProposalStage,
    pub priority: Priority,
    pub priority_score: f64,
    pub gap_evidence_sha256: String,
    pub priority_evidence_sha256: String,
    pub required_evidence: Vec<String>,
    pub resource_budget: ResourceRequirements,
    pub evidence_sha256: String,
}

impl TransferProposal {
    pub fn validate(&self) -> Result<(), String> {
        self.resource_budget.validate()?;
        if self.schema != "tidex.cross_model.transfer_proposal/v1"
            || self.proposal_id.trim().is_empty()
            || self.capability_name.trim().is_empty()
            || self.source_model.trim().is_empty()
            || self.target_model.trim().is_empty()
            || self.source_model == self.target_model
            || !self.priority_score.is_finite()
            || !(0.0..=1.0).contains(&self.priority_score)
            || self.required_evidence.is_empty()
            || self
                .required_evidence
                .iter()
                .any(|value| value.trim().is_empty())
        {
            return Err("transfer_proposal_invalid".into());
        }
        if proposal_digest(self)? != self.evidence_sha256 {
            return Err("transfer_proposal_digest_mismatch".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProposalGeneratorConfig {
    pub minimum_priority_score: f64,
    pub resource_budget: ResourceRequirements,
}

impl Default for ProposalGeneratorConfig {
    fn default() -> Self {
        Self {
            minimum_priority_score: 0.20,
            resource_budget: ResourceRequirements {
                maximum_compute_seconds: 3600,
                maximum_memory_bytes: 32 * 1024 * 1024 * 1024,
                maximum_storage_bytes: 64 * 1024 * 1024 * 1024,
                maximum_parallelism: 4,
            },
        }
    }
}

pub struct ProposalGenerator {
    config: ProposalGeneratorConfig,
}

impl ProposalGenerator {
    pub fn new(config: ProposalGeneratorConfig) -> Result<Self, String> {
        config.resource_budget.validate()?;
        if !config.minimum_priority_score.is_finite()
            || !(0.0..=1.0).contains(&config.minimum_priority_score)
        {
            return Err("proposal_generator_config_invalid".into());
        }
        Ok(Self { config })
    }

    pub fn generate_proposals(
        &self,
        gaps: &[CapabilityGap],
        priority_scores: &[PriorityScore],
    ) -> Result<Vec<TransferProposal>, String> {
        let by_name = gaps
            .iter()
            .map(|gap| (gap.capability_name.as_str(), gap))
            .collect::<BTreeMap<_, _>>();
        let mut seen = BTreeSet::new();
        let mut proposals = Vec::new();
        for score in priority_scores {
            if score.overall_score < self.config.minimum_priority_score {
                continue;
            }
            if !seen.insert(score.capability_name.clone()) {
                return Err("priority_score_duplicate".into());
            }
            let gap = by_name
                .get(score.capability_name.as_str())
                .ok_or("priority_gap_missing")?;
            gap.validate()?;
            if score.gap_evidence_sha256 != gap.evidence_sha256 {
                return Err("priority_gap_digest_mismatch".into());
            }
            let proposal_id = sha256_hex(
                format!(
                    "{}\0{}\0{}\0{}",
                    gap.capability_name, gap.source_model, gap.target_model, gap.evidence_sha256
                )
                .as_bytes(),
            );
            let mut proposal = TransferProposal {
                schema: "tidex.cross_model.transfer_proposal/v1".into(),
                proposal_id,
                capability_name: gap.capability_name.clone(),
                source_model: gap.source_model.clone(),
                target_model: gap.target_model.clone(),
                stage: ProposalStage::AcquireInternalEvidence,
                priority: score.priority,
                priority_score: score.overall_score,
                gap_evidence_sha256: gap.evidence_sha256.clone(),
                priority_evidence_sha256: score.evidence_sha256.clone(),
                required_evidence: vec![
                    "authenticated_internal_activation_pairs_or_weight_delta".into(),
                    "independent_receiver_preservation_baseline".into(),
                ],
                resource_budget: self.config.resource_budget.clone(),
                evidence_sha256: String::new(),
            };
            proposal.evidence_sha256 = proposal_digest(&proposal)?;
            proposal.validate()?;
            proposals.push(proposal);
        }
        proposals.sort_by(|a, b| {
            b.priority_score
                .total_cmp(&a.priority_score)
                .then_with(|| a.proposal_id.cmp(&b.proposal_id))
        });
        Ok(proposals)
    }

    pub fn generate_batch_proposal(
        &self,
        proposals: Vec<TransferProposal>,
    ) -> Result<BatchProposal, String> {
        if proposals.is_empty() {
            return Err("batch_proposal_empty".into());
        }
        for proposal in &proposals {
            proposal.validate()?;
        }
        let evidence_sha256 = sha256_hex(
            &serde_json::to_vec(&proposals).map_err(|e| format!("batch_proposal_serialize:{e}"))?,
        );
        Ok(BatchProposal {
            proposals,
            evidence_sha256,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BatchProposal {
    pub proposals: Vec<TransferProposal>,
    pub evidence_sha256: String,
}

fn proposal_digest(proposal: &TransferProposal) -> Result<String, String> {
    let mut unsigned = proposal.clone();
    unsigned.evidence_sha256.clear();
    serde_json::to_vec(&unsigned)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("transfer_proposal_serialize:{error}"))
}
impl Default for ProposalGenerator {
    fn default() -> Self {
        Self::new(ProposalGeneratorConfig::default()).expect("static proposal config")
    }
}
