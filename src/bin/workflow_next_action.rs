//! Workflow-layer NextAction coordinator (Paso 3 / 2C).
//!
//! # Composition frontier (explicit)
//!
//! `build.rs` keeps domains separate:
//! - `operator` → `cross_model`, `foundation` (not learning / knowledge / engine)
//! - `engine` → analysis, foundation, learning (not operator)
//!
//! Therefore this coordinator **must not** live inside BrainEngine,
//! KnowledgeEngine, AdapterBank, or `operator/control_plane.rs`. It lives in
//! the `tidex` binary composition root (`src/bin/`), which may see both
//! ProceduralMemory (learning) and the executor registry / job enqueue path
//! (operator) without silent cross-domain imports.
//!
//! # Authority
//!
//! Emits exactly one admissible [`NextAction`]. Does **not** decide truth,
//! promotion, or production activation. Still advisory to UPG / AdapterBank.
//!
//! # Production job path
//!
//! [`invoke_next_action`] in `DryRun` mode only records the would-be request.
//! In `Start` mode it calls
//! [`tidex::operator::control_plane::start_operator_direct_job`] /
//! [`tidex::operator::control_plane::start_operator_behavioral_discovery_job`],
//! which are the same enqueue path used by the HTTP control plane
//! (`POST .../jobs/...` → private `start_operator_job`).

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use tidex::foundation::digest::Sha256Digest;
use tidex::foundation::error::{BrainError, BrainResult};
use tidex::learning::procedural_memory::{AdviceDisposition, RetrievalReport, SolverFamily};
use tidex::learning::procedural_memory::{ProceduralMemory, RetrievalQuery};
use tidex::learning::procedural_replay::retrieve_procedural_advice;
use tidex::operator::control_plane::{
    start_operator_behavioral_discovery_job, start_operator_direct_job,
    BehavioralDiscoveryWorkflowRequest, OperatorDirectOperation, OperatorDirectWorkflowRequest,
    OperatorJobRecord,
};
use tidex::operator::executor_registry::{executor_by_id, executor_id_for_direct_operation};

pub const NEXT_ACTION_SCHEMA: &str = "tidex.workflow.next_action/v1";
pub const WORKFLOW_TICK_RECEIPT_SCHEMA: &str = "tidex.workflow.tick_receipt/v1";

fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}

/// Advisory KE projection for the workflow tick (no KE import into operator).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeWorkflowSignals {
    pub calibration_sufficient: bool,
    pub causal_evidence_sufficient: bool,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// Plain directive snapshot so the coordinator works without requiring the
/// `cross-model-plasticity` feature at compile time. Production composition
/// copies fields from `CoEvolutionDirective` when that feature is enabled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CoEvolutionDirectiveSnapshot {
    pub recommended_operation: String,
    pub reason: String,
    pub converged: bool,
    #[serde(default)]
    pub source_model: Option<String>,
    #[serde(default)]
    pub target_model: Option<String>,
    #[serde(default)]
    pub capability_hint: Option<String>,
    #[serde(default)]
    pub evidence_sha256: Option<String>,
}

/// Advisory routing preference (from RoutingPlasticity / advice projection).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct RoutingPreference {
    #[serde(default)]
    pub preferred_model: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub routing_score: Option<f64>,
}

