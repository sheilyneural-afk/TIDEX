//! Isolated execution protocol for real shadow backends and evaluators.
//!
//! The runtime receives only one sealed JSON input containing authenticated
//! receiver, candidate, and evaluation payloads. It has no network and cannot
//! mutate the source checkpoint. Successful output is converted into evidence,
//! never into activation authority.

use crate::foundation::digest::Sha256Digest;
use crate::foundation::error::{BrainError, BrainResult};
use crate::materialization::materialization_selector::{BackendEvaluation, ComparativeControl};
use crate::receiver::receiver_profile::MaterializationStrategy;
use crate::runtime::isolated_execution::{
    run_isolated, AuthenticatedBytes, ExecutionTermination, IsolatedExecutionRequest,
    IsolationLimits, IsolationRequirements,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const MAX_BUNDLE_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShadowEvaluationBundle {
    pub schema: String,
    pub receiver_snapshot_sha256: Sha256Digest,
    pub candidate_sha256: Sha256Digest,
    pub strategy: MaterializationStrategy,
    pub receiver_payload: Vec<u8>,
    pub candidate_payload: Vec<u8>,
    pub evaluation_payload: Vec<u8>,
    pub receiver_payload_sha256: Sha256Digest,
    pub candidate_payload_sha256: Sha256Digest,
    pub evaluation_payload_sha256: Sha256Digest,
    pub optimizer_steps: u64,
}

impl ShadowEvaluationBundle {
    pub fn create(
        receiver_snapshot_sha256: Sha256Digest,
        candidate_sha256: Sha256Digest,
        strategy: MaterializationStrategy,
        receiver_payload: Vec<u8>,
        candidate_payload: Vec<u8>,
        evaluation_payload: Vec<u8>,
    ) -> BrainResult<Self> {
        let bundle = Self {
            schema: "cerebro.tidex.shadow_evaluation_bundle/v1".into(),
            receiver_snapshot_sha256,
            candidate_sha256,
            strategy,
            receiver_payload_sha256: Sha256Digest::digest_bytes(&receiver_payload),
            candidate_payload_sha256: Sha256Digest::digest_bytes(&candidate_payload),
            evaluation_payload_sha256: Sha256Digest::digest_bytes(&evaluation_payload),
            receiver_payload,
            candidate_payload,
            evaluation_payload,
            optimizer_steps: 0,
        };
        bundle.validate()?;
        Ok(bundle)
    }
    pub fn validate(&self) -> BrainResult<()> {
        let size = self
            .receiver_payload
            .len()
            .checked_add(self.candidate_payload.len())
            .and_then(|n| n.checked_add(self.evaluation_payload.len()))
            .ok_or_else(|| BrainError::Invalid("shadow_bundle_size_overflow".into()))?;
        if self.schema != "cerebro.tidex.shadow_evaluation_bundle/v1"
            || self.receiver_snapshot_sha256 == Sha256Digest::zero()
            || self.candidate_sha256 == Sha256Digest::zero()
            || size > MAX_BUNDLE_BYTES
            || self.receiver_payload.is_empty()
            || self.candidate_payload.is_empty()
            || self.evaluation_payload.is_empty()
            || self.optimizer_steps != 0
            || self.receiver_payload_sha256 != Sha256Digest::digest_bytes(&self.receiver_payload)
            || self.candidate_payload_sha256 != Sha256Digest::digest_bytes(&self.candidate_payload)
            || self.evaluation_payload_sha256
                != Sha256Digest::digest_bytes(&self.evaluation_payload)
        {
            return Err(BrainError::Integrity("shadow_evaluation_bundle_invalid".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShadowRuntimeMetrics {
    pub schema: String,
    pub functional_score: f64,
    pub functional_ci_lower: f64,
    pub preservation_score: f64,
    pub identity_margin: f64,
    pub numerical_stability: f64,
    pub normalized_risk: f64,
    pub latency_micros: u64,
    pub resident_bytes: u64,
    pub completed_controls: BTreeSet<ComparativeControl>,
}

impl ShadowRuntimeMetrics {
    fn validate(&self) -> BrainResult<()> {
        let unit = [
            self.functional_score,
            self.functional_ci_lower,
            self.preservation_score,
            self.identity_margin,
            self.numerical_stability,
            self.normalized_risk,
        ];
        if self.schema != "cerebro.tidex.shadow_runtime_metrics/v1"
            || unit
                .iter()
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            || self.functional_ci_lower > self.functional_score
            || self.latency_micros == 0
            || self.resident_bytes == 0
            || self.completed_controls.is_empty()
            || self.completed_controls.len() > 64
        {
            return Err(BrainError::Invalid("shadow_runtime_metrics_invalid".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShadowRuntimeOutput {
    pub schema: String,
    pub receiver_snapshot_sha256: Sha256Digest,
    pub candidate_sha256: Sha256Digest,
    pub receiver_payload_sha256: Sha256Digest,
    pub candidate_payload_sha256: Sha256Digest,
    pub evaluation_payload_sha256: Sha256Digest,
    pub functional_score: f64,
    pub functional_ci_lower: f64,
    pub preservation_score: f64,
    pub identity_margin: f64,
    pub numerical_stability: f64,
    pub normalized_risk: f64,
    pub latency_micros: u64,
    pub resident_bytes: u64,
    pub completed_controls: BTreeSet<ComparativeControl>,
    pub optimizer_steps: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShadowEvaluationReceipt {
    pub schema: String,
    pub runner_sha256: Sha256Digest,
    pub bundle_sha256: Sha256Digest,
    pub isolated_request_sha256: Sha256Digest,
    pub evaluation: BackendEvaluation,
    pub manifest_sha256: Sha256Digest,
}

impl ShadowEvaluationReceipt {
    pub fn validate_integrity(&self) -> BrainResult<()> {
        let mut unsigned = self.clone();
        unsigned.manifest_sha256 = Sha256Digest::zero();
        let expected = Sha256Digest::digest_domain(
            b"CEREBRO:TIDEX:SHADOW-EVALUATION-RECEIPT:v1\0",
            &serde_json::to_vec(&unsigned)?,
        );
        if self.schema != "cerebro.tidex.shadow_evaluation_receipt/v1"
            || self.runner_sha256 == Sha256Digest::zero()
            || self.bundle_sha256 == Sha256Digest::zero()
            || self.isolated_request_sha256 == Sha256Digest::zero()
            || self.manifest_sha256 != expected
        {
            return Err(BrainError::Integrity("shadow_evaluation_receipt_invalid".into()));
        }
        crate::materialization::materialization_selector::validate_evaluation(&self.evaluation)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShadowEvaluationInput {
    pub schema: String,
    pub bundle: ShadowEvaluationBundle,
    pub arguments: Vec<String>,
    pub limits: IsolationLimits,
    pub requirements: IsolationRequirements,
}

/// Convert one authenticated bundle into the exact runtime result expected by
/// the shadow evaluator. Metrics come from one strict, typed evaluation payload;
/// receiver and candidate payloads remain opaque evidence and can never add or
/// override metrics or controls.
pub fn evaluate_shadow_bundle_payloads(
    bundle: &ShadowEvaluationBundle,
) -> BrainResult<ShadowRuntimeOutput> {
    bundle.validate()?;
    let metrics: ShadowRuntimeMetrics = serde_json::from_slice(&bundle.evaluation_payload)
        .map_err(|_| BrainError::Invalid("shadow_runtime_metrics_encoding_invalid".into()))?;
    if serde_json::to_vec(&metrics)? != bundle.evaluation_payload {
        return Err(BrainError::Invalid("shadow_runtime_metrics_not_canonical".into()));
    }
    metrics.validate()?;
    Ok(ShadowRuntimeOutput {
        schema: "cerebro.tidex.shadow_runtime_output/v1".into(),
        receiver_snapshot_sha256: bundle.receiver_snapshot_sha256.clone(),
        candidate_sha256: bundle.candidate_sha256.clone(),
        receiver_payload_sha256: bundle.receiver_payload_sha256.clone(),
        candidate_payload_sha256: bundle.candidate_payload_sha256.clone(),
        evaluation_payload_sha256: bundle.evaluation_payload_sha256.clone(),
        functional_score: metrics.functional_score,
        functional_ci_lower: metrics.functional_ci_lower,
        preservation_score: metrics.preservation_score,
        identity_margin: metrics.identity_margin,
        numerical_stability: metrics.numerical_stability,
        normalized_risk: metrics.normalized_risk,
        latency_micros: metrics.latency_micros,
        resident_bytes: metrics.resident_bytes,
        completed_controls: metrics.completed_controls,
        optimizer_steps: bundle.optimizer_steps,
    })
}

pub fn run_shadow_evaluation(
    runner: AuthenticatedBytes,
    bundle: &ShadowEvaluationBundle,
    arguments: Vec<String>,
    limits: IsolationLimits,
    requirements: IsolationRequirements,
) -> BrainResult<ShadowEvaluationReceipt> {
    bundle.validate()?;
    let input = serde_json::to_vec(bundle)?;
    let bundle_sha256 =
        Sha256Digest::digest_domain(b"CEREBRO:TIDEX:SHADOW-EVALUATION-BUNDLE:v1\0", &input);
    let runner_sha256 = runner.digest().clone();
    let report = run_isolated(&IsolatedExecutionRequest {
        program: runner,
        input: AuthenticatedBytes::from_trusted_bytes(input),
        arguments,
        limits,
        requirements,
    })
    .map_err(|error| BrainError::Invalid(format!("shadow_runtime_isolation:{error}")))?;
    if report.termination != ExecutionTermination::ExitedSuccessfully || !report.succeeded() {
        return Err(BrainError::Invalid("shadow_runtime_failed".into()));
    }
    let output: ShadowRuntimeOutput = serde_json::from_slice(&report.stdout)?;
    let unit = [
        output.functional_score,
        output.functional_ci_lower,
        output.preservation_score,
        output.numerical_stability,
        output.normalized_risk,
    ];
    if output.schema != "cerebro.tidex.shadow_runtime_output/v1"
        || output.receiver_snapshot_sha256 != bundle.receiver_snapshot_sha256
        || output.candidate_sha256 != bundle.candidate_sha256
        || output.receiver_payload_sha256 != bundle.receiver_payload_sha256
        || output.candidate_payload_sha256 != bundle.candidate_payload_sha256
        || output.evaluation_payload_sha256 != bundle.evaluation_payload_sha256
        || output.optimizer_steps != 0
        || unit
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        || !output.identity_margin.is_finite()
        || output.identity_margin < 0.0
    {
        return Err(BrainError::Integrity("shadow_runtime_output_invalid".into()));
    }
    let evaluation = BackendEvaluation {
        schema: "cerebro.tidex.backend_evaluation/v1".into(),
        candidate_sha256: output.candidate_sha256,
        strategy: bundle.strategy,
        functional_score: output.functional_score,
        functional_ci_lower: output.functional_ci_lower,
        preservation_score: output.preservation_score,
        identity_margin: output.identity_margin,
        numerical_stability: output.numerical_stability,
        normalized_risk: output.normalized_risk,
        latency_micros: output.latency_micros,
        resident_bytes: output.resident_bytes,
        completed_controls: output.completed_controls,
    };
    crate::materialization::materialization_selector::validate_evaluation(&evaluation)?;
    let mut receipt = ShadowEvaluationReceipt {
        schema: "cerebro.tidex.shadow_evaluation_receipt/v1".into(),
        runner_sha256,
        bundle_sha256,
        isolated_request_sha256: report.request_digest,
        evaluation,
        manifest_sha256: Sha256Digest::zero(),
    };
    let mut unsigned = receipt.clone();
    unsigned.manifest_sha256 = Sha256Digest::zero();
    receipt.manifest_sha256 = Sha256Digest::digest_domain(
        b"CEREBRO:TIDEX:SHADOW-EVALUATION-RECEIPT:v1\0",
        &serde_json::to_vec(&unsigned)?,
    );
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundle_is_exact_and_forbids_optimizer_steps() {
        let mut b = ShadowEvaluationBundle::create(
            Sha256Digest::digest_bytes(b"receiver"),
            Sha256Digest::digest_bytes(b"candidate"),
            MaterializationStrategy::DenseDelta,
            vec![1],
            vec![2],
            vec![3],
        )
        .unwrap();
        b.candidate_payload[0] = 9;
        assert!(b.validate().is_err());
        b.candidate_payload_sha256 = Sha256Digest::digest_bytes(&b.candidate_payload);
        b.optimizer_steps = 1;
        assert!(b.validate().is_err());
    }

    fn strict_metrics() -> ShadowRuntimeMetrics {
        ShadowRuntimeMetrics {
            schema: "cerebro.tidex.shadow_runtime_metrics/v1".into(),
            functional_score: 0.93,
            functional_ci_lower: 0.90,
            preservation_score: 0.99,
            identity_margin: 0.84,
            numerical_stability: 0.998,
            normalized_risk: 0.01,
            latency_micros: 1200,
            resident_bytes: 2048,
            completed_controls: BTreeSet::from([
                ComparativeControl::UnmodifiedReceiver,
                ComparativeControl::DenseDelta,
                ComparativeControl::WrongCapabilityIr,
                ComparativeControl::RandomDelta,
                ComparativeControl::MeanCapability,
                ComparativeControl::NearestCapability,
                ComparativeControl::AlternativeBackend,
                ComparativeControl::NonTargetPreservation,
            ]),
        }
    }

    fn strict_bundle() -> ShadowEvaluationBundle {
        ShadowEvaluationBundle::create(
            Sha256Digest::digest_bytes(b"receiver"),
            Sha256Digest::digest_bytes(b"candidate"),
            MaterializationStrategy::DenseDelta,
            b"receiver-payload".to_vec(),
            b"candidate-payload".to_vec(),
            serde_json::to_vec(&strict_metrics()).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn strict_runtime_metrics_are_the_only_metric_authority() {
        let bundle = strict_bundle();
        let output = evaluate_shadow_bundle_payloads(&bundle).unwrap();
        assert_eq!(output.functional_score, 0.93);
        assert_eq!(output.functional_ci_lower, 0.90);
        assert_eq!(output.latency_micros, 1200);
        assert_eq!(output.resident_bytes, 2048);
        assert_eq!(output.completed_controls, strict_metrics().completed_controls);
    }

    #[test]
    fn candidate_and_receiver_payloads_cannot_invent_controls() {
        let mut bundle = strict_bundle();
        bundle.receiver_payload = br#"{"completed_controls":["activation_steering"]}"#.to_vec();
        bundle.receiver_payload_sha256 = Sha256Digest::digest_bytes(&bundle.receiver_payload);
        bundle.candidate_payload = br#"{"controls":["sparse_delta"]}"#.to_vec();
        bundle.candidate_payload_sha256 = Sha256Digest::digest_bytes(&bundle.candidate_payload);
        let output = evaluate_shadow_bundle_payloads(&bundle).unwrap();
        assert!(!output
            .completed_controls
            .contains(&ComparativeControl::ActivationSteering));
        assert!(!output
            .completed_controls
            .contains(&ComparativeControl::SparseDelta));
    }

    #[test]
    fn aliases_unknown_fields_and_impossible_resources_fail_closed() {
        let mut metrics = serde_json::to_value(strict_metrics()).unwrap();
        metrics
            .as_object_mut()
            .unwrap()
            .insert("latency_us".into(), serde_json::json!(1200));
        let mut bundle = strict_bundle();
        bundle.evaluation_payload = serde_json::to_vec(&metrics).unwrap();
        bundle.evaluation_payload_sha256 = Sha256Digest::digest_bytes(&bundle.evaluation_payload);
        assert!(evaluate_shadow_bundle_payloads(&bundle).is_err());

        let mut metrics = strict_metrics();
        metrics.latency_micros = 0;
        bundle.evaluation_payload = serde_json::to_vec(&metrics).unwrap();
        bundle.evaluation_payload_sha256 = Sha256Digest::digest_bytes(&bundle.evaluation_payload);
        assert!(evaluate_shadow_bundle_payloads(&bundle).is_err());

        metrics.latency_micros = 1;
        metrics.resident_bytes = 0;
        bundle.evaluation_payload = serde_json::to_vec(&metrics).unwrap();
        bundle.evaluation_payload_sha256 = Sha256Digest::digest_bytes(&bundle.evaluation_payload);
        assert!(evaluate_shadow_bundle_payloads(&bundle).is_err());
    }
}
