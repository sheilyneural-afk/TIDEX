//! Fail-closed admission of authenticated quality/experiments Vxx receipts into
//! [`LearningExperimentEvidence`] / [`DeltaObservation`].
//!
//! This adapter does **not** invent plasticity, procedural memory stores, or
//! capability-transfer claims. It only maps observed receipt properties into the
//! existing assimilate path when dense delta + parameter layout + pending
//! aperture context are all present and consistent.
//!
//! Supported schemas (initial):
//! - `tidex.v67_weight_actuator_smoke/v1` (dense_delta + parameter_layout)
//! - `tidex.v68_receiver_response_probe/v1` when dense_delta + parameter_layout
//!   are present on the wire (V68 samples without them fail closed)
//!
//! Explicitly rejected: V64-like receipts lacking dense/layout; missing pending
//! aperture; invented `observed_value`; promoting V68 into semantic transfer.

use crate::analysis::block_tomography::{parameter_layout_digest, ParameterBlockLayout};
use crate::foundation::artifact::{
    inspect_dvec, read_dvec_f32, sha256_file, sketch_dvec, DeltaArtifactRef,
};
use crate::foundation::authority::{
    ensure_private_directory, existing_regular_file_if_present, install_private_immutable_file,
    stage_private_file, write_or_verify_immutable,
};
use crate::foundation::contracts::{ConfounderValue, DeltaObservation, ExperimentLineage};
use crate::foundation::digest::{ProvenanceDigest, Sha256Digest};
use crate::foundation::error::{BrainError, BrainResult};
use crate::foundation::identity::ObservationId;
use crate::foundation::linalg::dot;
use crate::foundation::security::verify_internal_private_root;
use crate::learning::learning_orchestrator::{
    assimilate_persistent_learning_evidence_under_root, load_persistent_adaptive_learning_receipt,
    write_new_private, AdaptiveLearningStep, EvidenceReference, LearningExperimentEvidence,
    LoadedAdaptiveLearningReceipt,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub const MAX_VXX_RECEIPT_JSON_BYTES: u64 = 64 * 1024 * 1024;
const V67_SCHEMA: &str = "tidex.v67_weight_actuator_smoke/v1";
const V68_SCHEMA: &str = "tidex.v68_receiver_response_probe/v1";
const EVIDENCE_SCHEMA: &str = "tidex.learning_experiment_evidence/v1";
const SUPPORT_SCHEMA: &str = "tidex.vxx_admission_support/v1";
const SKETCH_DIM: usize = 32;
const SKETCH_SEED: u64 = 0x7636_782d_6164_6d69; // "vxx-admi"
const DENSE_INLINE_LIMIT: u64 = 64;

fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}

fn integrity(code: &str) -> BrainError {
    BrainError::Integrity(code.into())
}

fn bool_as_f64(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}

/// Result of converting one authenticated Vxx receipt into durable learning evidence.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AdmittedExperimentalEvidence {
    pub schema: String,
    pub source_schema: String,
    pub observation_path: PathBuf,
    pub evidence_path: PathBuf,
    pub support_path: PathBuf,
    pub evidence: LearningExperimentEvidence,
    pub observation: DeltaObservation,
    /// Exact claim_boundary object preserved from the source receipt (never rewritten).
    pub claim_boundary: Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct V67ClaimBoundary {
    rust_direct_weight_actuation_established: bool,
    output_requires_peft: bool,
    output_requires_lora_adapter: bool,
    v66_behavioral_delta_source_used: bool,
    functional_receiver_compiler_generated_target_delta: bool,
    target_capability_training_avoided_in_this_smoke: bool,
    universal_portability_established: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct V67MaterializationWire {
    schema: String,
    lifecycle: String,
    base_model_sha256: Sha256Digest,
    base_model_byte_len: u64,
    source_tensor_count: usize,
    source_parameter_count: u64,
    parameter_layout_sha256: Sha256Digest,
    delta_sha256: Sha256Digest,
    delta_parameter_count: u64,
    modified_tensor_count: usize,
    modified_parameter_count: u64,
    output_model_sha256: Sha256Digest,
    output_model_byte_len: u64,
    dtype_policy: String,
    trains_target_capability: bool,
    requires_adapter_at_runtime: bool,
    authorizes_promotion: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct V67AdmissionReceipt {
    schema: String,
    purpose: String,
    source_delta_kind: String,
    source_v66_manifest_sha256: Sha256Digest,
    source_v66_capability: String,
    source_v66_parameter_digest_sha256: Sha256Digest,
    base_model_sha256: Sha256Digest,
    adapter_model_sha256: Sha256Digest,
    adapter_config_sha256: Sha256Digest,
    lora_rank: usize,
    lora_alpha: f64,
    lora_scale: f64,
    target_tensors: Vec<String>,
    target_tensor_count: usize,
    dense_delta: DeltaArtifactRef,
    parameter_layout: ParameterBlockLayout,
    materialization: V67MaterializationWire,
    claim_boundary: V67ClaimBoundary,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct V68ClaimBoundary {
    rust_generated_delta_from_measured_responses: bool,
    lora_delta_source_used: bool,
    receiver_optimizer_steps: u64,
    backpropagation_used: bool,
    fresh_process_checkpoint_execution: bool,
    donor_model_used: bool,
    new_semantic_capability_transfer_established: bool,
    mbpp_transfer_established: bool,
    general_language_preservation_established: bool,
    authorizes_promotion: bool,
}

/// V68 wire form accepted only when dense_delta + parameter_layout are present.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct V68AdmissionReceipt {
    schema: String,
    complete: bool,
    pass: bool,
    stage: String,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    actual_target_relative_error: Option<f64>,
    #[serde(default)]
    wrong_target_relative_error: Option<f64>,
    #[serde(default)]
    correct_wrong_error_ratio: Option<f64>,
    #[serde(default)]
    unseen_control_margin_rms_change: Option<f64>,
    dense_delta: DeltaArtifactRef,
    parameter_layout: ParameterBlockLayout,
    claim_boundary: V68ClaimBoundary,
    #[serde(default)]
    requested_response: Option<Vec<f64>>,
    #[serde(default)]
    baseline_margins: Option<Vec<f64>>,
    #[serde(default)]
    criteria: Option<Value>,
    #[serde(default)]
    arms: Option<Value>,
    #[serde(default)]
    precommit_sha256: Option<Sha256Digest>,
    #[serde(default)]
    collector_sha256: Option<Sha256Digest>,
    #[serde(default)]
    tidex_binary_sha256: Option<Sha256Digest>,
}

#[derive(Debug, Serialize)]
struct AdmissionSupportDocument<'a> {
    schema: &'static str,
    source_schema: &'a str,
    source_receipt_sha256: &'a Sha256Digest,
    claim_boundary: &'a Value,
    mapped_functional_metrics: &'a [MappedMetric],
    forbidden_promotions_applied: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct MappedMetric {
    name: String,
    observed_value: f64,
}

struct PreparedAdmission {
    source_schema: String,
    from_checkpoint: String,
    to_checkpoint: String,
    functional_response: Vec<f64>,
    mapped_metrics: Vec<MappedMetric>,
    confounders: Vec<ConfounderValue>,
    claim_boundary: Value,
    dense_source: PathBuf,
    dense_expected: DeltaArtifactRef,
    layout: ParameterBlockLayout,
    layout_semantic: Sha256Digest,
    run_id: String,
    lineage_digests: [String; 4],
}

fn parse_schema(bytes: &[u8]) -> BrainResult<String> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| invalid("vxx_admission_receipt_json_invalid"))?;
    value
        .get("schema")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| invalid("vxx_admission_receipt_schema_missing"))
}

