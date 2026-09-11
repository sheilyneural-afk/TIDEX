//! Explicit eligibility traces over measured normalized activation evidence.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EligibilityTrace {
    pub trace_value: f64,
    pub last_update: String,
    pub credit_accumulated: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EligibilityTraceConfig {
    pub initial_trace: f64,
    pub decay_factor: f64,
    pub trace_update_rate: f64,
    pub max_trace_value: f64,
}

impl Default for EligibilityTraceConfig {
    fn default() -> Self {
        Self {
            initial_trace: 0.0,
            decay_factor: 0.95,
            trace_update_rate: 0.1,
            max_trace_value: 1.0,
        }
    }
}

impl EligibilityTraceConfig {
    fn validate(&self) -> Result<(), String> {
        if !self.initial_trace.is_finite()
            || self.initial_trace < 0.0
            || !self.decay_factor.is_finite()
            || !(0.0..=1.0).contains(&self.decay_factor)
            || !self.trace_update_rate.is_finite()
            || self.trace_update_rate < 0.0
            || !self.max_trace_value.is_finite()
            || self.max_trace_value <= 0.0
            || self.initial_trace > self.max_trace_value
        {
            return Err("eligibility_trace_config_invalid".into());
        }
        Ok(())
    }
}

pub struct EligibilityTraces {
    config: EligibilityTraceConfig,
    traces: HashMap<String, EligibilityTrace>,
}

impl EligibilityTraces {
    pub fn new(config: EligibilityTraceConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            traces: HashMap::new(),
        })
    }

    pub fn initialize_trace(&mut self, capability_name: &str) -> Result<(), String> {
        if capability_name.trim().is_empty() || self.traces.contains_key(capability_name) {
            return Err("eligibility_trace_initialization_invalid".into());
        }
        self.traces.insert(
            capability_name.into(),
            EligibilityTrace {
                trace_value: self.config.initial_trace,
                last_update: chrono::Utc::now().to_rfc3339(),
                credit_accumulated: 0.0,
            },
        );
        Ok(())
    }

    pub fn update_trace(&mut self, capability_name: &str, activation: f64) -> Result<f64, String> {
        if !activation.is_finite() || !(0.0..=1.0).contains(&activation) {
            return Err("eligibility_activation_invalid".into());
        }
        let trace = self
            .traces
            .get_mut(capability_name)
            .ok_or("eligibility_trace_missing")?;
        trace.trace_value = (trace.trace_value * self.config.decay_factor
            + self.config.trace_update_rate * activation)
            .clamp(0.0, self.config.max_trace_value);
        trace.last_update = chrono::Utc::now().to_rfc3339();
        Ok(trace.trace_value)
    }

    pub fn accumulate_credit(&mut self, capability_name: &str, credit: f64) -> Result<f64, String> {
        if !credit.is_finite() {
            return Err("eligibility_credit_invalid".into());
        }
        let trace = self
            .traces
            .get_mut(capability_name)
            .ok_or("eligibility_trace_missing")?;
        trace.credit_accumulated += credit * trace.trace_value;
        if !trace.credit_accumulated.is_finite() {
            return Err("eligibility_credit_overflow".into());
        }
        Ok(trace.credit_accumulated)
    }

    pub fn get_trace(&self, capability_name: &str) -> Option<f64> {
        self.traces
            .get(capability_name)
            .map(|trace| trace.trace_value)
    }
    pub fn get_accumulated_credit(&self, capability_name: &str) -> Option<f64> {
        self.traces
            .get(capability_name)
            .map(|trace| trace.credit_accumulated)
    }

    pub fn reset_trace(&mut self, capability_name: &str) -> Result<(), String> {
        let trace = self
            .traces
            .get_mut(capability_name)
            .ok_or("eligibility_trace_missing")?;
        trace.trace_value = self.config.initial_trace;
        trace.credit_accumulated = 0.0;
        trace.last_update = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn decay_all(&mut self) {
        for trace in self.traces.values_mut() {
            trace.trace_value *= self.config.decay_factor;
        }
    }
    pub fn clear_all(&mut self) {
        self.traces.clear();
    }
    pub fn get_all_traces(&self) -> HashMap<String, f64> {
        self.traces
            .iter()
            .map(|(name, trace)| (name.clone(), trace.trace_value))
            .collect()
    }
}
impl Default for EligibilityTraces {
    fn default() -> Self {
        Self::new(EligibilityTraceConfig::default()).expect("static eligibility config")
    }
}
