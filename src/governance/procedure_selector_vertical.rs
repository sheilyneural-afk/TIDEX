//! Paso 6 — bounded procedure-selector vertical (productive path).
//!
//! # Acceptance vertical
//!
//! > Given context + historical procedures + prior results, select the most
//! > appropriate procedure **or** explore.
//!
//! # Pipeline (HARD invariant) — productive / demo acceptance
//!
//! ```text
//! live GPEM/donor → seal authenticated capacity
//!   → ResidencyDecision
//!   → Software / BoundedUnknown / Blocked: stop honestly (no CapabilityIR)
//!   → Weights / Hybrid-warranted: admit IR path only; document receptor hook
//!     (never invent CapabilityIR from donor source trees)
//! ```
//!
//! **Fail-closed:** if GPEM/donor does not respond, the productive path **ends**
//! — no capacity, no IR, no receptor, **no fixture substitute**.
//! `FixtureProcedureSelector` / `seal_fixture_procedure_selector_capacity` remain
//! for **unit tests only** (via [`run_from_package`]), never as a demo fallback.

use crate::capability::authenticated_capacity::{
    seal_live_gpem_v2_recommend_capacity, AuthenticatedCapacityPackage, CapacityProvenance,
    DonorKind, GpemV2RecommendDonorWire, PriorProcedureResult, SelectorStimulus,
};
use crate::foundation::digest::{AuthenticatedCapacityDigest, Sha256Digest};
use crate::foundation::error::{BrainError, BrainResult};
use crate::governance::authenticated_capacity_residency::{
    capability_ir_from_outcome, decide_from_authenticated_capacity,
    AuthenticatedCapacityResidencyOutcome, CapabilityIrPath, CapabilityIrStopReason,
    ProjectionBasis,
};
use crate::governance::residency_decision::{ResidencyCandidate, ResidencyDecision};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const RECEIPT_DOMAIN: &[u8] = b"TIDEX:PROCEDURE-SELECTOR-VERTICAL:v1\0";
const CAPACITY_KEY: &str = "procedure_selector_or_explore";

fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}

fn integrity(code: &str) -> BrainError {
    BrainError::Integrity(code.into())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcedureSelectorVerticalSchema {
    #[serde(rename = "tidex.procedure_selector_vertical/v1")]
    V1,
}

/// How donor observations were obtained for this demo tick.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum DonorExecutionRecord {
    /// Live SHEI/GPEM recommend observed and sealed (productive path success).
    GpemV2LiveRecommend {
        schema: String,
        store_root: PathBuf,
        donor_locator: String,
    },
    /// Live GPEM wire constructed but observe fail-closed (retained for receipts/tests).
    GpemWireUnavailable {
        schema: String,
        store_root: PathBuf,
        observe_error: String,
    },
    /// Unit-test fixture only — never a productive-path substitute.
    FixtureProcedureSelector {
        donor_locator: String,
        gpem_wire_schema_documented: String,
        gpem_observe_error: String,
    },
}

/// Documented next engines when residency admits Weights/Hybrid — never executed
/// here without an authenticated CapabilityIR source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReceptorPathHook {
    pub admitted_candidate: ResidencyCandidate,
    pub next_engines: Vec<String>,
    /// Always false in Paso 6: no IR invented from donor trees.
    pub invented_capability_ir: bool,
    /// Always false in Paso 6: no fake weights transplant.
    pub transplanted_weights: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "terminal", rename_all = "snake_case", deny_unknown_fields)]