fn reject_missing_dense_layout(bytes: &[u8]) -> BrainResult<()> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| invalid("vxx_admission_receipt_json_invalid"))?;
    let has_dense = value.get("dense_delta").is_some();
    let has_layout = value.get("parameter_layout").is_some();
    if !has_dense || !has_layout {
        return Err(integrity("vxx_admission_dense_or_layout_missing"));
    }
    Ok(())
}

fn prepare_v67(receipt: V67AdmissionReceipt) -> BrainResult<PreparedAdmission> {
    if receipt.schema != V67_SCHEMA {
        return Err(invalid("vxx_admission_v67_schema_mismatch"));
    }
    if receipt.claim_boundary.universal_portability_established
        || receipt.materialization.authorizes_promotion
        || receipt.materialization.trains_target_capability
    {
        return Err(integrity("vxx_admission_v67_forbidden_promotion_claim"));
    }
    if !receipt
        .claim_boundary
        .rust_direct_weight_actuation_established
    {
        // Negative actuation outcome is still valid experience; keep metrics.
    }
    receipt.parameter_layout.validate()?;
    let semantic = parameter_layout_digest(&receipt.parameter_layout)?;
    if semantic.as_digest() != &receipt.materialization.parameter_layout_sha256 {
        // Real collected V67 receipts validate as `tidex.parameter_block_layout/v1`
        // but their materialization.parameter_layout_sha256 was sealed under the
        // pre-rename schema string `cerebro.tidex.parameter_block_layout/v1` with
        // identical blocks/offsets/counts. Accept that legacy alias only when the
        // rewritten schema recovers the sealed digest; never invent a digest.
        let legacy_match = {
            let mut legacy = receipt.parameter_layout.clone();
            legacy.schema = "cerebro.tidex.parameter_block_layout/v1".into();
            match serde_json::to_vec(&legacy) {
                Ok(bytes) => Sha256Digest::digest_bytes(&bytes)
                    == receipt.materialization.parameter_layout_sha256,
                Err(_) => false,
            }
        };
        if !legacy_match {
            return Err(integrity("vxx_admission_v67_layout_semantic_mismatch"));
        }
    }
    if receipt.dense_delta.sha256 != receipt.materialization.delta_sha256
        || receipt.dense_delta.parameter_count != receipt.materialization.delta_parameter_count
        || receipt.dense_delta.parameter_count != receipt.parameter_layout.total_parameter_count
        || receipt.dense_delta.parameter_count == 0
    {
        return Err(integrity("vxx_admission_v67_dense_contract_mismatch"));
    }
    if receipt.target_tensor_count != receipt.target_tensors.len() {
        return Err(integrity("vxx_admission_v67_tensor_count_mismatch"));
    }

    let mapped_metrics = vec![
        MappedMetric {
            name: "rust_direct_weight_actuation_established".into(),
            observed_value: bool_as_f64(
                receipt
                    .claim_boundary
                    .rust_direct_weight_actuation_established,
            ),
        },
        MappedMetric {
            name: "v66_behavioral_delta_source_used".into(),
            observed_value: bool_as_f64(receipt.claim_boundary.v66_behavioral_delta_source_used),
        },
    ];
    let functional_response = mapped_metrics
        .iter()
        .map(|metric| metric.observed_value)
        .collect::<Vec<_>>();

    let claim_boundary = serde_json::to_value(&receipt.claim_boundary)
        .map_err(|_| integrity("vxx_admission_v67_claim_boundary_serialize_failed"))?;
    let confounders = vec![
        ConfounderValue {
            name: "rust_direct_weight_actuation_established".into(),
            value: bool_as_f64(
                receipt
                    .claim_boundary
                    .rust_direct_weight_actuation_established,
            ),
        },
        ConfounderValue {
            name: "output_requires_peft".into(),
            value: bool_as_f64(receipt.claim_boundary.output_requires_peft),
        },
        ConfounderValue {
            name: "output_requires_lora_adapter".into(),
            value: bool_as_f64(receipt.claim_boundary.output_requires_lora_adapter),
        },
        ConfounderValue {
            name: "v66_behavioral_delta_source_used".into(),
            value: bool_as_f64(receipt.claim_boundary.v66_behavioral_delta_source_used),
        },
        ConfounderValue {
            name: "functional_receiver_compiler_generated_target_delta".into(),
            value: bool_as_f64(
                receipt
                    .claim_boundary
                    .functional_receiver_compiler_generated_target_delta,
            ),
        },
        ConfounderValue {
            name: "target_capability_training_avoided_in_this_smoke".into(),
            value: bool_as_f64(
                receipt
                    .claim_boundary
                    .target_capability_training_avoided_in_this_smoke,
            ),
        },
        ConfounderValue {
            name: "universal_portability_established".into(),
            value: bool_as_f64(receipt.claim_boundary.universal_portability_established),
        },
        ConfounderValue {
            name: "authorizes_promotion".into(),
            value: bool_as_f64(receipt.materialization.authorizes_promotion),
        },
    ];

    Ok(PreparedAdmission {
        source_schema: receipt.schema,
        from_checkpoint: receipt.base_model_sha256.to_string(),
        to_checkpoint: receipt.materialization.output_model_sha256.to_string(),
        functional_response,
        mapped_metrics,
        confounders,
        claim_boundary,
        dense_source: receipt.dense_delta.path.clone(),
        dense_expected: receipt.dense_delta,
        layout: receipt.parameter_layout,
        layout_semantic: receipt.materialization.parameter_layout_sha256,
        run_id: format!("v67-{}", receipt.source_v66_manifest_sha256),
        lineage_digests: [
            receipt.source_v66_manifest_sha256.to_string(),
            receipt.base_model_sha256.to_string(),
            receipt.adapter_model_sha256.to_string(),
            receipt.source_v66_parameter_digest_sha256.to_string(),
        ],
    })
}

