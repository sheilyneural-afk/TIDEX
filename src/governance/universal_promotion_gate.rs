//! Final evidence gate. A passing result is readiness for a separate promotion
//! authority; this module has no activation or model-writing capability.
//! Self-consistent input hashes are not independent attestations of execution.

use crate::foundation::digest::Sha256Digest;
use crate::foundation::error::{BrainError, BrainResult};
use crate::materialization::materialization_selector::{
    BackendSelectionInput, BackendSelectionReceipt,
};
use crate::materialization::shadow_evaluation::ShadowEvaluationReceipt;
use crate::materialization::universality_evidence::{
    UniversalityEvidenceInput, UniversalityEvidenceReceipt,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UniversalPromotionPolicy {
    pub schema: String,
    pub minimum_universality_n: usize,
    pub minimum_global_wilson_lower_bound: f64,
    pub require_all_selected_candidates_evaluated: bool,
}

impl UniversalPromotionPolicy {
    fn validate(&self) -> BrainResult<()> {
        if self.schema != "tidex.universal_promotion_policy/v1"
            || self.minimum_universality_n == 0
            || !self.minimum_global_wilson_lower_bound.is_finite()
            || !(0.0..=1.0).contains(&self.minimum_global_wilson_lower_bound)
            || !self.require_all_selected_candidates_evaluated
        {
            return Err(BrainError::Invalid("universal_promotion_policy_invalid".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum UniversalPromotionBlocker {
    UniversalityNInsufficient,
    GlobalConfidenceInsufficient,
    SelectedCandidateMissingEvaluation,
    SelectionCandidateMissingEvaluation,
    HybridRequiresJointEvaluation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UniversalPromotionReadiness {
    ReadyForSeparatePromotionAuthority,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UniversalPromotionGateRequest {
    pub schema: String,
    pub selection_input: BackendSelectionInput,
    pub selection_receipt: BackendSelectionReceipt,
    pub universality_input: UniversalityEvidenceInput,
    pub universality_receipt: UniversalityEvidenceReceipt,
    pub shadow_evaluations: Vec<ShadowEvaluationReceipt>,
    pub policy: UniversalPromotionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UniversalPromotionGateReceipt {
    pub schema: String,
    pub selection_sha256: Sha256Digest,
    pub universality_sha256: Sha256Digest,
    pub shadow_evaluations_sha256: Sha256Digest,
    pub policy_sha256: Sha256Digest,
    pub readiness: UniversalPromotionReadiness,
    pub blockers: BTreeSet<UniversalPromotionBlocker>,
    pub authorizes_activation: bool,
    pub manifest_sha256: Sha256Digest,
}

pub fn evaluate_universal_promotion_gate(
    request: &UniversalPromotionGateRequest,
) -> BrainResult<UniversalPromotionGateReceipt> {
    if request.schema != "tidex.universal_promotion_gate_request/v1" {
        return Err(BrainError::Invalid("universal_promotion_gate_request_invalid".into()));
    }
    request.policy.validate()?;
    request
        .selection_receipt
        .validate_against(&request.selection_input)?;
    request
        .universality_receipt
        .validate_against(&request.universality_input)?;
    if request.shadow_evaluations.len() > 1_024 {
        return Err(BrainError::Invalid("shadow_evaluation_set_limit".into()));
    }
    let mut evaluated = BTreeSet::new();
    for receipt in &request.shadow_evaluations {
        receipt.validate_integrity()?;
        let supplied = request
            .selection_input
            .evaluations
            .iter()
            .find(|e| e.candidate_sha256 == receipt.evaluation.candidate_sha256)
            .ok_or_else(|| BrainError::Integrity("shadow_evaluation_not_in_selection".into()))?;
        if supplied != &receipt.evaluation {
            return Err(BrainError::Integrity("selection_shadow_evaluation_mismatch".into()));
        }
        if !evaluated.insert(&receipt.evaluation.candidate_sha256) {
            return Err(BrainError::Invalid("shadow_evaluation_candidate_duplicate".into()));
        }
    }
    let mut blockers = BTreeSet::new();
    if request
        .selection_input
        .evaluations
        .iter()
        .any(|evaluation| !evaluated.contains(&evaluation.candidate_sha256))
    {
        blockers.insert(UniversalPromotionBlocker::SelectionCandidateMissingEvaluation);
    }
    // Independent receipts for two constituents cannot attest to interference,
    // aggregate resource use, or behavior of their joint runtime composition.
    // The current selection wire has no independently materialized hybrid
    // candidate identity, therefore hybrid readiness must fail closed.
    if request.selection_receipt.selected_strategy
        == crate::receiver::receiver_profile::MaterializationStrategy::Hybrid
    {
        blockers.insert(UniversalPromotionBlocker::HybridRequiresJointEvaluation);
    }
    if request.universality_receipt.universality_n < request.policy.minimum_universality_n {
        blockers.insert(UniversalPromotionBlocker::UniversalityNInsufficient);
    }
    if request.universality_receipt.global_wilson_lower_bound
        < request.policy.minimum_global_wilson_lower_bound
    {
        blockers.insert(UniversalPromotionBlocker::GlobalConfidenceInsufficient);
    }
    if request
        .selection_receipt
        .selected_candidates
        .iter()
        .any(|candidate| !evaluated.contains(candidate))
    {
        blockers.insert(UniversalPromotionBlocker::SelectedCandidateMissingEvaluation);
    }
    let readiness = if blockers.is_empty() {
        UniversalPromotionReadiness::ReadyForSeparatePromotionAuthority
    } else {
        UniversalPromotionReadiness::Rejected
    };
    let selection_sha256 = request.selection_receipt.manifest_sha256.clone();
    let universality_sha256 = request.universality_receipt.manifest_sha256.clone();
    let shadow_evaluations_sha256 = Sha256Digest::digest_domain(
        b"TIDEX:SHADOW-EVALUATION-SET:v1\0",
        &serde_json::to_vec(&request.shadow_evaluations)?,
    );
    let policy_sha256 = Sha256Digest::digest_domain(
        b"TIDEX:UNIVERSAL-PROMOTION-POLICY:v1\0",
        &serde_json::to_vec(&request.policy)?,
    );
    let mut receipt = UniversalPromotionGateReceipt {
        schema: "tidex.universal_promotion_gate_receipt/v2".into(),
        selection_sha256,
        universality_sha256,
        shadow_evaluations_sha256,
        policy_sha256,
        readiness,
        blockers,
        authorizes_activation: false,
        manifest_sha256: Sha256Digest::zero(),
    };
    let mut unsigned = receipt.clone();
    unsigned.manifest_sha256 = Sha256Digest::zero();
    receipt.manifest_sha256 = Sha256Digest::digest_domain(
        b"TIDEX:UNIVERSAL-PROMOTION-GATE-RECEIPT:v2\0",
        &serde_json::to_vec(&unsigned)?,
    );
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materialization::materialization_selector::{
        BackendEvaluation, BackendSelectionPolicy, PairwiseComplementarity,
    };
    use crate::materialization::universality_evidence::{UniversalityProtocol, UniversalityTrial};
    use crate::receiver::receiver_profile::MaterializationStrategy;

    fn fixture_request() -> UniversalPromotionGateRequest {
        let candidate = Sha256Digest::digest_bytes(b"candidate");
        let controls = BackendSelectionPolicy::rigorous_default(100, 100).required_controls;
        let selection_input = BackendSelectionInput {
            schema: "tidex.backend_selection_input/v1".into(),
            evaluations: vec![BackendEvaluation {
                schema: "tidex.backend_evaluation/v1".into(),
                candidate_sha256: candidate,
                strategy: MaterializationStrategy::SparseDelta,
                functional_score: 1.0,
                functional_ci_lower: 1.0,
                preservation_score: 1.0,
                identity_margin: 1.0,
                numerical_stability: 1.0,
                normalized_risk: 0.0,
                latency_micros: 1,
                resident_bytes: 1,
                completed_controls: controls,
            }],
            complementarity: Vec::<PairwiseComplementarity>::new(),
            policy: BackendSelectionPolicy::rigorous_default(100, 100),
        };
        let selection_receipt = selection_input.execute().unwrap();
        let universality_input = UniversalityEvidenceInput {
            schema: "tidex.universality_evidence_input/v1".into(),
            calibration_capabilities: BTreeSet::from(["calibration:v1".into()]),
            trials: vec![
                UniversalityTrial {
                    schema: "tidex.universality_trial/v1".into(),
                    trial_id: "calibration-trial".into(),
                    capability_id: "calibration:v1".into(),
                    task_family_id: "calibration_family".into(),
                    receiver_id: "receiver.seen".into(),
                    receiver_family_id: "family.seen".into(),
                    seed: 0,
                    capability_was_calibration: true,
                    receiver_was_calibration: true,
                    target_optimizer_steps: 0,
                    target_score: 1.0,
                    preservation_score: 1.0,
                    wrong_ir_score: 0.0,
                    random_delta_score: 0.0,
                    unmodified_receiver_score: 0.0,
                },
                UniversalityTrial {
                    schema: "tidex.universality_trial/v1".into(),
                    trial_id: "held-out-trial".into(),
                    capability_id: "held-out:v1".into(),
                    task_family_id: "state_identity".into(),
                    receiver_id: "receiver.new".into(),
                    receiver_family_id: "family.new".into(),
                    seed: 1,
                    capability_was_calibration: false,
                    receiver_was_calibration: false,
                    target_optimizer_steps: 0,
                    target_score: 1.0,
                    preservation_score: 1.0,
                    wrong_ir_score: 0.0,
                    random_delta_score: 0.0,
                    unmodified_receiver_score: 0.0,
                },
            ],
            protocol: UniversalityProtocol {
                schema: "tidex.universality_protocol/v1".into(),
                minimum_calibration_capabilities: 1,
                minimum_held_out_capabilities: 1,
                minimum_receivers_per_capability: 1,
                minimum_receiver_families_per_capability: 1,
                minimum_seeds_per_capability: 1,
                minimum_task_families_per_capability: 1,
                scope: "held_out_capability_generalization".into(),
                require_unseen_receiver: true,
                minimum_target_score: 0.9,
                minimum_preservation_score: 0.9,
                minimum_identity_margin: 0.5,
                minimum_success_probability: 0.1,
                confidence_z: 1.96,
            },
        };
        let universality_receipt = universality_input.execute().unwrap();
        UniversalPromotionGateRequest {
            schema: "tidex.universal_promotion_gate_request/v1".into(),
            selection_input,
            selection_receipt,
            universality_input,
            universality_receipt,
            shadow_evaluations: vec![],
            policy: UniversalPromotionPolicy {
                schema: "tidex.universal_promotion_policy/v1".into(),
                minimum_universality_n: 1,
                minimum_global_wilson_lower_bound: 0.0,
                require_all_selected_candidates_evaluated: true,
            },
        }
    }

    #[test]
    fn passing_statistics_cannot_bypass_missing_shadow_evaluation_or_activate() {
        let receipt = evaluate_universal_promotion_gate(&fixture_request()).unwrap();
        assert_eq!(receipt.readiness, UniversalPromotionReadiness::Rejected);
        assert!(receipt
            .blockers
            .contains(&UniversalPromotionBlocker::SelectedCandidateMissingEvaluation));
        assert!(!receipt.authorizes_activation);
    }
    #[test]
    fn convergence_rejects_selection_metrics_substituted_for_execution_metrics() {
        let mut request = fixture_request();
        let mut evaluation = request.selection_input.evaluations[0].clone();
        evaluation.functional_score = 0.0;
        evaluation.functional_ci_lower = 0.0;
        let mut witness = ShadowEvaluationReceipt {
            schema: "tidex.shadow_evaluation_receipt/v1".into(),
            runner_sha256: Sha256Digest::digest_bytes(b"test-runner"),
            bundle_sha256: Sha256Digest::digest_bytes(b"test-bundle"),
            isolated_request_sha256: Sha256Digest::digest_bytes(b"test-request"),
            evaluation,
            manifest_sha256: Sha256Digest::zero(),
        };
        witness.manifest_sha256 = Sha256Digest::digest_domain(
            b"TIDEX:SHADOW-EVALUATION-RECEIPT:v1\0",
            &serde_json::to_vec(&witness).unwrap(),
        );
        witness.validate_integrity().unwrap();
        request.shadow_evaluations.push(witness);
        assert!(evaluate_universal_promotion_gate(&request).is_err());
    }

    fn shadow_receipt(evaluation: BackendEvaluation) -> ShadowEvaluationReceipt {
        let mut receipt = ShadowEvaluationReceipt {
            schema: "tidex.shadow_evaluation_receipt/v1".into(),
            runner_sha256: Sha256Digest::digest_bytes(b"runner"),
            bundle_sha256: Sha256Digest::digest_bytes(b"bundle"),
            isolated_request_sha256: Sha256Digest::digest_bytes(b"request"),
            evaluation,
            manifest_sha256: Sha256Digest::zero(),
        };
        let mut unsigned = receipt.clone();
        unsigned.manifest_sha256 = Sha256Digest::zero();
        receipt.manifest_sha256 = Sha256Digest::digest_domain(
            b"TIDEX:SHADOW-EVALUATION-RECEIPT:v1\0",
            &serde_json::to_vec(&unsigned).unwrap(),
        );
        receipt
    }

    #[test]
    fn selection_candidate_without_shadow_evaluation_blocks_readiness() {
        let mut request = fixture_request();
        let mut second = request.selection_input.evaluations[0].clone();
        second.candidate_sha256 = Sha256Digest::digest_bytes(b"candidate-second");
        second.strategy = MaterializationStrategy::DenseDelta;
        second.functional_score = 0.95;
        second.functional_ci_lower = 0.95;
        request.selection_input.evaluations.push(second);
        request.selection_receipt = request.selection_input.execute().unwrap();
        let selected = request
            .selection_input
            .evaluations
            .iter()
            .find(|evaluation| {
                request
                    .selection_receipt
                    .selected_candidates
                    .contains(&evaluation.candidate_sha256)
            })
            .unwrap()
            .clone();
        request.shadow_evaluations = vec![shadow_receipt(selected)];
        let receipt = evaluate_universal_promotion_gate(&request).unwrap();
        assert_eq!(receipt.readiness, UniversalPromotionReadiness::Rejected);
        assert!(receipt
            .blockers
            .contains(&UniversalPromotionBlocker::SelectionCandidateMissingEvaluation));
        assert!(!receipt
            .blockers
            .contains(&UniversalPromotionBlocker::SelectedCandidateMissingEvaluation));
        assert!(!receipt.authorizes_activation);
    }

    #[test]
    fn hybrid_selection_requires_joint_evaluation() {
        let mut request = fixture_request();
        let first = request.selection_input.evaluations[0].clone();
        let mut second = first.clone();
        second.candidate_sha256 = Sha256Digest::digest_bytes(b"hybrid-second");
        second.strategy = MaterializationStrategy::DenseDelta;
        second.functional_score = 0.99;
        second.functional_ci_lower = 0.99;
        request.selection_input.evaluations.push(second.clone());
        request.selection_input.complementarity = vec![PairwiseComplementarity {
            first_candidate_sha256: first.candidate_sha256.clone(),
            second_candidate_sha256: second.candidate_sha256.clone(),
            held_out_gain: 0.2,
            preservation_delta: 0.0,
        }];
        request.selection_receipt = request.selection_input.execute().unwrap();
        assert_eq!(request.selection_receipt.selected_strategy, MaterializationStrategy::Hybrid);
        request.shadow_evaluations = request
            .selection_input
            .evaluations
            .iter()
            .cloned()
            .map(shadow_receipt)
            .collect();
        let receipt = evaluate_universal_promotion_gate(&request).unwrap();
        assert_eq!(receipt.readiness, UniversalPromotionReadiness::Rejected);
        assert!(receipt
            .blockers
            .contains(&UniversalPromotionBlocker::HybridRequiresJointEvaluation));
        assert!(!receipt.authorizes_activation);
    }
}
