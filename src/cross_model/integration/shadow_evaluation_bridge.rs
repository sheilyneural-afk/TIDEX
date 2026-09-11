//! Thin adapter to the canonical isolated shadow evaluator.

use crate::error::BrainResult;
use crate::isolated_execution::{AuthenticatedBytes, IsolationLimits, IsolationRequirements};
use crate::shadow_evaluation::{
    run_shadow_evaluation, ShadowEvaluationBundle, ShadowEvaluationReceipt,
};

#[derive(Debug, Clone, Default)]
pub struct ShadowEvaluationBridgeConfig;

#[derive(Debug, Clone, Default)]
pub struct ShadowEvaluationBridge;

impl ShadowEvaluationBridge {
    pub fn new(_config: ShadowEvaluationBridgeConfig) -> Self {
        Self
    }

    pub fn evaluate_capability(
        &self,
        runner: AuthenticatedBytes,
        bundle: &ShadowEvaluationBundle,
        arguments: Vec<String>,
        limits: IsolationLimits,
        requirements: IsolationRequirements,
    ) -> BrainResult<ShadowEvaluationReceipt> {
        run_shadow_evaluation(runner, bundle, arguments, limits, requirements)
    }
}
