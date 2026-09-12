//! Canonical replay of [`ProceduralMemory`] from authenticated receipts.
//!
//! `ProceduralMemory` remains a bounded, non-persistent reducer. This module does
//! **not** introduce `operator/procedural_memory.json` or any second store. It
//! only derives memory from sealed `SolverAttempt` / `SolverRunFailureRecord`
//! values that already exist on an authenticated receipt.
//!
//! # Schema dispatch (fail-closed)
//!
//! [`rebuild_from_authenticated_receipt`] peeks `schema` and maps only receipts
//! that can honestly produce procedural attempts:
//!
//! | Schema | ProceduralMemory |
//! |--------|------------------|
//! | `tidex.numerical_evolution_receipt/v1` | yes — sealed `procedural_attempt` / `solver_run_failure` |
//! | `tidex.operator_run_view/v1` | yes — Operator run receipt + embedded stdout, bound by `stdout_sha256` |
//! | `tidex.operator_job/v1` | yes — when `run` embeds that same view |
//! | `tidex.operator_run_receipt/v1` | no — stdout is a path, not attempt bytes (CLI wraps as run view) |
//! | `tidex.operator_job_evidence_receipt/v1` | no — hashes only, no attempt structure |
//! | `tidex.v67_weight_actuator_smoke/v1` | no — `LearningExperimentEvidence` only |
//! | `tidex.v68_receiver_response_probe/v1` | no — `LearningExperimentEvidence` only |
//!
//! V67/V68 cannot map to [`SolverAttempt`] without inventing `AttemptBindings`,
//! `Applicability`, `SolverConfiguration`, lineage, and sealed evaluation. That
//! path stays in `experimental_evidence_admission.rs`.
//!
//! Composition with Operator run directories lives in the workflow root
//! (`src/bin/tidex.rs`), which may see both domains. This learning-side module
//! never imports `crate::operator` (build.rs frontier).
//!
//! # Paso 3 hook
//! [`retrieve_procedural_advice`] is advisory-only. The workflow coordinator in
//! `src/bin/workflow_next_action.rs` folds its [`RetrievalReport`] (via
//! `procedural_hint_from_retrieval`) into a single `NextAction` together with
//! KnowledgeEngine signals, `CoEvolutionDirective`, and RoutingPlasticity —
//! without promoting or selecting an executor *here*.

use crate::foundation::digest::Sha256Digest;
use crate::foundation::error::{BrainError, BrainResult};
use crate::learning::procedural_memory::{
    ProceduralMemory, RetrievalQuery, RetrievalReport, SolverAttempt, SolverRunFailureRecord,
};
use serde::Deserialize;
use serde_json::Value;

/// Bound matching Operator run stdout limits (`MAX_RESULT_BYTES`).
pub const MAX_PROCEDURAL_REPLAY_STDOUT_BYTES: u64 = 64 * 1024 * 1024;

/// Numerical.evolve stdout / receipt that already embeds sealed attempts.
pub const NUMERICAL_EVOLUTION_RECEIPT_SCHEMA: &str = "tidex.numerical_evolution_receipt/v1";
/// Operator run receipt plus the stdout bytes it authenticates.
pub const OPERATOR_RUN_VIEW_SCHEMA: &str = "tidex.operator_run_view/v1";
/// Operator job record; replay only when `run` embeds a run view.
pub const OPERATOR_JOB_SCHEMA: &str = "tidex.operator_job/v1";
const OPERATOR_RUN_RECEIPT_SCHEMA: &str = "tidex.operator_run_receipt/v1";
const OPERATOR_JOB_EVIDENCE_SCHEMA: &str = "tidex.operator_job_evidence_receipt/v1";
const V67_SCHEMA: &str = "tidex.v67_weight_actuator_smoke/v1";
const V68_SCHEMA: &str = "tidex.v68_receiver_response_probe/v1";

fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}

fn integrity(code: &str) -> BrainError {
    BrainError::Integrity(code.into())
}

#[derive(Debug, Deserialize)]
struct NumericalEvolutionReceiptDocument {
    schema: String,
    #[serde(default)]
    cycles: Vec<NumericalEvolutionCycleEvidence>,
    #[serde(default)]
    procedural_attempt_count: Option<usize>,
    #[serde(default)]
    authorizes_production: bool,
}

