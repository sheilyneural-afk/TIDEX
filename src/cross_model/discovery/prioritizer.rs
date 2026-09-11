//! Evidence-only prioritization of verified capability gaps.

use super::CapabilityGap;
use crate::cross_model::models::sha256_hex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PriorityScore {
    pub capability_name: String,
    pub overall_score: f64,
    pub measured_effect: f64,
    pub conservative_effect: f64,
    pub receiver_deficit: f64,
    pub confidence: f64,
    pub priority: Priority,
    pub gap_evidence_sha256: String,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PrioritizerConfig {
    pub effect_weight: f64,
    pub conservative_weight: f64,
    pub deficit_weight: f64,
    pub confidence_weight: f64,
    pub high_threshold: f64,
    pub critical_threshold: f64,
    pub minimum_score: f64,
}

impl Default for PrioritizerConfig {
    fn default() -> Self {
        Self {
            effect_weight: 0.30,
            conservative_weight: 0.35,
            deficit_weight: 0.20,
            confidence_weight: 0.15,
            high_threshold: 0.55,
            critical_threshold: 0.75,
            minimum_score: 0.20,
        }
    }
}

impl PrioritizerConfig {
    fn validate(&self) -> Result<(), String> {
        let weights = [
            self.effect_weight,
            self.conservative_weight,
            self.deficit_weight,
            self.confidence_weight,
        ];
        if weights
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
            || (weights.iter().sum::<f64>() - 1.0).abs() > 1e-12
            || !self.high_threshold.is_finite()
            || !self.critical_threshold.is_finite()
            || !self.minimum_score.is_finite()
            || !(0.0..=1.0).contains(&self.minimum_score)
            || !(self.minimum_score..=1.0).contains(&self.high_threshold)
            || !(self.high_threshold..=1.0).contains(&self.critical_threshold)
        {
            return Err("prioritizer_config_invalid".into());
        }
        Ok(())
    }
}

pub struct Prioritizer {
    config: PrioritizerConfig,
}

impl Prioritizer {
    pub fn new(config: PrioritizerConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn prioritize(&self, gaps: &[CapabilityGap]) -> Result<Vec<PriorityScore>, String> {
        self.config.validate()?;
        let mut scores = gaps
            .iter()
            .map(|gap| self.score_gap(gap))
            .collect::<Result<Vec<_>, _>>()?;
        scores.retain(|score| score.overall_score >= self.config.minimum_score);
        scores.sort_by(|a, b| {
            b.overall_score
                .total_cmp(&a.overall_score)
                .then_with(|| a.capability_name.cmp(&b.capability_name))
        });
        Ok(scores)
    }

    fn score_gap(&self, gap: &CapabilityGap) -> Result<PriorityScore, String> {
        gap.validate()?;
        let measured_effect = gap.mean_gap.clamp(0.0, 1.0);
        let conservative_effect = gap.conservative_gap_lcb.clamp(0.0, 1.0);
        let receiver_deficit = (1.0 - gap.target_score).clamp(0.0, 1.0);
        let confidence = gap.confidence_level.clamp(0.0, 1.0);
        let overall_score = self.config.effect_weight * measured_effect
            + self.config.conservative_weight * conservative_effect
            + self.config.deficit_weight * receiver_deficit
            + self.config.confidence_weight * confidence;
        let priority = if overall_score >= self.config.critical_threshold {
            Priority::Critical
        } else if overall_score >= self.config.high_threshold {
            Priority::High
        } else if overall_score >= (self.config.minimum_score + self.config.high_threshold) / 2.0 {
            Priority::Medium
        } else {
            Priority::Low
        };
        let mut result = PriorityScore {
            capability_name: gap.capability_name.clone(),
            overall_score,
            measured_effect,
            conservative_effect,
            receiver_deficit,
            confidence,
            priority,
            gap_evidence_sha256: gap.evidence_sha256.clone(),
            evidence_sha256: String::new(),
        };
        result.evidence_sha256 = priority_digest(&result)?;
        Ok(result)
    }

    pub fn get_top_n<'a>(&self, scores: &'a [PriorityScore], n: usize) -> Vec<&'a PriorityScore> {
        let mut refs = scores.iter().collect::<Vec<_>>();
        refs.sort_by(|a, b| {
            b.overall_score
                .total_cmp(&a.overall_score)
                .then_with(|| a.capability_name.cmp(&b.capability_name))
        });
        refs.truncate(n);
        refs
    }
}

fn priority_digest(score: &PriorityScore) -> Result<String, String> {
    let mut unsigned = score.clone();
    unsigned.evidence_sha256.clear();
    serde_json::to_vec(&unsigned)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("priority_score_serialize:{error}"))
}
impl Default for Prioritizer {
    fn default() -> Self {
        Self::new(PrioritizerConfig::default()).expect("static priority config")
    }
}
