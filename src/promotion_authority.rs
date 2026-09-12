//! Level-3 PromotionAuthority: UPG readiness → durable executor catalog install.
//!
//! [`crate::governance::universal_promotion_gate`] only emits readiness. This
//! top-level module is the separate authority that may install a verified
//! [`ExecutorDescriptor`] into the on-disk operational catalog merged by
//! [`crate::operator::executor_registry::executor_catalog_at`].
//!
//! Fail-closed: Ready UPG (revalidated), never activation, no production /
//! lifecycle authority on the descriptor, module_path must already exist in the
//! builtin catalog, no builtin id digest collisions.

use crate::foundation::authority::PrivateFileReference;
use crate::foundation::digest::Sha256Digest;
use crate::foundation::error::{BrainError, BrainResult};
use crate::governance::universal_promotion_gate::{
    evaluate_universal_promotion_gate, UniversalPromotionGateReceipt,
    UniversalPromotionGateRequest, UniversalPromotionReadiness,
};
use crate::operator::executor_registry::{
    builtin_executor_catalog, executor_by_id_in, executor_id_for_direct_operation, ExecutorDescriptor,
};
use crate::operator::promoted_executor_catalog::{
    persist_promoted_executor_record, record_digest, validate_operation_token,
    PromotedExecutorRecord, PROMOTED_RECORD_SCHEMA,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

const INSTALL_REQUEST_SCHEMA: &str = "tidex.promotion_authority_install_request/v1";
const INSTALL_RECEIPT_SCHEMA: &str = "tidex.promotion_authority_install_receipt/v1";
const RECEIPT_DOMAIN: &[u8] = b"TIDEX:PROMOTION-AUTHORITY-INSTALL-RECEIPT:v1\0";

fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}

