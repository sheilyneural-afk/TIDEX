//! Paired behavioral gap detection with distribution-free confidence bounds.

use super::{evaluate_model, BehavioralBenchmark, ModelEvaluation};
use crate::cross_model::models::{
    sha256_hex, CapabilityEvidenceKind, CapabilityMetadata, LLMModel,
};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GapDetectorConfig {
    pub minimum_probes: usize,
}

impl Default for GapDetectorConfig {
    fn default() -> Self {
        Self { minimum_probes: 4 }
    }
}

impl GapDetectorConfig {
    fn validate(&self) -> Result<(), String> {
        if self.minimum_probes < 2 || self.minimum_probes > 10_000 {
            return Err("gap_detector_config_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityGap {
    pub schema: String,
    pub capability_name: String,
    pub domain: String,
    pub source_model: String,
    pub target_model: String,
    pub source_score: f64,
    pub target_score: f64,
    pub mean_gap: f64,
    pub conservative_gap_lcb: f64,
    pub confidence_level: f64,
    pub source_evidence_sha256: String,
    pub target_evidence_sha256: String,
    pub evidence_sha256: String,
}

impl CapabilityGap {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "cerebro.cross_model.capability_gap/v1"
            || self.capability_name.trim().is_empty()
            || self.domain.trim().is_empty()
            || self.source_model.trim().is_empty()
            || self.target_model.trim().is_empty()
            || self.source_model == self.target_model
            || !self.source_score.is_finite()
            || !self.target_score.is_finite()
            || !self.mean_gap.is_finite()
            || !self.conservative_gap_lcb.is_finite()
            || !self.confidence_level.is_finite()
            || !(0.0..=1.0).contains(&self.source_score)
            || !(0.0..=1.0).contains(&self.target_score)
            || self.mean_gap < 0.0
            || !(0.5..1.0).contains(&self.confidence_level)
        {
            return Err("capability_gap_invalid".into());
        }
        if gap_digest(self)? != self.evidence_sha256 {
            return Err("capability_gap_digest_mismatch".into());
        }
        Ok(())
    }

    pub fn capability_metadata(&self) -> Result<CapabilityMetadata, String> {
        self.validate()?;
        let metadata = CapabilityMetadata {
            name: self.capability_name.clone(),
            source_model: self.source_model.clone(),
            source_layer: None,
            domain: self.domain.clone(),
            confidence: self.confidence_level,
            evidence_kind: CapabilityEvidenceKind::BehavioralVerified,
            evidence_sha256: self.evidence_sha256.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        metadata.validate()?;
        Ok(metadata)
    }
}

pub struct GapDetector {
    config: GapDetectorConfig,
}

impl GapDetector {
    pub fn new(config: GapDetectorConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn detect_gaps(
        &self,
        first: &dyn LLMModel,
        second: &dyn LLMModel,
        benchmark: &BehavioralBenchmark,
    ) -> Result<Vec<CapabilityGap>, Box<dyn Error + Send + Sync>> {
        benchmark.validate()?;
        if benchmark.probes.len() < self.config.minimum_probes {
            return Err("gap_detector_insufficient_probes".into());
        }
        let first_eval = evaluate_model(first, benchmark)?;
        let second_eval = evaluate_model(second, benchmark)?;
        self.detect_from_evaluations(&first_eval, &second_eval, benchmark)
            .map(|gap| gap.into_iter().collect())
            .map_err(Into::into)
    }

    pub fn detect_from_evaluations(
        &self,
        first: &ModelEvaluation,
        second: &ModelEvaluation,
        benchmark: &BehavioralBenchmark,
    ) -> Result<Option<CapabilityGap>, String> {
        self.config.validate()?;
        benchmark.validate()?;
        first.validate()?;
        second.validate()?;
        let benchmark_sha256 = benchmark.digest()?;
        if first.benchmark_sha256 != benchmark_sha256
            || second.benchmark_sha256 != benchmark_sha256
            || first.observations.len() != benchmark.probes.len()
            || second.observations.len() != benchmark.probes.len()
            || first.model == second.model
        {
            return Err("gap_evidence_binding_mismatch".into());
        }

        let (source, target) = if first.weighted_score > second.weighted_score {
            (first, second)
        } else if second.weighted_score > first.weighted_score {
            (second, first)
        } else {
            return Ok(None);
        };

        let mut weighted_difference = 0.0;
        let mut sum_weight = 0.0;
        let mut sum_weight_sq = 0.0;
        for ((source_row, target_row), probe) in source
            .observations
            .iter()
            .zip(&target.observations)
            .zip(&benchmark.probes)
        {
            if source_row.probe_id != probe.probe_id
                || target_row.probe_id != probe.probe_id
                || (source_row.weight - probe.weight).abs() > f64::EPSILON
                || (target_row.weight - probe.weight).abs() > f64::EPSILON
            {
                return Err("gap_probe_binding_mismatch".into());
            }
            weighted_difference += probe.weight * (source_row.score - target_row.score);
            sum_weight += probe.weight;
            sum_weight_sq += probe.weight * probe.weight;
        }
        if sum_weight <= 0.0 || sum_weight_sq <= 0.0 {
            return Err("gap_weight_geometry_invalid".into());
        }
        let mean_gap = weighted_difference / sum_weight;
        if mean_gap <= 0.0 {
            return Ok(None);
        }
        let radius = ((1.0 / benchmark.significance_alpha).ln() * sum_weight_sq
            / (2.0 * sum_weight * sum_weight))
            .sqrt();
        let conservative_gap_lcb = mean_gap - radius;
        if mean_gap < benchmark.minimum_mean_gap || conservative_gap_lcb <= 0.0 {
            return Ok(None);
        }

        let mut gap = CapabilityGap {
            schema: "cerebro.cross_model.capability_gap/v1".into(),
            capability_name: benchmark.benchmark_id.clone(),
            domain: benchmark.domain.clone(),
            source_model: source.model.clone(),
            target_model: target.model.clone(),
            source_score: source.weighted_score,
            target_score: target.weighted_score,
            mean_gap,
            conservative_gap_lcb,
            confidence_level: 1.0 - benchmark.significance_alpha,
            source_evidence_sha256: source.evidence_sha256.clone(),
            target_evidence_sha256: target.evidence_sha256.clone(),
            evidence_sha256: String::new(),
        };
        gap.evidence_sha256 = gap_digest(&gap)?;
        gap.validate()?;
        Ok(Some(gap))
    }
}

fn gap_digest(gap: &CapabilityGap) -> Result<String, String> {
    let mut unsigned = gap.clone();
    unsigned.evidence_sha256.clear();
    serde_json::to_vec(&unsigned)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("capability_gap_serialize:{error}"))
}

impl Default for GapDetector {
    fn default() -> Self {
        Self::new(GapDetectorConfig::default()).expect("static gap config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cross_model::discovery::{BehavioralProbe, ProbeObservation, ProbeVerifier};

    fn benchmark() -> BehavioralBenchmark {
        BehavioralBenchmark {
            schema: "cerebro.cross_model.behavioral_benchmark/v1".into(),
            benchmark_id: "arithmetic".into(),
            domain: "mathematics".into(),
            probes: (0..64)
                .map(|i| BehavioralProbe {
                    probe_id: format!("p{i}"),
                    prompt: format!("{i}+1"),
                    verifier: ProbeVerifier::Numeric {
                        expected: (i + 1) as f64,
                        absolute_tolerance: 0.0,
                    },
                    weight: 1.0,
                })
                .collect(),
            minimum_mean_gap: 0.1,
            significance_alpha: 0.05,
        }
    }

    fn evaluation(name: &str, score: f64, benchmark: &BehavioralBenchmark) -> ModelEvaluation {
        let count = benchmark.probes.len();
        let successes = (score * count as f64).round() as usize;
        let observations = benchmark
            .probes
            .iter()
            .enumerate()
            .map(|(index, probe)| ProbeObservation {
                probe_id: probe.probe_id.clone(),
                prompt_sha256: sha256_hex(probe.prompt.as_bytes()),
                response_sha256: sha256_hex(if index < successes { b"ok" } else { b"bad" }),
                response_text: if index < successes {
                    "ok".into()
                } else {
                    "bad".into()
                },
                score: if index < successes { 1.0 } else { 0.0 },
                weight: 1.0,
                total_duration_ns: None,
                prompt_eval_count: None,
                eval_count: None,
                execution_sha256: sha256_hex(format!("execution:{name}:{index}").as_bytes()),
                active_interventions_sha256: sha256_hex(b"no-active-interventions"),
                active_intervention_count: 0,
            })
            .collect::<Vec<_>>();
        let mut value = ModelEvaluation {
            schema: "cerebro.cross_model.model_evaluation/v1".into(),
            benchmark_id: benchmark.benchmark_id.clone(),
            benchmark_sha256: benchmark.digest().unwrap(),
            model: name.into(),
            runtime_metadata_sha256: sha256_hex(name.as_bytes()),
            observations,
            weighted_score: successes as f64 / count as f64,
            evidence_sha256: String::new(),
        };
        value.evidence_sha256 = super::super::evaluation_digest(&value).unwrap();
        value
    }

    #[test]
    fn conservative_gap_requires_real_margin() {
        let b = benchmark();
        let detector = GapDetector::default();
        let strong = evaluation("strong", 1.0, &b);
        let weak = evaluation("weak", 0.0, &b);
        let gap = detector
            .detect_from_evaluations(&strong, &weak, &b)
            .unwrap()
            .unwrap();
        assert_eq!(gap.source_model, "strong");
        assert!(gap.conservative_gap_lcb > 0.0);
    }
}