pub enum VerticalTerminal {
    /// Software / BoundedUnknown / Blocked — intelligence may stop as Software.
    StoppedHonestly {
        stop_reason: CapabilityIrStopReason,
        receptor_entered: bool,
    },
    /// IR path admitted; receptor hook documented only (no invent / no transplant).
    ReceptorPathDocumented {
        hook: ReceptorPathHook,
        receptor_entered: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcedureSelectorVerticalReceipt {
    schema: ProcedureSelectorVerticalSchema,
    capacity_key: String,
    donor_execution: DonorExecutionRecord,
    package_sha256: AuthenticatedCapacityDigest,
    package_donor_kind: DonorKind,
    residency_projection_basis: ProjectionBasis,
    residency_decision: ResidencyDecision,
    capability_ir_path: CapabilityIrPath,
    /// Explicit: Software never yields IR bytes.
    capability_ir_emitted: bool,
    terminal: VerticalTerminal,
    experience_notes: Vec<String>,
    authorizes_production: bool,
    manifest_sha256: Sha256Digest,
}

impl ProcedureSelectorVerticalReceipt {
    pub fn capacity_key(&self) -> &str {
        &self.capacity_key
    }

    pub fn donor_execution(&self) -> &DonorExecutionRecord {
        &self.donor_execution
    }

    pub fn residency_decision(&self) -> &ResidencyDecision {
        &self.residency_decision
    }

    pub fn capability_ir_path(&self) -> &CapabilityIrPath {
        &self.capability_ir_path
    }

    pub fn capability_ir_emitted(&self) -> bool {
        self.capability_ir_emitted
    }

    pub fn terminal(&self) -> &VerticalTerminal {
        &self.terminal
    }

    pub fn receptor_entered(&self) -> bool {
        match &self.terminal {
            VerticalTerminal::StoppedHonestly {
                receptor_entered, ..
            }
            | VerticalTerminal::ReceptorPathDocumented {
                receptor_entered, ..
            } => *receptor_entered,
        }
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
        if self.schema != ProcedureSelectorVerticalSchema::V1 {
            return Err(invalid("procedure_selector_vertical_schema_unsupported"));
        }
        if self.authorizes_production {
            return Err(integrity("procedure_selector_vertical_claims_production"));
        }
        if self.manifest_sha256 == Sha256Digest::zero()
            || self.calculate_digest()? != self.manifest_sha256
        {
            return Err(integrity("procedure_selector_vertical_digest_mismatch"));
        }
        // HARD: Software must never emit IR or enter receptor.
        if matches!(self.residency_decision, ResidencyDecision::Software {}) {
            if self.capability_ir_emitted || self.receptor_entered() {
                return Err(integrity("software_residency_must_stop_without_ir_or_receptor"));
            }
            if !matches!(
                self.capability_ir_path,
                CapabilityIrPath::Stopped {
                    reason: CapabilityIrStopReason::SoftwareResidency
                }
            ) {
                return Err(integrity("software_residency_ir_path_inconsistent"));
            }
        }
        if let VerticalTerminal::ReceptorPathDocumented { hook, .. } = &self.terminal {
            if hook.invented_capability_ir || hook.transplanted_weights {
                return Err(integrity("receptor_hook_must_not_fake_ir_or_transplant"));
            }
        }
        Ok(())
    }

    pub fn persist(&self, private_root: &Path) -> BrainResult<PathBuf> {
        self.verify()?;
        let destination = private_root
            .join("state/procedure_selector_vertical/by-sha")
            .join(format!("{}.json", self.manifest_sha256.as_str()));
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(self)?;
        if destination.exists() {
            let existing = std::fs::read(&destination)?;
            if existing != bytes {
                return Err(integrity("procedure_selector_vertical_immutable_conflict"));
            }
        } else {
            std::fs::write(&destination, bytes)?;
        }
        Ok(destination)
    }
}

fn receptor_hook_for(candidate: ResidencyCandidate) -> ReceptorPathHook {
    ReceptorPathHook {
        admitted_candidate: candidate,
        next_engines: vec![
            "capability_ir::authenticate_capability_ir (requires prior authenticated IR; never from donor tree)".into(),
            "materialization::shadow_materializer / materialization_selector".into(),
            "receiver::receiver_weight_binding::prepare_receiver_weight_candidate".into(),
            "governance::universal_promotion_gate / adapter_bank (measure + gates)".into(),
        ],
        invented_capability_ir: false,
        transplanted_weights: false,
        note: "Paso 6 documents the receptor path when residency admits Weights/Hybrid; it does not invent CapabilityIR or fake a weights transplant without authenticated IR evidence.".into(),
    }
}

fn terminal_from_outcome(
    outcome: &AuthenticatedCapacityResidencyOutcome,
) -> BrainResult<VerticalTerminal> {
    match outcome.capability_ir_path() {
        CapabilityIrPath::Stopped { reason } => Ok(VerticalTerminal::StoppedHonestly {
            stop_reason: reason.clone(),
            receptor_entered: false,
        }),
        CapabilityIrPath::Admitted { candidate } => Ok(VerticalTerminal::ReceptorPathDocumented {
            hook: receptor_hook_for(*candidate),
            receptor_entered: false,
        }),
    }
}

/// Productive donor acquire: live GPEM wire only. Fail-closed — **no fixture substitute**.
///
/// Returns `(package, donor_execution)` only when live SHEI/GPEM seals capacity.
/// Unavailable / misconfigured / insufficient live evidence → hard error codes
/// (`gpem_v2_recommend_donor_unavailable`, `…_misconfigured`,
/// `…_insufficient_live_evidence`, `…_invoke_failed`).
pub fn acquire_procedure_selector_package(
    gpem_store_root: PathBuf,
) -> BrainResult<(AuthenticatedCapacityPackage, DonorExecutionRecord)> {
    let wire = GpemV2RecommendDonorWire::new(
        gpem_store_root.clone(),
        vec![
            "route".into(),
            "capability_id".into(),
            "prior_procedure".into(),
        ],
    )?;
    let probe = SelectorStimulus::new(
        "route:analysis",
        vec![PriorProcedureResult::new("proc.alpha", "success")?],
        vec!["proc.alpha".into(), "proc.beta".into()],
    )?;
    // Probe first so unavailable donors fail-closed before sealing work.
    wire.observe(&probe).map_err(|err| {
        let msg = err.to_string();
        if msg.contains("gpem_v2_recommend_donor_unavailable") {
            invalid("gpem_v2_recommend_donor_unavailable")
        } else if msg.contains("gpem_v2_recommend_donor_misconfigured") {
            invalid("gpem_v2_recommend_donor_misconfigured")
        } else if msg.contains("gpem_v2_recommend_invoke_failed") {
            invalid("gpem_v2_recommend_invoke_failed")
        } else if msg.contains("gpem_v2_recommend_insufficient_live_evidence") {
            invalid("gpem_v2_recommend_insufficient_live_evidence")
        } else {
            invalid("gpem_wire_observe_unexpected_error")
        }
    })?;

    let donor_locator = format!("shei-gpem://{}", gpem_store_root.display());
    let package = seal_live_gpem_v2_recommend_capacity(
        &wire,
        CAPACITY_KEY,
        CapacityProvenance {
            acquisition_id: None,
            capture_receipt_sha256: None,
            donor_locator: Some(donor_locator.clone()),
        },
    )
    .map_err(|err| {
        let msg = err.to_string();
        if msg.contains("gpem_v2_recommend_insufficient_live_evidence") {
            invalid("gpem_v2_recommend_insufficient_live_evidence")
        } else if msg.contains("gpem_v2_recommend_donor_unavailable") {
            invalid("gpem_v2_recommend_donor_unavailable")
        } else if msg.contains("gpem_v2_recommend_donor_misconfigured") {
            invalid("gpem_v2_recommend_donor_misconfigured")
        } else if msg.contains("gpem_v2_recommend_invoke_failed") {
            invalid("gpem_v2_recommend_invoke_failed")
        } else {
            invalid("gpem_wire_observe_unexpected_error")
        }
    })?;

    Ok((
        package,
        DonorExecutionRecord::GpemV2LiveRecommend {
            schema: GpemV2RecommendDonorWire::SCHEMA.into(),
            store_root: gpem_store_root,
            donor_locator,
        },
    ))
}

/// Run the productive vertical: live donor only → residency → terminal.
///
/// Fail-closed when GPEM/donor is missing (no fixture continue).
pub fn run_procedure_selector_vertical(
    gpem_store_root: PathBuf,
) -> BrainResult<ProcedureSelectorVerticalReceipt> {
    let (package, donor_execution) = acquire_procedure_selector_package(gpem_store_root)?;
    run_from_package(package, donor_execution)
}

/// Decide residency from an already-sealed package (tests / Weights hook path).
pub fn run_from_package(
    package: AuthenticatedCapacityPackage,
    donor_execution: DonorExecutionRecord,
) -> BrainResult<ProcedureSelectorVerticalReceipt> {
    package.verify()?;
    let outcome = decide_from_authenticated_capacity(&package)?;
    let ir_probe = capability_ir_from_outcome(&outcome)?;
    let capability_ir_emitted = ir_probe.is_some();
    let terminal = terminal_from_outcome(&outcome)?;

    let mut experience_notes = vec![
        "paso4_seal:authenticated_capacity".into(),
        "paso5_decide:authenticated_capacity_residency".into(),
        format!("donor_kind:{:?}", package.donor_kind()),
    ];
    match &donor_execution {
        DonorExecutionRecord::GpemV2LiveRecommend { donor_locator, .. } => {
            experience_notes.push(format!("gpem_live_recommend:{donor_locator}"));
            experience_notes
                .push("software_residency_is_valid_intelligence:do_not_put_in_llm".into());
        }
        DonorExecutionRecord::GpemWireUnavailable { observe_error, .. } => {
            experience_notes.push(format!("gpem_observe_fail_closed:{observe_error}"));
        }
        DonorExecutionRecord::FixtureProcedureSelector {
            gpem_wire_schema_documented,
            gpem_observe_error,
            ..
        } => {
            experience_notes
                .push(format!("gpem_wire_documented_unit_fixture:{gpem_wire_schema_documented}"));
            experience_notes.push(format!("unit_fixture_note:{gpem_observe_error}"));
            experience_notes
                .push("software_residency_is_valid_intelligence:do_not_put_in_llm".into());
        }
    }
    if matches!(outcome.decision(), ResidencyDecision::Software {}) {
        experience_notes.push("terminal:stopped_as_software_no_ir".into());
    }

    let mut receipt = ProcedureSelectorVerticalReceipt {
        schema: ProcedureSelectorVerticalSchema::V1,
        capacity_key: package.capacity_key().to_string(),
        donor_execution,
        package_sha256: package.manifest_sha256().clone(),
        package_donor_kind: package.donor_kind(),
        residency_projection_basis: outcome.projection_basis(),
        residency_decision: outcome.decision().clone(),
        capability_ir_path: outcome.capability_ir_path().clone(),
        capability_ir_emitted,
        terminal,
        experience_notes,
        authorizes_production: false,
        manifest_sha256: Sha256Digest::zero(),
    };
    receipt.manifest_sha256 = receipt.calculate_digest()?;
    receipt.verify()?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::authenticated_capacity::{
        seal_fixture_procedure_selector_capacity, CapacityProvenance, FunctionalContractClaim,
        FunctionalContractStatus,
    };
    use crate::governance::authenticated_capacity_residency::claim_id;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn tmp(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "tidex-paso6-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn package_with_residency_claims(
        capacity_key: &str,
        claim_ids: &[&str],
    ) -> AuthenticatedCapacityPackage {
        let base =
            seal_fixture_procedure_selector_capacity(capacity_key, CapacityProvenance::default())
                .unwrap();
        let select_digest = base.observations()[0].observation_sha256().clone();
        let explore_digest = base.observations()[1].observation_sha256().clone();
        let intervene_digest = base.interventions()[0]
            .intervened_observation
            .observation_sha256()
            .clone();
        let mut contracts = vec![
            FunctionalContractClaim::new(
                "claim.select-best-historical",
                "functional select contract",
                FunctionalContractStatus::Supported,
                vec![select_digest.clone()],
            )
            .unwrap(),
            FunctionalContractClaim::new(
                "claim.explore-when-no-success",
                "functional explore contract",
                FunctionalContractStatus::Supported,
                vec![explore_digest],
            )
            .unwrap(),
        ];
        for claim_id in claim_ids {
            contracts.push(
                FunctionalContractClaim::new(
                    *claim_id,
                    format!("residency attestation {claim_id}"),
                    FunctionalContractStatus::Supported,
                    vec![select_digest.clone(), intervene_digest.clone()],
                )
                .unwrap(),
            );
        }
        AuthenticatedCapacityPackage::seal(
            capacity_key,
            DonorKind::FixtureProcedureSelector,
            CapacityProvenance::default(),
            base.observations().to_vec(),
            base.interventions().to_vec(),
            contracts,
            Vec::new(),
        )
        .unwrap()
    }

    #[test]
    fn unit_fixture_package_concludes_software_without_ir_or_receptor() {
        // Unit fixture only — not the productive acquire path.
        let root = tmp("software");
        let package = seal_fixture_procedure_selector_capacity(
            CAPACITY_KEY,
            CapacityProvenance::default(),
        )
        .unwrap();
        let receipt = run_from_package(
            package,
            DonorExecutionRecord::FixtureProcedureSelector {
                donor_locator: "fixture://unit-test-only".into(),
                gpem_wire_schema_documented: GpemV2RecommendDonorWire::SCHEMA.into(),
                gpem_observe_error: "unit_fixture_not_productive_path".into(),
            },
        )
        .unwrap();
        receipt.verify().unwrap();
        assert_eq!(receipt.capacity_key(), CAPACITY_KEY);
        assert_eq!(receipt.residency_decision(), &ResidencyDecision::Software {});
        assert!(!receipt.capability_ir_emitted());
        assert!(!receipt.receptor_entered());
        assert!(matches!(
            receipt.terminal(),
            VerticalTerminal::StoppedHonestly {
                stop_reason: CapabilityIrStopReason::SoftwareResidency,
                receptor_entered: false,
            }
        ));
        let path = receipt.persist(&root).unwrap();
        assert!(path.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn productive_acquire_fail_closed_without_fixture_substitute() {
        // Hermetic CI path: marker forces donor unavailable — never continue with fixture.
        let root = tmp("gpem-probe");
        let store = root.join("gpem-store");
        fs::create_dir_all(&store).unwrap();
        fs::write(store.join(".tidex_gpem_force_unavailable"), b"1").unwrap();
        let err = acquire_procedure_selector_package(store.clone())
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("gpem_v2_recommend_donor_unavailable")
                || err.contains("gpem_v2_recommend_donor_misconfigured"),
            "productive path must end without fixture continue: {err}"
        );
        let err2 = run_procedure_selector_vertical(store)
            .unwrap_err()
            .to_string();
        assert!(
            err2.contains("gpem_v2_recommend_donor_unavailable")
                || err2.contains("gpem_v2_recommend_donor_misconfigured"),
            "vertical must fail-closed: {err2}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn productive_acquire_live_gpem_seals_when_shei_available() {
        if std::path::Path::new("/home/yo/Projects/SHEI/research_python").is_dir() {
            let root = tmp("gpem-live-acq");
            let store = root.join("gpem-store");
            let wire = GpemV2RecommendDonorWire::new(
                store.clone(),
                vec![
                    "route".into(),
                    "capability_id".into(),
                    "prior_procedure".into(),
                ],
            )
            .unwrap();
            wire.seed_demo_traces().expect("seed live GPEM");
            let (package, donor) = acquire_procedure_selector_package(store).expect("live acquire");
            package.verify().unwrap();
            assert!(matches!(
                donor,
                DonorExecutionRecord::GpemV2LiveRecommend { .. }
            ));
            assert_eq!(package.donor_kind(), DonorKind::GpemV2Recommend);
            let receipt = run_from_package(package, donor).unwrap();
            receipt.verify().unwrap();
            assert_eq!(receipt.residency_decision(), &ResidencyDecision::Software {});
            assert!(!receipt.capability_ir_emitted());
            let _ = fs::remove_dir_all(&root);
        } else {
            // CI without SHEI: keep fail-closed vocabulary documented.
            let root = tmp("gpem-live-missing");
            let store = root.join("gpem-store");
            fs::create_dir_all(&store).unwrap();
            fs::write(store.join(".tidex_gpem_force_unavailable"), b"1").unwrap();
            let err = acquire_procedure_selector_package(store)
                .unwrap_err()
                .to_string();
            assert!(err.contains("gpem_v2_recommend_donor_unavailable"));
            let _ = fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn weights_admission_documents_receptor_hook_without_inventing_ir() {
        let package = package_with_residency_claims(
            "neuralizable_closed",
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
            ],
        );
        let receipt = run_from_package(
            package,
            DonorExecutionRecord::FixtureProcedureSelector {
                donor_locator: "fixture://weights-evidence".into(),
                gpem_wire_schema_documented: GpemV2RecommendDonorWire::SCHEMA.into(),
                gpem_observe_error: "unit_fixture_not_productive_path".into(),
            },
        )
        .unwrap();
        assert_eq!(receipt.residency_decision(), &ResidencyDecision::Weights {});
        assert!(!receipt.capability_ir_emitted());
        assert!(!receipt.receptor_entered());
        match receipt.terminal() {
            VerticalTerminal::ReceptorPathDocumented { hook, .. } => {
                assert_eq!(hook.admitted_candidate, ResidencyCandidate::Weights);
                assert!(!hook.invented_capability_ir);
                assert!(!hook.transplanted_weights);
                assert!(!hook.next_engines.is_empty());
            }
            other => panic!("expected receptor hook documentation, got {other:?}"),
        }
    }
}