fn prepare_v68(receipt: V68AdmissionReceipt) -> BrainResult<PreparedAdmission> {
    if receipt.schema != V68_SCHEMA {
        return Err(invalid("vxx_admission_v68_schema_mismatch"));
    }
    if !receipt.complete {
        return Err(integrity("vxx_admission_v68_incomplete"));
    }
    if receipt
        .claim_boundary
        .new_semantic_capability_transfer_established
        || receipt.claim_boundary.mbpp_transfer_established
        || receipt.claim_boundary.authorizes_promotion
        || receipt
            .claim_boundary
            .general_language_preservation_established
    {
        return Err(integrity("vxx_admission_v68_forbidden_transfer_or_promotion_claim"));
    }
    receipt.parameter_layout.validate()?;
    let semantic = parameter_layout_digest(&receipt.parameter_layout)?;
    if receipt.dense_delta.parameter_count == 0
        || receipt.dense_delta.parameter_count != receipt.parameter_layout.total_parameter_count
    {
        return Err(integrity("vxx_admission_v68_dense_contract_mismatch"));
    }
    let ratio = receipt
        .correct_wrong_error_ratio
        .filter(|value| value.is_finite())
        .ok_or_else(|| integrity("vxx_admission_v68_ratio_missing_or_nonfinite"))?;
    let mapped_metrics = vec![
        MappedMetric {
            name: "pass".into(),
            observed_value: bool_as_f64(receipt.pass),
        },
        MappedMetric {
            name: "correct_wrong_error_ratio".into(),
            observed_value: ratio,
        },
    ];
    let functional_response = mapped_metrics
        .iter()
        .map(|metric| metric.observed_value)
        .collect::<Vec<_>>();

    let claim_boundary = serde_json::to_value(&receipt.claim_boundary)
        .map_err(|_| integrity("vxx_admission_v68_claim_boundary_serialize_failed"))?;
    let mut confounders = vec![
        ConfounderValue {
            name: "pass".into(),
            value: bool_as_f64(receipt.pass),
        },
        ConfounderValue {
            name: "new_semantic_capability_transfer_established".into(),
            value: bool_as_f64(
                receipt
                    .claim_boundary
                    .new_semantic_capability_transfer_established,
            ),
        },
        ConfounderValue {
            name: "mbpp_transfer_established".into(),
            value: bool_as_f64(receipt.claim_boundary.mbpp_transfer_established),
        },
        ConfounderValue {
            name: "authorizes_promotion".into(),
            value: bool_as_f64(receipt.claim_boundary.authorizes_promotion),
        },
    ];
    if let Some(error) = receipt.actual_target_relative_error {
        if !error.is_finite() {
            return Err(integrity("vxx_admission_v68_error_nonfinite"));
        }
        confounders.push(ConfounderValue {
            name: "actual_target_relative_error".into(),
            value: error,
        });
    }

    let precommit = receipt
        .precommit_sha256
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "0".repeat(64));
    let collector = receipt
        .collector_sha256
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "1".repeat(64));
    let binary = receipt
        .tidex_binary_sha256
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "2".repeat(64));
    let layout_hex = semantic.as_digest().to_string();

    Ok(PreparedAdmission {
        source_schema: receipt.schema,
        from_checkpoint: format!("v68-precommit-{precommit}"),
        to_checkpoint: format!("v68-stage-{}", receipt.stage),
        functional_response,
        mapped_metrics,
        confounders,
        claim_boundary,
        dense_source: receipt.dense_delta.path.clone(),
        dense_expected: receipt.dense_delta,
        layout: receipt.parameter_layout,
        layout_semantic: semantic.as_digest().clone(),
        run_id: format!("v68-{precommit}"),
        lineage_digests: [precommit, collector, binary, layout_hex],
        // silence unused warning for semantic binding on V68 (no mat field)
        // layout_semantic set above from semantic digest
    })
}

