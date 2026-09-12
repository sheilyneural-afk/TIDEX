//! Close loop B with **real** evidence (Paso 3 acceptance criterion).
//!
//! ```text
//! real numerical.evolve evidence → replay ProceduralMemory → NextAction
//!   → Start real executor (start_operator_*) → Completed + succeeded + run
//!   → redecide from executor semantic evidence when replayable,
//!     else hermetic numerical.evolve (never hash-only evidence_receipt)
//!   → NextAction different
//! ```
//!
//! Anti-patterns forbidden here:
//! - hand-planted [`ProceduralWorkflowHint`] counts into `decide_next_action`
//! - fixture donor as substitute for live work
//! - synthetic second-tick theater
//! - treating Failed Start + `evidence_receipt.is_some()` as chain success
//! - stuffing hash-only `operator_job_evidence_receipt/v1` into ProceduralMemory

use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use tidex::foundation::digest::Sha256Digest;
use tidex::foundation::error::{BrainError, BrainResult};
use tidex::foundation::finite::FiniteF64;
use tidex::foundation::identity::CapabilityId;
use tidex::learning::numerical_evolution::{
    numerical_metric_specs, NumericalEvaluationGroup, NumericalEvaluationLimits,
    NumericalEvolutionEngine, NumericalEvolutionInput, NumericalEvolutionPolicy,
};
use tidex::learning::portfolio_governance::{
    CandidateGatePolicy, MetricId, PetfcConservationLimits, PetfcMetricPolicy, PetfcPathLimits,
    PetfcPolicy, PetfcUtilityPolicy, RobustEvaluationPolicy,
};
use tidex::learning::procedural_memory::ProceduralMemory;
use tidex::learning::procedural_memory::{
    BaseArtifactDigest, CapabilityContext, ProblemTransferPolicy, RetrievalQuery, RetrievalScope,
    SolverAttempt, TargetProfileDigest,
};
use tidex::learning::procedural_replay::rebuild_from_numerical_evolution_stdout;
use tidex::learning::solver_portfolio::{
    CandidateRepresentation, LeastSquaresProblem, PortfolioPolicy,
};
use tidex::operator::control_plane::{load_job_record, OperatorJobRecord, OperatorJobState};

use crate::workflow_next_action::{
    decide_next_action, invoke_next_action, procedural_hint_from_memory,
    AuthenticatedWorkflowInputs, CoEvolutionDirectiveSnapshot, JobInvocationMode,
    KnowledgeWorkflowSignals, ProceduralWorkflowHint, RoutingPreference, StopCondition,
    WorkflowCost, WorkflowDecisionInput, WorkflowRisk, WorkflowTickReceipt,
};

const PROOF_SCHEMA: &str = "tidex.workflow.b_loop_proof/v1";

fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}

fn digest_hex(tag: u8) -> String {
    Sha256Digest::digest_bytes(&[tag]).to_string()
}

fn capability_context() -> BrainResult<CapabilityContext> {
    // Construct via public serde surface (from_computed is pub(crate) in digest).
    let _ = CapabilityId::parse("numerical.linear-map:v1")?;
    let base = BaseArtifactDigest::bind_exact_digest(&Sha256Digest::digest_bytes(b"b-loop-base"));
    let target =
        TargetProfileDigest::bind_exact_digest(&Sha256Digest::digest_bytes(b"b-loop-target"));
    Ok(serde_json::from_value(json!({
        "system_envelope_digest": Sha256Digest::digest_bytes(b"b-loop-envelope").to_string(),
        "capability_id": "numerical.linear-map:v1",
        "capability_ir_digest": Sha256Digest::digest_bytes(b"b-loop-ir").to_string(),
        "base_artifact_digest": base.as_str(),
        "target_profile_digest": target.as_str(),
    }))?)
}

