//! Evidence-driven co-evolution history reducer.
//!
//! A cycle is recorded only after a real discovery/evaluation round. This
//! module does not mutate models, fabricate discoveries, or assign random
//! fitness. Physical changes must arrive as independently hashed receipts.

use crate::cross_model::models::sha256_hex;
use crate::cross_model::plasticity_engine::DiscoveryCycleReport;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AppliedInterventionEvidence {
    pub capability_name: String,
    pub target_model: String,
    pub receipt_sha256: String,
}

impl AppliedInterventionEvidence {
    fn validate(&self) -> Result<(), String> {
        if self.capability_name.trim().is_empty()
            || self.target_model.trim().is_empty()
            || self.receipt_sha256.len() != 64
            || !self
                .receipt_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("coevolution_intervention_evidence_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CoEvolutionStep {
    pub iteration: usize,
    pub benchmark_id: String,
    pub model_fitness: BTreeMap<String, f64>,
    pub discovered_gap_sha256: Vec<String>,
    pub applied_interventions: Vec<AppliedInterventionEvidence>,
    pub evidence_sha256: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BidirectionalLoopConfig {
    pub maximum_iterations: usize,
    pub convergence_threshold: f64,
    pub minimum_common_models: usize,
}

impl Default for BidirectionalLoopConfig {
    fn default() -> Self {
        Self {
            maximum_iterations: 1000,
            convergence_threshold: 0.01,
            minimum_common_models: 2,
        }
    }
}

impl BidirectionalLoopConfig {
    fn validate(&self) -> Result<(), String> {
        if self.maximum_iterations == 0
            || !self.convergence_threshold.is_finite()
            || self.convergence_threshold < 0.0
            || self.minimum_common_models < 2
        {
            return Err("coevolution_config_invalid".into());
        }
        Ok(())
    }
}

pub struct BidirectionalLoop {
    config: BidirectionalLoopConfig,
    evolution_history: Vec<CoEvolutionStep>,
}

impl BidirectionalLoop {
    pub fn new(config: BidirectionalLoopConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            evolution_history: Vec::new(),
        })
    }

    pub fn record_cycle(
        &mut self,
        report: &DiscoveryCycleReport,
        interventions: &[AppliedInterventionEvidence],
    ) -> Result<CoEvolutionStep, String> {
        self.config.validate()?;
        if self.evolution_history.len() >= self.config.maximum_iterations
            || report.schema != "tidex.cross_model.discovery_cycle/v1"
            || report.evaluations.len() < self.config.minimum_common_models
        {
            return Err("coevolution_cycle_invalid".into());
        }
        for intervention in interventions {
            intervention.validate()?;
        }
        let mut models = BTreeSet::new();
        let mut model_fitness = BTreeMap::new();
        for evaluation in &report.evaluations {
            evaluation.validate()?;
            if !models.insert(evaluation.model.clone()) {
                return Err("coevolution_duplicate_model_evaluation".into());
            }
            model_fitness.insert(evaluation.model.clone(), evaluation.weighted_score);
        }
        let discovered_gap_sha256 = report
            .gaps
            .iter()
            .map(|gap| gap.evidence_sha256.clone())
            .collect::<Vec<_>>();
        let iteration = self.evolution_history.len();
        let evidence_sha256 = sha256_hex(
            &serde_json::to_vec(&(
                iteration,
                report.benchmark_id.as_str(),
                &model_fitness,
                &discovered_gap_sha256,
                interventions,
            ))
            .map_err(|error| format!("coevolution_serialize:{error}"))?,
        );
        let step = CoEvolutionStep {
            iteration,
            benchmark_id: report.benchmark_id.clone(),
            model_fitness,
            discovered_gap_sha256,
            applied_interventions: interventions.to_vec(),
            evidence_sha256,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.evolution_history.push(step.clone());
        Ok(step)
    }

    pub fn check_convergence(&self) -> bool {
        if self.evolution_history.len() < 2 {
            return false;
        }
        let Some(current) = self.evolution_history.last() else {
            return false;
        };
        let previous = &self.evolution_history[self.evolution_history.len() - 2];
        if current.benchmark_id != previous.benchmark_id {
            return false;
        }
        let common = current
            .model_fitness
            .keys()
            .filter(|model| previous.model_fitness.contains_key(*model))
            .collect::<Vec<_>>();
        if common.len() < self.config.minimum_common_models {
            return false;
        }
        let mean_change = common
            .iter()
            .map(|model| (current.model_fitness[*model] - previous.model_fitness[*model]).abs())
            .sum::<f64>()
            / common.len() as f64;
        mean_change <= self.config.convergence_threshold
    }

    pub fn get_progress(&self) -> CoEvolutionProgress {
        let completed = self.evolution_history.len();
        let average_fitness = self.evolution_history.last().map(|step| {
            debug_assert!(!step.model_fitness.is_empty());
            step.model_fitness.values().sum::<f64>() / step.model_fitness.len() as f64
        });
        CoEvolutionProgress {
            iterations_completed: completed,
            maximum_iterations: self.config.maximum_iterations,
            progress_fraction: completed as f64 / self.config.maximum_iterations as f64,
            average_fitness,
            converged: self.check_convergence(),
        }
    }

    pub fn get_history(&self) -> &[CoEvolutionStep] {
        &self.evolution_history
    }

    pub fn export_history(&self) -> Vec<CoEvolutionStep> {
        self.evolution_history.clone()
    }

    pub fn import_history(&mut self, history: Vec<CoEvolutionStep>) -> Result<(), String> {
        self.config.validate()?;
        if history.len() > self.config.maximum_iterations {
            return Err("coevolution_import_invalid".into());
        }
        for (index, step) in history.iter().enumerate() {
            if step.iteration != index
                || step.benchmark_id.trim().is_empty()
                || step.model_fitness.len() < self.config.minimum_common_models
                || step.evidence_sha256.len() != 64
                || !step
                    .evidence_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
                || chrono::DateTime::parse_from_rfc3339(&step.timestamp).is_err()
            {
                return Err("coevolution_import_invalid".into());
            }
            for score in step.model_fitness.values() {
                if !score.is_finite() || !(0.0..=1.0).contains(score) {
                    return Err("coevolution_import_invalid".into());
                }
            }
            for gap in &step.discovered_gap_sha256 {
                if gap.len() != 64 || !gap.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err("coevolution_import_invalid".into());
                }
            }
            for intervention in &step.applied_interventions {
                intervention.validate()?;
            }
            let expected = sha256_hex(
                &serde_json::to_vec(&(
                    step.iteration,
                    step.benchmark_id.as_str(),
                    &step.model_fitness,
                    &step.discovered_gap_sha256,
                    &step.applied_interventions,
                ))
                .map_err(|error| format!("coevolution_serialize:{error}"))?,
            );
            if expected != step.evidence_sha256 {
                return Err("coevolution_import_evidence_mismatch".into());
            }
        }
        self.evolution_history = history;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.evolution_history.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CoEvolutionProgress {
    pub iterations_completed: usize,
    pub maximum_iterations: usize,
    pub progress_fraction: f64,
    pub average_fitness: Option<f64>,
    pub converged: bool,
}
impl Default for BidirectionalLoop {
    fn default() -> Self {
        Self::new(BidirectionalLoopConfig::default()).expect("static coevolution config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cross_model::discovery::{ModelEvaluation, ProbeObservation};
    use crate::cross_model::plasticity_engine::DiscoveryCycleReport;

    fn sealed_evaluation(model: &str, score: f64, benchmark: &str) -> ModelEvaluation {
        let mut evaluation = ModelEvaluation {
            schema: "tidex.cross_model.model_evaluation/v1".into(),
            benchmark_id: benchmark.into(),
            benchmark_sha256: sha256_hex(benchmark.as_bytes()),
            model: model.into(),
            runtime_metadata_sha256: sha256_hex(b"runtime-metadata"),
            observations: vec![ProbeObservation {
                probe_id: "p1".into(),
                prompt_sha256: sha256_hex(b"prompt"),
                response_sha256: sha256_hex(b"response"),
                response_text: "ok".into(),
                score,
                weight: 1.0,
                total_duration_ns: None,
                prompt_eval_count: None,
                eval_count: None,
                execution_sha256: sha256_hex(b"execution"),
                active_interventions_sha256: sha256_hex(b"[]"),
                active_intervention_count: 0,
            }],
            weighted_score: score,
            evidence_sha256: String::new(),
        };
        let mut unsigned = evaluation.clone();
        unsigned.evidence_sha256.clear();
        evaluation.evidence_sha256 = sha256_hex(
            &serde_json::to_vec(&unsigned).expect("evaluation serialize"),
        );
        evaluation.validate().expect("sealed evaluation");
        evaluation
    }

    #[test]
    fn bidirectional_loop_history_survives_export_import() {
        let mut loop_a = BidirectionalLoop::new(BidirectionalLoopConfig::default()).unwrap();
        let report = DiscoveryCycleReport {
            schema: "tidex.cross_model.discovery_cycle/v1".into(),
            benchmark_id: "integer_arithmetic_v1".into(),
            evaluations: vec![
                sealed_evaluation("model-a", 0.25, "integer_arithmetic_v1"),
                sealed_evaluation("model-b", 0.75, "integer_arithmetic_v1"),
            ],
            gaps: Vec::new(),
            priorities: Vec::new(),
            proposals: Vec::new(),
        };
        let step = loop_a.record_cycle(&report, &[]).unwrap();
        assert_eq!(step.iteration, 0);
        let exported = loop_a.export_history();
        assert_eq!(exported.len(), 1);

        let mut loop_b = BidirectionalLoop::new(BidirectionalLoopConfig::default()).unwrap();
        loop_b.import_history(exported.clone()).unwrap();
        assert_eq!(loop_b.get_history(), exported.as_slice());
        assert_eq!(loop_b.get_progress().iterations_completed, 1);

        let step2 = loop_b.record_cycle(&report, &[]).unwrap();
        assert_eq!(step2.iteration, 1);
        assert_eq!(loop_b.export_history().len(), 2);
    }

    #[test]
    fn bidirectional_loop_import_rejects_tampered_evidence() {
        let mut loop_a = BidirectionalLoop::new(BidirectionalLoopConfig::default()).unwrap();
        let report = DiscoveryCycleReport {
            schema: "tidex.cross_model.discovery_cycle/v1".into(),
            benchmark_id: "integer_arithmetic_v1".into(),
            evaluations: vec![
                sealed_evaluation("model-a", 0.2, "integer_arithmetic_v1"),
                sealed_evaluation("model-b", 0.8, "integer_arithmetic_v1"),
            ],
            gaps: Vec::new(),
            priorities: Vec::new(),
            proposals: Vec::new(),
        };
        let mut history = vec![loop_a.record_cycle(&report, &[]).unwrap()];
        history[0].evidence_sha256 =
            "0000000000000000000000000000000000000000000000000000000000000000".into();
        let mut loop_b = BidirectionalLoop::new(BidirectionalLoopConfig::default()).unwrap();
        let err = loop_b.import_history(history).unwrap_err();
        assert!(err.contains("coevolution_import_evidence_mismatch"));
    }
}
