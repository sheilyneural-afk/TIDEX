//! Thin adapter to CEREBRO3's canonical weight tomography.

use crate::contracts::BrainConfig;
use crate::error::BrainResult;
use crate::linalg::Matrix;
use crate::tomography::{reconstruct_skill_fields, TomographyResult};

#[derive(Debug, Clone, Default)]
pub struct WeightTomographyBridgeConfig;

#[derive(Debug, Clone, Default)]
pub struct WeightTomographyBridge;

impl WeightTomographyBridge {
    pub fn new(_config: WeightTomographyBridgeConfig) -> Self {
        Self
    }

    pub fn perform_tomography(
        &self,
        observations: &Matrix,
        base_weights: &[f64],
        groups: &[String],
        generation: u64,
        config: &BrainConfig,
    ) -> BrainResult<TomographyResult> {
        reconstruct_skill_fields(observations, base_weights, groups, generation, config)
    }
}
