//! Domain fitness from measured domain profiles.

use crate::cross_model::discovery::{Domain, DomainProfile};
use crate::cross_model::models::sha256_hex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DomainFitnessConfig {
    pub minimum_target_score: f64,
    pub maximum_regression: f64,
}

impl Default for DomainFitnessConfig {
    fn default() -> Self {
        Self {
            minimum_target_score: 0.5,
            maximum_regression: 0.02,
        }
    }
}

impl DomainFitnessConfig {
    fn validate(&self) -> Result<(), String> {
        if !self.minimum_target_score.is_finite()
            || !(0.0..=1.0).contains(&self.minimum_target_score)
            || !self.maximum_regression.is_finite()
            || !(0.0..=1.0).contains(&self.maximum_regression)
        {
            return Err("domain_fitness_config_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DomainFitnessResult {
    pub model: String,
    pub domain: Domain,
    pub baseline_score: f64,
    pub candidate_score: f64,
    pub delta: f64,
    pub passed: bool,
    pub baseline_evidence_sha256: String,
    pub candidate_evidence_sha256: String,
    pub evidence_sha256: String,
}

pub struct DomainFitness {
    config: DomainFitnessConfig,
}

impl DomainFitness {
    pub fn new(config: DomainFitnessConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn evaluate_fitness(
        &self,
        baseline: &DomainProfile,
        candidate: &DomainProfile,
    ) -> Result<DomainFitnessResult, String> {
        self.config.validate()?;
        if baseline.model != candidate.model || baseline.domain != candidate.domain {
            return Err("domain_fitness_profile_mismatch".into());
        }
        let delta = candidate.weighted_score - baseline.weighted_score;
        let passed = candidate.weighted_score >= self.config.minimum_target_score
            && delta >= -self.config.maximum_regression;
        let mut result = DomainFitnessResult {
            model: baseline.model.clone(),
            domain: baseline.domain.clone(),
            baseline_score: baseline.weighted_score,
            candidate_score: candidate.weighted_score,
            delta,
            passed,
            baseline_evidence_sha256: baseline.evidence_sha256.clone(),
            candidate_evidence_sha256: candidate.evidence_sha256.clone(),
            evidence_sha256: String::new(),
        };
        result.evidence_sha256 = sha256_hex(
            &serde_json::to_vec(&(
                &result.model,
                &result.domain,
                result.baseline_score,
                result.candidate_score,
                &result.baseline_evidence_sha256,
                &result.candidate_evidence_sha256,
            ))
            .map_err(|e| format!("domain_fitness_serialize:{e}"))?,
        );
        Ok(result)
    }

    pub fn find_best_domain<'a>(&self, profiles: &'a [DomainProfile]) -> Option<&'a DomainProfile> {
        profiles
            .iter()
            .max_by(|a, b| a.weighted_score.total_cmp(&b.weighted_score))
    }

    pub fn batch_evaluate(
        &self,
        pairs: &[(DomainProfile, DomainProfile)],
    ) -> Result<Vec<DomainFitnessResult>, String> {
        pairs
            .iter()
            .map(|(baseline, candidate)| self.evaluate_fitness(baseline, candidate))
            .collect()
    }
}
impl Default for DomainFitness {
    fn default() -> Self {
        Self::new(DomainFitnessConfig::default()).expect("static fitness config")
    }
}
