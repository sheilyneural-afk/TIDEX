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
            || report.schema != "cerebro.cross_model.discovery_cycle/v1"
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
