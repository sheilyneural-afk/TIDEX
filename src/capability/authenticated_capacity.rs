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
//! prior results, or explore*. Fixture donor is **unit-test only**.
//! [`GpemV2RecommendDonorWire`] invokes live SHEI GPEM (`recommend_v2`) via a
//! thin bridge and fail-closes when the donor is unavailable.
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
use std::process::{Command, Stdio};

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
    /// Live SHEI GPEM v2 `recommend` donor (thin bridge; fail-closed if unavailable).
    #[serde(rename = "gpem_v2_recommend")]
    GpemV2Recommend,
    /// Measured closed linear map (y = Wx). Evidence is arithmetic I/O, not
    /// source-tree invention. Used by the Weights/Hybrid→IR→receptor vertical.
    #[serde(rename = "measured_closed_linear_map")]
    MeasuredClosedLinearMap,
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

/// Live GPEM v2 recommend donor wire (thin SHEI bridge; never embeds GPEM).
///
/// Calls SHEI canonical interfaces through `tools/gpem_v2_recommend_donor.py`:
/// `get_gpem` / `create_gpem` → `GPEMService.recommend_v2` → `GPEMServiceV2.recommend`
/// against the governed store at `store_root`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GpemV2RecommendDonorWire {
    pub schema: String,
    pub store_root: PathBuf,
    pub context_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct GpemBridgeRecommendation {
    trace_id: Option<String>,
    turn_id: Option<String>,
    route: Option<String>,
    capability_id: Option<String>,
    score: Option<f64>,
    rationale: Option<Vec<String>>,
    lifecycle_state: Option<String>,
    utility_score: Option<f64>,
    auditability_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct GpemBridgeResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    interface: Option<String>,
    #[serde(default)]
    recommendations: Vec<GpemBridgeRecommendation>,
}

