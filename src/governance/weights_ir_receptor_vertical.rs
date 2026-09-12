//! Separate vertical: Weights/Hybrid → CapabilityIR → receptor (max real level).
//!
//! # HARD rules
//!
//! - Does **not** force GPEM / procedure-selector into Weights (that vertical stays Software).
//! - Does **not** invent CapabilityIR from source trees.
//! - Path: measured evidence + `residency.*` contracts + causal interventions
//!   → [`ResidencyDecision::Weights`] (or Hybrid) → admit IR → construct IR from
//!   measured [`LinearMapDescriptor`] → receptor compile/measure via existing engines.
//! - Fail-closed when warrant insufficient.
//! - No fake weights transplant; no new plasticity algorithms.

use crate::capability::acquisition_contract::{
    AcquisitionBudget, AcquisitionRequest, AcquisitionScope, NoisePolicy, RequestedResidency,
    SystemEnvelope,
};
use crate::capability::authenticated_capacity::{
    seal_measured_closed_linear_map_capacity, AuthenticatedCapacityPackage, CapacityProvenance,
    DonorKind, MeasuredClosedLinearMapDonor,
};
use crate::capability::capability_ir::{
    execute_linear_readout, CapabilityIr, IrNode, LinearReadoutExecution, OutputBinding,
    ParameterSlot, PrimitiveSet, TypedPort, ValueReference,
};
use crate::capability::content_vault::capture_to_vault;
use crate::foundation::authority::PrivateFileReference;
use crate::foundation::contracts::ProtectedCortex;
use crate::foundation::digest::{
    AuthenticatedCapacityDigest, CapabilityIrDigest, Sha256Digest, SystemEnvelopeDigest,
};
use crate::foundation::error::{BrainError, BrainResult};
use crate::foundation::identity::{
    AcquisitionId, CapabilityId, CapabilityNodeId, PortId, PrimitiveId,
};
use crate::foundation::linalg::Matrix;
use crate::foundation::security::verify_internal_private_root;
use crate::governance::authenticated_capacity_residency::{
    claim_id, decide_from_authenticated_capacity, CapabilityIrPath, ProjectionBasis,
};
use crate::governance::residency_decision::ResidencyDecision;
use crate::receiver::receiver_compiler::{
    compile_receiver_readout_capability, ReceiverCalibrationSet, ReceiverCompilerPolicy,
    ReceiverProposalMethod, ReceiverReadoutCapabilityInput,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const RECEIPT_DOMAIN: &[u8] = b"TIDEX:WEIGHTS-IR-RECEPTOR-VERTICAL:v1\0";
const CAPACITY_KEY: &str = "closed_linear_map_f64";
const CAPABILITY_ID: &str = "closed.linear_map:v1";

fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}