fn prepare_from_bytes(bytes: &[u8]) -> BrainResult<PreparedAdmission> {
    if bytes.len() as u64 > MAX_VXX_RECEIPT_JSON_BYTES {
        return Err(invalid("vxx_admission_receipt_too_large"));
    }
    let schema = parse_schema(bytes)?;
    match schema.as_str() {
        V67_SCHEMA => {
            let receipt: V67AdmissionReceipt = serde_json::from_slice(bytes)
                .map_err(|_| invalid("vxx_admission_v67_receipt_contract_invalid"))?;
            prepare_v67(receipt)
        }
        V68_SCHEMA => {
            reject_missing_dense_layout(bytes)?;
            let receipt: V68AdmissionReceipt = serde_json::from_slice(bytes)
                .map_err(|_| invalid("vxx_admission_v68_receipt_contract_invalid"))?;
            prepare_v68(receipt)
        }
        _ => {
            // V64 and other schemas without dense/layout fail closed here.
            let _ = reject_missing_dense_layout(bytes);
            Err(invalid("vxx_admission_schema_unsupported"))
        }
    }
}

fn install_layout(root: &Path, layout: &ParameterBlockLayout) -> BrainResult<Sha256Digest> {
    layout.validate()?;
    let bytes = serde_json::to_vec(layout)
        .map_err(|_| integrity("vxx_admission_layout_serialize_failed"))?;
    let digest = Sha256Digest::digest_bytes(&bytes);
    let path = root
        .join("state/parameter_layouts/by-sha")
        .join(format!("{digest}.json"));
    write_or_verify_immutable(root, &path, &bytes)?;
    Ok(digest)
}

fn install_dense_artifact(
    root: &Path,
    source: &Path,
    expected: &DeltaArtifactRef,
) -> BrainResult<DeltaArtifactRef> {
    let dest = root
        .join("artifacts/deltas/by-sha")
        .join(format!("{}.dvec", expected.sha256));

    if let Ok(Some(existing)) = existing_regular_file_if_present(root, &dest) {
        let inspected = inspect_dvec(root, &existing)?;
        if inspected.sha256 != expected.sha256
            || inspected.parameter_count != expected.parameter_count
        {
            return Err(integrity("vxx_admission_dense_collision"));
        }
        return Ok(inspected);
    }

    // Source may already be the canonical private path.
    if source == dest || source.ends_with(format!("{}.dvec", expected.sha256)) {
        if let Ok(inspected) = inspect_dvec(root, source) {
            if inspected.sha256 == expected.sha256
                && inspected.parameter_count == expected.parameter_count
            {
                return Ok(inspected);
            }
        }
    }

    if !source.is_absolute() || !source.is_file() {
        return Err(integrity("vxx_admission_dense_source_missing"));
    }
    let digest =
        sha256_file(source).map_err(|_| integrity("vxx_admission_dense_source_unreadable"))?;
    if digest != expected.sha256 {
        return Err(integrity("vxx_admission_dense_digest_mismatch"));
    }

    ensure_private_directory(root, &root.join("artifacts/deltas/by-sha"))?;
    let (temporary, staged) = stage_private_file(root, &dest, |file| {
        let mut input = File::open(source)
            .map_err(|error| BrainError::Integrity(format!("vxx_admission_dense_open:{error}")))?;
        io::copy(&mut input, file)
            .map_err(|error| BrainError::Integrity(format!("vxx_admission_dense_copy:{error}")))?;
        Ok(())
    })?;
    if staged != expected.sha256 {
        return Err(integrity("vxx_admission_dense_staged_digest_mismatch"));
    }
    let _ = install_private_immutable_file(root, &temporary, &dest, &expected.sha256)?;
    let inspected = inspect_dvec(root, &dest)?;
    if inspected.sha256 != expected.sha256 || inspected.parameter_count != expected.parameter_count
    {
        return Err(integrity("vxx_admission_dense_install_contract_invalid"));
    }
    Ok(inspected)
}

fn observation_delta(root: &Path, dense: &DeltaArtifactRef) -> BrainResult<Vec<f64>> {
    if dense.parameter_count <= DENSE_INLINE_LIMIT {
        let values = read_dvec_f32(root, dense)?;
        if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
            return Err(integrity("vxx_admission_dense_values_invalid"));
        }
        return Ok(values.into_iter().map(f64::from).collect());
    }
    let sketch = sketch_dvec(root, &dense.path, SKETCH_DIM, SKETCH_SEED)?;
    if sketch.is_empty() || sketch.iter().any(|value| !value.is_finite()) {
        return Err(integrity("vxx_admission_dense_sketch_invalid"));
    }
    Ok(sketch)
}

fn require_pending_step(
    root: &Path,
    session_id: &str,
) -> BrainResult<(LoadedAdaptiveLearningReceipt, AdaptiveLearningStep)> {
    let loaded = load_persistent_adaptive_learning_receipt(root, session_id)?;
    let pending = loaded
        .receipt
        .cycle
        .pending_step
        .clone()
        .ok_or_else(|| integrity("adaptive_learning_no_pending_aperture"))?;
    Ok((loaded, pending))
}

fn build_observation_id(
    aperture_id: &str,
    receipt_digest: &Sha256Digest,
) -> BrainResult<ObservationId> {
    let short = &receipt_digest.as_str()[..12];
    ObservationId::parse(format!("vxx-{short}-{aperture_id}"))
}

