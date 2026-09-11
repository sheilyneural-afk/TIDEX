//! Thin adapter to the canonical hash-chained ledger.

use crate::cross_model::models::CapabilityMetadata;
use crate::error::BrainResult;
use crate::ledger::{append, verify, LedgerEvent, LedgerStatus};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct LedgerBridgeConfig;

#[derive(Debug, Clone, Default)]
pub struct LedgerBridge;

impl LedgerBridge {
    pub fn new(_config: LedgerBridgeConfig) -> Self {
        Self
    }

    pub fn log_discovery(
        &self,
        root: &Path,
        capability: &CapabilityMetadata,
    ) -> BrainResult<LedgerEvent> {
        capability
            .validate()
            .map_err(crate::error::BrainError::Invalid)?;
        append(
            root,
            "cross_model.discovery",
            serde_json::to_value(capability)?,
        )
    }

    pub fn log_transfer(
        &self,
        root: &Path,
        capability: &CapabilityMetadata,
        target_model: &str,
        result: Value,
    ) -> BrainResult<LedgerEvent> {
        capability
            .validate()
            .map_err(crate::error::BrainError::Invalid)?;
        if target_model.trim().is_empty() {
            return Err(crate::error::BrainError::Invalid(
                "cross_model_ledger_target_invalid".into(),
            ));
        }
        append(
            root,
            "cross_model.transfer",
            json!({
                "capability": capability,
                "target_model": target_model,
                "result": result,
            }),
        )
    }

    pub fn verify_history(&self, root: &Path) -> BrainResult<LedgerStatus> {
        verify(root)
    }
}
