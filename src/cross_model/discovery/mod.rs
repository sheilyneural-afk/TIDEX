//! Verified behavioral capability discovery.
//!
//! A model is never scored from activation magnitude, response length, model
//! name, or another heuristic. Every score is produced by an explicit,
//! deterministic verifier bound to the exact prompt and response bytes.

pub mod domain_analyzer;
pub mod emergent_detector;
pub mod gap_detector;
pub mod prioritizer;
pub mod proposal_generator;

pub use domain_analyzer::{
    Domain, DomainAnalyzer, DomainAnalyzerConfig, DomainComparison, DomainProfile,
};
pub use emergent_detector::{
    EmergenceType, EmergentCapability, EmergentDetector, EmergentDetectorConfig, ScaleObservation,
};
pub use gap_detector::{CapabilityGap, GapDetector, GapDetectorConfig};
pub use prioritizer::{Prioritizer, PrioritizerConfig, Priority, PriorityScore};
pub use proposal_generator::{
    BatchProposal, ProposalGenerator, ProposalGeneratorConfig, ProposalStage, ResourceRequirements,
    TransferProposal,
};

use crate::cross_model::models::{sha256_hex, LLMModel};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProbeVerifier {
    ExactText {
        expected: String,
        #[serde(default = "default_true")]
        trim: bool,
        #[serde(default = "default_true")]
        case_sensitive: bool,
    },
    ContainsAll {
        required: Vec<String>,
        #[serde(default = "default_true")]
        case_sensitive: bool,
    },
    Numeric {
        expected: f64,
        absolute_tolerance: f64,
    },
    JsonPointerEquals {
        pointer: String,
        expected: Value,
    },
}

fn default_true() -> bool {
    true
}

