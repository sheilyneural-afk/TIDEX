//! Integration: real on-disk V67/V68 receipts → admit+assimilate → next aperture changes.
//!
//! Library code under integration tests is built *without* `cfg(test)`, so the
//! private root must be the configured `TIDEX_PRIVATE_ROOT`. Tests serialize on
//! a process mutex when mutating that env var.
//!
//! Dense artifact must exist at the absolute path on the collected receipt;
//! otherwise real-V67/V68 cases skip rather than fabricating success.
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tidex::foundation::authority::ensure_private_directory;
use tidex::foundation::identity::{CapabilityId, LearningTargetId};
use tidex::foundation::security::{secure_dir, secure_file};
use tidex::learning::experimental_evidence_admission::{
    admit_and_assimilate_vxx_receipt_under_root, admit_vxx_receipt_under_root,
};
use tidex::learning::learning_orchestrator::{
    issue_next_persistent_learning_aperture, start_persistent_adaptive_learning,
    AdaptiveLearningPolicy, LearningTarget,
};

static PRIVATE_ROOT_LOCK: Mutex<()> = Mutex::new(());

fn temporary_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir()
        .join(format!("tidex-vxx-admission-itest-{label}-{}-{nonce}", std::process::id()));
    fs::create_dir(&root).unwrap();
    secure_dir(&root).unwrap();
    root
}

fn with_private_root<T>(root: &Path, body: impl FnOnce() -> T) -> T {
    let _guard = PRIVATE_ROOT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: serialized by PRIVATE_ROOT_LOCK for the duration of body.
    std::env::set_var("TIDEX_PRIVATE_ROOT", root);
    let out = body();
    std::env::remove_var("TIDEX_PRIVATE_ROOT");
    out
}

fn policy() -> AdaptiveLearningPolicy {
    AdaptiveLearningPolicy {
        schema: "tidex.adaptive_learning_policy/v1".into(),
        outcome_utility_weight: 1.0,
        maximize_observed_value: true,
    }
}

fn target_two_cap(id: &str) -> LearningTarget {
    LearningTarget {
        target_id: LearningTargetId::parse(id).unwrap(),
        capability_ids: [
            "v67.actuation_established",
            "v67.v66_behavioral_delta_source_used",
        ]
        .into_iter()
        .map(|name| CapabilityId::parse(name).unwrap())
        .collect(),
        candidate_budget: 8,
        plan_steps: 4,
        noise_variance: 0.1,
        cost_weight: 0.0,
        risk_weight: 0.0,
    }
}

fn target_v68_two_cap(id: &str) -> LearningTarget {
    LearningTarget {
        target_id: LearningTargetId::parse(id).unwrap(),
        capability_ids: ["v68.pass", "v68.correct_wrong_error_ratio"]
            .into_iter()
            .map(|name| CapabilityId::parse(name).unwrap())
            .collect(),
        candidate_budget: 8,
        plan_steps: 4,
        noise_variance: 0.1,
        cost_weight: 0.0,
        risk_weight: 0.0,
    }
}

fn bind_dense_from_receipt(root: &Path, wire: &Value) {
    let dense_path = PathBuf::from(wire["dense_delta"]["path"].as_str().unwrap());
    assert!(
        dense_path.is_file(),
        "real dense missing at {} — refuse to fabricate V67 success",
        dense_path.display()
    );
    let sha = wire["dense_delta"]["sha256"].as_str().unwrap();
    let dest_dir = root.join("artifacts/deltas/by-sha");
    ensure_private_directory(root, &dest_dir).unwrap();
    let dest = dest_dir.join(format!("{sha}.dvec"));
    if let Err(error) = fs::hard_link(&dense_path, &dest) {
        fs::copy(&dense_path, &dest).unwrap_or_else(|copy_error| {
            panic!("bind dense failed hardlink:{error} copy:{copy_error}")
        });
    }
    secure_file(&dest).unwrap();
}

#[test]
fn integration_real_v67_admit_assimilate_changes_next_aperture() {
    let receipt_path = Path::new("collected_receipts/tidex-v67-real-11kb4b6z-receipt.json");
    let raw = fs::read(receipt_path).expect("collected real V67 receipt");
    let wire: Value = serde_json::from_slice(&raw).unwrap();
    let dense_path = PathBuf::from(wire["dense_delta"]["path"].as_str().unwrap());
    if !dense_path.is_file() {
        eprintln!(
            "skip integration_real_v67_admit_assimilate_changes_next_aperture: dense missing at {}",
            dense_path.display()
        );
        return;
    }

    let root = temporary_root("real-v67");
    let session = "vxx-v67-itest";
    with_private_root(&root, || {
        bind_dense_from_receipt(&root, &wire);

        let _ =
            start_persistent_adaptive_learning(&root, session, &target_two_cap(session), &policy())
                .unwrap();
        let issued = issue_next_persistent_learning_aperture(&root, session).unwrap();
        let pending_before = issued.receipt.cycle.pending_step.clone().unwrap();

        let (admitted, assimilated) =
            admit_and_assimilate_vxx_receipt_under_root(&root, session, &raw).unwrap();
        assert_eq!(admitted.source_schema, "tidex.v67_weight_actuator_smoke/v1");
        assert_eq!(
            admitted.claim_boundary["universal_portability_established"],
            Value::Bool(false)
        );
        assert!(assimilated.receipt.cycle.pending_step.is_none());
        assert_eq!(assimilated.receipt.cycle.completed_evidence.len(), 1);

        let next = issue_next_persistent_learning_aperture(&root, session).unwrap();
        let pending_after = next.receipt.cycle.pending_step.unwrap();
        assert_ne!(pending_before.aperture_id, pending_after.aperture_id);
        assert!(
            pending_before.capability_weights != pending_after.capability_weights
                || (pending_before.information_gain - pending_after.information_gain).abs() > 1e-12
                || pending_before.aperture_id != pending_after.aperture_id
        );
    });
}