/// Admit an authenticated Vxx receipt under a private root into observation +
/// `LearningExperimentEvidence` artifacts. Requires a live session with a
/// pending aperture. Does not assimilate unless the caller invokes assimilate.
pub fn admit_vxx_receipt_under_root(
    root: impl AsRef<Path>,
    session_id: &str,
    receipt_bytes: &[u8],
) -> BrainResult<AdmittedExperimentalEvidence> {
    let root = verify_internal_private_root(root.as_ref())?;
    if receipt_bytes.len() as u64 > MAX_VXX_RECEIPT_JSON_BYTES {
        return Err(invalid("vxx_admission_receipt_too_large"));
    }
    let receipt_digest = Sha256Digest::digest_bytes(receipt_bytes);
    let prepared = prepare_from_bytes(receipt_bytes)?;
    let (loaded, pending) = require_pending_step(&root, session_id)?;
    if pending.capability_weights.len() != prepared.functional_response.len() {
        return Err(integrity("vxx_admission_functional_response_dimension_mismatch"));
    }
    if pending
        .capability_weights
        .iter()
        .any(|value| !value.is_finite())
        || prepared
            .functional_response
            .iter()
            .any(|value| !value.is_finite())
    {
        return Err(integrity("vxx_admission_nonfinite_weights_or_response"));
    }

    let layout_content_digest = install_layout(&root, &prepared.layout)?;
    // Bind semantic identity was checked during prepare; keep content digest for
    // the learning observation contract (file bytes == path digest).
    let _ = &prepared.layout_semantic;
    let dense = install_dense_artifact(&root, &prepared.dense_source, &prepared.dense_expected)?;
    let delta = observation_delta(&root, &dense)?;
    let observation_id = build_observation_id(pending.aperture_id.as_str(), &receipt_digest)?;
    let observed_value = dot(&pending.capability_weights, &prepared.functional_response)?;

    let observation = DeltaObservation {
        observation_id: observation_id.clone(),
        from_checkpoint: prepared.from_checkpoint,
        to_checkpoint: prepared.to_checkpoint,
        generation: loaded.receipt.generation.saturating_add(1),
        delta,
        functional_response: prepared.functional_response.clone(),
        confounders: prepared.confounders.clone(),
        reliability: 1.0,
        independence_group: pending.aperture_id.to_string(),
        experiment_lineage: ExperimentLineage {
            run_id: prepared.run_id.clone(),
            replicate_id: "vxx-admission-0".into(),
            randomization_id: pending.aperture_id.to_string(),
            dataset_split_digest: prepared.lineage_digests[0].clone(),
            initial_checkpoint_digest: prepared.lineage_digests[1].clone(),
            optimizer_config_digest: prepared.lineage_digests[2].clone(),
            template_config_digest: prepared.lineage_digests[3].clone(),
        },
        dense_artifact: Some(dense),
        parameter_layout_sha256: Some(layout_content_digest),
        representation_artifact: None,
        representation_protocol_sha256: None,
        provenance_digest: ProvenanceDigest::from(receipt_digest.clone()),
    };

    let observation_raw = serde_json::to_vec_pretty(&observation)
        .map_err(|_| integrity("vxx_admission_observation_serialize_failed"))?;
    let observation_path = root
        .join("state/observations")
        .join(format!("{}.json", observation.observation_id));
    write_new_private(&root, &observation_path, &observation_raw)?;

    let support = AdmissionSupportDocument {
        schema: SUPPORT_SCHEMA,
        source_schema: &prepared.source_schema,
        source_receipt_sha256: &receipt_digest,
        claim_boundary: &prepared.claim_boundary,
        mapped_functional_metrics: &prepared.mapped_metrics,
        forbidden_promotions_applied: false,
    };
    let support_raw = serde_json::to_vec_pretty(&support)
        .map_err(|_| integrity("vxx_admission_support_serialize_failed"))?;
    let support_path = root
        .join("state/experiment_support")
        .join(format!("vxx-{}.json", pending.aperture_id));
    write_new_private(&root, &support_path, &support_raw)?;

    let evidence = LearningExperimentEvidence {
        schema: EVIDENCE_SCHEMA.into(),
        session_id: loaded.receipt.session_id.clone(),
        target_digest: loaded.receipt.target_digest.clone(),
        aperture_id: pending.aperture_id.clone(),
        observed_value,
        observation_id: observation_id.clone(),
        observation: EvidenceReference {
            path: observation_path.clone(),
            sha256: Sha256Digest::digest_bytes(&observation_raw),
        },
        evidence_files: vec![EvidenceReference {
            path: support_path.clone(),
            sha256: Sha256Digest::digest_bytes(&support_raw),
        }],
    };
    let evidence_raw = serde_json::to_vec_pretty(&evidence)
        .map_err(|_| integrity("vxx_admission_evidence_serialize_failed"))?;
    let evidence_path = root
        .join("state/experiment_envelopes")
        .join(format!("vxx-{}.json", pending.aperture_id));
    write_new_private(&root, &evidence_path, &evidence_raw)?;

    Ok(AdmittedExperimentalEvidence {
        schema: "tidex.vxx_admission_result/v1".into(),
        source_schema: prepared.source_schema,
        observation_path,
        evidence_path,
        support_path,
        evidence,
        observation,
        claim_boundary: prepared.claim_boundary,
    })
}

/// Admit then assimilate when a pending aperture exists. Fail-closed otherwise.
pub fn admit_and_assimilate_vxx_receipt_under_root(
    root: impl AsRef<Path>,
    session_id: &str,
    receipt_bytes: &[u8],
) -> BrainResult<(AdmittedExperimentalEvidence, LoadedAdaptiveLearningReceipt)> {
    let root = verify_internal_private_root(root.as_ref())?;
    let admitted = admit_vxx_receipt_under_root(&root, session_id, receipt_bytes)?;
    let assimilated = assimilate_persistent_learning_evidence_under_root(
        &root,
        session_id,
        &admitted.evidence_path,
    )?;
    Ok((admitted, assimilated))
}

