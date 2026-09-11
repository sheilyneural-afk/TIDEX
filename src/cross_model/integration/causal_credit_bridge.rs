//! Thin adapter to the canonical causal-credit reducer.

use crate::foundation::error::BrainResult;
use crate::learning::causal_credit::{
    estimate_causal_credit, CausalCreditReport, CounterfactualEvaluation,
};

#[derive(Debug, Clone, Default)]
pub struct CausalCreditBridgeConfig;

#[derive(Debug, Clone, Default)]
pub struct CausalCreditBridge;

impl CausalCreditBridge {
    pub fn new(_config: CausalCreditBridgeConfig) -> Self {
        Self
    }

    pub fn allocate_credit(
        &self,
        evaluations: &[CounterfactualEvaluation],
    ) -> BrainResult<CausalCreditReport> {
        estimate_causal_credit(evaluations)
    }
}