#[test]
fn integration_v68_forbidden_transfer_claim_fails_closed() {
    let root = temporary_root("v68-claim");
    let session = "vxx-v68-itest";
    with_private_root(&root, || {
        let _ =
            start_persistent_adaptive_learning(&root, session, &target_two_cap(session), &policy())
                .unwrap();
        let _ = issue_next_persistent_learning_aperture(&root, session).unwrap();

        let receipt = serde_json::json!({
            "schema": "tidex.v68_receiver_response_probe/v1",
            "complete": true,
            "pass": true,
            "stage": "standalone_forward_evaluated",
            "correct_wrong_error_ratio": 2.0,
            "dense_delta": {
                "path": root.join("missing.dvec"),
                "sha256": "a".repeat(64),
                "parameter_count": 2
            },
            "parameter_layout": {
                "schema": "tidex.parameter_block_layout/v1",
                "blocks": [{"name": "b0", "shape": [2], "offset": 0, "count": 2}],
                "total_parameter_count": 2
            },
            "claim_boundary": {
                "rust_generated_delta_from_measured_responses": true,
                "lora_delta_source_used": false,
                "receiver_optimizer_steps": 0,
                "backpropagation_used": false,
                "fresh_process_checkpoint_execution": true,
                "donor_model_used": false,
                "new_semantic_capability_transfer_established": true,
                "mbpp_transfer_established": false,
                "general_language_preservation_established": false,
                "authorizes_promotion": false
            }
        });
        let bytes = serde_json::to_vec(&receipt).unwrap();
        let err = admit_vxx_receipt_under_root(&root, session, &bytes).unwrap_err();
        assert!(
            format!("{err}").contains("vxx_admission_v68_forbidden_transfer_or_promotion_claim"),
            "{err}"
        );
    });
}

#[test]
fn integration_real_v68_admit_assimilate_changes_next_aperture() {
    let receipt_path =
        Path::new("collected_receipts/tidex-v68-definitive-prepost-20260912T1535Z-receipt.json");
    let raw = fs::read(receipt_path).expect("collected real V68 receipt");
    let wire: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(wire["schema"], "tidex.v68_receiver_response_probe/v1", "unexpected V68 schema");
    assert_eq!(
        wire["claim_boundary"]["new_semantic_capability_transfer_established"],
        Value::Bool(false),
        "V68 claim_boundary must stay transfer=false"
    );
    assert_eq!(wire["claim_boundary"]["mbpp_transfer_established"], Value::Bool(false));
    assert_eq!(wire["claim_boundary"]["authorizes_promotion"], Value::Bool(false));
    let dense = wire
        .get("dense_delta")
        .expect("V68 wire dense_delta required");
    let layout = wire
        .get("parameter_layout")
        .expect("V68 wire parameter_layout required");
    assert_eq!(
        dense["parameter_count"], layout["total_parameter_count"],
        "dense/layout contract"
    );
    let dense_path = PathBuf::from(dense["path"].as_str().unwrap());
    if !dense_path.is_file() {
        eprintln!(
            "skip integration_real_v68_admit_assimilate_changes_next_aperture: dense missing at {}",
            dense_path.display()
        );
        return;
    }

    let root = temporary_root("real-v68");
    let session = "vxx-v68-itest-real";
    with_private_root(&root, || {
        bind_dense_from_receipt(&root, &wire);

        let _ = start_persistent_adaptive_learning(
            &root,
            session,
            &target_v68_two_cap(session),
            &policy(),
        )
        .unwrap();
        let issued = issue_next_persistent_learning_aperture(&root, session).unwrap();
        let pending_before = issued.receipt.cycle.pending_step.clone().unwrap();

        let (admitted, assimilated) =
            admit_and_assimilate_vxx_receipt_under_root(&root, session, &raw).unwrap();
        assert_eq!(admitted.source_schema, "tidex.v68_receiver_response_probe/v1");
        assert_eq!(
            admitted.claim_boundary["new_semantic_capability_transfer_established"],
            Value::Bool(false)
        );
        assert_eq!(admitted.claim_boundary["mbpp_transfer_established"], Value::Bool(false));
        assert_eq!(admitted.claim_boundary["authorizes_promotion"], Value::Bool(false));
        assert!(assimilated.receipt.cycle.pending_step.is_none());
        assert_eq!(assimilated.receipt.cycle.completed_evidence.len(), 1);

        let next = issue_next_persistent_learning_aperture(&root, session).unwrap();
        let pending_after = next.receipt.cycle.pending_step.unwrap();
        assert_ne!(pending_before.aperture_id, pending_after.aperture_id);
        assert!(
            pending_before.capability_weights != pending_after.capability_weights
                || (pending_before.information_gain - pending_after.information_gain).abs() > 1e-12
                || pending_before.aperture_id != pending_after.aperture_id
        );
    });
}
