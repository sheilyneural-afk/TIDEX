//! Evidence-bound pairwise rating system.
//!
//! Ratings are updated from an observed outcome in [0,1]. The caller must bind
//! that outcome to external evidence; this module never invents a winner.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ELOState {
    pub rating: f64,
    pub comparisons: usize,
    pub last_evidence_sha256: Option<String>,
    pub last_update: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ELOConfig {
    pub initial_rating: f64,
    pub k_factor: f64,
    pub rating_floor: f64,
    pub rating_ceiling: f64,
    pub logistic_scale: f64,
}

impl Default for ELOConfig {
    fn default() -> Self {
        Self {
            initial_rating: 1500.0,
            k_factor: 32.0,
            rating_floor: 100.0,
            rating_ceiling: 3000.0,
            logistic_scale: 400.0,
        }
    }
}

impl ELOConfig {
    fn validate(&self) -> Result<(), String> {
        if !self.initial_rating.is_finite()
            || !self.k_factor.is_finite()
            || self.k_factor <= 0.0
            || !self.rating_floor.is_finite()
            || !self.rating_ceiling.is_finite()
            || self.rating_floor >= self.rating_ceiling
            || !(self.rating_floor..=self.rating_ceiling).contains(&self.initial_rating)
            || !self.logistic_scale.is_finite()
            || self.logistic_scale <= 0.0
        {
            return Err("elo_config_invalid".into());
        }
        Ok(())
    }
}

pub struct ELOSystem {
    config: ELOConfig,
    ratings: HashMap<String, ELOState>,
}

impl ELOSystem {
    pub fn new(config: ELOConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            ratings: HashMap::new(),
        })
    }

    pub fn initialize_rating(&mut self, entity_name: &str) -> Result<(), String> {
        if entity_name.trim().is_empty() || self.ratings.contains_key(entity_name) {
            return Err("elo_initialization_invalid".into());
        }
        self.ratings.insert(
            entity_name.into(),
            ELOState {
                rating: self.config.initial_rating,
                comparisons: 0,
                last_evidence_sha256: None,
                last_update: chrono::Utc::now().to_rfc3339(),
            },
        );
        Ok(())
    }

    pub fn update_observed(
        &mut self,
        first: &str,
        second: &str,
        first_observed_score: f64,
        evidence_sha256: &str,
    ) -> Result<(f64, f64), String> {
        if first == second
            || !first_observed_score.is_finite()
            || !(0.0..=1.0).contains(&first_observed_score)
            || evidence_sha256.len() != 64
            || !evidence_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("elo_observation_invalid".into());
        }
        let first_rating = self
            .ratings
            .get(first)
            .ok_or("elo_first_entity_missing")?
            .rating;
        let second_rating = self
            .ratings
            .get(second)
            .ok_or("elo_second_entity_missing")?
            .rating;
        let expected_first = 1.0
            / (1.0 + 10.0_f64.powf((second_rating - first_rating) / self.config.logistic_scale));
        let expected_second = 1.0 - expected_first;
        let observed_second = 1.0 - first_observed_score;
        let first_new = (first_rating
            + self.config.k_factor * (first_observed_score - expected_first))
            .clamp(self.config.rating_floor, self.config.rating_ceiling);
        let second_new = (second_rating
            + self.config.k_factor * (observed_second - expected_second))
            .clamp(self.config.rating_floor, self.config.rating_ceiling);
        let now = chrono::Utc::now().to_rfc3339();
        {
            let state = self
                .ratings
                .get_mut(first)
                .ok_or("elo_first_entity_missing")?;
            state.rating = first_new;
            state.comparisons = state
                .comparisons
                .checked_add(1)
                .ok_or("elo_comparison_overflow")?;
            state.last_evidence_sha256 = Some(evidence_sha256.into());
            state.last_update = now.clone();
        }
        {
            let state = self
                .ratings
                .get_mut(second)
                .ok_or("elo_second_entity_missing")?;
            state.rating = second_new;
            state.comparisons = state
                .comparisons
                .checked_add(1)
                .ok_or("elo_comparison_overflow")?;
            state.last_evidence_sha256 = Some(evidence_sha256.into());
            state.last_update = now;
        }
        Ok((first_new, second_new))
    }

    pub fn get_rating(&self, entity_name: &str) -> Option<f64> {
        self.ratings.get(entity_name).map(|state| state.rating)
    }
    pub fn get_state(&self, entity_name: &str) -> Option<&ELOState> {
        self.ratings.get(entity_name)
    }
    pub fn get_all_ratings(&self) -> HashMap<String, f64> {
        self.ratings
            .iter()
            .map(|(name, state)| (name.clone(), state.rating))
            .collect()
    }

    pub fn get_leaderboard(&self) -> Vec<(String, f64)> {
        let mut ratings = self.get_all_ratings().into_iter().collect::<Vec<_>>();
        ratings.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        ratings
    }

    pub fn reset_rating(&mut self, entity_name: &str) -> Result<(), String> {
        let state = self
            .ratings
            .get_mut(entity_name)
            .ok_or("elo_entity_missing")?;
        state.rating = self.config.initial_rating;
        state.comparisons = 0;
        state.last_evidence_sha256 = None;
        state.last_update = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn clear_all(&mut self) {
        self.ratings.clear();
    }

    pub fn config(&self) -> &ELOConfig {
        &self.config
    }

    pub fn export_ratings(&self) -> HashMap<String, ELOState> {
        self.ratings.clone()
    }

    pub fn import_ratings(&mut self, ratings: HashMap<String, ELOState>) -> Result<(), String> {
        for (name, state) in &ratings {
            if name.trim().is_empty()
                || !state.rating.is_finite()
                || !(self.config.rating_floor..=self.config.rating_ceiling).contains(&state.rating)
                || chrono::DateTime::parse_from_rfc3339(&state.last_update).is_err()
            {
                return Err("elo_import_invalid".into());
            }
            if let Some(evidence) = state.last_evidence_sha256.as_ref() {
                if evidence.len() != 64 || !evidence.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err("elo_import_invalid".into());
                }
            }
        }
        self.ratings = ratings;
        Ok(())
    }
}
impl Default for ELOSystem {
    fn default() -> Self {
        Self::new(ELOConfig::default()).expect("static elo config")
    }
}