#[derive(Debug, Deserialize)]
struct NumericalEvolutionCycleEvidence {
    #[serde(default)]
    procedural_attempt: Option<SolverAttempt>,
    #[serde(default)]
    solver_run_failure: Option<SolverRunFailureRecord>,
}

/// Local wire form of an Operator run receipt. Only authentication fields are
/// required; extra Operator fields are ignored so this crate never imports
/// `crate::operator`.
#[derive(Debug, Deserialize)]
struct OperatorRunReceiptWire {
    schema: String,
    stdout_sha256: Sha256Digest,
    #[serde(default)]
    authorizes_production: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OperatorRunViewDocument {
    schema: String,
    receipt: OperatorRunReceiptWire,
    stdout: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddedOperatorRunView {
    receipt: OperatorRunReceiptWire,
    stdout: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OperatorJobDocument {
    schema: String,
    #[serde(default)]
    run: Option<EmbeddedOperatorRunView>,
}

/// Rebuild from authenticated receipt bytes, dispatching by `schema`.
///
/// When `expected_sha256` is `Some`, it must match the digest of `bytes`
/// (fail-closed). Inner Operator stdout is additionally bound by the receipt's
/// `stdout_sha256`. Unknown schemas, path-only Operator run receipts, job
/// evidence hashes, and V67/V68 fail closed — they are not turned into invented
/// [`SolverAttempt`] values.
pub fn rebuild_from_authenticated_receipt(
    bytes: &[u8],
    expected_sha256: Option<&Sha256Digest>,
) -> BrainResult<ProceduralMemory> {
    if bytes.len() as u64 > MAX_PROCEDURAL_REPLAY_STDOUT_BYTES {
        return Err(invalid("procedural_replay_stdout_limit"));
    }
    if let Some(expected) = expected_sha256 {
        if Sha256Digest::digest_bytes(bytes) != *expected {
            return Err(integrity("procedural_replay_receipt_digest_mismatch"));
        }
    }
    dispatch_authenticated_receipt(bytes)
}

/// Verify stdout bytes against the Operator receipt digest, then rebuild.
///
/// Fail-closed on size, digest mismatch, schema, production claims, or
/// tampered/missing sealed attempt evidence. Dispatches by the stdout `schema`
/// (numerical.evolve today; other supported attempt-bearing schemas later).
pub fn rebuild_from_authenticated_stdout(
    stdout_bytes: &[u8],
    expected_stdout_sha256: &Sha256Digest,
) -> BrainResult<ProceduralMemory> {
    if stdout_bytes.len() as u64 > MAX_PROCEDURAL_REPLAY_STDOUT_BYTES {
        return Err(invalid("procedural_replay_stdout_limit"));
    }
    if Sha256Digest::digest_bytes(stdout_bytes) != *expected_stdout_sha256 {
        return Err(integrity("procedural_replay_stdout_digest_mismatch"));
    }
    dispatch_authenticated_receipt(stdout_bytes)
}

/// Rebuild from numerical.evolve receipt JSON bytes (already authenticated by
/// the caller, or used in tests with known fixtures).
pub fn rebuild_from_numerical_evolution_stdout(
    stdout_bytes: &[u8],
) -> BrainResult<ProceduralMemory> {
    if stdout_bytes.len() as u64 > MAX_PROCEDURAL_REPLAY_STDOUT_BYTES {
        return Err(invalid("procedural_replay_stdout_limit"));
    }
    let document: NumericalEvolutionReceiptDocument = serde_json::from_slice(stdout_bytes)
        .map_err(|_| invalid("procedural_replay_receipt_json_invalid"))?;
    rebuild_from_numerical_evolution_receipt(&document)
}

fn dispatch_authenticated_receipt(bytes: &[u8]) -> BrainResult<ProceduralMemory> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| invalid("procedural_replay_receipt_json_invalid"))?;
    if value.get("authorizes_production").and_then(Value::as_bool) == Some(true) {
        return Err(integrity("procedural_replay_receipt_claims_production"));
    }
    let schema = value
        .get("schema")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("procedural_replay_receipt_schema_missing"))?;
    match schema {
        NUMERICAL_EVOLUTION_RECEIPT_SCHEMA => {
            let document: NumericalEvolutionReceiptDocument = serde_json::from_value(value)
                .map_err(|_| invalid("procedural_replay_receipt_json_invalid"))?;
            rebuild_from_numerical_evolution_receipt(&document)
        }
        OPERATOR_RUN_VIEW_SCHEMA => {
            let document: OperatorRunViewDocument = serde_json::from_value(value)
                .map_err(|_| invalid("procedural_replay_receipt_json_invalid"))?;
            rebuild_from_operator_run_view(&document.receipt, &document.stdout)
        }
        OPERATOR_JOB_SCHEMA => {
            let document: OperatorJobDocument = serde_json::from_value(value)
                .map_err(|_| invalid("procedural_replay_receipt_json_invalid"))?;
            let run = document
                .run
                .as_ref()
                .ok_or_else(|| invalid("procedural_replay_operator_job_run_missing"))?;
            rebuild_from_operator_run_view(&run.receipt, &run.stdout)
        }
        OPERATOR_RUN_RECEIPT_SCHEMA => {
            Err(invalid("procedural_replay_operator_run_stdout_not_embedded"))
        }
        OPERATOR_JOB_EVIDENCE_SCHEMA => {
            Err(invalid("procedural_replay_schema_no_attempt_structure"))
        }
        V67_SCHEMA | V68_SCHEMA => Err(invalid("procedural_replay_schema_learning_evidence_only")),
        _ => Err(invalid("procedural_replay_receipt_schema_unsupported")),
    }
}