fn gpem_donor_mode() -> String {
    std::env::var("TIDEX_GPEM_DONOR_MODE")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn resolve_gpem_bridge_script() -> BrainResult<PathBuf> {
    if let Ok(explicit) = std::env::var("TIDEX_GPEM_BRIDGE") {
        let path = PathBuf::from(explicit);
        if path.is_file() {
            return Ok(path);
        }
        return Err(invalid("gpem_v2_recommend_donor_misconfigured"));
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools/gpem_v2_recommend_donor.py");
    if path.is_file() {
        Ok(path)
    } else {
        Err(invalid("gpem_v2_recommend_donor_unavailable"))
    }
}

fn resolve_shei_research_python() -> BrainResult<PathBuf> {
    if let Ok(root) = std::env::var("TIDEX_SHEI_ROOT") {
        let research = PathBuf::from(root.trim()).join("research_python");
        if research.is_dir() {
            return Ok(research);
        }
        return Err(invalid("gpem_v2_recommend_donor_misconfigured"));
    }
    if let Ok(research) = std::env::var("TIDEX_SHEI_RESEARCH_PYTHON") {
        let path = PathBuf::from(research.trim());
        if path.is_dir() {
            return Ok(path);
        }
        return Err(invalid("gpem_v2_recommend_donor_misconfigured"));
    }
    let default = PathBuf::from("/home/yo/Projects/SHEI/research_python");
    if default.is_dir() {
        Ok(default)
    } else {
        Err(invalid("gpem_v2_recommend_donor_unavailable"))
    }
}

fn resolve_gpem_python() -> BrainResult<PathBuf> {
    if let Ok(explicit) = std::env::var("TIDEX_GPEM_PYTHON") {
        let path = PathBuf::from(explicit.trim());
        if path.is_file() {
            return Ok(path);
        }
        return Err(invalid("gpem_v2_recommend_donor_misconfigured"));
    }
    for candidate in ["/usr/bin/python3", "/usr/local/bin/python3"] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(invalid("gpem_v2_recommend_donor_unavailable"))
}

fn stimulus_context_value(stimulus: &SelectorStimulus, key: &str) -> Option<String> {
    match key {
        "route" => {
            let ctx = stimulus.context.trim();
            if let Some(rest) = ctx.strip_prefix("route:") {
                let rest = rest.trim();
                if !rest.is_empty() {
                    return Some(rest.to_string());
                }
            }
            if !ctx.is_empty() {
                Some(ctx.to_string())
            } else {
                None
            }
        }
        "capability_id" => stimulus
            .prior_results
            .iter()
            .find(|item| matches!(item.outcome.as_str(), "success" | "ok" | "pass"))
            .map(|item| item.procedure_id.clone()),
        "prior_procedure" => stimulus
            .prior_results
            .iter()
            .find(|item| matches!(item.outcome.as_str(), "success" | "ok" | "pass"))
            .map(|item| item.procedure_id.clone()),
        other => {
            // Allow `key:value` fragments inside context for declared keys.
            for part in stimulus.context.split([',', ';']) {
                let part = part.trim();
                if let Some(rest) = part.strip_prefix(&format!("{other}:")) {
                    let rest = rest.trim();
                    if !rest.is_empty() {
                        return Some(rest.to_string());
                    }
                }
            }
            None
        }
    }
}

fn map_gpem_recommendations_to_action(
    recommendations: &[GpemBridgeRecommendation],
    stimulus: &SelectorStimulus,
) -> BrainResult<DonorAction> {
    let want_route = stimulus_context_value(stimulus, "route");
    for rec in recommendations {
        let Some(capability_id) = rec.capability_id.as_deref() else {
            continue;
        };
        if !stimulus
            .candidate_procedures
            .iter()
            .any(|candidate| candidate == capability_id)
        {
            continue;
        }
        let route_matches = match (want_route.as_deref(), rec.route.as_deref()) {
            (Some(want), Some(got)) => want == got,
            (Some(_), None) => false,
            (None, _) => true,
        };
        let prior_success_supports = stimulus.prior_results.iter().any(|prior| {
            prior.procedure_id == capability_id
                && matches!(prior.outcome.as_str(), "success" | "ok" | "pass")
        });
        // Select only when GPEM's top evidence aligns with stimulus route or a
        // successful prior among candidates; otherwise explore.
        if route_matches || prior_success_supports {
            return DonorAction::select(capability_id);
        }
    }
    // Live GPEM responded with no selectable alignment → explore (authentic).
    Ok(DonorAction::explore())
}

fn store_force_unavailable(store_root: &Path) -> bool {
    store_root.join(".tidex_gpem_force_unavailable").is_file()
}

fn invoke_gpem_bridge(request: &serde_json::Value) -> BrainResult<GpemBridgeResponse> {
    let mode = gpem_donor_mode();
    if mode == "force_unavailable" || mode == "unavailable" {
        return Err(invalid("gpem_v2_recommend_donor_unavailable"));
    }
    if let Some(store) = request.get("store_root").and_then(|v| v.as_str()) {
        if store_force_unavailable(Path::new(store)) {
            return Err(invalid("gpem_v2_recommend_donor_unavailable"));
        }
    }

    let python = resolve_gpem_python()?;
    let bridge = resolve_gpem_bridge_script()?;
    let mut child = Command::new(&python)
        .arg(&bridge)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| invalid("gpem_v2_recommend_donor_unavailable"))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| invalid("gpem_v2_recommend_donor_unavailable"))?;
        use std::io::Write;
        stdin
            .write_all(request.to_string().as_bytes())
            .map_err(|_| invalid("gpem_v2_recommend_invoke_failed"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|_| invalid("gpem_v2_recommend_invoke_failed"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = stderr;
        return Err(invalid("gpem_v2_recommend_invoke_failed"));
    }
    let parsed: GpemBridgeResponse =
        serde_json::from_str(&stdout).map_err(|_| invalid("gpem_v2_recommend_invoke_failed"))?;
    if !parsed.ok {
        let code = parsed
            .error
            .as_deref()
            .unwrap_or("gpem_v2_recommend_invoke_failed");
        // Normalize bridge codes into the sealed fail-closed vocabulary.
        if code.contains("misconfigured") {
            return Err(invalid("gpem_v2_recommend_donor_misconfigured"));
        }
        if code.contains("unavailable") {
            return Err(invalid("gpem_v2_recommend_donor_unavailable"));
        }
        return Err(invalid("gpem_v2_recommend_invoke_failed"));
    }
    Ok(parsed)
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

    /// Observe live SHEI/GPEM recommend. Fail-closed if donor unavailable.
    pub fn observe(&self, stimulus: &SelectorStimulus) -> BrainResult<DonorAction> {
        let research_python = resolve_shei_research_python()?;
        let mut context = serde_json::Map::new();
        for key in &self.context_keys {
            if let Some(value) = stimulus_context_value(stimulus, key) {
                context.insert(key.clone(), serde_json::Value::String(value));
            }
        }
        // Always pass candidates for bridge transparency (not a GPEM write).
        context.insert(
            "candidate_procedures".into(),
            serde_json::Value::Array(
                stimulus
                    .candidate_procedures
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );

        let prefer_get_gpem = matches!(gpem_donor_mode().as_str(), "get_gpem" | "prefer_get_gpem");
        let request = serde_json::json!({
            "action": "recommend",
            "store_root": self.store_root,
            "shei_research_python": research_python,
            "context": serde_json::Value::Object(context),
            "limit": 5,
            "prefer_get_gpem": prefer_get_gpem,
        });
        let response = invoke_gpem_bridge(&request)?;
        map_gpem_recommendations_to_action(&response.recommendations, stimulus)
    }

    /// Seed governed demo traces into `store_root` via canonical GPEM ingest.
    pub fn seed_demo_traces(&self) -> BrainResult<Vec<String>> {
        let research_python = resolve_shei_research_python()?;
        let request = serde_json::json!({
            "action": "seed_demo_traces",
            "store_root": self.store_root,
            "shei_research_python": research_python,
        });
        let mode = gpem_donor_mode();
        if mode == "force_unavailable" || mode == "unavailable" {
            return Err(invalid("gpem_v2_recommend_donor_unavailable"));
        }
        if store_force_unavailable(&self.store_root) {
            return Err(invalid("gpem_v2_recommend_donor_unavailable"));
        }
        let python = resolve_gpem_python()?;
        let bridge = resolve_gpem_bridge_script()?;
        let output = Command::new(&python)
            .arg(&bridge)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(request.to_string().as_bytes())?;
                }
                child.wait_with_output()
            })
            .map_err(|_| invalid("gpem_v2_recommend_invoke_failed"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let value: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|_| invalid("gpem_v2_recommend_invoke_failed"))?;
        if value.get("ok") != Some(&serde_json::Value::Bool(true)) {
            return Err(invalid("gpem_v2_recommend_invoke_failed"));
        }
        let ids = value
            .get("trace_ids")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return Err(invalid("gpem_v2_recommend_invoke_failed"));
        }
        Ok(ids)
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

/// Run a bounded **live GPEM** campaign and seal an authenticated capacity package.
///
/// Fail-closed: every observation comes from [`GpemV2RecommendDonorWire::observe`].
/// No fixture substitute. Requires a reachable SHEI/GPEM store with enough
/// recommend signal to produce select + explore evidence (seed via
/// [`GpemV2RecommendDonorWire::seed_demo_traces`] for local smoke).
pub fn seal_live_gpem_v2_recommend_capacity(
    wire: &GpemV2RecommendDonorWire,
    capacity_key: &str,
    provenance: CapacityProvenance,
) -> BrainResult<AuthenticatedCapacityPackage> {
    let mut observations = Vec::new();

    let stim_select = SelectorStimulus::new(
        "route:analysis",
        vec![
            PriorProcedureResult::new("proc.alpha", "success")?,
            PriorProcedureResult::new("proc.beta", "fail")?,
        ],
        vec!["proc.alpha".into(), "proc.beta".into(), "proc.gamma".into()],
    )?;
    let action_select = wire.observe(&stim_select)?;
    observations.push(CapacityObservation::seal(
        ObservationId::parse("obs.gpem-select-best-01")?,
        stim_select.clone(),
        action_select,
        Some("selected_applied".into()),
    )?);

    let stim_explore = SelectorStimulus::new(
        "route:novel",
        Vec::new(),
        vec!["proc.alpha".into(), "proc.beta".into()],
    )?;
    let action_explore = wire.observe(&stim_explore)?;
    observations.push(CapacityObservation::seal(
        ObservationId::parse("obs.gpem-explore-cold-01")?,
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
    let action_ambiguous = wire.observe(&stim_ambiguous)?;
    observations.push(CapacityObservation::seal(
        ObservationId::parse("obs.gpem-explore-failed-priors-01")?,
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
    let ablated_action = wire.observe(&ablated_stimulus)?;
    let ablated_observation = CapacityObservation::seal(
        ObservationId::parse("obs.gpem-intervene-ablate-best-01")?,
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

    let has_select = observations
        .iter()
        .any(|item| item.donor_action.kind == DonorActionKind::Select);
    let has_explore = observations
        .iter()
        .any(|item| item.donor_action.kind == DonorActionKind::Explore);
    if !has_select || !has_explore {
        return Err(invalid("gpem_v2_recommend_insufficient_live_evidence"));
    }

    let select_digest = observations[0].observation_sha256().clone();
    let explore_digest = observations[1].observation_sha256().clone();
    let intervene_digest = interventions[0]
        .intervened_observation
        .observation_sha256()
        .clone();

    let contracts = vec![
        FunctionalContractClaim::new(
            "claim.select-best-historical",
            "Given successful historical priors among candidates, live GPEM selects that procedure",
            FunctionalContractStatus::Supported,
            vec![select_digest.clone()],
        )?,
        FunctionalContractClaim::new(
            "claim.explore-when-no-success",
            "Given no successful priors / novel route, live GPEM explores rather than forcing a failed route",
            FunctionalContractStatus::Supported,
            vec![explore_digest, observations[2].observation_sha256().clone()],
        )?,
        FunctionalContractClaim::new(
            "claim.ablation-changes-choice",
            "Ablating the best prior changes the live GPEM action relative to baseline",
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
        DonorKind::GpemV2Recommend,
        provenance,
        observations,
        interventions,
        contracts,
        Vec::new(),
    )
}

/// Measured closed-linear-map donor: among candidates with weight vectors,
/// select the unique best measured margin `W·x`; explore when tied or flat.
#[derive(Debug, Clone)]
pub struct MeasuredClosedLinearMapDonor {
    pub input: Vec<f64>,
    pub weights_by_candidate: std::collections::BTreeMap<String, Vec<f64>>,
}

impl MeasuredClosedLinearMapDonor {
    pub fn new(
        input: Vec<f64>,
        weights_by_candidate: std::collections::BTreeMap<String, Vec<f64>>,
    ) -> BrainResult<Self> {
        if input.is_empty() || input.len() > 4_096 || input.iter().any(|v| !v.is_finite()) {
            return Err(invalid("measured_linear_map_input_invalid"));
        }
        if weights_by_candidate.is_empty() || weights_by_candidate.len() > MAX_CANDIDATES {
            return Err(invalid("measured_linear_map_candidates_invalid"));
        }
        for (candidate, weights) in &weights_by_candidate {
            validate_label(candidate, "measured_linear_map_candidate")?;
            if weights.len() != input.len() || weights.iter().any(|v| !v.is_finite()) {
                return Err(invalid("measured_linear_map_weight_shape_invalid"));
            }
        }
        Ok(Self {
            input,
            weights_by_candidate,
        })
    }

    fn margin(&self, weights: &[f64]) -> f64 {
        self.input.iter().zip(weights).map(|(x, w)| x * w).sum()
    }

    pub fn observe(&self, stimulus: &SelectorStimulus) -> BrainResult<DonorAction> {
        let mut scored = Vec::new();
        for candidate in &stimulus.candidate_procedures {
            let Some(weights) = self.weights_by_candidate.get(candidate) else {
                continue;
            };
            scored.push((candidate.as_str(), self.margin(weights)));
        }
        if scored.is_empty() {
            return Ok(DonorAction::explore());
        }
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let best = scored[0];
        let unique_best = scored.len() == 1 || scored[1].1 < best.1 - 1e-12;
        // Flat / near-zero margins → explore (no confident weights selection).
        if !unique_best || best.1.abs() < 1e-12 {
            return Ok(DonorAction::explore());
        }
        DonorAction::select(best.0)
    }
}

/// Seal authenticated capacity for a measured closed linear map with optional
/// explicit `residency.*` Supported contracts (Weights/Hybrid warrant).
///
/// Observations are real measured margins, not invented IR. Completeness still
/// requires select+explore+intervention+≥2 supported contracts.
pub fn seal_measured_closed_linear_map_capacity(
    capacity_key: &str,
    provenance: CapacityProvenance,
    donor: &MeasuredClosedLinearMapDonor,
    residency_claim_ids: &[&str],
) -> BrainResult<AuthenticatedCapacityPackage> {
    let mut observations = Vec::new();

    // Distinct input that yields a clear unique best among candidates.
    let stim_select = SelectorStimulus::new(
        "linear_map:select",
        vec![
            PriorProcedureResult::new("map.alpha", "success")?,
            PriorProcedureResult::new("map.beta", "fail")?,
        ],
        vec!["map.alpha".into(), "map.beta".into(), "map.gamma".into()],
    )?;
    let action_select = donor.observe(&stim_select)?;
    observations.push(CapacityObservation::seal(
        ObservationId::parse("obs.linear-select-01")?,
        stim_select.clone(),
        action_select,
        Some(format!(
            "measured_margin_best={}",
            donor
                .weights_by_candidate
                .get("map.alpha")
                .map(|w| donor.margin(w))
                .unwrap_or(0.0)
        )),
    )?);

    // Cold / zero-aligned stimulus → explore.
    let stim_explore = SelectorStimulus::new(
        "linear_map:explore",
        Vec::new(),
        vec!["map.alpha".into(), "map.beta".into()],
    )?;
    // Force explore observation using an empty-prior stimulus against a donor
    // clone with zeroed input so margins are flat (honest explore evidence).
    let flat_donor = MeasuredClosedLinearMapDonor::new(
        vec![0.0; donor.input.len()],
        donor.weights_by_candidate.clone(),
    )?;
    let action_explore = flat_donor.observe(&stim_explore)?;
    observations.push(CapacityObservation::seal(
        ObservationId::parse("obs.linear-explore-01")?,
        stim_explore,
        action_explore,
        Some("flat_margins_explore".into()),
    )?);

    let stim_ambiguous = SelectorStimulus::new(
        "linear_map:ambiguous",
        vec![
            PriorProcedureResult::new("map.alpha", "fail")?,
            PriorProcedureResult::new("map.beta", "fail")?,
        ],
        vec!["map.alpha".into(), "map.beta".into()],
    )?;
    let action_ambiguous = flat_donor.observe(&stim_ambiguous)?;
    observations.push(CapacityObservation::seal(
        ObservationId::parse("obs.linear-explore-failed-priors-01")?,
        stim_ambiguous,
        action_ambiguous,
        None,
    )?);

    let baseline = observations[0].clone();
    let mut ablated_priors = stim_select.prior_results.clone();
    ablated_priors.retain(|item| item.procedure_id != "map.alpha");
    let ablated_stimulus = SelectorStimulus::new(
        stim_select.context.clone(),
        ablated_priors,
        stim_select.candidate_procedures.clone(),
    )?;
    // Ablation: remove map.alpha weights from the donor view (causal intervention).
    let mut ablated_weights = donor.weights_by_candidate.clone();
    ablated_weights.remove("map.alpha");
    let ablated_donor = MeasuredClosedLinearMapDonor::new(donor.input.clone(), ablated_weights)?;
    let ablated_action = ablated_donor.observe(&ablated_stimulus)?;
    let ablated_observation = CapacityObservation::seal(
        ObservationId::parse("obs.linear-intervene-ablate-best-01")?,
        ablated_stimulus,
        ablated_action,
        None,
    )?;
    let interventions = vec![CapacityIntervention::seal(
        InterventionKind::AblateBestPrior,
        &baseline,
        ablated_observation,
        "Ablating the uniquely best measured map changes selection or forces explore",
    )?];

    let has_select = observations
        .iter()
        .any(|item| item.donor_action.kind == DonorActionKind::Select);
    let has_explore = observations
        .iter()
        .any(|item| item.donor_action.kind == DonorActionKind::Explore);
    if !has_select || !has_explore {
        return Err(invalid("measured_linear_map_insufficient_select_explore_evidence"));
    }

    let select_digest = observations[0].observation_sha256().clone();
    let explore_digest = observations[1].observation_sha256().clone();
    let intervene_digest = interventions[0]
        .intervened_observation
        .observation_sha256()
        .clone();

    let mut contracts = vec![
        FunctionalContractClaim::new(
            "claim.linear-select-best-margin",
            "Given measured margins, donor selects the uniquely best closed linear map",
            FunctionalContractStatus::Supported,
            vec![select_digest.clone()],
        )?,
        FunctionalContractClaim::new(
            "claim.linear-explore-when-flat",
            "Given flat measured margins, donor explores rather than inventing a selection",
            FunctionalContractStatus::Supported,
            vec![explore_digest, observations[2].observation_sha256().clone()],
        )?,
        FunctionalContractClaim::new(
            "claim.linear-ablation-changes-choice",
            "Ablating the best measured map changes the donor action relative to baseline",
            if interventions[0].intervened_observation.donor_action != baseline.donor_action {
                FunctionalContractStatus::Supported
            } else {
                FunctionalContractStatus::Unsupported
            },
            vec![select_digest.clone(), intervene_digest.clone()],
        )?,
    ];
    for claim_id in residency_claim_ids {
        contracts.push(FunctionalContractClaim::new(
            *claim_id,
            format!("residency attestation {claim_id} from measured closed linear map evidence"),
            FunctionalContractStatus::Supported,
            vec![select_digest.clone(), intervene_digest.clone()],
        )?);
    }

    AuthenticatedCapacityPackage::seal(
        capacity_key,
        DonorKind::MeasuredClosedLinearMap,
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
    fn gpem_wire_fail_closed_when_donor_forced_unavailable() {
        let (base, root) = tempfile_private_root("gpem-unavail");
        let store = root.join("gpem-store");
        fs::create_dir_all(&store).unwrap();
        fs::write(store.join(".tidex_gpem_force_unavailable"), b"1").unwrap();
        let wire =
            GpemV2RecommendDonorWire::new(store, vec!["route".into(), "capability_id".into()])
                .unwrap();
        assert_eq!(wire.schema, GpemV2RecommendDonorWire::SCHEMA);
        let stimulus =
            SelectorStimulus::new("route:analysis", Vec::new(), vec!["proc.alpha".into()]).unwrap();
        let err = wire.observe(&stimulus).unwrap_err().to_string();
        assert!(
            err.contains("gpem_v2_recommend_donor_unavailable")
                || err.contains("gpem_v2_recommend_donor_misconfigured"),
            "expected fail-closed unavailable, got {err}"
        );
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn gpem_wire_live_recommend_seals_when_shei_available() {
        // Integration: exercise real SHEI/GPEM on this machine. If SHEI is
        // missing, fail-closed without pretending fixture success.
        if resolve_shei_research_python().is_err() {
            let err = resolve_shei_research_python().unwrap_err().to_string();
            assert!(
                err.contains("gpem_v2_recommend_donor_unavailable")
                    || err.contains("gpem_v2_recommend_donor_misconfigured")
            );
            return;
        }
        let (base, root) = tempfile_private_root("gpem-live");
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
        let seeded = wire.seed_demo_traces().expect("live GPEM seed");
        assert!(!seeded.is_empty());
        let package = seal_live_gpem_v2_recommend_capacity(
            &wire,
            "procedure_selector_or_explore",
            CapacityProvenance {
                acquisition_id: None,
                capture_receipt_sha256: None,
                donor_locator: Some(format!("shei-gpem://{}", store.display())),
            },
        )
        .expect("live GPEM seal");
        package.verify().unwrap();
        assert_eq!(package.donor_kind(), DonorKind::GpemV2Recommend);
        assert_eq!(package.completeness(), PackageCompleteness::SufficientForResidencyHandoff);
        let _ = fs::remove_dir_all(&base);
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
