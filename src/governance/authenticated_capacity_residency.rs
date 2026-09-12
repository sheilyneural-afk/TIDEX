//! Paso 5 — residency / representation from authenticated capacity.
//!
//! # Invariant (HARD)
//!
//! `evidence → ResidencyDecision → CapabilityIR` — never `code → CapabilityIR`.
//! [`ResidencyDecision::Software`] ("don't put this in the LLM") is a valid outcome.
//!
//! # Scope
//!
//! Projects a sealed [`AuthenticatedCapacityPackage`] into
//! [`ResidencyFactInputs`], then reuses [`decide_from_fact_inputs`] /
//! `evaluate_fact_matrix`. CapabilityIR is **gated**, never invented from donor
//! source trees.

use crate::capability::authenticated_capacity::{
    AuthenticatedCapacityPackage, DonorKind, FunctionalContractStatus, PackageCompleteness,
    ResidencyHandoffSummary,
};
use crate::foundation::digest::{AuthenticatedCapacityDigest, Sha256Digest};
use crate::foundation::error::{BrainError, BrainResult};
use crate::governance::residency_decision::{
    decide_from_fact_inputs, EffectSemantics, ExecutionRequirements, ExternalStateSemantics,
    ObservabilitySemantics, ResidencyCandidate, ResidencyDecision, ResidencyFactInputs,
    ResidencySelectionBasis, ResidencyUnknownReason,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const OUTCOME_DOMAIN: &[u8] = b"TIDEX:AUTHENTICATED-CAPACITY-RESIDENCY:v1\0";
fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}

fn integrity(code: &str) -> BrainError {
    BrainError::Integrity(code.into())
}

/// Reserved Supported-contract claim_ids that attest residency facts.
/// Absent claims leave that dimension on the software-donor default (or unknown
/// when the package is too thin to decide).
pub mod claim_id {
    pub const REQUIREMENTS_CLOSED: &str = "residency.semantics.requirements.closed_computation";
    pub const REQUIREMENTS_BOUNDARY: &str = "residency.semantics.requirements.boundary_runtime";
    pub const REQUIREMENTS_SOFTWARE: &str = "residency.semantics.requirements.software_runtime";
    pub const EFFECTS_PURE: &str = "residency.semantics.effects.pure";
    pub const EFFECTS_BOUNDARY: &str = "residency.semantics.effects.boundary_effects";
    pub const EFFECTS_SOFTWARE: &str = "residency.semantics.effects.software_effects";
    pub const EXTERNAL_NONE: &str = "residency.semantics.external_state.none";
    pub const EXTERNAL_BOUNDARY: &str = "residency.semantics.external_state.boundary_managed";
    pub const EXTERNAL_SOFTWARE: &str = "residency.semantics.external_state.software_authoritative";
    pub const OBS_WEIGHT: &str = "residency.semantics.observability.weight_complete";
    pub const OBS_BOUNDARY: &str = "residency.semantics.observability.boundary_complete";
    pub const OBS_SOFTWARE: &str = "residency.semantics.observability.software_complete";
    pub const WEIGHTS_REPR_TRUE: &str = "residency.representability.weights.true";
    pub const WEIGHTS_REPR_FALSE: &str = "residency.representability.weights.false";
    pub const HYBRID_REPR_TRUE: &str = "residency.representability.hybrid.true";
    pub const HYBRID_REPR_FALSE: &str = "residency.representability.hybrid.false";
    pub const SOFTWARE_REPR_TRUE: &str = "residency.representability.software.true";
    pub const SOFTWARE_REPR_FALSE: &str = "residency.representability.software.false";
    pub const WEIGHTS_TARGET_OK: &str = "residency.target.weights.compatible";
    pub const WEIGHTS_TARGET_NO: &str = "residency.target.weights.incompatible";
    pub const HYBRID_TARGET_OK: &str = "residency.target.hybrid.compatible";
    pub const HYBRID_TARGET_NO: &str = "residency.target.hybrid.incompatible";
    pub const SOFTWARE_TARGET_OK: &str = "residency.target.software.compatible";
    pub const SOFTWARE_TARGET_NO: &str = "residency.target.software.incompatible";
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthenticatedCapacityResidencySchema {
    #[serde(rename = "tidex.authenticated_capacity_residency/v1")]
    V1,
}

/// Which projection path produced the fact inputs.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionBasis {
    /// Package too thin / incomplete → BoundedUnknown without inventing facts.
    InsufficientEvidence,
    /// Provenance-like / non-neuralizable donor defaults to Software residency.
    SoftwareDonorDefault,
    /// Explicit Supported residency.* contracts + causal interventions raised facts.
    ExplicitResidencyContracts,
}