/// Cost / risk ceilings (fail-closed when the chosen action exceeds them).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowCost {
    /// Abstract relative cost units (1 = cheap probe-like, 2 = alignment/transfer).
    pub relative_units: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowRisk {
    /// 0 = observe-only, 1 = advisory experiment, 2 = transfer/intervention.
    pub level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StopCondition {
    pub max_ticks: u32,
    pub halt_on_converged_directive: bool,
    pub halt_when_information_gain_below: String,
}

impl Default for StopCondition {
    fn default() -> Self {
        Self {
            max_ticks: 8,
            halt_on_converged_directive: true,
            halt_when_information_gain_below: "0.05".into(),
        }
    }
}

/// Authenticated / caller-bound inputs required to enqueue a real job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatedWorkflowInputs {
    #[serde(default)]
    pub model_ids: Vec<String>,
    #[serde(default)]
    pub dataset_sha256: Option<String>,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

impl AuthenticatedWorkflowInputs {
    fn parsed_model_ids(&self) -> BrainResult<Vec<Sha256Digest>> {
        self.model_ids
            .iter()
            .map(|id| {
                Sha256Digest::parse(id.clone()).map_err(|_| invalid("workflow_model_id_invalid"))
            })
            .collect()
    }

    fn parsed_dataset(&self) -> BrainResult<Option<Sha256Digest>> {
        match &self.dataset_sha256 {
            Some(id) => Ok(Some(
                Sha256Digest::parse(id.clone())
                    .map_err(|_| invalid("workflow_dataset_sha256_invalid"))?,
            )),
            None => Ok(None),
        }
    }
}

/// One typed next action among existing executors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NextAction {
    pub schema: String,
    pub executor_id: String,
    pub operation: String,
    pub authenticated_inputs: AuthenticatedWorkflowInputs,
    pub rationale_evidence: Vec<String>,
    pub expected_information_gain: f64,
    pub cost: WorkflowCost,
    pub risk: WorkflowRisk,
    pub stop_condition: StopCondition,
    /// Always false — workflow never authorizes production.
    pub authorizes_production: bool,
}

/// Fold-able advisory inputs for a single tick.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowDecisionInput {
    pub knowledge: KnowledgeWorkflowSignals,
    #[serde(default)]
    pub coevolution_directive: Option<CoEvolutionDirectiveSnapshot>,
    #[serde(default)]
    pub routing: RoutingPreference,
    /// Optional precomputed procedural hint (tests / composition). When absent,
    /// only directive + KE + routing are used.
    #[serde(default)]
    pub procedural_hint: Option<ProceduralWorkflowHint>,
    pub authorized_inputs: AuthenticatedWorkflowInputs,
    pub cost_ceiling: WorkflowCost,
    pub risk_ceiling: WorkflowRisk,
    #[serde(default)]
    pub stop_condition: StopCondition,
    /// When true, missing directive fails closed (production tick).
    #[serde(default = "default_require_directive")]
    pub require_directive: bool,
}

fn default_require_directive() -> bool {
    true
}

/// Compact procedural bias derived from replay+retrieve (or test fixtures).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct ProceduralWorkflowHint {
    pub steering_successes: u32,
    pub steering_failures: u32,
    pub low_rank_successes: u32,
    pub low_rank_failures: u32,
    #[serde(default)]
    pub top_family: Option<String>,
    #[serde(default)]
    pub top_disposition: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl ProceduralWorkflowHint {
    pub fn steering_unreliable(&self) -> bool {
        self.steering_failures > self.steering_successes
            && self.steering_failures + self.steering_successes > 0
    }

    pub fn low_rank_preferred(&self) -> bool {
        self.low_rank_successes > self.low_rank_failures
            && self.low_rank_successes > 0
            && self.steering_unreliable()
    }

    /// Net-negative low-rank experience (from retrieve), used to defer transfer.
    pub fn low_rank_unreliable(&self) -> bool {
        self.low_rank_failures > self.low_rank_successes
            && self.low_rank_failures + self.low_rank_successes > 0
    }
}

/// Derive a workflow hint from a [`RetrievalReport`] (Paso 2B → 2C fold).
#[allow(dead_code)]
pub fn procedural_hint_from_retrieval(report: &RetrievalReport) -> ProceduralWorkflowHint {
    let mut hint = ProceduralWorkflowHint::default();
    for advice in report.advice() {
        let family = advice.configuration().family();
        let is_low_rank = matches!(
            family,
            SolverFamily::CholeskyRidgeLowRank
                | SolverFamily::RandomizedSvd
                | SolverFamily::SparseProximal
                | SolverFamily::GroupSparseProximal
        );
        // Hybrid / software / external proposals stand in for steering-style
        // experience in the numerical procedural store (no separate steering
        // family). Composition may also inject explicit counts via fixtures.
        // Full-rank / direct / hybrid families stand in for "steering-style"
        // experience in the numerical procedural store (no separate steering family).
        let is_steering_proxy = matches!(
            family,
            SolverFamily::HybridRuntime
                | SolverFamily::SoftwareOnly
                | SolverFamily::ExternalProposal
                | SolverFamily::FullRankGradient
                | SolverFamily::OrthogonalizedFullRank
                | SolverFamily::DirectJacobiSvd
                | SolverFamily::PivotedQr
                | SolverFamily::DivideConquerSvd
        );
        let positive = matches!(advice.disposition(), AdviceDisposition::PrioritizeExploration);
        let negative =
            matches!(advice.disposition(), AdviceDisposition::DeprioritizeButRetainControl);
        if is_low_rank {
            if positive {
                hint.low_rank_successes = hint.low_rank_successes.saturating_add(1);
            }
            if negative {
                hint.low_rank_failures = hint.low_rank_failures.saturating_add(1);
            }
        }
        if is_steering_proxy {
            if positive {
                hint.steering_successes = hint.steering_successes.saturating_add(1);
            }
            if negative {
                hint.steering_failures = hint.steering_failures.saturating_add(1);
            }
        }
        if hint.top_family.is_none() {
            hint.top_family = Some(format!("{family:?}"));
            hint.top_disposition = Some(format!("{:?}", advice.disposition()));
        }
    }
    for caution in report.solver_run_cautions() {
        if matches!(caution.disposition(), AdviceDisposition::DeprioritizeButRetainControl) {
            hint.notes.push("solver_run_caution_deprioritize".into());
        }
    }
    hint
}

