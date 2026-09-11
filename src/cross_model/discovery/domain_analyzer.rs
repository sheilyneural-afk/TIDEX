//! Domain profiles derived only from verified benchmark evaluations.

use super::ModelEvaluation;
use crate::cross_model::models::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Mathematics,
    Programming,
    Reasoning,
    CreativeWriting,
    FactualKnowledge,
    LanguageUnderstanding,
    Scientific,
    Other(String),
}

impl Domain {
    pub fn from_label(label: &str) -> Result<Self, String> {
        let normalized = label.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Err("domain_empty".into());
        }
        Ok(match normalized.as_str() {
            "mathematics" => Self::Mathematics,
            "programming" => Self::Programming,
            "reasoning" => Self::Reasoning,
            "creative_writing" => Self::CreativeWriting,
            "factual_knowledge" => Self::FactualKnowledge,
            "language_understanding" => Self::LanguageUnderstanding,
            "scientific" => Self::Scientific,
            _ => Self::Other(normalized),
        })
    }

    pub fn label(&self) -> String {
        match self {
            Self::Mathematics => "mathematics".into(),
            Self::Programming => "programming".into(),
            Self::Reasoning => "reasoning".into(),
            Self::CreativeWriting => "creative_writing".into(),
            Self::FactualKnowledge => "factual_knowledge".into(),
            Self::LanguageUnderstanding => "language_understanding".into(),
            Self::Scientific => "scientific".into(),
            Self::Other(value) => value.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DomainProfile {
    pub model: String,
    pub domain: Domain,
    pub benchmark_count: usize,
    pub probe_count: usize,
    pub weighted_score: f64,
    pub minimum_benchmark_score: f64,
    pub maximum_benchmark_score: f64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DomainComparison {
    pub first_model: String,
    pub second_model: String,
    pub domain: Domain,
    pub first_score: f64,
    pub second_score: f64,
    pub score_difference: f64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DomainAnalyzerConfig {
    pub minimum_benchmarks: usize,
}

impl Default for DomainAnalyzerConfig {
    fn default() -> Self {
        Self {
            minimum_benchmarks: 1,
        }
    }
}

pub struct DomainAnalyzer {
    config: DomainAnalyzerConfig,
    profiles: HashMap<(String, String), DomainProfile>,
}

impl DomainAnalyzer {
    pub fn new(config: DomainAnalyzerConfig) -> Result<Self, String> {
        if config.minimum_benchmarks == 0 || config.minimum_benchmarks > 100_000 {
            return Err("domain_analyzer_config_invalid".into());
        }
        Ok(Self {
            config,
            profiles: HashMap::new(),
        })
    }

    pub fn analyze_domain(
        &mut self,
        model: &str,
        domain: Domain,
        evaluations: &[ModelEvaluation],
    ) -> Result<DomainProfile, String> {
        if model.trim().is_empty() || evaluations.len() < self.config.minimum_benchmarks {
            return Err("domain_profile_input_invalid".into());
        }
        let mut benchmark_ids = BTreeSet::new();
        let mut score_sum = 0.0;
        let mut probe_count = 0usize;
        let mut minimum = f64::INFINITY;
        let mut maximum = f64::NEG_INFINITY;
        for evaluation in evaluations {
            evaluation.validate()?;
            if evaluation.model != model || !benchmark_ids.insert(evaluation.benchmark_id.clone()) {
                return Err("domain_profile_evidence_mismatch".into());
            }
            score_sum += evaluation.weighted_score;
            probe_count = probe_count
                .checked_add(evaluation.observations.len())
                .ok_or("domain_probe_count_overflow")?;
            minimum = minimum.min(evaluation.weighted_score);
            maximum = maximum.max(evaluation.weighted_score);
        }
        let evidence_sha256 = sha256_hex(
            &serde_json::to_vec(evaluations)
                .map_err(|e| format!("domain_profile_serialize:{e}"))?,
        );
        let profile = DomainProfile {
            model: model.into(),
            domain: domain.clone(),
            benchmark_count: evaluations.len(),
            probe_count,
            weighted_score: score_sum / evaluations.len() as f64,
            minimum_benchmark_score: minimum,
            maximum_benchmark_score: maximum,
            evidence_sha256,
        };
        self.profiles
            .insert((model.into(), domain.label()), profile.clone());
        Ok(profile)
    }

    pub fn compare_domains(
        &self,
        first: &DomainProfile,
        second: &DomainProfile,
    ) -> Result<DomainComparison, String> {
        if first.domain != second.domain || first.model == second.model {
            return Err("domain_comparison_mismatch".into());
        }
        let evidence_sha256 = sha256_hex(
            &serde_json::to_vec(&(first, second))
                .map_err(|e| format!("domain_compare_serialize:{e}"))?,
        );
        Ok(DomainComparison {
            first_model: first.model.clone(),
            second_model: second.model.clone(),
            domain: first.domain.clone(),
            first_score: first.weighted_score,
            second_score: second.weighted_score,
            score_difference: first.weighted_score - second.weighted_score,
            evidence_sha256,
        })
    }

    pub fn get_model_profiles(&self, model: &str) -> Vec<&DomainProfile> {
        let mut values = self
            .profiles
            .iter()
            .filter_map(|((stored_model, _), profile)| (stored_model == model).then_some(profile))
            .collect::<Vec<_>>();
        values.sort_by_key(|a| a.domain.label());
        values
    }

    pub fn find_best_domain(&self, model: &str) -> Option<&DomainProfile> {
        self.get_model_profiles(model)
            .into_iter()
            .max_by(|a, b| a.weighted_score.total_cmp(&b.weighted_score))
    }
}
impl Default for DomainAnalyzer {
    fn default() -> Self {
        Self::new(DomainAnalyzerConfig::default()).expect("static domain config")
    }
}