impl ProbeVerifier {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::ExactText { expected, .. } => {
                if expected.is_empty() {
                    return Err("exact_verifier_expected_empty".into());
                }
            }
            Self::ContainsAll { required, .. } => {
                if required.is_empty() || required.iter().any(|value| value.is_empty()) {
                    return Err("contains_all_verifier_invalid".into());
                }
            }
            Self::Numeric {
                expected,
                absolute_tolerance,
            } => {
                if !expected.is_finite()
                    || !absolute_tolerance.is_finite()
                    || *absolute_tolerance < 0.0
                {
                    return Err("numeric_verifier_invalid".into());
                }
            }
            Self::JsonPointerEquals { pointer, .. } => {
                if pointer.is_empty() || !pointer.starts_with('/') {
                    return Err("json_pointer_verifier_invalid".into());
                }
            }
        }
        Ok(())
    }

    pub fn score(&self, response: &str) -> Result<f64, String> {
        self.validate()?;
        let passed = match self {
            Self::ExactText {
                expected,
                trim,
                case_sensitive,
            } => {
                let left = if *trim { response.trim() } else { response };
                let right = if *trim {
                    expected.trim()
                } else {
                    expected.as_str()
                };
                if *case_sensitive {
                    left == right
                } else {
                    left.eq_ignore_ascii_case(right)
                }
            }
            Self::ContainsAll {
                required,
                case_sensitive,
            } => {
                if *case_sensitive {
                    required.iter().all(|needle| response.contains(needle))
                } else {
                    let haystack = response.to_lowercase();
                    required
                        .iter()
                        .all(|needle| haystack.contains(&needle.to_lowercase()))
                }
            }
            Self::Numeric {
                expected,
                absolute_tolerance,
            } => response
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .is_some_and(|value| (value - expected).abs() <= *absolute_tolerance),
            Self::JsonPointerEquals { pointer, expected } => {
                serde_json::from_str::<Value>(response)
                    .ok()
                    .and_then(|value| value.pointer(pointer).cloned())
                    .is_some_and(|value| value == *expected)
            }
        };
        Ok(if passed { 1.0 } else { 0.0 })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BehavioralProbe {
    pub probe_id: String,
    pub prompt: String,
    pub verifier: ProbeVerifier,
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 {
    1.0
}

impl BehavioralProbe {
    pub fn validate(&self) -> Result<(), String> {
        if self.probe_id.trim().is_empty()
            || self.probe_id.len() > 256
            || self.prompt.trim().is_empty()
            || self.prompt.len() > 1_048_576
            || !self.weight.is_finite()
            || self.weight <= 0.0
        {
            return Err("behavioral_probe_invalid".into());
        }
        self.verifier.validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BehavioralBenchmark {
    pub schema: String,
    pub benchmark_id: String,
    pub domain: String,
    pub probes: Vec<BehavioralProbe>,
    pub minimum_mean_gap: f64,
    pub significance_alpha: f64,
}

impl BehavioralBenchmark {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "cerebro.cross_model.behavioral_benchmark/v1"
            || self.benchmark_id.trim().is_empty()
            || self.domain.trim().is_empty()
            || self.probes.len() < 2
            || self.probes.len() > 10_000
            || !self.minimum_mean_gap.is_finite()
            || !(0.0..=1.0).contains(&self.minimum_mean_gap)
            || !self.significance_alpha.is_finite()
            || !(0.0..0.5).contains(&self.significance_alpha)
        {
            return Err("behavioral_benchmark_invalid".into());
        }
        let mut ids = BTreeSet::new();
        for probe in &self.probes {
            probe.validate()?;
            if !ids.insert(&probe.probe_id) {
                return Err("behavioral_probe_duplicate".into());
            }
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_vec(self)
            .map(|bytes| sha256_hex(&bytes))
            .map_err(|error| format!("behavioral_benchmark_serialize:{error}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProbeObservation {
    pub probe_id: String,
    pub prompt_sha256: String,
    pub response_sha256: String,
    pub response_text: String,
    pub score: f64,
    pub weight: f64,
    pub total_duration_ns: Option<u64>,
    pub prompt_eval_count: Option<u64>,
    pub eval_count: Option<u64>,
    pub execution_sha256: String,
    pub active_interventions_sha256: String,
    pub active_intervention_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelEvaluation {
    pub schema: String,
    pub benchmark_id: String,
    pub benchmark_sha256: String,
    pub model: String,
    pub runtime_metadata_sha256: String,
    pub observations: Vec<ProbeObservation>,
    pub weighted_score: f64,
    pub evidence_sha256: String,
}

impl ModelEvaluation {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "cerebro.cross_model.model_evaluation/v1"
            || self.benchmark_id.trim().is_empty()
            || self.model.trim().is_empty()
            || self.observations.is_empty()
            || self.observations.iter().any(|row| {
                !row.score.is_finite()
                    || !(0.0..=1.0).contains(&row.score)
                    || !row.weight.is_finite()
                    || row.weight <= 0.0
            })
            || !self.weighted_score.is_finite()
            || !(0.0..=1.0).contains(&self.weighted_score)
        {
            return Err("model_evaluation_invalid".into());
        }
        let digest = evaluation_digest(self)?;
        if digest != self.evidence_sha256 {
            return Err("model_evaluation_digest_mismatch".into());
        }
        Ok(())
    }
}

pub fn evaluate_model(
    model: &dyn LLMModel,
    benchmark: &BehavioralBenchmark,
) -> Result<ModelEvaluation, Box<dyn Error + Send + Sync>> {
    benchmark.validate()?;
    model.config().validate()?;
    let benchmark_sha256 = benchmark.digest()?;
    let mut observations = Vec::with_capacity(benchmark.probes.len());
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    for probe in &benchmark.probes {
        let output = model.generate(&probe.prompt)?;
        if output.model.trim().is_empty()
            || output.response_sha256 != sha256_hex(output.text.as_bytes())
        {
            return Err("runtime_generation_evidence_invalid".into());
        }
        let score = probe.verifier.score(&output.text)?;
        weighted_sum += score * probe.weight;
        total_weight += probe.weight;
        observations.push(ProbeObservation {
            probe_id: probe.probe_id.clone(),
            prompt_sha256: sha256_hex(probe.prompt.as_bytes()),
            response_sha256: output.response_sha256,
            response_text: output.text,
            score,
            weight: probe.weight,
            total_duration_ns: output.total_duration_ns,
            prompt_eval_count: output.prompt_eval_count,
            eval_count: output.eval_count,
            execution_sha256: output.execution_sha256.clone(),
            active_interventions_sha256: output.active_interventions_sha256.clone(),
            active_intervention_count: output.active_intervention_count,
        });
    }
    if total_weight <= 0.0 {
        return Err("benchmark_total_weight_invalid".into());
    }
    let mut evaluation = ModelEvaluation {
        schema: "cerebro.cross_model.model_evaluation/v1".into(),
        benchmark_id: benchmark.benchmark_id.clone(),
        benchmark_sha256,
        model: model.name().to_string(),
        runtime_metadata_sha256: model.config().runtime_metadata_sha256.clone(),
        observations,
        weighted_score: weighted_sum / total_weight,
        evidence_sha256: String::new(),
    };
    evaluation.evidence_sha256 = evaluation_digest(&evaluation)?;
    evaluation.validate()?;
    Ok(evaluation)
}

fn evaluation_digest(evaluation: &ModelEvaluation) -> Result<String, String> {
    let mut unsigned = evaluation.clone();
    unsigned.evidence_sha256.clear();
    serde_json::to_vec(&unsigned)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("model_evaluation_serialize:{error}"))
}