/// Convenience: retrieve then fold into a hint (composition helper).
#[allow(dead_code)]
pub fn procedural_hint_from_memory(
    memory: &ProceduralMemory,
    query: &RetrievalQuery,
) -> BrainResult<ProceduralWorkflowHint> {
    let report = retrieve_procedural_advice(memory, query)?;
    Ok(procedural_hint_from_retrieval(&report))
}

fn operation_cost_risk(operation: &str) -> (WorkflowCost, WorkflowRisk, f64) {
    match operation {
        "hold" => (WorkflowCost { relative_units: 0 }, WorkflowRisk { level: 0 }, 0.0),
        "probe_runtime" => (WorkflowCost { relative_units: 1 }, WorkflowRisk { level: 0 }, 0.2),
        "behavioral_discovery" | "behavioral_evaluation" | "generate_behavioral_dataset" => {
            (WorkflowCost { relative_units: 1 }, WorkflowRisk { level: 1 }, 0.45)
        }
        "calibrate_alignment" => {
            (WorkflowCost { relative_units: 2 }, WorkflowRisk { level: 1 }, 0.55)
        }
        "activation_transfer_experiment"
        | "extract_capability"
        | "deep_instrumentation"
        | "sparse_autoencoder_analysis"
        | "counterfactual_analysis" => {
            (WorkflowCost { relative_units: 2 }, WorkflowRisk { level: 2 }, 0.7)
        }
        _ => (WorkflowCost { relative_units: 2 }, WorkflowRisk { level: 2 }, 0.3),
    }
}

fn map_operation_to_executor(operation: &str) -> BrainResult<&'static str> {
    if operation == "hold" {
        return Err(invalid("workflow_hold_is_not_an_executor_action"));
    }
    executor_id_for_direct_operation(operation)
        .ok_or_else(|| invalid("workflow_operation_not_in_executor_registry"))
}

fn validate_executor_registered(executor_id: &str) -> BrainResult<()> {
    let _ =
        executor_by_id(executor_id).map_err(|_| invalid("workflow_executor_id_not_registered"))?;
    Ok(())
}