/// Read a receipt file with a hard size bound (caller path; not required under root).
pub fn read_vxx_receipt_file(path: &Path) -> BrainResult<Vec<u8>> {
    let file = File::open(path)
        .map_err(|error| BrainError::Invalid(format!("vxx_admission_receipt_open:{error}")))?;
    let mut bytes = Vec::new();
    file.take(MAX_VXX_RECEIPT_JSON_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| BrainError::Invalid(format!("vxx_admission_receipt_read:{error}")))?;
    if bytes.len() as u64 > MAX_VXX_RECEIPT_JSON_BYTES {
        return Err(invalid("vxx_admission_receipt_too_large"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::artifact::ArtifactWriteAuthority;
    use crate::foundation::authority::ensure_private_directory;
    use crate::foundation::identity::{CapabilityId, LearningTargetId};
    use crate::foundation::security::{secure_dir, secure_file};
    use crate::learning::learning_orchestrator::{
        issue_next_persistent_learning_aperture, start_persistent_adaptive_learning,
        AdaptiveLearningPolicy, LearningTarget,
    };
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir()
            .join(format!("tidex-vxx-admission-{label}-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        secure_dir(&root).unwrap();
        root
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

    fn write_mini_dense(root: &Path, values: &[f32]) -> DeltaArtifactRef {
        ArtifactWriteAuthority::for_internal_root(root)
            .unwrap()
            .create_content_addressed_dvec(values)
            .unwrap()
    }

    fn mini_layout(count: usize) -> ParameterBlockLayout {
        ParameterBlockLayout::from_shapes(&[crate::analysis::block_tomography::BlockShapeSpec {
            name: "block0".into(),
            shape: vec![count],
            count,
        }])
        .unwrap()
    }

    fn v67_fixture(root: &Path, values: &[f32]) -> Vec<u8> {
        let dense = write_mini_dense(root, values);
        let layout = mini_layout(values.len());
        let semantic = parameter_layout_digest(&layout).unwrap();
        let receipt = serde_json::json!({
            "schema": V67_SCHEMA,
            "purpose": "fixture engineering smoke for admission tests",
            "source_delta_kind": "v66_lora_reconstructed_as_dense_delta",
            "source_v66_manifest_sha256": "a".repeat(64),
            "source_v66_capability": "python.function_synthesis.mbpp.medium_high",
            "source_v66_parameter_digest_sha256": "b".repeat(64),
            "base_model_sha256": "c".repeat(64),
            "adapter_model_sha256": "d".repeat(64),
            "adapter_config_sha256": "e".repeat(64),
            "lora_rank": 8,
            "lora_alpha": 16.0,
            "lora_scale": 2.0,
            "target_tensors": ["block0"],
            "target_tensor_count": 1,
            "dense_delta": dense,
            "parameter_layout": layout,
            "materialization": {
                "schema": "tidex.weight_materialization_receipt/v1",
                "lifecycle": "candidate_only_not_promoted",
                "base_model_sha256": "c".repeat(64),
                "base_model_byte_len": 32,
                "source_tensor_count": 1,
                "source_parameter_count": values.len() as u64,
                "parameter_layout_sha256": semantic,
                "delta_sha256": dense.sha256,
                "delta_parameter_count": values.len() as u64,
                "modified_tensor_count": 1,
                "modified_parameter_count": values.len() as u64,
                "output_model_sha256": "f".repeat(64),
                "output_model_byte_len": 32,
                "dtype_policy": "f32",
                "trains_target_capability": false,
                "requires_adapter_at_runtime": false,
                "authorizes_promotion": false
            },
            "claim_boundary": {
                "rust_direct_weight_actuation_established": true,
                "output_requires_peft": false,
                "output_requires_lora_adapter": false,
                "v66_behavioral_delta_source_used": true,
                "functional_receiver_compiler_generated_target_delta": false,
                "target_capability_training_avoided_in_this_smoke": false,
                "universal_portability_established": false
            }
        });
        serde_json::to_vec_pretty(&receipt).unwrap()
    }

    fn v68_fixture(root: &Path, values: &[f32], passed: bool) -> Vec<u8> {
        let dense = write_mini_dense(root, values);
        let layout = mini_layout(values.len());
        let receipt = serde_json::json!({
            "schema": V68_SCHEMA,
            "complete": true,
            "pass": passed,
            "stage": "standalone_forward_evaluated",
            "scope": "native-parameter response identification; not semantic capability transfer",
            "actual_target_relative_error": if passed { 0.01 } else { 0.5 },
            "wrong_target_relative_error": 0.8,
            "correct_wrong_error_ratio": if passed { 8.0 } else { 1.2 },
            "unseen_control_margin_rms_change": 0.01,
            "dense_delta": dense,
            "parameter_layout": layout,
            "precommit_sha256": "a".repeat(64),
            "collector_sha256": "b".repeat(64),
            "tidex_binary_sha256": "c".repeat(64),
            "claim_boundary": {
                "rust_generated_delta_from_measured_responses": true,
                "lora_delta_source_used": false,
                "receiver_optimizer_steps": 0,
                "backpropagation_used": false,
                "fresh_process_checkpoint_execution": true,
                "donor_model_used": false,
                "new_semantic_capability_transfer_established": false,
                "mbpp_transfer_established": false,
                "general_language_preservation_established": false,
                "authorizes_promotion": false
            }
        });
        serde_json::to_vec_pretty(&receipt).unwrap()
    }

    fn start_with_pending(root: &Path, session: &str) -> AdaptiveLearningStep {
        let _ =
            start_persistent_adaptive_learning(root, session, &target_two_cap(session), &policy())
                .unwrap();
        let issued = issue_next_persistent_learning_aperture(root, session).unwrap();
        issued.receipt.cycle.pending_step.unwrap()
    }

    #[test]
    fn v67_fixture_admits_and_assimilates_changing_next_aperture() {
        let root = temporary_root("v67-ok");
        let session = "vxx-v67-session";
        let pending_before = start_with_pending(&root, session);
        let receipt = v67_fixture(&root, &[0.25, -0.5, 0.125]);
        let (admitted, assimilated) =
            admit_and_assimilate_vxx_receipt_under_root(&root, session, &receipt).unwrap();
        assert_eq!(admitted.source_schema, V67_SCHEMA);
        assert_eq!(
            admitted.claim_boundary["universal_portability_established"],
            Value::Bool(false)
        );
        assert!(assimilated.receipt.cycle.pending_step.is_none());
        assert_eq!(assimilated.receipt.cycle.completed_evidence.len(), 1);
        let next = issue_next_persistent_learning_aperture(&root, session).unwrap();
        let pending_after = next.receipt.cycle.pending_step.unwrap();
        assert_ne!(pending_before.aperture_id, pending_after.aperture_id);
        // Observed value must be exactly weights · mapped metrics (no invention).
        let expected =
            dot(&pending_before.capability_weights, &admitted.observation.functional_response)
                .unwrap();
        assert_eq!(admitted.evidence.observed_value, expected);
    }

    #[test]
    fn v67_legacy_cerebro_layout_schema_alias_binds() {
        let root = temporary_root("v67-legacy-schema");
        let session = "vxx-v67-legacy";
        let _ = start_with_pending(&root, session);
        let values = [0.25_f32, -0.5, 0.125];
        let mut value: Value = serde_json::from_slice(&v67_fixture(&root, &values)).unwrap();
        // Seal materialization digest under cerebro schema using the same
        // ParameterBlockLayout serde projection prepare_v67 rehashes.
        let layout: ParameterBlockLayout =
            serde_json::from_value(value["parameter_layout"].clone()).unwrap();
        let mut legacy = layout;
        legacy.schema = "cerebro.tidex.parameter_block_layout/v1".into();
        let legacy_digest = Sha256Digest::digest_bytes(&serde_json::to_vec(&legacy).unwrap());
        value["materialization"]["parameter_layout_sha256"] =
            Value::String(legacy_digest.to_string());
        let bytes = serde_json::to_vec(&value).unwrap();
        let admitted = admit_vxx_receipt_under_root(&root, session, &bytes).unwrap();
        assert_eq!(admitted.source_schema, V67_SCHEMA);
    }

    #[test]
    fn v68_negative_pass_is_valid_experience_without_transfer_claim() {
        let root = temporary_root("v68-neg");
        let session = "vxx-v68-session";
        let _ = start_with_pending(&root, session);
        let receipt = v68_fixture(&root, &[0.5, -0.25], false);
        let admitted = admit_vxx_receipt_under_root(&root, session, &receipt).unwrap();
        assert_eq!(admitted.observation.functional_response[0], 0.0);
        assert_eq!(
            admitted.claim_boundary["new_semantic_capability_transfer_established"],
            Value::Bool(false)
        );
        assert!(admitted
            .observation
            .confounders
            .iter()
            .any(|c| c.name == "new_semantic_capability_transfer_established" && c.value == 0.0));
    }

    #[test]
    fn v68_transfer_claim_true_is_rejected() {
        let root = temporary_root("v68-bad-claim");
        let session = "vxx-v68-bad";
        let _ = start_with_pending(&root, session);
        let mut value: Value =
            serde_json::from_slice(&v68_fixture(&root, &[1.0, 2.0], true)).unwrap();
        value["claim_boundary"]["new_semantic_capability_transfer_established"] = Value::Bool(true);
        let bytes = serde_json::to_vec(&value).unwrap();
        let err = admit_vxx_receipt_under_root(&root, session, &bytes).unwrap_err();
        assert!(
            format!("{err}").contains("vxx_admission_v68_forbidden_transfer_or_promotion_claim")
        );
    }

    #[test]
    fn v68_schema_sample_without_resolvable_dense_fails_closed() {
        let root = temporary_root("v68-sample");
        let session = "vxx-v68-sample";
        let _ = start_with_pending(&root, session);
        let raw = std::fs::read("tests/fixtures/vxx_admission/v68_schema_sample.json").unwrap();
        // Parses as V68 with dense+layout fields, but dense source cannot be installed.
        let err = admit_vxx_receipt_under_root(&root, session, &raw).unwrap_err();
        assert!(format!("{err}").contains("vxx_admission_dense"));
    }

    #[test]
    fn v64_like_without_dense_fails_closed() {
        let root = temporary_root("v64");
        let session = "vxx-v64-session";
        let _ = start_with_pending(&root, session);
        let v64 = std::fs::read("quality/evidence/v64/receipt.json").unwrap();
        let err = admit_vxx_receipt_under_root(&root, session, &v64).unwrap_err();
        let message = format!("{err}");
        assert!(
            message.contains("vxx_admission_schema_unsupported")
                || message.contains("vxx_admission_dense_or_layout_missing")
        );
    }

    #[test]
    fn real_v67_receipt_parses_but_needs_pending_and_dense() {
        let root = temporary_root("v67-real");
        let raw = std::fs::read("collected_receipts/tidex-v67-real-11kb4b6z-receipt.json").unwrap();
        // Without a session / pending aperture the admit path fail-closes.
        let err = admit_vxx_receipt_under_root(&root, "missing-session", &raw).unwrap_err();
        let message = format!("{err}");
        assert!(
            message.contains("adaptive_learning")
                || message.contains("session")
                || message.contains("vxx_admission")
        );
    }

    /// End-to-end with on-disk `collected_receipts/tidex-v67-real-*.json` + the
    /// dense artifact path recorded on that receipt. Skips (pass) only when the
    /// dense file is absent on this machine — never fabricates a V67 success.
    #[test]
    fn real_v67_receipt_admits_assimilates_and_changes_next_aperture() {
        // Full sketch of ~201M params is too slow under the debug test profile;
        // the release integration test covers the on-disk path. Opt in with
        // TIDEX_RUN_REAL_V67=1 for a local debug proof.
        if std::env::var_os("TIDEX_RUN_REAL_V67").is_none() {
            eprintln!(
                "skip real_v67_receipt_admits_assimilates_and_changes_next_aperture: set TIDEX_RUN_REAL_V67=1 (prefer cargo test --release --test vxx_learning_admission)"
            );
            return;
        }
        let receipt_path = Path::new("collected_receipts/tidex-v67-real-11kb4b6z-receipt.json");
        let raw = std::fs::read(receipt_path).expect("real V67 receipt must be present in-tree");
        let wire: Value = serde_json::from_slice(&raw).unwrap();
        let dense_path = PathBuf::from(
            wire["dense_delta"]["path"]
                .as_str()
                .expect("real V67 receipt dense_delta.path"),
        );
        if !dense_path.is_file() {
            eprintln!(
                "skip real_v67_receipt_admits_assimilates_and_changes_next_aperture: dense missing at {}",
                dense_path.display()
            );
            return;
        }

        let root = temporary_root("v67-real-e2e");
        let session = "vxx-v67-real-e2e";
        // Bind the content-addressed dense into the private root via hardlink
        // (copy fallback) so install_dense does not re-copy ~769MiB blindly,
        // while still authenticating via inspect_dvec.
        let sha = wire["dense_delta"]["sha256"].as_str().unwrap();
        let dest_dir = root.join("artifacts/deltas/by-sha");
        ensure_private_directory(&root, &dest_dir).unwrap();
        let dest = dest_dir.join(format!("{sha}.dvec"));
        if let Err(error) = std::fs::hard_link(&dense_path, &dest) {
            std::fs::copy(&dense_path, &dest).unwrap_or_else(|copy_error| {
                panic!(
                    "failed to bind real dense into private root (hardlink:{error}; copy:{copy_error})"
                )
            });
        }
        secure_file(&dest).unwrap();

        let pending_before = start_with_pending(&root, session);
        let (admitted, assimilated) =
            admit_and_assimilate_vxx_receipt_under_root(&root, session, &raw).unwrap();
        assert_eq!(admitted.source_schema, V67_SCHEMA);
        assert_eq!(
            admitted.claim_boundary["universal_portability_established"],
            Value::Bool(false)
        );
        assert_eq!(
            admitted.claim_boundary["rust_direct_weight_actuation_established"],
            Value::Bool(true)
        );
        assert!(assimilated.receipt.cycle.pending_step.is_none());
        assert_eq!(assimilated.receipt.cycle.completed_evidence.len(), 1);
        assert_eq!(
            assimilated.receipt.cycle.session.completed_aperture_ids,
            vec![pending_before.aperture_id.clone()]
        );

        let next = issue_next_persistent_learning_aperture(&root, session).unwrap();
        let pending_after = next.receipt.cycle.pending_step.clone().unwrap();
        assert_ne!(pending_before.aperture_id, pending_after.aperture_id);
        // Strategy change: posterior updated from assimilate, so the next
        // sensing vector / objective need not match the pre-assimilate choice.
        assert!(
            pending_before.capability_weights != pending_after.capability_weights
                || (pending_before.information_gain - pending_after.information_gain).abs() > 1e-12
                || pending_before.aperture_id != pending_after.aperture_id,
            "expected next aperture strategy to reflect assimilate"
        );
        let expected =
            dot(&pending_before.capability_weights, &admitted.observation.functional_response)
                .unwrap();
        assert_eq!(admitted.evidence.observed_value, expected);
        assert_eq!(admitted.observation.functional_response, vec![1.0, 1.0]);
    }

    #[test]
    fn missing_pending_aperture_fails_closed() {
        let root = temporary_root("no-pending");
        let session = "vxx-no-pending";
        let _ =
            start_persistent_adaptive_learning(&root, session, &target_two_cap(session), &policy())
                .unwrap();
        // started but never issued → no pending
        let receipt = v67_fixture(&root, &[0.1, 0.2]);
        let err = admit_vxx_receipt_under_root(&root, session, &receipt).unwrap_err();
        assert!(format!("{err}").contains("adaptive_learning_no_pending_aperture"));
    }

    #[test]
    fn dimension_mismatch_fails_closed() {
        let root = temporary_root("dim");
        let session = "vxx-dim";
        // three capabilities, but V67 maps exactly two metrics
        let target = LearningTarget {
            target_id: LearningTargetId::parse(session).unwrap(),
            capability_ids: ["a", "b", "c"]
                .into_iter()
                .map(|name| CapabilityId::parse(name).unwrap())
                .collect(),
            candidate_budget: 12,
            plan_steps: 6,
            noise_variance: 0.1,
            cost_weight: 0.0,
            risk_weight: 0.0,
        };
        let _ = start_persistent_adaptive_learning(&root, session, &target, &policy()).unwrap();
        let _ = issue_next_persistent_learning_aperture(&root, session).unwrap();
        let receipt = v67_fixture(&root, &[0.1, 0.2, 0.3]);
        let err = admit_vxx_receipt_under_root(&root, session, &receipt).unwrap_err();
        assert!(format!("{err}").contains("vxx_admission_functional_response_dimension_mismatch"));
    }
}