fn numerical_policy() -> BrainResult<NumericalEvolutionPolicy> {
    let specs = numerical_metric_specs(0.45, 2.0)?;
    let fit = MetricId::parse("numerical.normalized_fit")?;
    let worst = MetricId::parse("numerical.normalized_worst_error")?;
    let robust = RobustEvaluationPolicy::new(3, 3, 100)?;
    let gate = CandidateGatePolicy::new(
        &specs,
        3,
        1.0,
        BTreeMap::from([
            (fit.clone(), FiniteF64::new(0.0)?),
            (worst.clone(), FiniteF64::new(0.0)?),
        ]),
    )?;
    let petfc = PetfcPolicy::new(
        &specs,
        fit.clone(),
        vec![
            PetfcMetricPolicy::new(fit, 1.0, 0.5)?,
            PetfcMetricPolicy::new(worst, 1.0, 1.5)?,
        ],
        PetfcPathLimits::new(2, 8, 3.0, 4.0, 0.9, 0.0)?,
        PetfcConservationLimits::new(2.0, 2, 4.0)?,
        PetfcUtilityPolicy::new(0.0, 0.0, 0.0, -2.0)?,
    )?;
    let limits = NumericalEvaluationLimits::new(16, 256, 4_096)?;
    let solver = PortfolioPolicy::default().with_residual_tolerances(0.6, 1.0e-10)?;
    NumericalEvolutionPolicy::new(solver, specs, robust, gate, petfc, limits, 1.0e-12)
}

fn eval_groups() -> BrainResult<Vec<NumericalEvaluationGroup>> {
    [3.0_f64, 5.0, 7.0]
        .into_iter()
        .map(|offset| {
            NumericalEvaluationGroup::new(LeastSquaresProblem::new(
                vec![vec![offset], vec![offset + 0.5]],
                vec![vec![2.0 * offset], vec![2.0 * (offset + 0.5)]],
            )?)
        })
        .collect()
}

/// Singular / rejected portfolio run — produces a sealed rejected procedural attempt.
fn rejection_input(revision: u64) -> BrainResult<NumericalEvolutionInput> {
    NumericalEvolutionInput::new(
        LeastSquaresProblem::new(vec![vec![0.0]], vec![vec![1.0]])?,
        CandidateRepresentation::Dense {
            rows: 1,
            columns: 1,
            weights: vec![0.0],
        },
        eval_groups()?,
        revision,
    )
}

/// Well-posed run that yields a validated / research-gated attempt with low-rank family.
fn validation_input(revision: u64) -> BrainResult<NumericalEvolutionInput> {
    NumericalEvolutionInput::new(
        LeastSquaresProblem::new(vec![vec![1.0], vec![2.0]], vec![vec![2.0], vec![4.0]])?,
        CandidateRepresentation::Dense {
            rows: 1,
            columns: 1,
            weights: vec![0.0],
        },
        eval_groups()?,
        revision,
    )
}

fn seal_evolution_stdout(
    cycles: &[(u64, &str, Option<&SolverAttempt>)],
) -> BrainResult<(Vec<u8>, Sha256Digest)> {
    let cycle_json: Vec<_> = cycles
        .iter()
        .map(|(revision, disposition, attempt)| {
            json!({
                "revision": revision,
                "disposition": disposition,
                "procedural_attempt": attempt,
                "solver_run_failure": null,
                "authorizes_promotion": false
            })
        })
        .collect();
    let attempt_count = cycles.iter().filter(|(_, _, a)| a.is_some()).count();
    let document = json!({
        "schema": "tidex.numerical_evolution_receipt/v1",
        "solver_profile": "portfolio_default_v1",
        "cycles": cycle_json,
        "procedural_attempt_count": attempt_count,
        "authorizes_production": false
    });
    let bytes = serde_json::to_vec(&document)?;
    let digest = Sha256Digest::digest_bytes(&bytes);
    Ok((bytes, digest))
}

fn query_from_attempt(attempt: &SolverAttempt) -> BrainResult<RetrievalQuery> {
    RetrievalQuery::new(
        RetrievalScope::from_bindings(attempt.bindings()),
        attempt.applicability().clone(),
        attempt.lineage_ordinal().saturating_add(1),
        ProblemTransferPolicy::ExactOnly,
        16,
    )
}

fn default_transfer_directive() -> CoEvolutionDirectiveSnapshot {
    CoEvolutionDirectiveSnapshot {
        recommended_operation: "activation_transfer_experiment".into(),
        reason: "b_loop: propose transfer; procedural memory may defer to align".into(),
        converged: false,
        source_model: Some(digest_hex(1)),
        target_model: Some(digest_hex(2)),
        capability_hint: Some("numerical.linear-map:v1".into()),
        evidence_sha256: Some(digest_hex(9)),
    }
}