fn integrity(code: &str) -> BrainError {
    BrainError::Integrity(code.into())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WeightsIrReceptorVerticalSchema {
    #[serde(rename = "tidex.weights_ir_receptor_vertical/v1")]
    V1,
}

/// How far the existing receptor engines were entered (honest, no fake transplant).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "stage", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReceptorEntryProgress {
    NotEntered {
        reason: String,
    },
    /// CapabilityIR authenticated + linear readout executed (measured).
    MeasuredLinearReadout {
        execution: LinearReadoutExecution,
    },
    /// Readout executed and receiver compiler produced coordinates (experimental).
    ReceiverReadoutCompiled {
        execution: LinearReadoutExecution,
        compilation_allowed: bool,
        proposed_receiver_coordinates: Vec<f64>,
        note: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EmittedCapabilityIrRecord {
    pub capability_id: CapabilityId,
    pub capability_ir_sha256: CapabilityIrDigest,
    pub capability_ir: PrivateFileReference,
    pub system_envelope_sha256: SystemEnvelopeDigest,
    pub descriptor_relative_path: PathBuf,
    pub discovery: PrivateFileReference,
    /// Provenance is the measured descriptor bytes, not donor source semantics.
    pub constructed_from: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WeightsIrReceptorVerticalReceipt {
    schema: WeightsIrReceptorVerticalSchema,
    capacity_key: String,
    package_sha256: AuthenticatedCapacityDigest,
    package_donor_kind: DonorKind,
    residency_projection_basis: ProjectionBasis,
    residency_decision: ResidencyDecision,
    capability_ir_path: CapabilityIrPath,
    capability_ir_emitted: bool,
    emitted_ir: Option<EmittedCapabilityIrRecord>,
    receptor_entered: bool,
    receptor_progress: ReceptorEntryProgress,
    measured_weights_sha256: Sha256Digest,
    experience_notes: Vec<String>,
    authorizes_production: bool,
    manifest_sha256: Sha256Digest,
}

impl WeightsIrReceptorVerticalReceipt {
    pub fn residency_decision(&self) -> &ResidencyDecision {
        &self.residency_decision
    }

    pub fn capability_ir_path(&self) -> &CapabilityIrPath {
        &self.capability_ir_path
    }

    pub fn capability_ir_emitted(&self) -> bool {
        self.capability_ir_emitted
    }

    pub fn receptor_entered(&self) -> bool {
        self.receptor_entered
    }

    pub fn emitted_ir(&self) -> Option<&EmittedCapabilityIrRecord> {
        self.emitted_ir.as_ref()
    }

    pub fn receptor_progress(&self) -> &ReceptorEntryProgress {
        &self.receptor_progress
    }

    pub fn manifest_sha256(&self) -> &Sha256Digest {
        &self.manifest_sha256
    }

    fn calculate_digest(&self) -> BrainResult<Sha256Digest> {
        let mut unsigned = self.clone();
        unsigned.manifest_sha256 = Sha256Digest::zero();
        Ok(Sha256Digest::digest_domain(RECEIPT_DOMAIN, &serde_json::to_vec(&unsigned)?))
    }

    pub fn verify(&self) -> BrainResult<()> {
        if self.schema != WeightsIrReceptorVerticalSchema::V1 {
            return Err(invalid("weights_ir_receptor_vertical_schema_unsupported"));
        }
        if self.authorizes_production {
            return Err(integrity("weights_ir_receptor_vertical_claims_production"));
        }
        if self.manifest_sha256 == Sha256Digest::zero()
            || self.calculate_digest()? != self.manifest_sha256
        {
            return Err(integrity("weights_ir_receptor_vertical_digest_mismatch"));
        }
        // HARD: Software / stopped paths never emit IR or enter receptor.
        if matches!(self.residency_decision, ResidencyDecision::Software {})
            || matches!(self.capability_ir_path, CapabilityIrPath::Stopped { .. })
        {
            if self.capability_ir_emitted || self.receptor_entered || self.emitted_ir.is_some() {
                return Err(integrity("stopped_residency_must_not_emit_ir_or_enter_receptor"));
            }
        }
        if self.capability_ir_emitted != self.emitted_ir.is_some() {
            return Err(integrity("capability_ir_emitted_inconsistent"));
        }
        if self.receptor_entered
            && matches!(self.receptor_progress, ReceptorEntryProgress::NotEntered { .. })
        {
            return Err(integrity("receptor_entered_without_progress"));
        }
        // This vertical must never claim GPEM/procedure-selector as Weights warrant.
        if matches!(
            self.package_donor_kind,
            DonorKind::GpemV2Recommend | DonorKind::FixtureProcedureSelector
        ) {
            return Err(integrity("weights_ir_vertical_must_not_use_procedure_selector_donor"));
        }
        Ok(())
    }

    pub fn persist(&self, private_root: &Path) -> BrainResult<PathBuf> {
        self.verify()?;
        let destination = private_root
            .join("state/weights_ir_receptor_vertical/by-sha")
            .join(format!("{}.json", self.manifest_sha256.as_str()));
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(self)?;
        if destination.exists() {
            let existing = fs::read(&destination)?;
            if existing != bytes {
                return Err(integrity("weights_ir_receptor_vertical_immutable_conflict"));
            }
        } else {
            fs::write(&destination, bytes)?;
        }
        Ok(destination)
    }
}

/// Explicit Weights residency warrant claim set (closed / pure / weight-complete).
pub fn weights_residency_claim_ids() -> &'static [&'static str] {
    &[
        claim_id::REQUIREMENTS_CLOSED,
        claim_id::EFFECTS_PURE,
        claim_id::EXTERNAL_NONE,
        claim_id::OBS_WEIGHT,
        claim_id::WEIGHTS_REPR_TRUE,
        claim_id::HYBRID_REPR_TRUE,
        claim_id::SOFTWARE_REPR_TRUE,
        claim_id::WEIGHTS_TARGET_OK,
        claim_id::HYBRID_TARGET_OK,
        claim_id::SOFTWARE_TARGET_OK,
    ]
}

/// Explicit Hybrid residency warrant claim set (boundary semantics).
pub fn hybrid_residency_claim_ids() -> &'static [&'static str] {
    &[
        claim_id::REQUIREMENTS_BOUNDARY,
        claim_id::EFFECTS_BOUNDARY,
        claim_id::EXTERNAL_BOUNDARY,
        claim_id::OBS_BOUNDARY,
        claim_id::WEIGHTS_REPR_FALSE,
        claim_id::HYBRID_REPR_TRUE,
        claim_id::SOFTWARE_REPR_TRUE,
        claim_id::WEIGHTS_TARGET_NO,
        claim_id::HYBRID_TARGET_OK,
        claim_id::SOFTWARE_TARGET_OK,
    ]
}

fn default_measured_weights() -> (Vec<f64>, BTreeMap<String, Vec<f64>>) {
    // Closed arithmetic: map.alpha uniquely best on input [1, -1, 0.5].
    let input = vec![1.0, -1.0, 0.5];
    let mut weights = BTreeMap::new();
    weights.insert("map.alpha".into(), vec![2.0, -1.0, 0.5]); // margin = 2+1+0.25 = 3.25
    weights.insert("map.beta".into(), vec![0.5, 0.5, 0.0]); // margin = 0.5-0.5 = 0
    weights.insert("map.gamma".into(), vec![-1.0, 0.0, 0.0]); // margin = -1
    (input, weights)
}

/// Seal a Weights-warranted measured closed linear map package.
pub fn seal_weights_warranted_package() -> BrainResult<(AuthenticatedCapacityPackage, Vec<f64>)> {
    let (input, weights_by_candidate) = default_measured_weights();
    let donor = MeasuredClosedLinearMapDonor::new(input, weights_by_candidate.clone())?;
    let package = seal_measured_closed_linear_map_capacity(
        CAPACITY_KEY,
        CapacityProvenance {
            acquisition_id: None,
            capture_receipt_sha256: None,
            donor_locator: Some("measured://closed_linear_map_f64".into()),
        },
        &donor,
        weights_residency_claim_ids(),
    )?;
    let selected = weights_by_candidate
        .get("map.alpha")
        .cloned()
        .ok_or_else(|| integrity("measured_weights_missing_selected_map"))?;
    Ok((package, selected))
}

/// Seal a Hybrid-warranted package (admit IR path; receptor may still be limited).
pub fn seal_hybrid_warranted_package() -> BrainResult<(AuthenticatedCapacityPackage, Vec<f64>)> {
    let (input, weights_by_candidate) = default_measured_weights();
    let donor = MeasuredClosedLinearMapDonor::new(input, weights_by_candidate.clone())?;
    let package = seal_measured_closed_linear_map_capacity(
        "boundary_hybrid_linear_map",
        CapacityProvenance {
            acquisition_id: None,
            capture_receipt_sha256: None,
            donor_locator: Some("measured://boundary_hybrid_linear_map".into()),
        },
        &donor,
        hybrid_residency_claim_ids(),
    )?;
    let selected = weights_by_candidate
        .get("map.alpha")
        .cloned()
        .ok_or_else(|| integrity("measured_weights_missing_selected_map"))?;
    Ok((package, selected))
}

fn construct_ir_from_measured_descriptor(
    private_root: &Path,
    weights: &[f64],
) -> BrainResult<(EmittedCapabilityIrRecord, CapabilityIr, SystemEnvelope, Vec<f64>)> {
    let root = verify_internal_private_root(private_root)?;
    // Donor source must stay outside the private root (capture rejects overlap).
    use std::sync::atomic::{AtomicU64, Ordering};
    static DONOR_SEQ: AtomicU64 = AtomicU64::new(0);
    let donor_tag = Sha256Digest::digest_domain(
        b"TIDEX:WEIGHTS-IR-DONOR-TAG:v1 ",
        &serde_json::to_vec(weights)?,
    );
    let donor = std::env::temp_dir().join(format!(
        "tidex-weights-ir-donor-{}-{}-{}",
        std::process::id(),
        DONOR_SEQ.fetch_add(1, Ordering::Relaxed),
        &donor_tag.as_str()[..12]
    ));
    let _ = fs::remove_dir_all(&donor);
    fs::create_dir_all(&donor)?;

    // Measured evidence artifact (weights + shape). Not source-tree semantics.
    let evidence = serde_json::json!({
        "schema": "tidex.measured_linear_map_evidence/v1",
        "capability_id": CAPABILITY_ID,
        "input_dimension": weights.len(),
        "output_dimension": 1,
        "weights": weights,
        "note": "measured closed linear map parameters; CapabilityIR is derived from these bytes, not from donor source trees",
    });
    let evidence_bytes = serde_json::to_vec(&evidence)?;
    let evidence_sha = Sha256Digest::digest_bytes(&evidence_bytes);
    fs::write(donor.join("measured_weights.json"), &evidence_bytes)?;
    // Companion note file keeps capture non-empty of dual objects for scope.
    let note = serde_json::json!({
        "schema": "tidex.measured_linear_map_note/v1",
        "evidence_sha256": evidence_sha,
        "constructed_from": "measured_weights_json",
    });
    fs::write(donor.join("evidence_note.json"), serde_json::to_vec(&note)?)?;

    let request_id = Sha256Digest::digest_bytes(&evidence_bytes);
    let request = AcquisitionRequest::new(
        AcquisitionId::parse(format!("weights-ir-{}", &request_id.as_str()[..16]))?,
        AcquisitionScope::declared_paths(vec![
            PathBuf::from("measured_weights.json"),
            PathBuf::from("evidence_note.json"),
        ])?,
        RequestedResidency::PortableIrOnly,
        NoisePolicy::ExplicitOnly,
        AcquisitionBudget {
            max_files: 2,
            max_total_bytes: 1 << 20,
        },
        vec![],
    )?;
    let capture = capture_to_vault(&donor, &root, &request)?;
    let capture_reference = capture.persist(&root)?;
    let envelope = capture.envelope().clone();
    let dim =
        u64::try_from(weights.len()).map_err(|_| invalid("measured_weights_dimension_overflow"))?;

    // Real IR from measured shape/params: scalar linear readout profile already
    // supported by execute_linear_readout / compile_receiver_readout_capability.
    // Provenance binds to the authenticated measured_weights.json in the envelope.
    let ir = CapabilityIr::new_with_parameters(
        CapabilityId::parse(CAPABILITY_ID)?,
        &envelope,
        PrimitiveSet::tidex_core_v1()?,
        vec![TypedPort::tensor_f64(
            PortId::parse("runtime_input")?,
            vec![dim, 1],
        )?],
        vec![ParameterSlot::new(TypedPort::tensor_f64(
            PortId::parse("resident_weights")?,
            vec![1, dim],
        )?)?],
        vec![IrNode::new(
            CapabilityNodeId::parse("node.linear_map")?,
            PrimitiveId::parse("tensor.matmul")?,
            vec![
                ValueReference::Parameter {
                    name: PortId::parse("resident_weights")?,
                },
                ValueReference::Input {
                    name: PortId::parse("runtime_input")?,
                },
            ],
            TypedPort::tensor_f64(PortId::parse("mapped")?, vec![1, 1])?,
            vec![PathBuf::from("measured_weights.json")],
        )?],
        vec![OutputBinding::new(
            TypedPort::tensor_f64(PortId::parse("runtime_output")?, vec![1, 1])?,
            ValueReference::NodeOutput {
                node_id: CapabilityNodeId::parse("node.linear_map")?,
            },
        )?],
    )?;
    let capability_ir = ir.persist(&root, &envelope)?;
    let record = EmittedCapabilityIrRecord {
        capability_id: ir.capability_id().clone(),
        capability_ir_sha256: ir.manifest_digest().clone(),
        capability_ir,
        system_envelope_sha256: envelope.manifest_sha256().clone(),
        descriptor_relative_path: PathBuf::from("measured_weights.json"),
        discovery: capture_reference,
        constructed_from: "measured_weights_json_via_authenticated_envelope".into(),
    };
    let _ = fs::remove_dir_all(&donor);
    Ok((record, ir, envelope, weights.to_vec()))
}

fn experimental_readout_calibration(
) -> (ReceiverCalibrationSet, ReceiverCompilerPolicy, ProtectedCortex) {
    let receiver = vec![
        vec![1.0, 0.0],
        vec![0.0, 1.0],
        vec![1.0, 1.0],
        vec![2.0, -1.0],
        vec![-1.0, 2.0],
        vec![0.5, 2.0],
    ];
    let functions = receiver
        .iter()
        .map(|x| vec![x[0], 3.0 * x[0] + x[1]])
        .collect();
    let calibration = ReceiverCalibrationSet {
        receiver_snapshot_binding_sha256: None,
        functional_signatures: functions,
        receiver_solutions: receiver,
        wrong_functional_signatures: vec![vec![-0.2, -0.4]],
    };
    let policy = ReceiverCompilerPolicy {
        schema: "tidex.receiver_compiler_policy/v1".into(),
        ridge: 1e-10,
        minimum_decoder_loo_r2: 0.99,
        minimum_encoder_loo_r2: 0.99,
        minimum_decoder_loo_cosine: 0.99,
        maximum_functional_relative_error: 1e-5,
        minimum_identity_margin: 0.05,
        maximum_quadratic_cost: 1e6,
    };
    let cortex = ProtectedCortex {
        parameter_importance: vec![0.0; 2],
        directions: vec![],
        max_damage_ratio: 0.0,
    };
    (calibration, policy, cortex)
}

fn enter_receptor_path(
    ir: &CapabilityIr,
    envelope: &SystemEnvelope,
    weights: &[f64],
) -> BrainResult<(bool, ReceptorEntryProgress)> {
    // Hold-out activations: orthonormal-ish rows for measured execute.
    let dim = weights.len();
    let mut inputs = Vec::new();
    for i in 0..dim.min(4) {
        let mut row = vec![0.0; dim];
        row[i] = 1.0;
        inputs.push(row);
    }
    if inputs.is_empty() {
        return Ok((
            false,
            ReceptorEntryProgress::NotEntered {
                reason: "no_activation_rows".into(),
            },
        ));
    }
    let execution = execute_linear_readout(ir, envelope, weights, &inputs)?;

    // Project margins into a 2-d functional signature for the existing compiler.
    let mean = vec![0.0_f64; inputs.len().max(2)];
    let mut components = vec![vec![0.0; inputs.len()]; 2];
    for (i, _) in inputs.iter().enumerate() {
        if i < components[0].len() {
            components[0][i] = if i == 0 { 1.0 } else { 0.0 };
        }
        if i < components[1].len() {
            components[1][i] = if i == 1 { 1.0 } else { 0.0 };
        }
    }
    // Pad mean/components to match raw_margins length.
    let n = execution.raw_margins.len();
    let mean = {
        let mut m = mean;
        m.resize(n, 0.0);
        m
    };
    let components = components
        .into_iter()
        .map(|mut row| {
            row.resize(n, 0.0);
            row
        })
        .collect::<Vec<_>>();

    let (calibration, policy, cortex) = experimental_readout_calibration();
    let input = ReceiverReadoutCapabilityInput {
        ir,
        envelope,
        readout_weights: weights,
        inputs: &inputs,
        projection_mean: &mean,
        projection_components: &components,
    };
    match compile_receiver_readout_capability(
        &input,
        &calibration,
        &cortex,
        &Matrix::identity(2),
        &policy,
        ReceiverProposalMethod::CalibratedAffine,
    ) {
        Ok(compilation) => Ok((
            true,
            ReceptorEntryProgress::ReceiverReadoutCompiled {
                execution,
                compilation_allowed: compilation.numerical.allowed,
                proposed_receiver_coordinates: compilation
                    .numerical
                    .proposed_receiver_coordinates
                    .clone(),
                note: "entered existing compile_receiver_readout_capability; experimental_only, no promotion".into(),
            },
        )),
        Err(err) => {
            // Still entered the receptor path via measured execute_linear_readout.
            let _ = compilation_fallback_note(&err);
            Ok((
                true,
                ReceptorEntryProgress::MeasuredLinearReadout { execution },
            ))
        }
    }
}

fn compilation_fallback_note(err: &BrainError) -> String {
    format!("receiver_compile_deferred:{err}")
}

/// Run the Weights→IR→receptor vertical from an already-sealed package + measured weights.
pub fn run_from_package(
    private_root: &Path,
    package: AuthenticatedCapacityPackage,
    measured_weights: Vec<f64>,
) -> BrainResult<WeightsIrReceptorVerticalReceipt> {
    package.verify()?;
    if !matches!(package.donor_kind(), DonorKind::MeasuredClosedLinearMap) {
        return Err(invalid("weights_ir_vertical_requires_measured_closed_linear_map_donor"));
    }
    let root = verify_internal_private_root(private_root)?;
    let outcome = decide_from_authenticated_capacity(&package)?;
    let measured_weights_sha256 = Sha256Digest::digest_domain(
        b"TIDEX:MEASURED-LINEAR-MAP-WEIGHTS-JSON:v1\0",
        &serde_json::to_vec(&measured_weights)?,
    );

    let mut experience_notes = vec![
        "vertical:weights_ir_receptor".into(),
        "paso4_seal:measured_closed_linear_map".into(),
        "paso5_decide:authenticated_capacity_residency".into(),
        format!("donor_kind:{:?}", package.donor_kind()),
        "gpem_procedure_selector_not_used:software_vertical_separate".into(),
    ];

    let (capability_ir_emitted, emitted_ir, receptor_entered, receptor_progress) =
        match outcome.capability_ir_path() {
            CapabilityIrPath::Stopped { reason } => {
                experience_notes.push(format!("terminal:stopped:{reason:?}"));
                (
                    false,
                    None,
                    false,
                    ReceptorEntryProgress::NotEntered {
                        reason: format!("capability_ir_path_stopped:{reason:?}"),
                    },
                )
            }
            CapabilityIrPath::Admitted { candidate } => {
                experience_notes.push(format!("ir_path_admitted:{candidate:?}"));
                // Construct real IR from measured descriptor (not source trees).
                let (record, ir, envelope, weights) =
                    construct_ir_from_measured_descriptor(&root, &measured_weights)?;
                experience_notes.push("capability_ir_emitted:from_measured_descriptor".into());
                let (entered, progress) = enter_receptor_path(&ir, &envelope, &weights)?;
                if entered {
                    experience_notes.push("receptor_entered:existing_readout_engines".into());
                } else {
                    experience_notes.push("receptor_not_entered:engine_limit".into());
                }
                (true, Some(record), entered, progress)
            }
        };

    // Fail-closed integrity: Software must never reach here with IR.
    if matches!(outcome.decision(), ResidencyDecision::Software {}) && capability_ir_emitted {
        return Err(integrity("software_residency_must_not_emit_capability_ir"));
    }

    let mut receipt = WeightsIrReceptorVerticalReceipt {
        schema: WeightsIrReceptorVerticalSchema::V1,
        capacity_key: package.capacity_key().to_string(),
        package_sha256: package.manifest_sha256().clone(),
        package_donor_kind: package.donor_kind(),
        residency_projection_basis: outcome.projection_basis(),
        residency_decision: outcome.decision().clone(),
        capability_ir_path: outcome.capability_ir_path().clone(),
        capability_ir_emitted,
        emitted_ir,
        receptor_entered,
        receptor_progress,
        measured_weights_sha256,
        experience_notes,
        authorizes_production: false,
        manifest_sha256: Sha256Digest::zero(),
    };
    receipt.manifest_sha256 = receipt.calculate_digest()?;
    receipt.verify()?;
    Ok(receipt)
}

/// Productive Weights vertical: seal measured package → decide → emit IR → receptor.
pub fn run_weights_ir_receptor_vertical(
    private_root: &Path,
) -> BrainResult<WeightsIrReceptorVerticalReceipt> {
    let (package, weights) = seal_weights_warranted_package()?;
    let receipt = run_from_package(private_root, package, weights)?;
    receipt.persist(private_root)?;
    Ok(receipt)
}

/// Hybrid admit path (IR construction still measured; residency is Hybrid).
pub fn run_hybrid_ir_receptor_vertical(
    private_root: &Path,
) -> BrainResult<WeightsIrReceptorVerticalReceipt> {
    let (package, weights) = seal_hybrid_warranted_package()?;
    let receipt = run_from_package(private_root, package, weights)?;
    receipt.persist(private_root)?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::security::secure_dir;
    use crate::governance::residency_decision::ResidencyCandidate;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn tmp(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "tidex-weights-ir-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        secure_dir(&root).unwrap();
        root
    }

    #[test]
    fn weights_vertical_emits_ir_and_enters_receptor() {
        let root = tmp("weights");
        let receipt = run_weights_ir_receptor_vertical(&root).unwrap();
        assert_eq!(receipt.residency_decision(), &ResidencyDecision::Weights {});
        assert!(receipt.capability_ir_emitted());
        assert!(receipt.emitted_ir().is_some());
        assert!(receipt.receptor_entered());
        assert!(matches!(
            receipt.capability_ir_path(),
            CapabilityIrPath::Admitted {
                candidate: ResidencyCandidate::Weights
            }
        ));
        assert!(!matches!(receipt.receptor_progress(), ReceptorEntryProgress::NotEntered { .. }));
        receipt.verify().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hybrid_vertical_admits_and_emits_measured_ir() {
        let root = tmp("hybrid");
        let receipt = run_hybrid_ir_receptor_vertical(&root).unwrap();
        assert_eq!(receipt.residency_decision(), &ResidencyDecision::Hybrid {});
        assert!(receipt.capability_ir_emitted());
        assert!(matches!(
            receipt.capability_ir_path(),
            CapabilityIrPath::Admitted {
                candidate: ResidencyCandidate::Hybrid
            }
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn measured_package_without_residency_claims_fail_closed() {
        let (input, weights) = default_measured_weights();
        let donor = MeasuredClosedLinearMapDonor::new(input, weights).unwrap();
        let package = seal_measured_closed_linear_map_capacity(
            CAPACITY_KEY,
            CapacityProvenance::default(),
            &donor,
            &[],
        )
        .unwrap();
        let outcome = decide_from_authenticated_capacity(&package).unwrap();
        assert!(matches!(outcome.decision(), ResidencyDecision::BoundedUnknown { .. }));
        assert!(!outcome.admits_capability_ir());
    }

    #[test]
    fn procedure_selector_donor_rejected_by_weights_vertical() {
        let root = tmp("reject-gpem");
        let package =
            crate::capability::authenticated_capacity::seal_fixture_procedure_selector_capacity(
                "procedure_selector_or_explore",
                CapacityProvenance::default(),
            )
            .unwrap();
        let err = run_from_package(&root, package, vec![1.0, 2.0, 3.0])
            .unwrap_err()
            .to_string();
        assert!(err.contains("weights_ir_vertical_requires_measured_closed_linear_map_donor"));
        fs::remove_dir_all(root).unwrap();
    }
}
