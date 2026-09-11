//! Thin adapter to the canonical governed adapter bank.

use crate::foundation::authority::PrivateFileReference;
use crate::foundation::error::BrainResult;
use crate::governance::adapter_bank::{
    AdapterActivationRequest, AdapterBank, AdapterBankCommit, AdapterBankHistoryStatus,
    AdapterBankQuery, AdapterBankReport, AdapterCandidateMaterializationRequest,
    AdapterCompositionRequest, AdapterImportRequest, AdapterRevocationRequest,
    AdapterRollbackRequest,
};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct AdapterBankBridgeConfig;

pub struct AdapterBankBridge {
    bank: AdapterBank,
}

impl AdapterBankBridge {
    pub fn open(root: impl AsRef<Path>) -> BrainResult<Self> {
        Ok(Self {
            bank: AdapterBank::open(root)?,
        })
    }

    pub fn import_lora(&self, request: &AdapterImportRequest) -> BrainResult<AdapterBankCommit> {
        self.bank.import_lora(request)
    }

    pub fn compose_exact(
        &self,
        request: &AdapterCompositionRequest,
    ) -> BrainResult<AdapterBankCommit> {
        self.bank.compose_exact(request)
    }

    pub fn materialize_candidate(
        &self,
        request: &AdapterCandidateMaterializationRequest,
    ) -> BrainResult<PrivateFileReference> {
        self.bank.materialize_candidate(request)
    }

    pub fn activate(&self, request: &AdapterActivationRequest) -> BrainResult<AdapterBankCommit> {
        self.bank.activate(request)
    }

    pub fn revoke(&self, request: &AdapterRevocationRequest) -> BrainResult<AdapterBankCommit> {
        self.bank.revoke(request)
    }

    pub fn rollback(&self, request: &AdapterRollbackRequest) -> BrainResult<AdapterBankCommit> {
        self.bank.rollback(request)
    }

    pub fn query(&self, query: &AdapterBankQuery) -> BrainResult<AdapterBankReport> {
        self.bank.query(query)
    }

    pub fn verify_history(&self) -> BrainResult<AdapterBankHistoryStatus> {
        self.bank.verify_history()
    }
}
