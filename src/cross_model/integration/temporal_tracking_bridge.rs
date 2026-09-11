//! Thin adapter to the canonical SBAS temporal reconstruction.

use crate::error::BrainResult;
use crate::temporal_tracking::{sbas_inversion, DifferentialPair, SbasTimeSeries};

#[derive(Debug, Clone, Default)]
pub struct TemporalTrackingBridgeConfig;

#[derive(Debug, Clone, Default)]
pub struct TemporalTrackingBridge;

impl TemporalTrackingBridge {
    pub fn new(_config: TemporalTrackingBridgeConfig) -> Self {
        Self
    }

    pub fn track_capability(
        &self,
        num_epochs: usize,
        pairs: &[DifferentialPair],
        regularisation: f64,
    ) -> BrainResult<SbasTimeSeries> {
        sbas_inversion(num_epochs, pairs, regularisation)
    }
}
