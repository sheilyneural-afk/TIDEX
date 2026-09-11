//! Thin adapter to the canonical persistent-topology implementation.

use crate::analysis::pythagoras_topology::{TopologicalManifoldReport, TopologicalSkillManifold};
use crate::foundation::error::BrainResult;

#[derive(Debug, Clone, Default)]
pub struct PythagorasBridgeConfig;

#[derive(Debug, Clone, Default)]
pub struct PythagorasBridge;

impl PythagorasBridge {
    pub fn new(_config: PythagorasBridgeConfig) -> Self {
        Self
    }

    pub fn get_topology(
        &self,
        points: &[Vec<f64>],
        distance_threshold: f64,
    ) -> BrainResult<TopologicalManifoldReport> {
        TopologicalSkillManifold::analyze_topology(points, distance_threshold)
    }
}