fn rebuild_from_operator_run_view(
    receipt: &OperatorRunReceiptWire,
    stdout: &str,
) -> BrainResult<ProceduralMemory> {
    if receipt.schema != OPERATOR_RUN_RECEIPT_SCHEMA {
        return Err(invalid("procedural_replay_operator_run_receipt_schema_invalid"));
    }
    if receipt.authorizes_production {
        return Err(integrity("procedural_replay_operator_run_claims_production"));
    }
    let stdout_bytes = stdout.as_bytes();
    if stdout_bytes.len() as u64 > MAX_PROCEDURAL_REPLAY_STDOUT_BYTES {
        return Err(invalid("procedural_replay_stdout_limit"));
    }
    if Sha256Digest::digest_bytes(stdout_bytes) != receipt.stdout_sha256 {
        return Err(integrity("procedural_replay_stdout_digest_mismatch"));
    }
    dispatch_authenticated_receipt(stdout_bytes)
}

fn rebuild_from_numerical_evolution_receipt(
    document: &NumericalEvolutionReceiptDocument,
) -> BrainResult<ProceduralMemory> {
    if document.schema != NUMERICAL_EVOLUTION_RECEIPT_SCHEMA {
        return Err(invalid("procedural_replay_receipt_schema_invalid"));
    }
    if document.authorizes_production {
        return Err(integrity("procedural_replay_receipt_claims_production"));
    }

    let mut attempts = Vec::new();
    let mut failures = Vec::new();
    for cycle in &document.cycles {
        if let Some(attempt) = cycle.procedural_attempt.as_ref() {
            // Authenticate before rebuild so tamper fails with the attempt code.
            attempt.authenticate()?;
            attempts.push(attempt.clone());
        }
        if let Some(failure) = cycle.solver_run_failure.as_ref() {
            failure.authenticate()?;
            failures.push(failure.clone());
        }
    }

    if let Some(declared) = document.procedural_attempt_count {
        if declared != attempts.len() {
            return Err(integrity("procedural_replay_attempt_count_mismatch"));
        }
    }

    ProceduralMemory::rebuild_with_solver_failures(attempts, failures, Vec::new())
}