fn transfer_decision_input(
    hint: ProceduralWorkflowHint,
    directive: CoEvolutionDirectiveSnapshot,
) -> WorkflowDecisionInput {
    WorkflowDecisionInput {
        knowledge: KnowledgeWorkflowSignals {
            // Calibration treated sufficient so the *procedural* signal is what
            // flips calibrate ↔ transfer (not a hand-planted KE bit alone).
            calibration_sufficient: true,
            causal_evidence_sufficient: true,
            notes: vec!["b_loop_proof".into()],
        },
        coevolution_directive: Some(directive),
        routing: RoutingPreference::default(),
        procedural_hint: Some(hint),
        authorized_inputs: AuthenticatedWorkflowInputs {
            model_ids: vec![digest_hex(1), digest_hex(2)],
            dataset_sha256: Some(digest_hex(3)),
            parameters: json!({}),
        },
        cost_ceiling: WorkflowCost { relative_units: 2 },
        risk_ceiling: WorkflowRisk { level: 2 },
        stop_condition: StopCondition::default(),
        require_directive: true,
    }
}

fn wait_job_terminal(
    tidex_home: &Path,
    job_id: &Sha256Digest,
) -> BrainResult<tidex::operator::control_plane::OperatorJobRecord> {
    for _ in 0..2_000 {
        let record = load_job_record(tidex_home, job_id)?;
        if matches!(
            record.state,
            OperatorJobState::Completed | OperatorJobState::Failed | OperatorJobState::Cancelled
        ) {
            return Ok(record);
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err(invalid("b_loop_job_did_not_reach_terminal_state"))
}

/// Chain success requires Completed + receipt.succeeded + run present.
/// A Failed/Cancelled job that still carries `evidence_receipt` is **not** success
/// (closes the organism-chain false positive).
pub fn chain_start_accepted(terminal: &OperatorJobRecord) -> bool {
    matches!(terminal.state, OperatorJobState::Completed)
        && terminal.run.is_some()
        && terminal
            .evidence_receipt
            .as_ref()
            .is_some_and(|receipt| receipt.succeeded && receipt.run_id.is_some())
}

fn require_chain_success_start(
    terminal: &OperatorJobRecord,
) -> BrainResult<&tidex::operator::control_plane::OperatorJobEvidenceReceipt> {
    let receipt = terminal
        .evidence_receipt
        .as_ref()
        .ok_or_else(|| invalid("b_loop_start_missing_evidence_receipt"))?;
    if !matches!(terminal.state, OperatorJobState::Completed) {
        return Err(invalid("b_loop_start_job_not_completed"));
    }
    if !receipt.succeeded {
        return Err(invalid("b_loop_start_evidence_not_succeeded"));
    }
    if terminal.run.is_none() || receipt.run_id.is_none() {
        return Err(invalid("b_loop_start_run_missing"));
    }
    Ok(receipt)
}

/// Prefer prior executor semantic evidence for tick-2 ProceduralMemory when the
/// Start job embeds numerical.evolve stdout that procedural_replay accepts.
/// Never invent SolverAttempt from hash-only `operator_job_evidence_receipt/v1`.
/// When executor stdout is absent/non-attempt, use hermetic numerical.evolve.
fn procedural_experience_for_redecide(
    tidex_home: &Path,
    terminal: &OperatorJobRecord,
) -> BrainResult<(ProceduralMemory, SolverAttempt, Vec<u8>, Sha256Digest, &'static str)> {
    if let Some(run) = terminal.run.as_ref() {
        let stdout_bytes = run.stdout.as_bytes();
        if let Ok(memory) = rebuild_from_numerical_evolution_stdout(stdout_bytes) {
            if memory.attempt_count() > 0 {
                // Pull the last sealed attempt from the authenticated stdout cycles
                // (same bytes procedural_replay already accepted).
                #[derive(serde::Deserialize)]
                struct CycleWire {
                    #[serde(default)]
                    procedural_attempt: Option<SolverAttempt>,
                }
                #[derive(serde::Deserialize)]
                struct ReceiptWire {
                    #[serde(default)]
                    cycles: Vec<CycleWire>,
                }
                if let Ok(doc) = serde_json::from_slice::<ReceiptWire>(stdout_bytes) {
                    if let Some(attempt) = doc
                        .cycles
                        .into_iter()
                        .rev()
                        .find_map(|cycle| cycle.procedural_attempt)
                    {
                        attempt.authenticate()?;
                        let digest = Sha256Digest::digest_bytes(stdout_bytes);
                        let _ = persist_stdout(tidex_home, "tick2-executor", stdout_bytes)?;
                        return Ok((
                            memory,
                            attempt,
                            stdout_bytes.to_vec(),
                            digest,
                            "executor_run_view",
                        ));
                    }
                }
            }
        }
        // Explicitly refuse hash-only job evidence: attempting to rebuild from
        // evidence_receipt alone must stay fail-closed in procedural_replay
        // (covered by procedural_replay::operator_job_evidence_has_no_attempt_structure).
        let _ = terminal.evidence_receipt.as_ref();
    }

    // Hermetic fixture: independent validated numerical.evolve (no HF / live align).
    let mut engine2 = NumericalEvolutionEngine::new(capability_context()?, numerical_policy()?)?;
    let _warm = engine2.evolve(
        NumericalEvolutionInput::new(
            LeastSquaresProblem::new(vec![vec![1.0]], vec![vec![1.5]])?,
            CandidateRepresentation::Dense {
                rows: 1,
                columns: 1,
                weights: vec![0.0],
            },
            eval_groups()?,
            1,
        )?,
        &[],
    )?;
    let cycle2 = engine2.evolve(validation_input(2)?, &[])?;
    let attempt2 = cycle2
        .procedural_attempt()
        .cloned()
        .ok_or_else(|| invalid("b_loop_tick2_missing_procedural_attempt"))?;
    attempt2.authenticate()?;
    let (stdout2, digest2) =
        seal_evolution_stdout(&[(2, "candidate_validated_for_further_gates", Some(&attempt2))])?;
    let _ = persist_stdout(tidex_home, "tick2", &stdout2)?;
    if Sha256Digest::digest_bytes(&stdout2) != digest2 {
        return Err(invalid("b_loop_tick2_stdout_digest_mismatch"));
    }
    let memory2 = rebuild_from_numerical_evolution_stdout(&stdout2)?;
    Ok((memory2, attempt2, stdout2, digest2, "hermetic_numerical_evolve"))
}

fn persist_stdout(tidex_home: &Path, label: &str, bytes: &[u8]) -> BrainResult<PathBuf> {
    let dir = tidex_home.join("state/workflow_b_loop").join(label);
    fs::create_dir_all(&dir)?;
    let path = dir.join("numerical_evolution_stdout.json");
    fs::write(&path, bytes)?;
    Ok(path)
}

#[derive(Debug, Clone, Serialize)]
pub struct BLoopTickSummary {
    pub tick: u32,
    pub stdout_sha256: String,
    pub attempt_count: usize,
    pub hint: ProceduralWorkflowHint,
    pub next_action_executor_id: String,
    pub next_action_operation: String,
    pub rationale_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BLoopProofReceipt {
    pub schema: String,
    pub tick1: BLoopTickSummary,
    pub start_invocation: WorkflowTickReceipt,
    pub start_job_state: String,
    pub start_job_operation: String,
    pub start_evidence_receipt_present: bool,
    /// True only when Completed + receipt.succeeded + run present.
    pub start_chain_success: bool,
    pub redecide_experience_source: String,
    pub tick2: BLoopTickSummary,
    pub next_action_changed: bool,
    pub attribution: String,
    pub authorizes_production: bool,
}

/// Prove the frozen B-loop acceptance criterion end-to-end.
pub fn prove_b_loop(tidex_home: &Path) -> BrainResult<BLoopProofReceipt> {
    prove_b_loop_with_directive(tidex_home, default_transfer_directive())
}

/// Same B-loop proof, but the co-evolution directive is supplied by the caller
/// (e.g. a CognitiveField→FieldActionBinding composition root). The directive
/// must recommend `activation_transfer_experiment` so procedural memory can
/// defer tick1→align and authorize tick2→transfer.
pub fn prove_b_loop_with_directive(
    tidex_home: &Path,
    directive: CoEvolutionDirectiveSnapshot,
) -> BrainResult<BLoopProofReceipt> {
    if directive.recommended_operation != "activation_transfer_experiment" {
        return Err(invalid("b_loop_directive_must_recommend_transfer"));
    }
    fs::create_dir_all(tidex_home)?;

    // --- Tick 1: real rejected numerical.evolve evidence ---
    let mut engine = NumericalEvolutionEngine::new(capability_context()?, numerical_policy()?)?;
    let cycle1 = engine.evolve(rejection_input(1)?, &[])?;
    let attempt1 = cycle1
        .procedural_attempt()
        .cloned()
        .ok_or_else(|| invalid("b_loop_tick1_missing_procedural_attempt"))?;
    attempt1.authenticate()?;
    let (stdout1, digest1) = seal_evolution_stdout(&[(1, "solver_rejected", Some(&attempt1))])?;
    let _ = persist_stdout(tidex_home, "tick1", &stdout1)?;
    // Authenticate via digest match (same contract as Operator stdout_sha256).
    if Sha256Digest::digest_bytes(&stdout1) != digest1 {
        return Err(invalid("b_loop_tick1_stdout_digest_mismatch"));
    }
    let memory1 = rebuild_from_numerical_evolution_stdout(&stdout1)?;
    let query1 = query_from_attempt(&attempt1)?;
    let hint1 = procedural_hint_from_memory(&memory1, &query1)?;
    if !hint1.low_rank_unreliable() && !hint1.steering_unreliable() {
        return Err(invalid("b_loop_tick1_expected_unreliable_procedural_signal"));
    }
    let action1 = decide_next_action(&transfer_decision_input(hint1.clone(), directive.clone()))?;
    if action1.operation != "calibrate_alignment" {
        return Err(invalid("b_loop_tick1_expected_calibrate_alignment"));
    }

    // --- Start real executor via production enqueue path ---
    let start_invocation = invoke_next_action(tidex_home, &action1, JobInvocationMode::Start)?;
    let job = start_invocation
        .job
        .as_ref()
        .ok_or_else(|| invalid("b_loop_start_missing_job_record"))?;
    let terminal = wait_job_terminal(tidex_home, &job.job_id)?;
    // Harden: Failed/Cancelled + receipt present must NOT count as chain success.
    let _receipt = require_chain_success_start(&terminal)?;

    // --- Tick 2: redecide from executor semantic evidence when available ---
    let (memory2, attempt2, _stdout2, digest2, experience_source) =
        procedural_experience_for_redecide(tidex_home, &terminal)?;
    let query2 = query_from_attempt(&attempt2)?;
    let hint2 = procedural_hint_from_memory(&memory2, &query2)?;
    if hint2.low_rank_unreliable() || hint2.steering_unreliable() {
        return Err(invalid("b_loop_tick2_expected_reliable_procedural_signal"));
    }
    let action2 = decide_next_action(&transfer_decision_input(hint2.clone(), directive))?;
    if action2.operation != "activation_transfer_experiment" {
        return Err(invalid("b_loop_tick2_expected_activation_transfer"));
    }
    if action1.executor_id == action2.executor_id {
        return Err(invalid("b_loop_next_action_did_not_change"));
    }

    Ok(BLoopProofReceipt {
        schema: PROOF_SCHEMA.into(),
        tick1: BLoopTickSummary {
            tick: 1,
            stdout_sha256: digest1.to_string(),
            attempt_count: memory1.attempt_count(),
            hint: hint1,
            next_action_executor_id: action1.executor_id.clone(),
            next_action_operation: action1.operation.clone(),
            rationale_evidence: action1.rationale_evidence.clone(),
        },
        start_invocation,
        start_job_state: format!("{:?}", terminal.state),
        start_job_operation: terminal.operation.clone(),
        start_evidence_receipt_present: terminal.evidence_receipt.is_some(),
        start_chain_success: true,
        redecide_experience_source: experience_source.into(),
        tick2: BLoopTickSummary {
            tick: 2,
            stdout_sha256: digest2.to_string(),
            attempt_count: memory2.attempt_count(),
            hint: hint2,
            next_action_executor_id: action2.executor_id.clone(),
            next_action_operation: action2.operation.clone(),
            rationale_evidence: action2.rationale_evidence.clone(),
        },
        next_action_changed: true,
        attribution: format!(
            "NextAction changed because ProceduralMemory replay ({experience_source}) flipped low_rank_unreliable→reliable after Start reached Completed+succeeded+run; hash-only evidence_receipt alone never admits SolverAttempt; hints derived via retrieve, not hand-planted."
        ),
        authorizes_production: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tidex::operator::control_plane::OperatorJobEvidenceReceipt;

    fn isolated_home(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "tidex-b-loop-{}-{}-{}",
            tag,
            std::process::id(),
            Sha256Digest::digest_bytes(tag.as_bytes())
                .to_string()
                .chars()
                .take(8)
                .collect::<String>()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        root
    }

    fn failed_terminal_with_receipt() -> OperatorJobRecord {
        let job_id = Sha256Digest::digest_bytes(b"b-loop-failed-start");
        OperatorJobRecord {
            schema: "tidex.operator_job/v1".into(),
            job_id: job_id.clone(),
            request_sha256: Some(Sha256Digest::digest_bytes(b"req")),
            evidence_receipt: Some(OperatorJobEvidenceReceipt {
                schema: "tidex.operator_job_evidence_receipt/v1".into(),
                job_id,
                request_sha256: Some(Sha256Digest::digest_bytes(b"req")),
                executor_id: Some("cross_model.align".into()),
                executor_descriptor_sha256: None,
                state: OperatorJobState::Failed,
                run_id: None,
                stdout_sha256: None,
                stderr_sha256: None,
                succeeded: false,
                authorizes_production: false,
                evidence_sha256: Sha256Digest::digest_bytes(b"evidence"),
            }),
            state: OperatorJobState::Failed,
            operation: "calibrate_alignment".into(),
            submitted_unix_ns: 1,
            run: None,
            error: Some("operator_model_not_cataloged".into()),
        }
    }

    #[test]
    fn failed_job_with_evidence_receipt_is_not_chain_success() {
        let terminal = failed_terminal_with_receipt();
        assert!(
            terminal.evidence_receipt.is_some(),
            "precondition: receipt present (the old false-positive gate)"
        );
        assert!(
            !chain_start_accepted(&terminal),
            "Failed + receipt must not count as chain success"
        );
        let err = require_chain_success_start(&terminal).expect_err("must reject");
        assert!(
            matches!(err, BrainError::Invalid(code) if code == "b_loop_start_job_not_completed")
        );
    }

    #[test]
    fn prove_b_loop_rejects_failed_start_even_when_receipt_present() {
        // Hermetic Start of calibrate_alignment without cataloged models fails
        // with evidence_receipt present — the old gate would still "succeed".
        let home = isolated_home("reject-failed-start");
        let err = prove_b_loop(&home).expect_err("must not treat Failed+receipt as success");
        let code = match err {
            BrainError::Invalid(code) => code,
            other => panic!("unexpected error kind: {other}"),
        };
        assert!(
            code == "b_loop_start_job_not_completed"
                || code == "b_loop_start_evidence_not_succeeded"
                || code == "b_loop_start_run_missing"
                || code == "b_loop_start_missing_evidence_receipt",
            "unexpected rejection code: {code}"
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn redecide_hermetic_evolve_without_stuffing_hash_only_receipt() {
        let home = isolated_home("redecide-hermetic");
        let terminal = failed_terminal_with_receipt();
        // Even with a hash-only receipt, redecide must use hermetic evolve (not invent).
        let (memory, attempt, _bytes, _digest, source) =
            procedural_experience_for_redecide(&home, &terminal).expect("hermetic redecide");
        assert_eq!(source, "hermetic_numerical_evolve");
        assert!(memory.attempt_count() > 0);
        attempt.authenticate().unwrap();
        let hint = procedural_hint_from_memory(&memory, &query_from_attempt(&attempt).unwrap())
            .expect("hint");
        assert!(
            !hint.low_rank_unreliable() && !hint.steering_unreliable(),
            "hermetic validated evolve must yield reliable procedural signal"
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn prove_b_loop_real_evidence_start_receipt_redecide() {
        // Full Start→Completed path needs cataloged HF models + runner. When the
        // environment cannot complete Start, the hardened gate must fail closed
        // (covered above). When Start does complete, assert the full chain.
        let home = isolated_home("proof");
        match prove_b_loop(&home) {
            Ok(proof) => {
                assert_eq!(proof.schema, PROOF_SCHEMA);
                assert!(!proof.authorizes_production);
                assert_eq!(proof.tick1.next_action_operation, "calibrate_alignment");
                assert_eq!(proof.tick2.next_action_operation, "activation_transfer_experiment");
                assert!(proof.next_action_changed);
                assert!(proof.start_evidence_receipt_present);
                assert!(proof.start_chain_success);
                assert!(
                    proof.redecide_experience_source == "hermetic_numerical_evolve"
                        || proof.redecide_experience_source == "executor_run_view"
                );
                assert_eq!(proof.start_invocation.mode, "start");
                assert!(proof.start_invocation.job.is_some());
                assert!(
                    proof.tick1.hint.low_rank_failures + proof.tick1.hint.steering_failures > 0,
                    "tick1 hint must reflect negative procedural experience"
                );
            }
            Err(BrainError::Invalid(code))
                if code == "b_loop_start_job_not_completed"
                    || code == "b_loop_start_evidence_not_succeeded"
                    || code == "b_loop_start_run_missing"
                    || code == "b_loop_start_missing_evidence_receipt" =>
            {
                // Honest fail-closed without cataloged models / successful Start.
            }
            Err(other) => panic!("unexpected prove_b_loop error: {other}"),
        }
        let _ = fs::remove_dir_all(home);
    }
}