/// Why CapabilityIR generation must stop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stop", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapabilityIrStopReason {
    SoftwareResidency,
    BoundedUnknown,
    Blocked,
    /// Hybrid was selected but causal+contract warrant for IR is missing.
    HybridNotWarranted,
}

/// Gate after residency: admit IR compilation only when warranted.
///
/// Never invents a [`crate::capability::capability_ir::CapabilityIr`]. Admission
/// only authorizes a later IR path that already has an authenticated IR source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "path", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapabilityIrPath {
    Stopped {
        reason: CapabilityIrStopReason,
    },
    /// Weights, or Hybrid with causal+contract warrant.
    Admitted {
        candidate: ResidencyCandidate,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatedCapacityResidencyOutcome {
    schema: AuthenticatedCapacityResidencySchema,
    capacity_key: String,
    package_sha256: AuthenticatedCapacityDigest,
    projection_basis: ProjectionBasis,
    decision: ResidencyDecision,
    selection_basis: Option<ResidencySelectionBasis>,
    capability_ir_path: CapabilityIrPath,
    obligations: Vec<String>,
    manifest_sha256: Sha256Digest,
}

impl AuthenticatedCapacityResidencyOutcome {
    pub fn capacity_key(&self) -> &str {
        &self.capacity_key
    }

    pub fn package_sha256(&self) -> &AuthenticatedCapacityDigest {
        &self.package_sha256
    }

    pub fn projection_basis(&self) -> ProjectionBasis {
        self.projection_basis
    }

    pub fn decision(&self) -> &ResidencyDecision {
        &self.decision
    }

    pub fn selection_basis(&self) -> Option<&ResidencySelectionBasis> {
        self.selection_basis.as_ref()
    }

    pub fn capability_ir_path(&self) -> &CapabilityIrPath {
        &self.capability_ir_path
    }

    pub fn obligations(&self) -> &[String] {
        &self.obligations
    }

    pub fn manifest_sha256(&self) -> &Sha256Digest {
        &self.manifest_sha256
    }

    /// Software (and non-admitted) paths never yield CapabilityIR.
    pub fn admits_capability_ir(&self) -> bool {
        matches!(self.capability_ir_path, CapabilityIrPath::Admitted { .. })
    }

    fn calculate_digest(&self) -> BrainResult<Sha256Digest> {
        let mut unsigned = self.clone();
        unsigned.manifest_sha256 = Sha256Digest::zero();
        Ok(Sha256Digest::digest_domain(OUTCOME_DOMAIN, &serde_json::to_vec(&unsigned)?))
    }

    pub fn verify(&self) -> BrainResult<()> {
        if self.schema != AuthenticatedCapacityResidencySchema::V1 {
            return Err(invalid("authenticated_capacity_residency_schema_unsupported"));
        }
        if self.manifest_sha256 == Sha256Digest::zero()
            || self.calculate_digest()? != self.manifest_sha256
        {
            return Err(integrity("authenticated_capacity_residency_digest_mismatch"));
        }
        // HARD: Software must never admit IR.
        if matches!(self.decision, ResidencyDecision::Software {}) && self.admits_capability_ir() {
            return Err(integrity("software_residency_must_not_admit_capability_ir"));
        }
        Ok(())
    }

    pub fn persist(&self, private_root: &Path) -> BrainResult<PathBuf> {
        self.verify()?;
        let destination = outcome_path(private_root, &self.manifest_sha256);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(self)?;
        if destination.exists() {
            let existing = std::fs::read(&destination)?;
            if existing != bytes {
                return Err(integrity("authenticated_capacity_residency_immutable_conflict"));
            }
        } else {
            std::fs::write(&destination, bytes)?;
        }
        Ok(destination)
    }
}

fn outcome_path(private_root: &Path, digest: &Sha256Digest) -> PathBuf {
    private_root
        .join("state/residency_decision/from_authenticated_capacity/by-sha")
        .join(format!("{}.json", digest.as_str()))
}

/// Decide residency from a sealed authenticated capacity package.
///
/// Fail-closed: thin packages → [`ResidencyDecision::BoundedUnknown`];
/// provenance-like / non-neuralizable donors → [`ResidencyDecision::Software`];
/// Weights/Hybrid only when Supported `residency.*` contracts plus causal
/// interventions warrant those candidates through [`decide_from_fact_inputs`].
pub fn decide_from_authenticated_capacity(
    package: &AuthenticatedCapacityPackage,
) -> BrainResult<AuthenticatedCapacityResidencyOutcome> {
    package.verify()?;
    let handoff = package.residency_handoff_summary()?;
    decide_from_handoff(package, &handoff)
}

fn decide_from_handoff(
    package: &AuthenticatedCapacityPackage,
    handoff: &ResidencyHandoffSummary,
) -> BrainResult<AuthenticatedCapacityResidencyOutcome> {
    if handoff.package_sha256 != *package.manifest_sha256()
        || handoff.capacity_key != package.capacity_key()
    {
        return Err(integrity("residency_handoff_package_mismatch"));
    }

    let (projection_basis, decision, selection_basis) = if package.completeness()
        == PackageCompleteness::Insufficient
        || package.observations().is_empty()
    {
        (
            ProjectionBasis::InsufficientEvidence,
            ResidencyDecision::BoundedUnknown {
                reason: ResidencyUnknownReason::SemanticDimensions {
                    dimensions: [
                        crate::governance::residency_decision::ResidencyDimension::Requirements,
                        crate::governance::residency_decision::ResidencyDimension::Effects,
                        crate::governance::residency_decision::ResidencyDimension::ExternalState,
                        crate::governance::residency_decision::ResidencyDimension::Observability,
                    ]
                    .into_iter()
                    .collect(),
                },
                unresolved_obligations: BTreeSet::new(),
            },
            None,
        )
    } else if let Some(inputs) = try_explicit_residency_inputs(package)? {
        let (decision, basis) = decide_from_fact_inputs(&inputs)?;
        (ProjectionBasis::ExplicitResidencyContracts, decision, Some(basis))
    } else if matches!(package.donor_kind(), DonorKind::MeasuredClosedLinearMap) {
        // Neuralizable donor without explicit residency.* contracts: fail closed.
        // Do not invent Software (or Weights) from donor kind alone.
        (
            ProjectionBasis::InsufficientEvidence,
            ResidencyDecision::BoundedUnknown {
                reason: ResidencyUnknownReason::SemanticDimensions {
                    dimensions: [
                        crate::governance::residency_decision::ResidencyDimension::Requirements,
                        crate::governance::residency_decision::ResidencyDimension::Effects,
                        crate::governance::residency_decision::ResidencyDimension::ExternalState,
                        crate::governance::residency_decision::ResidencyDimension::Observability,
                    ]
                    .into_iter()
                    .collect(),
                },
                unresolved_obligations: BTreeSet::new(),
            },
            None,
        )
    } else {
        // Provenance-like / non-neuralizable software donors: Software is valid.
        let inputs = software_donor_default_inputs(package.donor_kind());
        let (decision, basis) = decide_from_fact_inputs(&inputs)?;
        (ProjectionBasis::SoftwareDonorDefault, decision, Some(basis))
    };

    let capability_ir_path = gate_capability_ir_path(package, &decision);
    let mut outcome = AuthenticatedCapacityResidencyOutcome {
        schema: AuthenticatedCapacityResidencySchema::V1,
        capacity_key: package.capacity_key().to_string(),
        package_sha256: package.manifest_sha256().clone(),
        projection_basis,
        decision,
        selection_basis,
        capability_ir_path,
        obligations: package.obligations().to_vec(),
        manifest_sha256: Sha256Digest::zero(),
    };
    outcome.manifest_sha256 = outcome.calculate_digest()?;
    outcome.verify()?;
    Ok(outcome)
}

fn software_donor_default_inputs(donor_kind: DonorKind) -> ResidencyFactInputs {
    // Fixture procedure selector and GPEM recommend are algorithmic software:
    // do not put them in the LLM / weights without explicit neural evidence.
    let _ = donor_kind;
    ResidencyFactInputs {
        requirements: ExecutionRequirements::SoftwareRuntime,
        effects: EffectSemantics::SoftwareEffects,
        external_state: ExternalStateSemantics::SoftwareAuthoritative,
        observability: ObservabilitySemantics::SoftwareComplete,
        weights_representable: Some(false),
        hybrid_representable: Some(false),
        software_representable: Some(true),
        weights_target_compatible: Some(false),
        hybrid_target_compatible: Some(false),
        software_target_compatible: Some(true),
    }
}

fn supported_ids(package: &AuthenticatedCapacityPackage) -> BTreeSet<&str> {
    package
        .contracts()
        .iter()
        .filter(|claim| claim.status == FunctionalContractStatus::Supported)
        .map(|claim| claim.claim_id.as_str())
        .collect()
}

fn try_explicit_residency_inputs(
    package: &AuthenticatedCapacityPackage,
) -> BrainResult<Option<ResidencyFactInputs>> {
    let ids = supported_ids(package);
    let has_any_residency_claim = ids.iter().any(|id| id.starts_with("residency."));
    if !has_any_residency_claim {
        return Ok(None);
    }
    // Causal warrant required to raise above software default.
    if package.interventions().is_empty() {
        return Ok(None);
    }
    if package.completeness() != PackageCompleteness::SufficientForResidencyHandoff {
        return Ok(None);
    }

    let requirements = pick_exclusive(
        &ids,
        &[
            (claim_id::REQUIREMENTS_CLOSED, ExecutionRequirements::ClosedComputation),
            (claim_id::REQUIREMENTS_BOUNDARY, ExecutionRequirements::BoundaryRuntime),
            (claim_id::REQUIREMENTS_SOFTWARE, ExecutionRequirements::SoftwareRuntime),
        ],
    )?;
    let effects = pick_exclusive(
        &ids,
        &[
            (claim_id::EFFECTS_PURE, EffectSemantics::Pure),
            (claim_id::EFFECTS_BOUNDARY, EffectSemantics::BoundaryEffects),
            (claim_id::EFFECTS_SOFTWARE, EffectSemantics::SoftwareEffects),
        ],
    )?;
    let external_state = pick_exclusive(
        &ids,
        &[
            (claim_id::EXTERNAL_NONE, ExternalStateSemantics::None),
            (claim_id::EXTERNAL_BOUNDARY, ExternalStateSemantics::BoundaryManaged),
            (claim_id::EXTERNAL_SOFTWARE, ExternalStateSemantics::SoftwareAuthoritative),
        ],
    )?;
    let observability = pick_exclusive(
        &ids,
        &[
            (claim_id::OBS_WEIGHT, ObservabilitySemantics::WeightComplete),
            (claim_id::OBS_BOUNDARY, ObservabilitySemantics::BoundaryComplete),
            (claim_id::OBS_SOFTWARE, ObservabilitySemantics::SoftwareComplete),
        ],
    )?;

    // If any semantic dimension is attested, all four must be attested or we
    // fail closed to BoundedUnknown via Unknown dimensions.
    let (requirements, effects, external_state, observability) =
        match (requirements, effects, external_state, observability) {
            (Some(r), Some(e), Some(x), Some(o)) => (r, e, x, o),
            _ => {
                return Ok(Some(ResidencyFactInputs {
                    requirements: ExecutionRequirements::Unknown,
                    effects: EffectSemantics::Unknown,
                    external_state: ExternalStateSemantics::Unknown,
                    observability: ObservabilitySemantics::Unknown,
                    weights_representable: None,
                    hybrid_representable: None,
                    software_representable: None,
                    weights_target_compatible: None,
                    hybrid_target_compatible: None,
                    software_target_compatible: None,
                }));
            }
        };

    Ok(Some(ResidencyFactInputs {
        requirements,
        effects,
        external_state,
        observability,
        weights_representable: pick_bool(
            &ids,
            claim_id::WEIGHTS_REPR_TRUE,
            claim_id::WEIGHTS_REPR_FALSE,
        )?,
        hybrid_representable: pick_bool(
            &ids,
            claim_id::HYBRID_REPR_TRUE,
            claim_id::HYBRID_REPR_FALSE,
        )?,
        software_representable: pick_bool(
            &ids,
            claim_id::SOFTWARE_REPR_TRUE,
            claim_id::SOFTWARE_REPR_FALSE,
        )?,
        weights_target_compatible: pick_bool(
            &ids,
            claim_id::WEIGHTS_TARGET_OK,
            claim_id::WEIGHTS_TARGET_NO,
        )?,
        hybrid_target_compatible: pick_bool(
            &ids,
            claim_id::HYBRID_TARGET_OK,
            claim_id::HYBRID_TARGET_NO,
        )?,
        software_target_compatible: pick_bool(
            &ids,
            claim_id::SOFTWARE_TARGET_OK,
            claim_id::SOFTWARE_TARGET_NO,
        )?,
    }))
}

fn pick_exclusive<T: Copy>(ids: &BTreeSet<&str>, options: &[(&str, T)]) -> BrainResult<Option<T>> {
    let mut found = None;
    for (id, value) in options {
        if ids.contains(id) {
            if found.is_some() {
                return Err(invalid("residency_semantics_claim_conflict"));
            }
            found = Some(*value);
        }
    }
    Ok(found)
}

fn pick_bool(ids: &BTreeSet<&str>, true_id: &str, false_id: &str) -> BrainResult<Option<bool>> {
    match (ids.contains(true_id), ids.contains(false_id)) {
        (true, true) => Err(invalid("residency_bool_claim_conflict")),
        (true, false) => Ok(Some(true)),
        (false, true) => Ok(Some(false)),
        (false, false) => Ok(None),
    }
}

fn gate_capability_ir_path(
    package: &AuthenticatedCapacityPackage,
    decision: &ResidencyDecision,
) -> CapabilityIrPath {
    match decision {
        ResidencyDecision::Software {} => CapabilityIrPath::Stopped {
            reason: CapabilityIrStopReason::SoftwareResidency,
        },
        ResidencyDecision::BoundedUnknown { .. } => CapabilityIrPath::Stopped {
            reason: CapabilityIrStopReason::BoundedUnknown,
        },
        ResidencyDecision::Blocked { .. } => CapabilityIrPath::Stopped {
            reason: CapabilityIrStopReason::Blocked,
        },
        ResidencyDecision::Weights {} => CapabilityIrPath::Admitted {
            candidate: ResidencyCandidate::Weights,
        },
        ResidencyDecision::Hybrid {} => {
            // Hybrid-and-warranted: need causal intervention + ≥2 supported contracts.
            let supported = package
                .contracts()
                .iter()
                .filter(|c| c.status == FunctionalContractStatus::Supported)
                .count();
            if !package.interventions().is_empty() && supported >= 2 {
                CapabilityIrPath::Admitted {
                    candidate: ResidencyCandidate::Hybrid,
                }
            } else {
                CapabilityIrPath::Stopped {
                    reason: CapabilityIrStopReason::HybridNotWarranted,
                }
            }
        }
    }
}

/// Explicit non-goal helper used by tests and callers: Software never yields IR.
pub fn capability_ir_from_outcome(
    outcome: &AuthenticatedCapacityResidencyOutcome,
) -> BrainResult<Option<()>> {
    outcome.verify()?;
    match outcome.capability_ir_path() {
        CapabilityIrPath::Stopped { .. } => Ok(None),
        CapabilityIrPath::Admitted { .. } => {
            // Paso 5 admits the path only; it does not synthesize CapabilityIR
            // from donor code or from the capacity package itself.
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::authenticated_capacity::{
        seal_fixture_procedure_selector_capacity, CapacityObservation, CapacityProvenance,
        DonorAction, FunctionalContractClaim, SelectorStimulus,
    };
    use crate::foundation::identity::ObservationId;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn tmp(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "tidex-ac-residency-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn thin_package() -> AuthenticatedCapacityPackage {
        let stim = SelectorStimulus::new(
            "route:thin",
            Vec::new(),
            vec!["proc.alpha".into(), "proc.beta".into()],
        )
        .unwrap();
        let obs = CapacityObservation::seal(
            ObservationId::parse("obs.thin-01").unwrap(),
            stim,
            DonorAction::explore(),
            None,
        )
        .unwrap();
        AuthenticatedCapacityPackage::seal(
            "thin_capacity",
            DonorKind::FixtureProcedureSelector,
            CapacityProvenance::default(),
            vec![obs],
            Vec::new(),
            Vec::new(),
            vec!["need_more_observations".into()],
        )
        .unwrap()
    }

    fn package_with_residency_claims(
        capacity_key: &str,
        claim_ids: &[&str],
    ) -> AuthenticatedCapacityPackage {
        // Reuse fixture campaign structure (select+explore+intervention) then
        // replace contracts with residency attestations + keep functional ones.
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
    fn fixture_procedure_selector_decides_software_and_stops_ir() {
        let package = seal_fixture_procedure_selector_capacity(
            "procedure_selector_or_explore",
            CapacityProvenance::default(),
        )
        .unwrap();
        let outcome = decide_from_authenticated_capacity(&package).unwrap();
        assert_eq!(outcome.projection_basis(), ProjectionBasis::SoftwareDonorDefault);
        assert_eq!(outcome.decision(), &ResidencyDecision::Software {});
        assert!(!outcome.admits_capability_ir());
        assert_eq!(
            outcome.capability_ir_path(),
            &CapabilityIrPath::Stopped {
                reason: CapabilityIrStopReason::SoftwareResidency,
            }
        );
        assert!(capability_ir_from_outcome(&outcome).unwrap().is_none());
    }

    #[test]
    fn thin_package_is_bounded_unknown_without_ir() {
        let package = thin_package();
        assert_eq!(package.completeness(), PackageCompleteness::Insufficient);
        let outcome = decide_from_authenticated_capacity(&package).unwrap();
        assert_eq!(outcome.projection_basis(), ProjectionBasis::InsufficientEvidence);
        assert!(matches!(outcome.decision(), ResidencyDecision::BoundedUnknown { .. }));
        assert!(!outcome.admits_capability_ir());
    }

    #[test]
    fn weights_only_with_sufficient_causal_and_contract_evidence() {
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
        let outcome = decide_from_authenticated_capacity(&package).unwrap();
        assert_eq!(outcome.projection_basis(), ProjectionBasis::ExplicitResidencyContracts);
        assert_eq!(outcome.decision(), &ResidencyDecision::Weights {});
        assert!(outcome.admits_capability_ir());
        assert_eq!(
            outcome.capability_ir_path(),
            &CapabilityIrPath::Admitted {
                candidate: ResidencyCandidate::Weights,
            }
        );
        // Path admitted, but Paso 5 still does not invent IR bytes.
        assert!(capability_ir_from_outcome(&outcome).unwrap().is_none());
    }

    #[test]
    fn hybrid_admitted_only_when_warranted() {
        let package = package_with_residency_claims(
            "boundary_hybrid",
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
            ],
        );
        let outcome = decide_from_authenticated_capacity(&package).unwrap();
        assert_eq!(outcome.decision(), &ResidencyDecision::Hybrid {});
        assert!(outcome.admits_capability_ir());
    }

    #[test]
    fn software_outcome_never_produces_capability_ir_fields() {
        let package = seal_fixture_procedure_selector_capacity(
            "procedure_selector_or_explore",
            CapacityProvenance::default(),
        )
        .unwrap();
        let outcome = decide_from_authenticated_capacity(&package).unwrap();
        let encoded = serde_json::to_value(&outcome).unwrap();
        assert!(encoded.get("capability_ir").is_none());
        assert_eq!(encoded["capability_ir_path"]["path"], "stopped");
        assert_eq!(encoded["decision"]["decision"], "software");
    }

    #[test]
    fn outcome_persists_and_rejects_software_ir_admission_tamper() {
        let root = tmp("persist");
        let package = seal_fixture_procedure_selector_capacity(
            "procedure_selector_or_explore",
            CapacityProvenance::default(),
        )
        .unwrap();
        let outcome = decide_from_authenticated_capacity(&package).unwrap();
        let path = outcome.persist(&root).unwrap();
        assert!(path.exists());

        let mut tampered = outcome.clone();
        tampered.capability_ir_path = CapabilityIrPath::Admitted {
            candidate: ResidencyCandidate::Weights,
        };
        tampered.manifest_sha256 = tampered.calculate_digest().unwrap();
        assert!(tampered.verify().is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn partial_explicit_claims_fail_closed_to_bounded_unknown() {
        let package = package_with_residency_claims(
            "partial_residency_claims",
            &[claim_id::REQUIREMENTS_CLOSED, claim_id::WEIGHTS_REPR_TRUE],
        );
        let outcome = decide_from_authenticated_capacity(&package).unwrap();
        assert!(matches!(outcome.decision(), ResidencyDecision::BoundedUnknown { .. }));
        assert!(!outcome.admits_capability_ir());
    }
}