/// Workflow-facing advisory retrieve (Paso 2B).
///
/// Returns ranked procedural experience for the next tick. Does **not** emit
/// `NextAction`, choose an executor, or authorize promotion — see
/// `src/bin/workflow_next_action.rs` (Paso 3).
pub fn retrieve_procedural_advice(
    memory: &ProceduralMemory,
    query: &RetrievalQuery,
) -> BrainResult<RetrievalReport> {
    memory.retrieve(query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::digest::EvaluationReceiptDigest;
    use crate::foundation::digest::Sha256Digest;
    use crate::foundation::identity::CapabilityId;
    use crate::learning::procedural_memory::{
        AdviceDisposition, Applicability, AttemptBindings, AttemptLineage, AttemptOutcome,
        AttemptStatus, BaseArtifactDigest, CapabilityContext, CorrectionKind, CorrectionRecord,
        CorrectionStatus, EvaluationDesignDigest, EvaluatorPolicyDigest, ExecutionSemantics,
        FailureKind, FailureRecord, FailureSeverity, FailureStage, MatrixDimensions,
        MatrixStructure, NumericPrecision, NumericProblemProfile, OutcomeMetrics,
        ProblemTransferPolicy, ProceduralLineageId, ResearchReportReceiptDigest,
        ResearchValidation, RetrievalScope, SolverArtifactContext, SolverCandidateDigest,
        SolverConfiguration, SolverFamily, SolverParameters, SolverPolicyDigest,
        StatisticalProfile, TargetProfileDigest, VerifiedAttemptDraft,
    };
    use crate::learning::solver_portfolio::{ExactSolverProblemDigest, LeastSquaresProblem};
    use serde_json::json;

    fn raw_digest(tag: u8) -> Sha256Digest {
        Sha256Digest::digest_bytes(&[tag])
    }

    fn sealed_digest<T>(tag: u8) -> T
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        serde_json::from_str(&format!("\"{}\"", raw_digest(tag))).unwrap()
    }

    fn exact_problem(tag: u8) -> ExactSolverProblemDigest {
        LeastSquaresProblem::new(
            vec![vec![1.0, f64::from(tag)], vec![2.0, 1.0]],
            vec![vec![0.5], vec![1.5]],
        )
        .unwrap()
        .digest()
        .unwrap()
    }

    fn applicability() -> Applicability {
        Applicability::new(
            MatrixDimensions::new(4, 2, Some(2), 0).unwrap(),
            StatisticalProfile::fully_observed(Some(2.0), 0.0, 0.2, 0.05).unwrap(),
            NumericProblemProfile::new(
                NumericPrecision::Float64,
                MatrixStructure::Dense,
                ExecutionSemantics::PureTensor,
            ),
        )
        .unwrap()
    }

    fn bindings(
        system_tag: u8,
        capability: &str,
        problem: ExactSolverProblemDigest,
        candidate_tag: u8,
    ) -> AttemptBindings {
        let capability = CapabilityContext::new(
            sealed_digest(system_tag),
            CapabilityId::parse(capability).unwrap(),
            sealed_digest(system_tag.wrapping_add(1)),
            BaseArtifactDigest::bind_exact_digest(&raw_digest(system_tag.wrapping_add(2))),
            TargetProfileDigest::bind_exact_digest(&raw_digest(system_tag.wrapping_add(3))),
        )
        .unwrap();
        let solver = SolverArtifactContext::new(
            problem,
            SolverPolicyDigest::of_exact_bytes(b"solver-policy-v1"),
            SolverCandidateDigest::of_exact_bytes(&[candidate_tag]),
        )
        .unwrap();
        AttemptBindings::new(capability, solver).unwrap()
    }

    fn config(family: SolverFamily) -> SolverConfiguration {
        let (parameters, regularization, tolerance, max_iterations) = match family {
            SolverFamily::CholeskyRidgeLowRank => {
                (SolverParameters::LowRank { rank: 2 }, Some(1e-6), Some(1e-9), None)
            }
            SolverFamily::PivotedQr | SolverFamily::DivideConquerSvd => {
                (SolverParameters::Direct, None, Some(1e-9), None)
            }
            other => panic!("unexpected family in replay fixture: {other:?}"),
        };
        SolverConfiguration::new(family, parameters, regularization, tolerance, max_iterations)
            .unwrap()
    }

    fn evaluated_attempt(
        bindings: AttemptBindings,
        configuration: SolverConfiguration,
        status: AttemptStatus,
        evidence_tag: u8,
        lineage_tag: u8,
        revision: u64,
    ) -> SolverAttempt {
        let failure =
            matches!(status, AttemptStatus::Rejected | AttemptStatus::Failed).then(|| {
                FailureRecord::new(
                    FailureKind::IllConditioned,
                    FailureStage::ResearchValidation,
                    FailureSeverity::Serious,
                )
            });
        let correction = failure.as_ref().map(|_| {
            CorrectionRecord::new(CorrectionKind::SwitchToSvd, CorrectionStatus::Proposed)
        });
        let outcome = AttemptOutcome::new(
            status,
            OutcomeMetrics::new(
                Some(0.05),
                match status {
                    AttemptStatus::Validated => Some(true),
                    AttemptStatus::Rejected | AttemptStatus::Failed => Some(false),
                    AttemptStatus::Inconclusive => None,
                },
                Some(100),
            )
            .unwrap(),
            failure,
            correction,
        )
        .unwrap();
        let evaluation = ResearchValidation::new(
            ResearchReportReceiptDigest::of_exact_bytes(&[evidence_tag]),
            EvaluationReceiptDigest::from(raw_digest(evidence_tag)),
            EvaluatorPolicyDigest::of_exact_bytes(b"independent-evaluator-v1"),
            EvaluationDesignDigest::of_exact_bytes(&[evidence_tag]),
            bindings.candidate_digest().clone(),
            8,
        )
        .unwrap();
        let draft = VerifiedAttemptDraft::new(
            bindings,
            applicability(),
            configuration,
            AttemptLineage::root(ProceduralLineageId::of_exact_bytes(&[lineage_tag])),
            revision,
            None,
        )
        .unwrap();
        SolverAttempt::seal(draft, Some(evaluation), outcome).unwrap()
    }

    fn query(bindings: &AttemptBindings) -> RetrievalQuery {
        RetrievalQuery::new(
            RetrievalScope::from_bindings(bindings),
            applicability(),
            100,
            ProblemTransferPolicy::ExactOnly,
            16,
        )
        .unwrap()
    }

    fn receipt_bytes(attempts: &[SolverAttempt], declare_count: bool) -> Vec<u8> {
        let cycles: Vec<_> = attempts
            .iter()
            .map(|attempt| {
                json!({
                    "revision": attempt.lineage_ordinal() + 1,
                    "disposition": "fixture",
                    "procedural_attempt": attempt,
                    "solver_run_failure": null,
                    "authorizes_promotion": false
                })
            })
            .collect();
        let mut document = json!({
            "schema": NUMERICAL_EVOLUTION_RECEIPT_SCHEMA,
            "solver_profile": "fixture",
            "cycles": cycles,
            "authorizes_production": false
        });
        if declare_count {
            document["procedural_attempt_count"] = json!(attempts.len());
        }
        serde_json::to_vec(&document).unwrap()
    }

    fn operator_receipt_wire(stdout: &[u8], authorizes_production: bool) -> serde_json::Value {
        json!({
            "schema": "tidex.operator_run_receipt/v1",
            "stdout_sha256": Sha256Digest::digest_bytes(stdout),
            "authorizes_production": authorizes_production
        })
    }

    fn operator_run_view_bytes(stdout: &[u8]) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "schema": OPERATOR_RUN_VIEW_SCHEMA,
            "receipt": operator_receipt_wire(stdout, false),
            "stdout": String::from_utf8(stdout.to_vec()).unwrap()
        }))
        .unwrap()
    }

    fn operator_job_bytes(stdout: &[u8]) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "schema": OPERATOR_JOB_SCHEMA,
            "job_id": "a".repeat(64),
            "state": "completed",
            "operation": "numerical_evolve_fixture",
            "submitted_unix_ns": 1,
            "run": {
                "receipt": operator_receipt_wire(stdout, false),
                "stdout": String::from_utf8(stdout.to_vec()).unwrap(),
                "stderr": ""
            }
        }))
        .unwrap()
    }

    fn ranked_pair() -> (AttemptBindings, Vec<u8>) {
        let problem = exact_problem(17);
        let good_bindings = bindings(60, "capability.multischema:v1", problem.clone(), 61);
        let bad_bindings = bindings(60, "capability.multischema:v1", problem, 62);
        let good = evaluated_attempt(
            good_bindings.clone(),
            config(SolverFamily::DivideConquerSvd),
            AttemptStatus::Validated,
            63,
            64,
            1,
        );
        let bad = evaluated_attempt(
            bad_bindings,
            config(SolverFamily::CholeskyRidgeLowRank),
            AttemptStatus::Failed,
            65,
            66,
            1,
        );
        (good_bindings, receipt_bytes(&[bad, good], true))
    }

    fn assert_ranked_svd_over_ridge(memory: &ProceduralMemory, bindings: &AttemptBindings) {
        let report = retrieve_procedural_advice(memory, &query(bindings)).unwrap();
        assert_eq!(report.advice().len(), 2);
        let good = report
            .advice()
            .iter()
            .find(|value| value.configuration().family() == SolverFamily::DivideConquerSvd)
            .unwrap();
        let bad = report
            .advice()
            .iter()
            .find(|value| value.configuration().family() == SolverFamily::CholeskyRidgeLowRank)
            .unwrap();
        assert!(good.priority_score() > bad.priority_score());
        assert_eq!(bad.disposition(), AdviceDisposition::DeprioritizeButRetainControl);
        assert!(!good.authorizes_promotion());
    }

    #[test]
    fn replay_from_fixture_receipt_rebuilds_ranked_advice() {
        let problem = exact_problem(7);
        let good_bindings = bindings(10, "capability.replay:v1", problem.clone(), 11);
        let bad_bindings = bindings(10, "capability.replay:v1", problem, 12);
        let good = evaluated_attempt(
            good_bindings.clone(),
            config(SolverFamily::DivideConquerSvd),
            AttemptStatus::Validated,
            13,
            14,
            1,
        );
        let bad = evaluated_attempt(
            bad_bindings,
            config(SolverFamily::CholeskyRidgeLowRank),
            AttemptStatus::Failed,
            15,
            16,
            1,
        );

        let stdout = receipt_bytes(&[bad.clone(), good.clone()], true);
        let digest = Sha256Digest::digest_bytes(&stdout);
        let memory = rebuild_from_authenticated_stdout(&stdout, &digest).unwrap();
        assert_eq!(memory.attempt_count(), 2);

        let report = retrieve_procedural_advice(&memory, &query(&good_bindings)).unwrap();
        assert_eq!(report.advice().len(), 2);
        let good_advice = report
            .advice()
            .iter()
            .find(|value| value.configuration().family() == SolverFamily::DivideConquerSvd)
            .unwrap();
        let bad_advice = report
            .advice()
            .iter()
            .find(|value| value.configuration().family() == SolverFamily::CholeskyRidgeLowRank)
            .unwrap();
        assert!(good_advice.priority_score() > bad_advice.priority_score());
        assert_eq!(bad_advice.disposition(), AdviceDisposition::DeprioritizeButRetainControl);
        assert!(!good_advice.authorizes_promotion());
    }

    #[test]
    fn replay_is_invariant_to_cycle_order() {
        let problem = exact_problem(8);
        let first_bindings = bindings(20, "capability.order:v1", problem.clone(), 21);
        let second_bindings = bindings(20, "capability.order:v1", problem, 22);
        let first = evaluated_attempt(
            first_bindings.clone(),
            config(SolverFamily::PivotedQr),
            AttemptStatus::Validated,
            23,
            24,
            1,
        );
        let second = evaluated_attempt(
            second_bindings,
            config(SolverFamily::DivideConquerSvd),
            AttemptStatus::Validated,
            25,
            26,
            1,
        );
        let forward = rebuild_from_numerical_evolution_stdout(&receipt_bytes(
            &[first.clone(), second.clone()],
            true,
        ))
        .unwrap();
        let reverse =
            rebuild_from_numerical_evolution_stdout(&receipt_bytes(&[second, first], true))
                .unwrap();
        assert_eq!(forward.attempt_count(), reverse.attempt_count());
        let forward_advice = retrieve_procedural_advice(&forward, &query(&first_bindings)).unwrap();
        let reverse_advice = retrieve_procedural_advice(&reverse, &query(&first_bindings)).unwrap();
        assert_eq!(forward_advice.advice(), reverse_advice.advice());
    }

    #[test]
    fn tampered_attempt_inside_receipt_fail_closed() {
        let problem = exact_problem(9);
        let attempt_bindings = bindings(30, "capability.tamper:v1", problem, 31);
        let attempt = evaluated_attempt(
            attempt_bindings,
            config(SolverFamily::PivotedQr),
            AttemptStatus::Validated,
            32,
            33,
            1,
        );
        let mut wire = serde_json::to_value(&attempt).unwrap();
        wire["bindings"]["capability"]["capability_id"] =
            serde_json::Value::String("capability.relabelled:v1".into());
        let document = json!({
            "schema": NUMERICAL_EVOLUTION_RECEIPT_SCHEMA,
            "cycles": [{ "procedural_attempt": wire }],
            "procedural_attempt_count": 1,
            "authorizes_production": false
        });
        let bytes = serde_json::to_vec(&document).unwrap();
        assert!(matches!(
            rebuild_from_numerical_evolution_stdout(&bytes),
            Err(BrainError::Integrity(code)) if code == "procedural_attempt_digest_mismatch"
        ));
    }

    #[test]
    fn missing_stdout_digest_evidence_fail_closed() {
        let problem = exact_problem(10);
        let attempt_bindings = bindings(40, "capability.digest:v1", problem, 41);
        let attempt = evaluated_attempt(
            attempt_bindings,
            config(SolverFamily::PivotedQr),
            AttemptStatus::Validated,
            42,
            43,
            1,
        );
        let stdout = receipt_bytes(&[attempt], true);
        let wrong = Sha256Digest::digest_bytes(b"not-the-stdout");
        assert!(matches!(
            rebuild_from_authenticated_stdout(&stdout, &wrong),
            Err(BrainError::Integrity(code)) if code == "procedural_replay_stdout_digest_mismatch"
        ));
    }

    #[test]
    fn production_claim_and_bad_schema_fail_closed() {
        let bad_schema = json!({
            "schema": "tidex.numerical_evolution_receipt/v0",
            "cycles": [],
            "authorizes_production": false
        });
        assert!(matches!(
            rebuild_from_numerical_evolution_stdout(&serde_json::to_vec(&bad_schema).unwrap()),
            Err(BrainError::Invalid(code)) if code == "procedural_replay_receipt_schema_invalid"
        ));
        let production = json!({
            "schema": NUMERICAL_EVOLUTION_RECEIPT_SCHEMA,
            "cycles": [],
            "authorizes_production": true
        });
        assert!(matches!(
            rebuild_from_numerical_evolution_stdout(&serde_json::to_vec(&production).unwrap()),
            Err(BrainError::Integrity(code)) if code == "procedural_replay_receipt_claims_production"
        ));
    }

    #[test]
    fn attempt_count_mismatch_fail_closed() {
        let problem = exact_problem(11);
        let attempt_bindings = bindings(50, "capability.count:v1", problem, 51);
        let attempt = evaluated_attempt(
            attempt_bindings,
            config(SolverFamily::PivotedQr),
            AttemptStatus::Validated,
            52,
            53,
            1,
        );
        let mut document: serde_json::Value =
            serde_json::from_slice(&receipt_bytes(&[attempt], false)).unwrap();
        document["procedural_attempt_count"] = json!(99);
        assert!(matches!(
            rebuild_from_numerical_evolution_stdout(&serde_json::to_vec(&document).unwrap()),
            Err(BrainError::Integrity(code)) if code == "procedural_replay_attempt_count_mismatch"
        ));
    }

    #[test]
    fn unified_entry_replays_numerical_receipt_and_optional_digest() {
        let (bindings, stdout) = ranked_pair();
        let digest = Sha256Digest::digest_bytes(&stdout);
        let with_digest = rebuild_from_authenticated_receipt(&stdout, Some(&digest)).unwrap();
        let without_digest = rebuild_from_authenticated_receipt(&stdout, None).unwrap();
        assert_eq!(with_digest.attempt_count(), 2);
        assert_eq!(without_digest.attempt_count(), 2);
        assert_ranked_svd_over_ridge(&with_digest, &bindings);
    }

    #[test]
    fn unified_entry_wrong_digest_fail_closed() {
        let (_bindings, stdout) = ranked_pair();
        let wrong = Sha256Digest::digest_bytes(b"not-the-receipt");
        assert!(matches!(
            rebuild_from_authenticated_receipt(&stdout, Some(&wrong)),
            Err(BrainError::Integrity(code)) if code == "procedural_replay_receipt_digest_mismatch"
        ));
    }

    #[test]
    fn operator_run_view_rebuilds_ranked_advice() {
        let (bindings, stdout) = ranked_pair();
        let view = operator_run_view_bytes(&stdout);
        let memory = rebuild_from_authenticated_receipt(&view, None).unwrap();
        assert_eq!(memory.attempt_count(), 2);
        assert_ranked_svd_over_ridge(&memory, &bindings);
    }

    #[test]
    fn operator_job_with_embedded_run_rebuilds_ranked_advice() {
        let (bindings, stdout) = ranked_pair();
        let job = operator_job_bytes(&stdout);
        let memory = rebuild_from_authenticated_receipt(&job, None).unwrap();
        assert_eq!(memory.attempt_count(), 2);
        assert_ranked_svd_over_ridge(&memory, &bindings);
    }

    #[test]
    fn operator_run_receipt_without_embedded_stdout_fail_closed() {
        let bytes = serde_json::to_vec(&json!({
            "schema": "tidex.operator_run_receipt/v1",
            "stdout_sha256": Sha256Digest::digest_bytes(b"unused"),
            "authorizes_production": false
        }))
        .unwrap();
        assert!(matches!(
            rebuild_from_authenticated_receipt(&bytes, None),
            Err(BrainError::Invalid(code))
                if code == "procedural_replay_operator_run_stdout_not_embedded"
        ));
    }

    #[test]
    fn operator_run_view_stdout_digest_mismatch_fail_closed() {
        let (_bindings, stdout) = ranked_pair();
        let mut view: serde_json::Value =
            serde_json::from_slice(&operator_run_view_bytes(&stdout)).unwrap();
        view["receipt"]["stdout_sha256"] = json!(Sha256Digest::digest_bytes(b"tampered-stdout"));
        assert!(matches!(
            rebuild_from_authenticated_receipt(&serde_json::to_vec(&view).unwrap(), None),
            Err(BrainError::Integrity(code)) if code == "procedural_replay_stdout_digest_mismatch"
        ));
    }

    #[test]
    fn operator_run_view_production_claim_fail_closed() {
        let (_bindings, stdout) = ranked_pair();
        let view = json!({
            "schema": OPERATOR_RUN_VIEW_SCHEMA,
            "receipt": operator_receipt_wire(&stdout, true),
            "stdout": String::from_utf8(stdout).unwrap()
        });
        assert!(matches!(
            rebuild_from_authenticated_receipt(&serde_json::to_vec(&view).unwrap(), None),
            Err(BrainError::Integrity(code))
                if code == "procedural_replay_operator_run_claims_production"
        ));
    }

    #[test]
    fn operator_job_missing_run_fail_closed() {
        let bytes = serde_json::to_vec(&json!({
            "schema": OPERATOR_JOB_SCHEMA,
            "job_id": "b".repeat(64),
            "state": "failed",
            "operation": "fixture",
            "submitted_unix_ns": 1,
            "run": null
        }))
        .unwrap();
        assert!(matches!(
            rebuild_from_authenticated_receipt(&bytes, None),
            Err(BrainError::Invalid(code)) if code == "procedural_replay_operator_job_run_missing"
        ));
    }

    #[test]
    fn operator_job_evidence_has_no_attempt_structure() {
        let bytes = serde_json::to_vec(&json!({
            "schema": "tidex.operator_job_evidence_receipt/v1",
            "job_id": "c".repeat(64),
            "authorizes_production": false
        }))
        .unwrap();
        assert!(matches!(
            rebuild_from_authenticated_receipt(&bytes, None),
            Err(BrainError::Invalid(code))
                if code == "procedural_replay_schema_no_attempt_structure"
        ));
    }

    #[test]
    fn v67_and_v68_stay_learning_evidence_only() {
        for schema in [
            "tidex.v67_weight_actuator_smoke/v1",
            "tidex.v68_receiver_response_probe/v1",
        ] {
            let bytes = serde_json::to_vec(&json!({
                "schema": schema,
                "authorizes_production": false
            }))
            .unwrap();
            assert!(
                matches!(
                    rebuild_from_authenticated_receipt(&bytes, None),
                    Err(BrainError::Invalid(code))
                        if code == "procedural_replay_schema_learning_evidence_only"
                ),
                "schema {schema} must stay LearningExperimentEvidence-only"
            );
        }
    }

    #[test]
    fn unknown_schema_fail_closed() {
        let bytes = serde_json::to_vec(&json!({
            "schema": "tidex.not_a_real_receipt/v0",
            "cycles": []
        }))
        .unwrap();
        assert!(matches!(
            rebuild_from_authenticated_receipt(&bytes, None),
            Err(BrainError::Invalid(code))
                if code == "procedural_replay_receipt_schema_unsupported"
        ));
    }

    #[test]
    fn missing_schema_fail_closed() {
        let bytes = serde_json::to_vec(&json!({ "cycles": [] })).unwrap();
        assert!(matches!(
            rebuild_from_authenticated_receipt(&bytes, None),
            Err(BrainError::Invalid(code)) if code == "procedural_replay_receipt_schema_missing"
        ));
    }
}
