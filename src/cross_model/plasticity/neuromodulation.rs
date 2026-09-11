//! Explicit multi-signal modulation controller.
//!
//! The biological labels are only names for independent normalized control
//! channels. No channel is inferred or substituted when missing.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Neuromodulator {
    Reward,
    Attention,
    Novelty,
    Stability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NeuromodulationSignal {
    pub modulator: Neuromodulator,
    pub level: f64,
    pub timestamp: String,
    pub source: String,
    pub source_sha256: String,
}

impl NeuromodulationSignal {
    fn validate(&self) -> Result<(), String> {
        if !self.level.is_finite()
            || !(0.0..=1.0).contains(&self.level)
            || self.source.trim().is_empty()
            || self.source_sha256.len() != 64
            || !self.source_sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || chrono::DateTime::parse_from_rfc3339(&self.timestamp).is_err()
        {
            return Err("neuromodulation_signal_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NeuromodulationConfig {
    pub weights: BTreeMap<Neuromodulator, f64>,
    pub decay_factor: f64,
}

impl Default for NeuromodulationConfig {
    fn default() -> Self {
        Self {
            weights: BTreeMap::from([
                (Neuromodulator::Reward, 0.4),
                (Neuromodulator::Attention, 0.3),
                (Neuromodulator::Novelty, 0.2),
                (Neuromodulator::Stability, 0.1),
            ]),
            decay_factor: 0.99,
        }
    }
}

impl NeuromodulationConfig {
    fn validate(&self) -> Result<(), String> {
        let required = [
            Neuromodulator::Reward,
            Neuromodulator::Attention,
            Neuromodulator::Novelty,
            Neuromodulator::Stability,
        ];
        if required.iter().any(|key| !self.weights.contains_key(key))
            || self.weights.len() != required.len()
            || self
                .weights
                .values()
                .any(|value| !value.is_finite() || *value < 0.0)
            || (self.weights.values().sum::<f64>() - 1.0).abs() > 1e-12
            || !self.decay_factor.is_finite()
            || !(0.0..=1.0).contains(&self.decay_factor)
        {
            return Err("neuromodulation_config_invalid".into());
        }
        Ok(())
    }
}

pub struct Neuromodulation {
    config: NeuromodulationConfig,
    current_levels: HashMap<Neuromodulator, f64>,
    signal_history: Vec<NeuromodulationSignal>,
}

impl Neuromodulation {
    pub fn new(config: NeuromodulationConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            current_levels: HashMap::new(),
            signal_history: Vec::new(),
        })
    }

    pub fn emit_signal(&mut self, signal: NeuromodulationSignal) -> Result<(), String> {
        signal.validate()?;
        self.current_levels.insert(signal.modulator, signal.level);
        self.signal_history.push(signal);
        Ok(())
    }

    pub fn get_level(&self, modulator: Neuromodulator) -> Result<f64, String> {
        self.current_levels
            .get(&modulator)
            .copied()
            .ok_or("neuromodulation_signal_missing".into())
    }

    pub fn calculate_plasticity_modulation(&self) -> Result<f64, String> {
        self.config.validate()?;
        let mut modulation = 0.0;
        for (modulator, weight) in &self.config.weights {
            modulation += *weight * self.get_level(*modulator)?;
        }
        if !modulation.is_finite() {
            return Err("neuromodulation_output_invalid".into());
        }
        Ok(modulation.clamp(0.0, 1.0))
    }

    pub fn modulate_learning_rate(&self, base_rate: f64) -> Result<f64, String> {
        if !base_rate.is_finite() || base_rate < 0.0 {
            return Err("base_learning_rate_invalid".into());
        }
        Ok(base_rate * self.calculate_plasticity_modulation()?)
    }

    pub fn apply_decay(&mut self) {
        for level in self.current_levels.values_mut() {
            *level *= self.config.decay_factor;
        }
    }

    pub fn get_signal_history(&self) -> &[NeuromodulationSignal] {
        &self.signal_history
    }
    pub fn clear_history(&mut self) {
        self.signal_history.clear();
    }
    pub fn reset(&mut self) {
        self.current_levels.clear();
        self.signal_history.clear();
    }
}
impl Default for Neuromodulation {
    fn default() -> Self {
        Self::new(NeuromodulationConfig::default()).expect("static modulation config")
    }
}
