//! Paso 4 — functional acquisition of external software (thin closed slice).
//!
//! # Invariant (HARD)
//!
//! `evidence → ResidencyDecision → CapabilityIR` — never `code → CapabilityIR`.
//! This module seals an **authenticated capacity** package from observed donor
//! behavior, interventions/counterfactuals, and functional contracts. It does
//! **not** emit [`CapabilityIR`](crate::capability::capability_ir::CapabilityIR).
//!
//! # Scope
//!
//! Vertical: *select the most appropriate historical procedure given context +
//! prior results, or explore*. Fixture donor simulates that behavior for tests;
//! [`GpemV2RecommendDonorWire`] documents the real-GPEM shape without executing it.
//!
//! # Provenance
//!
//! Optional link to an `acquire_system` capture receipt. Tree bytes remain
//! provenance only; capacity claims require sealed observations.

use crate::foundation::authority::{write_or_verify_immutable, PrivateFileReference};
use crate::foundation::digest::{AuthenticatedCapacityDigest, CaptureReceiptDigest, Sha256Digest};
use crate::foundation::error::{BrainError, BrainResult};
use crate::foundation::identity::{AcquisitionId, ObservationId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const PACKAGE_DOMAIN: &[u8] = b"TIDEX:AUTHENTICATED-CAPACITY:v1\0";
const OBSERVATION_DOMAIN: &[u8] = b"TIDEX:CAPACITY-OBSERVATION:v1\0";
const INTERVENTION_DOMAIN: &[u8] = b"TIDEX:CAPACITY-INTERVENTION:v1\0";
const MAX_OBSERVATIONS: usize = 4_096;
const MAX_INTERVENTIONS: usize = 4_096;
const MAX_CONTRACTS: usize = 256;
const MAX_OBLIGATIONS: usize = 256;
const MAX_STRING_BYTES: usize = 4_096;
const MAX_PRIOR_RESULTS: usize = 256;
const MAX_CANDIDATES: usize = 256;

fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}

fn integrity(code: &str) -> BrainError {
    BrainError::Integrity(code.into())
}

fn validate_label(value: &str, label: &str) -> BrainResult<()> {
    if value.is_empty()
        || value.len() > 128
        || value.starts_with('.')
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b'.'
        })
        || value.split('.').any(str::is_empty)
    {
        return Err(invalid(&format!("{label}_invalid")));
    }
    Ok(())
}

