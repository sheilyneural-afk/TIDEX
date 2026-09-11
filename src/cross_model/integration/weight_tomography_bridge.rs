//! Thin adapter to TIDE-X's canonical weight tomography.

use crate::analysis::tomography::{reconstruct_skill_fields, TomographyResult};
use crate::foundation::contracts::BrainConfig;
use crate::foundation::error::BrainResult;
use crate::foundation::linalg::Matrix;

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