fn integrity(code: &str) -> BrainError {
    BrainError::Integrity(code.into())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PromotionAuthorityInstallRequest {
    pub schema: String,
    pub gate_request: UniversalPromotionGateRequest,
    pub gate_receipt: UniversalPromotionGateReceipt,
    pub descriptor: ExecutorDescriptor,
    pub operation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PromotionAuthorityInstallReceipt {
    pub schema: String,
    pub gate_receipt_sha256: Sha256Digest,
    pub executor_id: String,
    pub descriptor_sha256: Sha256Digest,
    pub operation: Option<String>,
    pub record: PrivateFileReference,
    pub idempotent: bool,
    pub authorizes_activation: bool,
    pub manifest_sha256: Sha256Digest,
}

fn receipt_digest(receipt: &PromotionAuthorityInstallReceipt) -> BrainResult<Sha256Digest> {
    let mut unsigned = receipt.clone();
    unsigned.manifest_sha256 = Sha256Digest::zero();
    Ok(Sha256Digest::digest_domain(
        RECEIPT_DOMAIN,
        &serde_json::to_vec(&unsigned)?,
    ))
}

fn verify_gate_binding(
    request: &UniversalPromotionGateRequest,
    claimed: &UniversalPromotionGateReceipt,
) -> BrainResult<UniversalPromotionGateReceipt> {
    let recomputed = evaluate_universal_promotion_gate(request)?;
    if &recomputed != claimed {
        return Err(integrity("promotion_authority_gate_receipt_mismatch"));
    }
    if recomputed.readiness != UniversalPromotionReadiness::ReadyForSeparatePromotionAuthority {
        return Err(invalid("promotion_authority_gate_not_ready"));
    }
    if recomputed.authorizes_activation {
        return Err(integrity("promotion_authority_upg_must_not_authorize_activation"));
    }
    if !recomputed.blockers.is_empty() {
        return Err(integrity("promotion_authority_ready_with_blockers"));
    }
    Ok(recomputed)
}

fn verify_descriptor_for_install(descriptor: &ExecutorDescriptor) -> BrainResult<()> {
    descriptor.validate()?;
    if descriptor.production_authority {
        return Err(invalid("promotion_authority_rejects_production_authority"));
    }
    if descriptor.authority
        == crate::operator::executor_registry::ExecutorAuthorityClass::LifecycleAuthority
    {
        return Err(invalid("promotion_authority_rejects_lifecycle_authority"));
    }
    let builtin = builtin_executor_catalog()?;
    if !builtin
        .iter()
        .any(|entry| entry.module_path == descriptor.module_path)
    {
        return Err(invalid("promotion_authority_module_path_not_in_builtin_catalog"));
    }
    if let Some(existing) = builtin
        .iter()
        .find(|entry| entry.executor_id == descriptor.executor_id)
    {
        if existing.descriptor_sha256 != descriptor.descriptor_sha256 {
            return Err(integrity("promotion_authority_builtin_id_collision"));
        }
    }
    Ok(())
}

/// Install a verified executor descriptor into the durable operational catalog.
pub fn install_promoted_executor(
    tidex_home: &Path,
    request: &PromotionAuthorityInstallRequest,
) -> BrainResult<PromotionAuthorityInstallReceipt> {
    if request.schema != INSTALL_REQUEST_SCHEMA {
        return Err(invalid("promotion_authority_install_request_invalid"));
    }
    let gate = verify_gate_binding(&request.gate_request, &request.gate_receipt)?;
    verify_descriptor_for_install(&request.descriptor)?;
    if let Some(op) = &request.operation {
        validate_operation_token(op)?;
        if executor_id_for_direct_operation(op).is_some() {
            return Err(invalid("promotion_authority_operation_shadows_builtin"));
        }
    }

    let mut record = PromotedExecutorRecord {
        schema: PROMOTED_RECORD_SCHEMA.into(),
        gate_receipt_sha256: gate.manifest_sha256.clone(),
        descriptor: request.descriptor.clone(),
        operation: request.operation.clone(),
        record_sha256: Sha256Digest::zero(),
    };
    record.record_sha256 = record_digest(&record)?;

    let persisted = persist_promoted_executor_record(tidex_home, &record)?;
    let _ = executor_by_id_in(tidex_home, &record.descriptor.executor_id)?;

    let mut receipt = PromotionAuthorityInstallReceipt {
        schema: INSTALL_RECEIPT_SCHEMA.into(),
        gate_receipt_sha256: gate.manifest_sha256.clone(),
        executor_id: record.descriptor.executor_id.clone(),
        descriptor_sha256: record.descriptor.descriptor_sha256.clone(),
        operation: record.operation.clone(),
        record: persisted.reference,
        idempotent: persisted.idempotent,
        authorizes_activation: false,
        manifest_sha256: Sha256Digest::zero(),
    };
    receipt.manifest_sha256 = receipt_digest(&receipt)?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::universal_promotion_gate::{
        UniversalPromotionBlocker, UniversalPromotionPolicy,
    };
    use crate::materialization::materialization_selector::{
        BackendEvaluation, BackendSelectionInput, BackendSelectionPolicy, PairwiseComplementarity,
    };
    use crate::materialization::shadow_evaluation::ShadowEvaluationReceipt;
    use crate::materialization::universality_evidence::{
        UniversalityEvidenceInput, UniversalityProtocol, UniversalityTrial,
    };
    use crate::operator::executor_registry::{
        executor_catalog_at, executor_id_for_operation_at, ExecutorAuthorityClass,
        ExecutorDescriptorDraft, ExecutorEffectClass, ExecutorState, ExecutorSurface,
    };
    use crate::receiver::receiver_profile::MaterializationStrategy;
    use crate::foundation::security::secure_dir;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn tmp(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "tidex-promotion-authority-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        secure_dir(&root).unwrap();
        root
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

    fn ready_gate_request() -> UniversalPromotionGateRequest {
        let candidate = Sha256Digest::digest_bytes(b"candidate-ready");
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
        let request = UniversalPromotionGateRequest {
            schema: "tidex.universal_promotion_gate_request/v1".into(),
            selection_input: selection_input.clone(),
            selection_receipt,
            universality_input,
            universality_receipt,
            shadow_evaluations: selection_input
                .evaluations
                .iter()
                .cloned()
                .map(shadow_receipt)
                .collect(),
            policy: UniversalPromotionPolicy {
                schema: "tidex.universal_promotion_policy/v1".into(),
                minimum_universality_n: 1,
                minimum_global_wilson_lower_bound: 0.0,
                require_all_selected_candidates_evaluated: true,
            },
        };
        let receipt = evaluate_universal_promotion_gate(&request).unwrap();
        assert_eq!(
            receipt.readiness,
            UniversalPromotionReadiness::ReadyForSeparatePromotionAuthority,
            "blockers={:?}",
            receipt.blockers
        );
        assert!(!receipt.authorizes_activation);
        assert!(!receipt
            .blockers
            .contains(&UniversalPromotionBlocker::HybridRequiresJointEvaluation));
        request
    }

    fn sample_descriptor(executor_id: &str) -> ExecutorDescriptor {
        ExecutorDescriptor::new(ExecutorDescriptorDraft {
            executor_id,
            title: "Promoted capability executor",
            module_path: "shadow_evaluation",
            owner: "promotion_authority",
            state: ExecutorState::Operational,
            authority: ExecutorAuthorityClass::Executor,
            effect_class: ExecutorEffectClass::CandidateArtifact,
            surfaces: &[ExecutorSurface::TidexOperator, ExecutorSurface::TidexCli],
            accepted_needs: &["promoted_capability_execution"],
            requires: &["shadow"],
            produces: &["readiness_decision"],
            operator_recipe_id: Some("shadow.evaluate"),
            notes: "Installed by PromotionAuthority after UPG Ready; not production authority.",
        })
        .unwrap()
    }

    #[test]
    fn ready_gate_installs_into_catalog_for_lookup_and_operation_binding() {
        let home = tmp("install");
        let gate_request = ready_gate_request();
        let gate_receipt = evaluate_universal_promotion_gate(&gate_request).unwrap();
        let descriptor = sample_descriptor("promoted.capability.demo");
        let request = PromotionAuthorityInstallRequest {
            schema: INSTALL_REQUEST_SCHEMA.into(),
            gate_request,
            gate_receipt,
            descriptor: descriptor.clone(),
            operation: Some("promoted_capability_demo".into()),
        };
        let receipt = install_promoted_executor(&home, &request).unwrap();
        assert!(!receipt.authorizes_activation);
        assert!(!receipt.idempotent);
        assert_eq!(receipt.executor_id, "promoted.capability.demo");

        let catalog = executor_catalog_at(&home).unwrap();
        assert!(catalog
            .iter()
            .any(|e| e.executor_id == "promoted.capability.demo"));
        let found = executor_by_id_in(&home, "promoted.capability.demo").unwrap();
        assert_eq!(found.descriptor_sha256, descriptor.descriptor_sha256);
        assert_eq!(
            executor_id_for_operation_at(&home, "promoted_capability_demo")
                .unwrap()
                .as_deref(),
            Some("promoted.capability.demo")
        );

        let again = install_promoted_executor(&home, &request).unwrap();
        assert!(again.idempotent);
        assert_eq!(again.record.sha256, receipt.record.sha256);
    }

    #[test]
    fn rejected_gate_cannot_install() {
        let home = tmp("reject");
        let mut gate_request = ready_gate_request();
        gate_request.shadow_evaluations.clear();
        let gate_receipt = evaluate_universal_promotion_gate(&gate_request).unwrap();
        assert_eq!(gate_receipt.readiness, UniversalPromotionReadiness::Rejected);
        let request = PromotionAuthorityInstallRequest {
            schema: INSTALL_REQUEST_SCHEMA.into(),
            gate_request,
            gate_receipt,
            descriptor: sample_descriptor("promoted.capability.rejected"),
            operation: None,
        };
        let err = install_promoted_executor(&home, &request).unwrap_err();
        assert!(matches!(err, BrainError::Invalid(code) if code == "promotion_authority_gate_not_ready"));
        assert!(executor_catalog_at(&home)
            .unwrap()
            .iter()
            .all(|e| e.executor_id != "promoted.capability.rejected"));
    }

    #[test]
    fn unknown_module_path_is_rejected() {
        let home = tmp("bad-module");
        let gate_request = ready_gate_request();
        let gate_receipt = evaluate_universal_promotion_gate(&gate_request).unwrap();
        let descriptor = ExecutorDescriptor::new(ExecutorDescriptorDraft {
            executor_id: "promoted.capability.badmodule",
            title: "Bad",
            module_path: "does_not_exist_anywhere",
            owner: "test",
            state: ExecutorState::Operational,
            authority: ExecutorAuthorityClass::Executor,
            effect_class: ExecutorEffectClass::CandidateArtifact,
            surfaces: &[ExecutorSurface::TidexCli],
            accepted_needs: &["x"],
            requires: &[],
            produces: &["readiness_decision"],
            operator_recipe_id: None,
            notes: "x",
        })
        .unwrap();
        let request = PromotionAuthorityInstallRequest {
            schema: INSTALL_REQUEST_SCHEMA.into(),
            gate_request,
            gate_receipt,
            descriptor,
            operation: None,
        };
        let err = install_promoted_executor(&home, &request).unwrap_err();
        assert!(matches!(
            err,
            BrainError::Invalid(code) if code == "promotion_authority_module_path_not_in_builtin_catalog"
        ));
    }
}