fn validate_bounded_text(value: &str, label: &str) -> BrainResult<()> {
    if value.is_empty() || value.len() > MAX_STRING_BYTES {
        return Err(invalid(&format!("{label}_invalid")));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthenticatedCapacitySchema {
    #[serde(rename = "tidex.authenticated_capacity/v1")]
    V1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DonorKind {
    #[serde(rename = "fixture_procedure_selector")]
    FixtureProcedureSelector,
    /// Wire-only placeholder for GPEM v2 `recommend` (not executed in Paso 4).
    #[serde(rename = "gpem_v2_recommend")]
    GpemV2Recommend,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageCompleteness {
    /// Enough sealed observations + contracts for ResidencyDecision handoff (Paso 5).
    SufficientForResidencyHandoff,
    /// Fail-closed: package authenticates evidence but is not yet decisive.
    Insufficient,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DonorActionKind {
    Select,
    Explore,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PriorProcedureResult {
    pub procedure_id: String,
    /// Canonical outcome token observed for that procedure (e.g. success/fail score band).
    pub outcome: String,
}

impl PriorProcedureResult {
    pub fn new(procedure_id: impl Into<String>, outcome: impl Into<String>) -> BrainResult<Self> {
        let procedure_id = procedure_id.into();
        let outcome = outcome.into();
        validate_label(&procedure_id, "procedure_id")?;
        validate_bounded_text(&outcome, "prior_outcome")?;
        Ok(Self {
            procedure_id,
            outcome,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectorStimulus {
    pub context: String,
    pub prior_results: Vec<PriorProcedureResult>,
    pub candidate_procedures: Vec<String>,
}

impl SelectorStimulus {
    pub fn new(
        context: impl Into<String>,
        prior_results: Vec<PriorProcedureResult>,
        candidate_procedures: Vec<String>,
    ) -> BrainResult<Self> {
        let context = context.into();
        validate_bounded_text(&context, "selector_context")?;
        if prior_results.len() > MAX_PRIOR_RESULTS {
            return Err(invalid("prior_results_limit_exceeded"));
        }
        if candidate_procedures.is_empty() || candidate_procedures.len() > MAX_CANDIDATES {
            return Err(invalid("candidate_procedures_invalid"));
        }
        let mut seen = BTreeSet::new();
        for candidate in &candidate_procedures {
            validate_label(candidate, "candidate_procedure_id")?;
            if !seen.insert(candidate.clone()) {
                return Err(invalid("candidate_procedures_duplicate"));
            }
        }
        Ok(Self {
            context,
            prior_results,
            candidate_procedures,
        })
    }

    pub fn canonical_digest(&self) -> BrainResult<Sha256Digest> {
        Ok(Sha256Digest::digest_domain(
            b"TIDEX:SELECTOR-STIMULUS:v1\0",
            &serde_json::to_vec(self)?,
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DonorAction {
    pub kind: DonorActionKind,
    /// Present iff `kind == Select`.
    pub selected_procedure_id: Option<String>,
}

impl DonorAction {
    pub fn select(procedure_id: impl Into<String>) -> BrainResult<Self> {
        let selected_procedure_id = procedure_id.into();
        validate_label(&selected_procedure_id, "selected_procedure_id")?;
        Ok(Self {
            kind: DonorActionKind::Select,
            selected_procedure_id: Some(selected_procedure_id),
        })
    }

    pub fn explore() -> Self {
        Self {
            kind: DonorActionKind::Explore,
            selected_procedure_id: None,
        }
    }

    pub fn verify(&self) -> BrainResult<()> {
        match self.kind {
            DonorActionKind::Select => {
                let id = self
                    .selected_procedure_id
                    .as_deref()
                    .ok_or_else(|| integrity("donor_action_select_missing_id"))?;
                validate_label(id, "selected_procedure_id")?;
            }
            DonorActionKind::Explore => {
                if self.selected_procedure_id.is_some() {
                    return Err(integrity("donor_action_explore_has_selection"));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapacityObservation {
    pub observation_id: ObservationId,
    pub stimulus: SelectorStimulus,
    pub stimulus_sha256: Sha256Digest,
    pub donor_action: DonorAction,
    /// Optional measured outcome after the donor acted (may be absent for pure choice trials).
    pub measured_outcome: Option<String>,
    observation_sha256: Sha256Digest,
}

impl CapacityObservation {
    pub fn seal(
        observation_id: ObservationId,
        stimulus: SelectorStimulus,
        donor_action: DonorAction,
        measured_outcome: Option<String>,
    ) -> BrainResult<Self> {
        donor_action.verify()?;
        if let Some(outcome) = &measured_outcome {
            validate_bounded_text(outcome, "measured_outcome")?;
        }
        if let DonorActionKind::Select = donor_action.kind {
            let selected = donor_action
                .selected_procedure_id
                .as_ref()
                .ok_or_else(|| integrity("donor_action_select_missing_id"))?;
            if !stimulus.candidate_procedures.contains(selected) {
                return Err(invalid("selected_procedure_not_in_candidates"));
            }
        }
        let stimulus_sha256 = stimulus.canonical_digest()?;
        let mut observation = Self {
            observation_id,
            stimulus,
            stimulus_sha256,
            donor_action,
            measured_outcome,
            observation_sha256: Sha256Digest::zero(),
        };
        observation.observation_sha256 = observation.calculate_digest()?;
        observation.verify()?;
        Ok(observation)
    }

    pub fn observation_sha256(&self) -> &Sha256Digest {
        &self.observation_sha256
    }

    pub fn verify(&self) -> BrainResult<()> {
        self.donor_action.verify()?;
        if self.stimulus.canonical_digest()? != self.stimulus_sha256 {
            return Err(integrity("capacity_observation_stimulus_digest_mismatch"));
        }
        if self.observation_sha256 == Sha256Digest::zero()
            || self.calculate_digest()? != self.observation_sha256
        {
            return Err(integrity("capacity_observation_digest_mismatch"));
        }
        Ok(())
    }

    fn calculate_digest(&self) -> BrainResult<Sha256Digest> {
        let mut unsigned = self.clone();
        unsigned.observation_sha256 = Sha256Digest::zero();
        Ok(Sha256Digest::digest_domain(OBSERVATION_DOMAIN, &serde_json::to_vec(&unsigned)?))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterventionKind {
    /// Remove the historically best prior from the stimulus and re-observe.
    AblateBestPrior,
    /// Force an empty prior-results list (cold start) and re-observe.
    ColdStartPriors,
    /// Swap outcome labels of the top two priors and re-observe.
    SwapTopPriorOutcomes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapacityIntervention {
    pub kind: InterventionKind,
    pub baseline_observation_sha256: Sha256Digest,
    pub intervened_observation: CapacityObservation,
    pub note: String,
    intervention_sha256: Sha256Digest,
}

impl CapacityIntervention {
    pub fn seal(
        kind: InterventionKind,
        baseline: &CapacityObservation,
        intervened_observation: CapacityObservation,
        note: impl Into<String>,
    ) -> BrainResult<Self> {
        baseline.verify()?;
        intervened_observation.verify()?;
        let note = note.into();
        validate_bounded_text(&note, "intervention_note")?;
        let mut intervention = Self {
            kind,
            baseline_observation_sha256: baseline.observation_sha256().clone(),
            intervened_observation,
            note,
            intervention_sha256: Sha256Digest::zero(),
        };
        intervention.intervention_sha256 = intervention.calculate_digest()?;
        intervention.verify()?;
        Ok(intervention)
    }

    pub fn intervention_sha256(&self) -> &Sha256Digest {
        &self.intervention_sha256
    }

    pub fn verify(&self) -> BrainResult<()> {
        self.intervened_observation.verify()?;
        if self.intervention_sha256 == Sha256Digest::zero()
            || self.calculate_digest()? != self.intervention_sha256
        {
            return Err(integrity("capacity_intervention_digest_mismatch"));
        }
        Ok(())
    }

    fn calculate_digest(&self) -> BrainResult<Sha256Digest> {
        let mut unsigned = self.clone();
        unsigned.intervention_sha256 = Sha256Digest::zero();
        Ok(Sha256Digest::digest_domain(
            INTERVENTION_DOMAIN,
            &serde_json::to_vec(&unsigned)?,
        ))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FunctionalContractStatus {
    Supported,
    Unsupported,
    InsufficientEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FunctionalContractClaim {
    pub claim_id: String,
    /// Human-readable functional predicate (not CapabilityIR).
    pub predicate: String,
    pub status: FunctionalContractStatus,
    pub supporting_observation_sha256: Vec<Sha256Digest>,
}

impl FunctionalContractClaim {
    pub fn new(
        claim_id: impl Into<String>,
        predicate: impl Into<String>,
        status: FunctionalContractStatus,
        supporting_observation_sha256: Vec<Sha256Digest>,
    ) -> BrainResult<Self> {
        let claim_id = claim_id.into();
        let predicate = predicate.into();
        validate_label(&claim_id, "functional_claim_id")?;
        validate_bounded_text(&predicate, "functional_predicate")?;
        if supporting_observation_sha256.len() > MAX_OBSERVATIONS {
            return Err(invalid("functional_claim_support_limit_exceeded"));
        }
        Ok(Self {
            claim_id,
            predicate,
            status,
            supporting_observation_sha256,
        })
    }
}

/// Provenance that may accompany sealed capacity evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct CapacityProvenance {
    pub acquisition_id: Option<AcquisitionId>,
    pub capture_receipt_sha256: Option<CaptureReceiptDigest>,
    /// Free-form donor locator for future GPEM wiring (never treated as semantics).
    pub donor_locator: Option<String>,
}

impl CapacityProvenance {
    pub fn verify(&self) -> BrainResult<()> {
        if let Some(locator) = &self.donor_locator {
            validate_bounded_text(locator, "donor_locator")?;
        }
        if let Some(receipt) = &self.capture_receipt_sha256 {
            if receipt.is_draft() {
                return Err(integrity("capacity_provenance_capture_receipt_draft"));
            }
        }
        Ok(())
    }
}

/// Wire shape for a future real GPEM v2 recommend donor (Paso 4 documents only).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GpemV2RecommendDonorWire {
    pub schema: String,
    pub store_root: PathBuf,
    pub context_keys: Vec<String>,
}

impl GpemV2RecommendDonorWire {
    pub const SCHEMA: &'static str = "tidex.donor.gpem_v2_recommend/v1";

    pub fn new(store_root: PathBuf, context_keys: Vec<String>) -> BrainResult<Self> {
        if context_keys.is_empty() || context_keys.len() > 64 {
            return Err(invalid("gpem_donor_context_keys_invalid"));
        }
        for key in &context_keys {
            validate_label(key, "gpem_context_key")?;
        }
        Ok(Self {
            schema: Self::SCHEMA.into(),
            store_root,
            context_keys,
        })
    }

    /// Fail-closed until a later pass wires live GPEM execution.
    pub fn observe(&self, _stimulus: &SelectorStimulus) -> BrainResult<DonorAction> {
        Err(invalid("gpem_v2_recommend_donor_not_wired"))
    }
}

/// Fixture donor: prefer the historically best successful candidate, else explore.
#[derive(Debug, Clone, Default)]
pub struct FixtureProcedureSelector;

impl FixtureProcedureSelector {
    pub fn observe(&self, stimulus: &SelectorStimulus) -> BrainResult<DonorAction> {
        let mut best: Option<(&str, i64)> = None;
        for prior in &stimulus.prior_results {
            if !stimulus
                .candidate_procedures
                .iter()
                .any(|candidate| candidate == &prior.procedure_id)
            {
                continue;
            }
            let score = outcome_rank(&prior.outcome);
            match best {
                None => best = Some((prior.procedure_id.as_str(), score)),
                Some((_, best_score)) if score > best_score => {
                    best = Some((prior.procedure_id.as_str(), score));
                }
                _ => {}
            }
        }
        match best {
            Some((procedure_id, score)) if score > 0 => DonorAction::select(procedure_id),
            _ => Ok(DonorAction::explore()),
        }
    }
}

fn outcome_rank(outcome: &str) -> i64 {
    match outcome {
        "success" | "ok" | "pass" => 2,
        "partial" => 1,
        "fail" | "error" => -1,
        _ => 0,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatedCapacityPackage {
    schema: AuthenticatedCapacitySchema,
    /// Stable functional name of the observed capacity (not CapabilityIR).
    capacity_key: String,
    donor_kind: DonorKind,
    provenance: CapacityProvenance,
    observations: Vec<CapacityObservation>,
    interventions: Vec<CapacityIntervention>,
    contracts: Vec<FunctionalContractClaim>,
    completeness: PackageCompleteness,
    obligations: Vec<String>,
    manifest_sha256: AuthenticatedCapacityDigest,
}

impl AuthenticatedCapacityPackage {
    pub fn seal(
        capacity_key: impl Into<String>,
        donor_kind: DonorKind,
        provenance: CapacityProvenance,
        observations: Vec<CapacityObservation>,
        interventions: Vec<CapacityIntervention>,
        contracts: Vec<FunctionalContractClaim>,
        obligations: Vec<String>,
    ) -> BrainResult<Self> {
        let capacity_key = capacity_key.into();
        validate_label(&capacity_key, "capacity_key")?;
        provenance.verify()?;
        if observations.is_empty() || observations.len() > MAX_OBSERVATIONS {
            return Err(invalid("authenticated_capacity_observations_invalid"));
        }
        if interventions.len() > MAX_INTERVENTIONS {
            return Err(invalid("authenticated_capacity_interventions_limit"));
        }
        if contracts.len() > MAX_CONTRACTS {
            return Err(invalid("authenticated_capacity_contracts_limit"));
        }
        if obligations.len() > MAX_OBLIGATIONS {
            return Err(invalid("authenticated_capacity_obligations_limit"));
        }
        for obligation in &obligations {
            validate_bounded_text(obligation, "capacity_obligation")?;
        }

        let mut observation_digests = BTreeSet::new();
        for observation in &observations {
            observation.verify()?;
            if !observation_digests.insert(observation.observation_sha256().clone()) {
                return Err(invalid("authenticated_capacity_observation_duplicate"));
            }
        }
        for intervention in &interventions {
            intervention.verify()?;
            if !observation_digests.contains(&intervention.baseline_observation_sha256) {
                return Err(invalid("intervention_baseline_unknown"));
            }
        }
        for contract in &contracts {
            validate_label(&contract.claim_id, "functional_claim_id")?;
            validate_bounded_text(&contract.predicate, "functional_predicate")?;
            for support in &contract.supporting_observation_sha256 {
                if !observation_digests.contains(support)
                    && !interventions
                        .iter()
                        .any(|item| item.intervened_observation.observation_sha256() == support)
                {
                    return Err(invalid("functional_claim_support_unknown"));
                }
            }
        }

        let completeness = derive_completeness(&observations, &interventions, &contracts);
        let mut package = Self {
            schema: AuthenticatedCapacitySchema::V1,
            capacity_key,
            donor_kind,
            provenance,
            observations,
            interventions,
            contracts,
            completeness,
            obligations,
            manifest_sha256: AuthenticatedCapacityDigest::draft_marker(),
        };
        package.manifest_sha256 = package.calculate_digest()?;
        package.verify()?;
        Ok(package)
    }

    pub fn capacity_key(&self) -> &str {
        &self.capacity_key
    }

    pub fn donor_kind(&self) -> DonorKind {
        self.donor_kind
    }

    pub fn provenance(&self) -> &CapacityProvenance {
        &self.provenance
    }

    pub fn observations(&self) -> &[CapacityObservation] {
        &self.observations
    }

    pub fn interventions(&self) -> &[CapacityIntervention] {
        &self.interventions
    }

    pub fn contracts(&self) -> &[FunctionalContractClaim] {
        &self.contracts
    }

    pub fn completeness(&self) -> PackageCompleteness {
        self.completeness
    }

    pub fn obligations(&self) -> &[String] {
        &self.obligations
    }

    pub fn manifest_sha256(&self) -> &AuthenticatedCapacityDigest {
        &self.manifest_sha256
    }

    /// Explicit non-goal: this package never carries CapabilityIR.
    pub fn contains_capability_ir_fields(&self) -> bool {
        false
    }

    pub fn verify(&self) -> BrainResult<()> {
        self.provenance.verify()?;
        if self.observations.is_empty() {
            return Err(integrity("authenticated_capacity_observations_empty"));
        }
        for observation in &self.observations {
            observation.verify()?;
        }
        for intervention in &self.interventions {
            intervention.verify()?;
        }
        if self.manifest_sha256.is_draft() || self.calculate_digest()? != self.manifest_sha256 {
            return Err(integrity("authenticated_capacity_digest_mismatch"));
        }
        let expected =
            derive_completeness(&self.observations, &self.interventions, &self.contracts);
        if expected != self.completeness {
            return Err(integrity("authenticated_capacity_completeness_mismatch"));
        }
        Ok(())
    }

    pub fn persist(&self, private_root: &Path) -> BrainResult<PrivateFileReference> {
        self.verify()?;
        let destination = package_path(private_root, &self.manifest_sha256);
        let bytes = serde_json::to_vec(self)?;
        let sha256 = write_or_verify_immutable(private_root, &destination, &bytes)?;
        Ok(PrivateFileReference::new(destination, sha256))
    }

    pub fn load_and_authenticate(
        private_root: &Path,
        digest: &AuthenticatedCapacityDigest,
    ) -> BrainResult<Self> {
        if digest.is_draft() {
            return Err(integrity("authenticated_capacity_digest_draft"));
        }
        let path = package_path(private_root, digest);
        let bytes = std::fs::read(&path)?;
        let package: Self = serde_json::from_slice(&bytes)?;
        package.verify()?;
        if package.manifest_sha256 != *digest {
            return Err(integrity("authenticated_capacity_loaded_digest_mismatch"));
        }
        Ok(package)
    }

    fn calculate_digest(&self) -> BrainResult<AuthenticatedCapacityDigest> {
        let mut unsigned = self.clone();
        unsigned.manifest_sha256 = AuthenticatedCapacityDigest::draft_marker();
        Ok(AuthenticatedCapacityDigest::from_computed(Sha256Digest::digest_domain(
            PACKAGE_DOMAIN,
            &serde_json::to_vec(&unsigned)?,
        )))
    }
}

fn package_path(private_root: &Path, digest: &AuthenticatedCapacityDigest) -> PathBuf {
    private_root
        .join("state/acquisitions/authenticated-capacity/by-sha")
        .join(format!("{}.json", digest.as_str()))
}

fn derive_completeness(
    observations: &[CapacityObservation],
    interventions: &[CapacityIntervention],
    contracts: &[FunctionalContractClaim],
) -> PackageCompleteness {
    let has_select = observations
        .iter()
        .any(|item| item.donor_action.kind == DonorActionKind::Select);
    let has_explore = observations
        .iter()
        .any(|item| item.donor_action.kind == DonorActionKind::Explore);
    let supported = contracts
        .iter()
        .filter(|claim| claim.status == FunctionalContractStatus::Supported)
        .count();
    if observations.len() >= 2
        && has_select
        && has_explore
        && !interventions.is_empty()
        && supported >= 2
    {
        PackageCompleteness::SufficientForResidencyHandoff
    } else {
        PackageCompleteness::Insufficient
    }
}

/// Run a bounded fixture campaign and seal an authenticated capacity package.
pub fn seal_fixture_procedure_selector_capacity(
    capacity_key: &str,
    provenance: CapacityProvenance,
) -> BrainResult<AuthenticatedCapacityPackage> {
    let donor = FixtureProcedureSelector;
    let mut observations = Vec::new();

    let stim_select = SelectorStimulus::new(
        "route:analysis",
        vec![
            PriorProcedureResult::new("proc.alpha", "success")?,
            PriorProcedureResult::new("proc.beta", "fail")?,
        ],
        vec!["proc.alpha".into(), "proc.beta".into(), "proc.gamma".into()],
    )?;
    let action_select = donor.observe(&stim_select)?;
    observations.push(CapacityObservation::seal(
        ObservationId::parse("obs.select-best-01")?,
        stim_select.clone(),
        action_select,
        Some("selected_applied".into()),
    )?);

    let stim_explore = SelectorStimulus::new(
        "route:novel",
        Vec::new(),
        vec!["proc.alpha".into(), "proc.beta".into()],
    )?;
    let action_explore = donor.observe(&stim_explore)?;
    observations.push(CapacityObservation::seal(
        ObservationId::parse("obs.explore-cold-01")?,
        stim_explore,
        action_explore,
        Some("exploration_opened".into()),
    )?);

    let stim_ambiguous = SelectorStimulus::new(
        "route:ambiguous",
        vec![
            PriorProcedureResult::new("proc.alpha", "fail")?,
            PriorProcedureResult::new("proc.beta", "fail")?,
        ],
        vec!["proc.alpha".into(), "proc.beta".into()],
    )?;
    let action_ambiguous = donor.observe(&stim_ambiguous)?;
    observations.push(CapacityObservation::seal(
        ObservationId::parse("obs.explore-failed-priors-01")?,
        stim_ambiguous,
        action_ambiguous,
        None,
    )?);

    let baseline = observations[0].clone();
    let mut ablated_priors = stim_select.prior_results.clone();
    ablated_priors.retain(|item| item.procedure_id != "proc.alpha");
    let ablated_stimulus = SelectorStimulus::new(
        stim_select.context.clone(),
        ablated_priors,
        stim_select.candidate_procedures.clone(),
    )?;
    let ablated_action = donor.observe(&ablated_stimulus)?;
    let ablated_observation = CapacityObservation::seal(
        ObservationId::parse("obs.intervene-ablate-best-01")?,
        ablated_stimulus,
        ablated_action,
        None,
    )?;
    let interventions = vec![CapacityIntervention::seal(
        InterventionKind::AblateBestPrior,
        &baseline,
        ablated_observation,
        "Removing the historically best prior should change selection or force explore",
    )?];

    let select_digest = observations[0].observation_sha256().clone();
    let explore_digest = observations[1].observation_sha256().clone();
    let intervene_digest = interventions[0]
        .intervened_observation
        .observation_sha256()
        .clone();

    let contracts = vec![
        FunctionalContractClaim::new(
            "claim.select-best-historical",
            "Given successful historical priors among candidates, donor selects that procedure",
            FunctionalContractStatus::Supported,
            vec![select_digest.clone()],
        )?,
        FunctionalContractClaim::new(
            "claim.explore-when-no-success",
            "Given no successful priors, donor explores rather than selecting a failed route",
            FunctionalContractStatus::Supported,
            vec![explore_digest, observations[2].observation_sha256().clone()],
        )?,
        FunctionalContractClaim::new(
            "claim.ablation-changes-choice",
            "Ablating the best prior changes the donor action relative to baseline",
            if interventions[0].intervened_observation.donor_action != baseline.donor_action {
                FunctionalContractStatus::Supported
            } else {
                FunctionalContractStatus::Unsupported
            },
            vec![select_digest, intervene_digest],
        )?,
    ];

    AuthenticatedCapacityPackage::seal(
        capacity_key,
        DonorKind::FixtureProcedureSelector,
        provenance,
        observations,
        interventions,
        contracts,
        Vec::new(),
    )
}

/// Summarize a sealed package for ResidencyDecision handoff (Paso 5 consumes this shape).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResidencyHandoffSummary {
    pub capacity_key: String,
    pub package_sha256: AuthenticatedCapacityDigest,
    pub completeness: PackageCompleteness,
    pub donor_kind: DonorKind,
    pub supported_contract_ids: Vec<String>,
    pub observation_count: usize,
    pub intervention_count: usize,
    pub obligations: Vec<String>,
    /// Explicit: Paso 5 may decide Software|Hybrid|Weights|BoundedUnknown from this.
    pub residency_decision_pending: bool,
    pub capability_ir_pending: bool,
}

impl AuthenticatedCapacityPackage {
    pub fn residency_handoff_summary(&self) -> BrainResult<ResidencyHandoffSummary> {
        self.verify()?;
        let supported_contract_ids = self
            .contracts
            .iter()
            .filter(|claim| claim.status == FunctionalContractStatus::Supported)
            .map(|claim| claim.claim_id.clone())
            .collect();
        Ok(ResidencyHandoffSummary {
            capacity_key: self.capacity_key.clone(),
            package_sha256: self.manifest_sha256.clone(),
            completeness: self.completeness,
            donor_kind: self.donor_kind,
            supported_contract_ids,
            observation_count: self.observations.len(),
            intervention_count: self.interventions.len(),
            obligations: self.obligations.clone(),
            residency_decision_pending: true,
            capability_ir_pending: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn fixture_campaign_seals_sufficient_authenticated_capacity() {
        let package = seal_fixture_procedure_selector_capacity(
            "procedure_selector_or_explore",
            CapacityProvenance::default(),
        )
        .unwrap();
        package.verify().unwrap();
        assert_eq!(package.completeness(), PackageCompleteness::SufficientForResidencyHandoff);
        assert!(!package.contains_capability_ir_fields());
        assert!(package.observations().len() >= 2);
        assert!(!package.interventions().is_empty());
        assert!(package
            .contracts()
            .iter()
            .any(|claim| claim.status == FunctionalContractStatus::Supported));
        let handoff = package.residency_handoff_summary().unwrap();
        assert!(handoff.residency_decision_pending);
        assert!(handoff.capability_ir_pending);
        assert_eq!(handoff.capacity_key, "procedure_selector_or_explore");
    }

    #[test]
    fn tampered_observation_fails_closed() {
        let mut package = seal_fixture_procedure_selector_capacity(
            "procedure_selector_or_explore",
            CapacityProvenance::default(),
        )
        .unwrap();
        package.observations[0].measured_outcome = Some("tampered".into());
        assert!(package.verify().is_err());
    }

    #[test]
    fn gpem_wire_is_documented_but_fail_closed() {
        let wire = GpemV2RecommendDonorWire::new(
            PathBuf::from("/tmp/gpem-store"),
            vec!["route".into(), "capability_id".into()],
        )
        .unwrap();
        assert_eq!(wire.schema, GpemV2RecommendDonorWire::SCHEMA);
        let stimulus =
            SelectorStimulus::new("route:analysis", Vec::new(), vec!["proc.alpha".into()]).unwrap();
        let err = wire.observe(&stimulus).unwrap_err().to_string();
        assert!(err.contains("gpem_v2_recommend_donor_not_wired"));
    }

    #[test]
    fn package_persists_and_reauthenticates_under_private_root() {
        let (base, root) = tempfile_private_root("persist");
        let package = seal_fixture_procedure_selector_capacity(
            "procedure_selector_or_explore",
            CapacityProvenance {
                acquisition_id: Some(AcquisitionId::parse("acq.fixture-selector-01").unwrap()),
                capture_receipt_sha256: None,
                donor_locator: Some("fixture://procedure_selector".into()),
            },
        )
        .unwrap();
        let reference = package.persist(&root).unwrap();
        assert!(reference.path.exists());
        let loaded =
            AuthenticatedCapacityPackage::load_and_authenticate(&root, package.manifest_sha256())
                .unwrap();
        assert_eq!(loaded.manifest_sha256(), package.manifest_sha256());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn empty_observations_rejected() {
        let err = AuthenticatedCapacityPackage::seal(
            "procedure_selector_or_explore",
            DonorKind::FixtureProcedureSelector,
            CapacityProvenance::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("authenticated_capacity_observations_invalid"));
    }

    #[test]
    fn fixture_selects_best_or_explores() {
        let donor = FixtureProcedureSelector;
        let select = donor
            .observe(
                &SelectorStimulus::new(
                    "ctx",
                    vec![PriorProcedureResult::new("proc.alpha", "success").unwrap()],
                    vec!["proc.alpha".into(), "proc.beta".into()],
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(select.kind, DonorActionKind::Select);
        assert_eq!(select.selected_procedure_id.as_deref(), Some("proc.alpha"));

        let explore = donor
            .observe(&SelectorStimulus::new("ctx", Vec::new(), vec!["proc.alpha".into()]).unwrap())
            .unwrap();
        assert_eq!(explore.kind, DonorActionKind::Explore);
    }

    fn tempfile_private_root(label: &str) -> (PathBuf, PathBuf) {
        use crate::foundation::security::secure_dir;
        let base =
            std::env::temp_dir().join(format!("tidex-authcap-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("private");
        fs::create_dir_all(&root).unwrap();
        secure_dir(&root).unwrap();
        (base, root)
    }
}
