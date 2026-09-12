//! BCM-style adaptive threshold controller over normalized measured signals.
//!
//! This is a numerical controller only. It never claims to modify model weights.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BCMState {
    pub theta_m: f64,
    pub sliding_window: Vec<f64>,
    pub learning_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BCMConfig {
    pub initial_theta: f64,
    pub window_size: usize,
    pub learning_rate: f64,
    pub theta_decay: f64,
}

impl Default for BCMConfig {
    fn default() -> Self {
        Self {
            initial_theta: 0.5,
            window_size: 100,
            learning_rate: 0.01,
            theta_decay: 0.001,
        }
    }
}

impl BCMConfig {
    fn validate(&self) -> Result<(), String> {
        if !self.initial_theta.is_finite()
            || !(0.0..=1.0).contains(&self.initial_theta)
            || self.window_size == 0
            || self.window_size > 1_000_000
            || !self.learning_rate.is_finite()
            || self.learning_rate <= 0.0
            || self.learning_rate > 1.0
            || !self.theta_decay.is_finite()
            || !(0.0..1.0).contains(&self.theta_decay)
        {
            return Err("bcm_config_invalid".into());
        }
        Ok(())
    }
}

pub struct BCMMetaplasticity {
    config: BCMConfig,
    states: HashMap<String, BCMState>,
}

impl BCMMetaplasticity {
    pub fn new(config: BCMConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            states: HashMap::new(),
        })
    }

    pub fn initialize_state(&mut self, capability_name: &str) -> Result<(), String> {
        if capability_name.trim().is_empty() {
            return Err("bcm_capability_name_invalid".into());
        }
        if self.states.contains_key(capability_name) {
            return Err("bcm_state_already_initialized".into());
        }
        self.states.insert(
            capability_name.into(),
            BCMState {
                theta_m: self.config.initial_theta,
                sliding_window: Vec::with_capacity(self.config.window_size),
                learning_rate: self.config.learning_rate,
            },
        );
        Ok(())
    }

    pub fn update_threshold(
        &mut self,
        capability_name: &str,
        activation: f64,
    ) -> Result<f64, String> {
        if !activation.is_finite() || !(0.0..=1.0).contains(&activation) {
            return Err("bcm_activation_invalid".into());
        }
        let state = self
            .states
            .get_mut(capability_name)
            .ok_or("bcm_state_missing")?;
        state.sliding_window.push(activation);
        if state.sliding_window.len() > self.config.window_size {
            state.sliding_window.remove(0);
        }
        let mean_squared = state
            .sliding_window
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            / state.sliding_window.len() as f64;
        state.theta_m =
            (state.theta_m + state.learning_rate * (mean_squared - state.theta_m)).clamp(0.0, 1.0);
        Ok(state.theta_m)
    }

    pub fn calculate_weight_change(
        &self,
        capability_name: &str,
        pre_synaptic: f64,
        post_synaptic: f64,
    ) -> Result<f64, String> {
        if !pre_synaptic.is_finite() || !post_synaptic.is_finite() {
            return Err("bcm_signal_invalid".into());
        }
        let state = self
            .states
            .get(capability_name)
            .ok_or("bcm_state_missing")?;
        Ok(state.learning_rate * pre_synaptic * post_synaptic * (post_synaptic - state.theta_m))
    }

    pub fn get_threshold(&self, capability_name: &str) -> Option<f64> {
        self.states.get(capability_name).map(|state| state.theta_m)
    }
    pub fn get_state(&self, capability_name: &str) -> Option<&BCMState> {
        self.states.get(capability_name)
    }

    pub fn reset_state(&mut self, capability_name: &str) -> Result<(), String> {
        let state = self
            .states
            .get_mut(capability_name)
            .ok_or("bcm_state_missing")?;
        state.theta_m = self.config.initial_theta;
        state.sliding_window.clear();
        Ok(())
    }

    pub fn clear_all(&mut self) {
        self.states.clear();
    }

    pub fn apply_decay(&mut self) {
        for state in self.states.values_mut() {
            state.theta_m = (state.theta_m * (1.0 - self.config.theta_decay)).clamp(0.0, 1.0);
        }
    }

    pub fn config(&self) -> &BCMConfig {
        &self.config
    }

    pub fn set_learning_rate(
        &mut self,
        capability_name: &str,
        learning_rate: f64,
    ) -> Result<(), String> {
        if !learning_rate.is_finite() || learning_rate <= 0.0 || learning_rate > 1.0 {
            return Err("bcm_learning_rate_invalid".into());
        }
        let state = self
            .states
            .get_mut(capability_name)
            .ok_or("bcm_state_missing")?;
        state.learning_rate = learning_rate;
        Ok(())
    }

    pub fn export_states(&self) -> HashMap<String, BCMState> {
        self.states.clone()
    }

    pub fn import_states(&mut self, states: HashMap<String, BCMState>) -> Result<(), String> {
        for (name, state) in &states {
            if name.trim().is_empty()
                || !state.theta_m.is_finite()
                || !(0.0..=1.0).contains(&state.theta_m)
                || !state.learning_rate.is_finite()
                || state.learning_rate <= 0.0
                || state.learning_rate > 1.0
                || state.sliding_window.len() > self.config.window_size
                || state
                    .sliding_window
                    .iter()
                    .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            {
                return Err("bcm_import_invalid".into());
            }
        }
        self.states = states;
        Ok(())
    }
}
impl Default for BCMMetaplasticity {
    fn default() -> Self {
        Self::new(BCMConfig::default()).expect("static bcm config")
    }
}
