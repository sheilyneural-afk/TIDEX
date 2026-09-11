//! Validated PI controller for bounded control-plane parameters.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PIControllerState {
    pub integral: f64,
    pub last_error: f64,
    pub last_output: f64,
    pub updates: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PIControllerConfig {
    pub proportional_gain: f64,
    pub integral_gain: f64,
    pub output_min: f64,
    pub output_max: f64,
    pub integral_windup_limit: f64,
}

impl Default for PIControllerConfig {
    fn default() -> Self {
        Self {
            proportional_gain: 0.5,
            integral_gain: 0.1,
            output_min: 0.0,
            output_max: 1.0,
            integral_windup_limit: 1.0,
        }
    }
}

impl PIControllerConfig {
    fn validate(&self) -> Result<(), String> {
        if !self.proportional_gain.is_finite()
            || !self.integral_gain.is_finite()
            || !self.output_min.is_finite()
            || !self.output_max.is_finite()
            || self.output_min >= self.output_max
            || !self.integral_windup_limit.is_finite()
            || self.integral_windup_limit <= 0.0
        {
            return Err("pi_controller_config_invalid".into());
        }
        Ok(())
    }
}

pub struct PIController {
    config: PIControllerConfig,
    state: PIControllerState,
}

impl PIController {
    pub fn new(config: PIControllerConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            state: PIControllerState {
                integral: 0.0,
                last_error: 0.0,
                last_output: 0.0,
                updates: 0,
            },
        })
    }

    pub fn update(&mut self, setpoint: f64, measurement: f64, dt: f64) -> Result<f64, String> {
        if !setpoint.is_finite() || !measurement.is_finite() || !dt.is_finite() || dt <= 0.0 {
            return Err("pi_controller_input_invalid".into());
        }
        let error = setpoint - measurement;
        let candidate_integral = (self.state.integral + error * dt).clamp(
            -self.config.integral_windup_limit,
            self.config.integral_windup_limit,
        );
        let raw =
            self.config.proportional_gain * error + self.config.integral_gain * candidate_integral;
        let output = raw.clamp(self.config.output_min, self.config.output_max);
        // Conditional integration anti-windup: do not accumulate further into saturation.
        let saturated_high = raw > self.config.output_max && error > 0.0;
        let saturated_low = raw < self.config.output_min && error < 0.0;
        if !saturated_high && !saturated_low {
            self.state.integral = candidate_integral;
        }
        self.state.last_error = error;
        self.state.last_output = output;
        self.state.updates = self
            .state
            .updates
            .checked_add(1)
            .ok_or("pi_update_overflow")?;
        Ok(output)
    }

    pub fn reset(&mut self) {
        self.state = PIControllerState {
            integral: 0.0,
            last_error: 0.0,
            last_output: 0.0,
            updates: 0,
        };
    }
    pub fn get_state(&self) -> &PIControllerState {
        &self.state
    }

    pub fn set_gains(&mut self, proportional_gain: f64, integral_gain: f64) -> Result<(), String> {
        let mut candidate = self.config.clone();
        candidate.proportional_gain = proportional_gain;
        candidate.integral_gain = integral_gain;
        candidate.validate()?;
        self.config = candidate;
        Ok(())
    }

    pub fn get_gains(&self) -> (f64, f64) {
        (self.config.proportional_gain, self.config.integral_gain)
    }
}
impl Default for PIController {
    fn default() -> Self {
        Self::new(PIControllerConfig::default()).expect("static pi config")
    }
}