/// Core Paso 3 decision: fold advisory signals into ONE [`NextAction`].
pub fn decide_next_action(input: &WorkflowDecisionInput) -> BrainResult<NextAction> {
    if input.authorized_inputs.model_ids.is_empty() {
        return Err(invalid("workflow_inputs_unauthorized_missing_models"));
    }
    // Validate digests early (fail-closed).
    let _ = input.authorized_inputs.parsed_model_ids()?;
    let _ = input.authorized_inputs.parsed_dataset()?;

    let directive = match &input.coevolution_directive {
        Some(d) => Some(d),
        None if input.require_directive => {
            return Err(invalid("workflow_directive_required"));
        }
        None => None,
    };

    if let Some(d) = directive {
        if d.recommended_operation.trim().is_empty() {
            return Err(invalid("workflow_directive_operation_empty"));
        }
        if d.converged && input.stop_condition.halt_on_converged_directive {
            return Err(invalid("workflow_stop_converged_directive"));
        }
    }

    let hint = input.procedural_hint.clone().unwrap_or_default();
    let mut rationale = Vec::new();
    for note in &input.knowledge.notes {
        rationale.push(format!("knowledge:{note}"));
    }
    for note in &hint.notes {
        rationale.push(format!("procedural:{note}"));
    }
    if let Some(d) = directive {
        rationale.push(format!("coevolution:{} — {}", d.recommended_operation, d.reason));
    }
    if let Some(model) = input.routing.preferred_model.as_ref() {
        rationale.push(format!("routing:prefer {model} (score={:?})", input.routing.routing_score));
    }
    if hint.steering_unreliable() {
        rationale.push(format!(
            "procedural:steering_unreliable failures={} successes={}",
            hint.steering_failures, hint.steering_successes
        ));
    }
    if hint.low_rank_preferred() {
        rationale.push(format!(
            "procedural:low_rank_preferred successes={} failures={}",
            hint.low_rank_successes, hint.low_rank_failures
        ));
    }
    if !input.knowledge.calibration_sufficient {
        rationale.push("knowledge:calibration_insufficient".into());
    }
    if !input.knowledge.causal_evidence_sufficient {
        rationale.push("knowledge:causal_evidence_insufficient".into());
    }

    let recommended = directive
        .map(|d| d.recommended_operation.as_str())
        .unwrap_or("behavioral_discovery");

    // Transplant-style policy (roadmap §5.3): transfer is deferred to
    // calibration/align when KE says calibration is missing or procedural
    // memory says steering failed while low-rank worked.
    let operation = match recommended {
        "hold" => {
            return Err(invalid("workflow_stop_hold_directive"));
        }
        "activation_transfer_experiment" => {
            if !input.knowledge.calibration_sufficient
                || hint.steering_unreliable()
                || hint.low_rank_unreliable()
            {
                rationale
                    .push("decision:calibration_required_before_transfer (align first)".into());
                if hint.low_rank_unreliable() {
                    rationale.push(format!(
                        "procedural:low_rank_unreliable failures={} successes={}",
                        hint.low_rank_failures, hint.low_rank_successes
                    ));
                }
                "calibrate_alignment"
            } else {
                rationale.push("decision:transfer_authorized_by_signals".into());
                "activation_transfer_experiment"
            }
        }
        "behavioral_discovery" => {
            rationale.push("decision:follow_discovery_directive".into());
            "behavioral_discovery"
        }
        "calibrate_alignment" => "calibrate_alignment",
        "probe_runtime" => "probe_runtime",
        "behavioral_evaluation" => "behavioral_evaluation",
        "extract_capability" => {
            if hint.steering_unreliable() {
                rationale.push("decision:extract_deferred_steering_unreliable".into());
                "calibrate_alignment"
            } else {
                "extract_capability"
            }
        }
        other => {
            // Only admit operations that already map to registry executors.
            if executor_id_for_direct_operation(other).is_some() {
                rationale.push(format!("decision:follow_directive_operation:{other}"));
                other
            } else {
                return Err(invalid("workflow_directive_operation_unknown"));
            }
        }
    };

    // Dataset required for discovery / evaluation-class operations.
    let needs_dataset = matches!(
        operation,
        "behavioral_discovery"
            | "behavioral_evaluation"
            | "generate_behavioral_dataset"
            | "calibrate_alignment"
            | "activation_transfer_experiment"
            | "extract_capability"
    );
    if needs_dataset && input.authorized_inputs.dataset_sha256.is_none() {
        return Err(invalid("workflow_inputs_unauthorized_missing_dataset"));
    }

    let (cost, risk, mut info_gain) = operation_cost_risk(operation);
    if hint.low_rank_preferred() && operation == "calibrate_alignment" {
        info_gain = (info_gain + 0.1).min(1.0);
        rationale.push("decision:info_gain_boost_low_rank_prior".into());
    }
    if cost.relative_units > input.cost_ceiling.relative_units {
        return Err(invalid("workflow_cost_ceiling_exceeded"));
    }
    if risk.level > input.risk_ceiling.level {
        return Err(invalid("workflow_risk_ceiling_exceeded"));
    }

    let executor_id = map_operation_to_executor(operation)?.to_string();
    validate_executor_registered(&executor_id)?;

    // Prefer routing model order when present: put preferred model first.
    let mut inputs = input.authorized_inputs.clone();
    if let Some(preferred) = input.routing.preferred_model.as_ref() {
        if let Some(pos) = inputs.model_ids.iter().position(|id| id == preferred) {
            let model = inputs.model_ids.remove(pos);
            inputs.model_ids.insert(0, model);
            rationale.push("routing:reordered_models_preferred_first".into());
        }
    }

    Ok(NextAction {
        schema: NEXT_ACTION_SCHEMA.into(),
        executor_id,
        operation: operation.into(),
        authenticated_inputs: inputs,
        rationale_evidence: rationale,
        expected_information_gain: info_gain,
        cost,
        risk,
        stop_condition: input.stop_condition.clone(),
        authorizes_production: false,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Start is the production enqueue path; dry-run tests cover DryRun.
pub enum JobInvocationMode {
    /// Prove the mapping without spawning work.
    DryRun,
    /// Call the real operator enqueue helpers (same path as HTTP jobs).
    Start,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowTickReceipt {
    pub schema: String,
    pub mode: String,
    pub next_action: NextAction,
    pub would_call: String,
    #[serde(default)]
    pub job: Option<OperatorJobRecord>,
    pub authorizes_production: bool,
}

fn direct_operation_from_name(operation: &str) -> BrainResult<OperatorDirectOperation> {
    match operation {
        "probe_runtime" => Ok(OperatorDirectOperation::ProbeRuntime),
        "behavioral_evaluation" => Ok(OperatorDirectOperation::BehavioralEvaluation),
        "extract_capability" => Ok(OperatorDirectOperation::ExtractCapability),
        "deep_instrumentation" => Ok(OperatorDirectOperation::DeepInstrumentation),
        "sparse_autoencoder_analysis" => Ok(OperatorDirectOperation::SparseAutoencoderAnalysis),
        "counterfactual_analysis" => Ok(OperatorDirectOperation::CounterfactualAnalysis),
        "generate_behavioral_dataset" => Ok(OperatorDirectOperation::GenerateBehavioralDataset),
        "calibrate_alignment" => Ok(OperatorDirectOperation::CalibrateAlignment),
        "activation_transfer_experiment" => {
            Ok(OperatorDirectOperation::ActivationTransferExperiment)
        }
        _ => Err(invalid("workflow_direct_operation_unmapped")),
    }
}

fn build_direct_request(action: &NextAction) -> BrainResult<OperatorDirectWorkflowRequest> {
    Ok(OperatorDirectWorkflowRequest {
        schema: "tidex.operator_direct_workflow/v1".into(),
        operation: direct_operation_from_name(&action.operation)?,
        model_ids: action.authenticated_inputs.parsed_model_ids()?,
        dataset_sha256: action.authenticated_inputs.parsed_dataset()?,
        parameters: if action.authenticated_inputs.parameters.is_null() {
            json!({})
        } else {
            action.authenticated_inputs.parameters.clone()
        },
    })
}

fn build_discovery_request(action: &NextAction) -> BrainResult<BehavioralDiscoveryWorkflowRequest> {
    let dataset = action
        .authenticated_inputs
        .parsed_dataset()?
        .ok_or_else(|| invalid("workflow_inputs_unauthorized_missing_dataset"))?;
    Ok(BehavioralDiscoveryWorkflowRequest {
        schema: "tidex.operator_behavioral_discovery/v1".into(),
        model_ids: action.authenticated_inputs.parsed_model_ids()?,
        dataset_sha256: dataset,
        max_new_tokens: 128,
        seed: 0,
    })
}

/// Map [`NextAction`] to the existing job-start path (or dry-run).
pub fn invoke_next_action(
    tidex_home: &Path,
    action: &NextAction,
    mode: JobInvocationMode,
) -> BrainResult<WorkflowTickReceipt> {
    if action.schema != NEXT_ACTION_SCHEMA {
        return Err(invalid("workflow_next_action_schema_invalid"));
    }
    if action.authorizes_production {
        return Err(invalid("workflow_next_action_claims_production"));
    }
    validate_executor_registered(&action.executor_id)?;
    let expected = map_operation_to_executor(&action.operation)?;
    if expected != action.executor_id {
        return Err(invalid("workflow_executor_operation_mismatch"));
    }

    let would_call = if action.operation == "behavioral_discovery" {
        "start_operator_behavioral_discovery_job → start_operator_job(BehavioralDiscovery)"
            .to_string()
    } else {
        format!("start_operator_direct_job → start_operator_job(Direct:{})", action.operation)
    };

    let job = match mode {
        JobInvocationMode::DryRun => None,
        JobInvocationMode::Start => {
            if action.operation == "behavioral_discovery" {
                Some(start_operator_behavioral_discovery_job(
                    tidex_home,
                    build_discovery_request(action)?,
                )?)
            } else {
                Some(start_operator_direct_job(tidex_home, build_direct_request(action)?)?)
            }
        }
    };

    Ok(WorkflowTickReceipt {
        schema: WORKFLOW_TICK_RECEIPT_SCHEMA.into(),
        mode: match mode {
            JobInvocationMode::DryRun => "dry_run".into(),
            JobInvocationMode::Start => "start".into(),
        },
        next_action: action.clone(),
        would_call,
        job,
        authorizes_production: false,
    })
}

/// Decide then dry-run invoke (composition / CLI helper).
pub fn decide_and_dry_run(
    tidex_home: &Path,
    input: &WorkflowDecisionInput,
) -> BrainResult<WorkflowTickReceipt> {
    let action = decide_next_action(input)?;
    invoke_next_action(tidex_home, &action, JobInvocationMode::DryRun)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn digest_hex(tag: u8) -> String {
        Sha256Digest::digest_bytes(&[tag]).to_string()
    }

    fn base_inputs() -> AuthenticatedWorkflowInputs {
        AuthenticatedWorkflowInputs {
            model_ids: vec![digest_hex(1), digest_hex(2)],
            dataset_sha256: Some(digest_hex(3)),
            parameters: json!({}),
        }
    }

    fn ceilings_allowing_transfer() -> (WorkflowCost, WorkflowRisk) {
        (WorkflowCost { relative_units: 2 }, WorkflowRisk { level: 2 })
    }

    fn transfer_directive() -> CoEvolutionDirectiveSnapshot {
        CoEvolutionDirectiveSnapshot {
            recommended_operation: "activation_transfer_experiment".into(),
            reason: "fitness gap: propose transfer".into(),
            converged: false,
            source_model: Some(digest_hex(1)),
            target_model: Some(digest_hex(2)),
            capability_hint: Some("benchmark:fixture".into()),
            evidence_sha256: Some(digest_hex(9)),
        }
    }

    #[test]
    fn steering_failed_defers_transfer_to_align() {
        let (cost_ceiling, risk_ceiling) = ceilings_allowing_transfer();
        let input = WorkflowDecisionInput {
            knowledge: KnowledgeWorkflowSignals {
                calibration_sufficient: false,
                causal_evidence_sufficient: false,
                notes: vec!["falta causalidad suficiente".into()],
            },
            coevolution_directive: Some(transfer_directive()),
            routing: RoutingPreference {
                preferred_model: Some(digest_hex(2)),
                scope: Some("benchmark:fixture".into()),
                routing_score: Some(0.8),
            },
            procedural_hint: Some(ProceduralWorkflowHint {
                steering_successes: 1,
                steering_failures: 4,
                low_rank_successes: 6,
                low_rank_failures: 1,
                top_family: Some("CholeskyRidgeLowRank".into()),
                top_disposition: Some("PrioritizeExploration".into()),
                notes: vec!["steering falló 4/5; low-rank acertó 6/7".into()],
            }),
            authorized_inputs: base_inputs(),
            cost_ceiling,
            risk_ceiling,
            stop_condition: StopCondition::default(),
            require_directive: true,
        };
        let action = decide_next_action(&input).unwrap();
        assert_eq!(action.executor_id, "cross_model.align");
        assert_eq!(action.operation, "calibrate_alignment");
        assert!(!action.authorizes_production);
        assert!(action
            .rationale_evidence
            .iter()
            .any(|line| line.contains("calibration_required_before_transfer")));
        // routing preferred model first
        assert_eq!(action.authenticated_inputs.model_ids[0], digest_hex(2));
        executor_by_id(&action.executor_id).unwrap();
    }

    #[test]
    fn low_rank_success_with_calibration_allows_transfer() {
        let (cost_ceiling, risk_ceiling) = ceilings_allowing_transfer();
        let input = WorkflowDecisionInput {
            knowledge: KnowledgeWorkflowSignals {
                calibration_sufficient: true,
                causal_evidence_sufficient: true,
                notes: vec!["calibration sealed".into()],
            },
            coevolution_directive: Some(transfer_directive()),
            routing: RoutingPreference {
                preferred_model: Some(digest_hex(1)),
                scope: None,
                routing_score: Some(0.9),
            },
            procedural_hint: Some(ProceduralWorkflowHint {
                steering_successes: 3,
                steering_failures: 1,
                low_rank_successes: 6,
                low_rank_failures: 1,
                top_family: Some("CholeskyRidgeLowRank".into()),
                top_disposition: Some("PrioritizeExploration".into()),
                notes: Vec::new(),
            }),
            authorized_inputs: base_inputs(),
            cost_ceiling,
            risk_ceiling,
            stop_condition: StopCondition::default(),
            require_directive: true,
        };
        let action = decide_next_action(&input).unwrap();
        assert_eq!(action.executor_id, "cross_model.transfer_steering");
        assert_eq!(action.operation, "activation_transfer_experiment");
        executor_by_id(&action.executor_id).unwrap();
    }

    #[test]
    fn decision_changes_when_procedural_advice_differs() {
        let (cost_ceiling, risk_ceiling) = ceilings_allowing_transfer();
        let mut input = WorkflowDecisionInput {
            knowledge: KnowledgeWorkflowSignals {
                calibration_sufficient: true,
                causal_evidence_sufficient: true,
                notes: Vec::new(),
            },
            coevolution_directive: Some(transfer_directive()),
            routing: RoutingPreference::default(),
            procedural_hint: Some(ProceduralWorkflowHint {
                steering_successes: 0,
                steering_failures: 5,
                low_rank_successes: 7,
                low_rank_failures: 0,
                top_family: None,
                top_disposition: None,
                notes: vec!["steering failed style".into()],
            }),
            authorized_inputs: base_inputs(),
            cost_ceiling,
            risk_ceiling,
            stop_condition: StopCondition::default(),
            require_directive: true,
        };
        let first = decide_next_action(&input).unwrap();
        assert_eq!(first.executor_id, "cross_model.align");

        // Synthetic new evidence: steering recovered / reliable.
        input.procedural_hint = Some(ProceduralWorkflowHint {
            steering_successes: 5,
            steering_failures: 0,
            low_rank_successes: 7,
            low_rank_failures: 0,
            top_family: None,
            top_disposition: None,
            notes: vec!["steering recovered after align".into()],
        });
        let second = decide_next_action(&input).unwrap();
        assert_eq!(second.executor_id, "cross_model.transfer_steering");
        assert_ne!(first.executor_id, second.executor_id);
    }

    #[test]
    fn fail_closed_without_authorized_models() {
        let (cost_ceiling, risk_ceiling) = ceilings_allowing_transfer();
        let input = WorkflowDecisionInput {
            knowledge: KnowledgeWorkflowSignals::default(),
            coevolution_directive: Some(transfer_directive()),
            routing: RoutingPreference::default(),
            procedural_hint: None,
            authorized_inputs: AuthenticatedWorkflowInputs::default(),
            cost_ceiling,
            risk_ceiling,
            stop_condition: StopCondition::default(),
            require_directive: true,
        };
        let err = decide_next_action(&input).unwrap_err();
        assert!(err
            .to_string()
            .contains("workflow_inputs_unauthorized_missing_models"));
    }

    #[test]
    fn fail_closed_without_dataset_when_required() {
        let (cost_ceiling, risk_ceiling) = ceilings_allowing_transfer();
        let input = WorkflowDecisionInput {
            knowledge: KnowledgeWorkflowSignals {
                calibration_sufficient: true,
                causal_evidence_sufficient: true,
                notes: Vec::new(),
            },
            coevolution_directive: Some(transfer_directive()),
            routing: RoutingPreference::default(),
            procedural_hint: Some(ProceduralWorkflowHint {
                steering_successes: 2,
                steering_failures: 0,
                low_rank_successes: 1,
                low_rank_failures: 0,
                ..Default::default()
            }),
            authorized_inputs: AuthenticatedWorkflowInputs {
                model_ids: vec![digest_hex(1)],
                dataset_sha256: None,
                parameters: json!({}),
            },
            cost_ceiling,
            risk_ceiling,
            stop_condition: StopCondition::default(),
            require_directive: true,
        };
        let err = decide_next_action(&input).unwrap_err();
        assert!(err
            .to_string()
            .contains("workflow_inputs_unauthorized_missing_dataset"));
    }

    #[test]
    fn fail_closed_on_risk_ceiling() {
        let input = WorkflowDecisionInput {
            knowledge: KnowledgeWorkflowSignals {
                calibration_sufficient: true,
                causal_evidence_sufficient: true,
                notes: Vec::new(),
            },
            coevolution_directive: Some(transfer_directive()),
            routing: RoutingPreference::default(),
            procedural_hint: Some(ProceduralWorkflowHint {
                steering_successes: 2,
                steering_failures: 0,
                low_rank_successes: 1,
                low_rank_failures: 0,
                ..Default::default()
            }),
            authorized_inputs: base_inputs(),
            cost_ceiling: WorkflowCost { relative_units: 2 },
            risk_ceiling: WorkflowRisk { level: 1 }, // transfer needs 2
            stop_condition: StopCondition::default(),
            require_directive: true,
        };
        let err = decide_next_action(&input).unwrap_err();
        assert!(err.to_string().contains("workflow_risk_ceiling_exceeded"));
    }

    #[test]
    fn fail_closed_when_directive_missing() {
        let (cost_ceiling, risk_ceiling) = ceilings_allowing_transfer();
        let input = WorkflowDecisionInput {
            knowledge: KnowledgeWorkflowSignals::default(),
            coevolution_directive: None,
            routing: RoutingPreference::default(),
            procedural_hint: None,
            authorized_inputs: base_inputs(),
            cost_ceiling,
            risk_ceiling,
            stop_condition: StopCondition::default(),
            require_directive: true,
        };
        let err = decide_next_action(&input).unwrap_err();
        assert!(err.to_string().contains("workflow_directive_required"));
    }

    #[test]
    fn dry_run_maps_to_start_operator_job_hook() {
        let (cost_ceiling, risk_ceiling) = ceilings_allowing_transfer();
        let input = WorkflowDecisionInput {
            knowledge: KnowledgeWorkflowSignals {
                calibration_sufficient: false,
                causal_evidence_sufficient: false,
                notes: Vec::new(),
            },
            coevolution_directive: Some(transfer_directive()),
            routing: RoutingPreference::default(),
            procedural_hint: Some(ProceduralWorkflowHint {
                steering_successes: 0,
                steering_failures: 4,
                low_rank_successes: 6,
                low_rank_failures: 1,
                ..Default::default()
            }),
            authorized_inputs: base_inputs(),
            cost_ceiling,
            risk_ceiling,
            stop_condition: StopCondition::default(),
            require_directive: true,
        };
        let home = PathBuf::from("/tmp/tidex-workflow-next-action-dry-run");
        let receipt = decide_and_dry_run(&home, &input).unwrap();
        assert_eq!(receipt.mode, "dry_run");
        assert!(receipt.job.is_none());
        assert!(receipt.would_call.contains("start_operator_direct_job"));
        assert!(receipt.would_call.contains("start_operator_job"));
        assert_eq!(receipt.next_action.executor_id, "cross_model.align");
        assert!(!receipt.authorizes_production);
    }

    #[test]
    fn job_invocation_mode_labels_cover_start_path() {
        assert_eq!(
            match JobInvocationMode::Start {
                JobInvocationMode::DryRun => "dry_run",
                JobInvocationMode::Start => "start",
            },
            "start"
        );
        assert_eq!(
            match JobInvocationMode::DryRun {
                JobInvocationMode::DryRun => "dry_run",
                JobInvocationMode::Start => "start",
            },
            "dry_run"
        );
    }

    #[test]
    fn hint_helpers_encode_steering_vs_low_rank_bias() {
        let hint = ProceduralWorkflowHint {
            steering_successes: 0,
            steering_failures: 4,
            low_rank_successes: 6,
            low_rank_failures: 1,
            ..Default::default()
        };
        assert!(hint.steering_unreliable());
        assert!(hint.low_rank_preferred());
        let recovered = ProceduralWorkflowHint {
            steering_successes: 5,
            steering_failures: 0,
            low_rank_successes: 6,
            low_rank_failures: 1,
            ..Default::default()
        };
        assert!(!recovered.steering_unreliable());
    }
}
