//! Durable promoted-executor overlay for the canonical catalog.
//!
//! Records are content-addressed under `tidex_home/operator/promoted-executors`.
//! Only [`crate::promotion_authority`] may write them after UPG readiness.
//! This module does not import governance (architecture boundary).

use crate::foundation::authority::{
    ensure_private_directory, replace_private_file_atomic, with_private_authority_lock,
    write_or_verify_immutable, PrivateFileReference,
};
use crate::foundation::digest::Sha256Digest;
use crate::foundation::error::{BrainError, BrainResult};
use crate::operator::executor_registry::ExecutorDescriptor;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub const PROMOTED_RECORD_SCHEMA: &str = "tidex.promoted_executor_record/v1";
const RECORD_DOMAIN: &[u8] = b"TIDEX:PROMOTED-EXECUTOR-RECORD:v1\0";

fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}

fn integrity(code: &str) -> BrainError {
    BrainError::Integrity(code.into())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PromotedExecutorRecord {
    pub schema: String,
    pub gate_receipt_sha256: Sha256Digest,
    pub descriptor: ExecutorDescriptor,
    pub operation: Option<String>,
    pub record_sha256: Sha256Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
struct PromotedExecutorIndex {
    schema: String,
    records: BTreeSet<String>,
}

fn promoted_root(tidex_home: &Path) -> PathBuf {
    tidex_home.join("operator/promoted-executors")
}

pub fn record_path(tidex_home: &Path, record_sha256: &Sha256Digest) -> PathBuf {
    promoted_root(tidex_home)
        .join("by-sha")
        .join(format!("{}.json", record_sha256))
}

fn index_path(tidex_home: &Path) -> PathBuf {
    promoted_root(tidex_home).join("index.json")
}

fn lock_path(tidex_home: &Path) -> PathBuf {
    promoted_root(tidex_home).join("LOCK")
}

pub fn record_digest(record: &PromotedExecutorRecord) -> BrainResult<Sha256Digest> {
    let mut unsigned = record.clone();
    unsigned.record_sha256 = Sha256Digest::zero();
    Ok(Sha256Digest::digest_domain(
        RECORD_DOMAIN,
        &serde_json::to_vec(&unsigned)?,
    ))
}

pub fn ensure_promoted_layout(tidex_home: &Path) -> BrainResult<()> {
    if !tidex_home.is_absolute() {
        return Err(invalid("promoted_executor_home_must_be_absolute"));
    }
    ensure_private_directory(tidex_home, &promoted_root(tidex_home))?;
    ensure_private_directory(tidex_home, &promoted_root(tidex_home).join("by-sha"))?;
    Ok(())
}

pub fn validate_operation_token(operation: &str) -> BrainResult<()> {
    let trimmed = operation.trim();
    if trimmed.is_empty()
        || trimmed.len() > 128
        || trimmed == "hold"
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
    {
        return Err(invalid("promoted_executor_operation_invalid"));
    }
    Ok(())
}

fn load_index(tidex_home: &Path) -> BrainResult<PromotedExecutorIndex> {
    let path = index_path(tidex_home);
    if !path.exists() {
        return Ok(PromotedExecutorIndex {
            schema: "tidex.promoted_executor_index/v1".into(),
            records: BTreeSet::new(),
        });
    }
    let bytes = fs::read(&path).map_err(|_| integrity("promoted_executor_index_unreadable"))?;
    let index: PromotedExecutorIndex = serde_json::from_slice(&bytes)?;
    if index.schema != "tidex.promoted_executor_index/v1" {
        return Err(integrity("promoted_executor_index_schema_invalid"));
    }
    Ok(index)
}

fn persist_index(tidex_home: &Path, index: &PromotedExecutorIndex) -> BrainResult<()> {
    let bytes = serde_json::to_vec(index)?;
    replace_private_file_atomic(tidex_home, &index_path(tidex_home), &bytes, None)?;
    Ok(())
}

fn read_record(tidex_home: &Path, record_sha256: &Sha256Digest) -> BrainResult<PromotedExecutorRecord> {
    let path = record_path(tidex_home, record_sha256);
    let bytes = fs::read(&path).map_err(|_| invalid("promoted_executor_record_missing"))?;
    let record: PromotedExecutorRecord = serde_json::from_slice(&bytes)?;
    if record.schema != PROMOTED_RECORD_SCHEMA {
        return Err(integrity("promoted_executor_record_schema_invalid"));
    }
    if record.record_sha256 != *record_sha256 || record_digest(&record)? != record.record_sha256 {
        return Err(integrity("promoted_executor_record_digest_mismatch"));
    }
    record.descriptor.validate()?;
    if let Some(op) = &record.operation {
        validate_operation_token(op)?;
    }
    Ok(record)
}

pub fn load_promoted_executor_records(
    tidex_home: &Path,
) -> BrainResult<Vec<PromotedExecutorRecord>> {
    if !promoted_root(tidex_home).exists() {
        return Ok(Vec::new());
    }
    let index = load_index(tidex_home)?;
    let mut out = Vec::with_capacity(index.records.len());
    let mut seen_ids = BTreeSet::new();
    let mut seen_ops = BTreeSet::new();
    for sha in &index.records {
        let digest = Sha256Digest::parse(sha)
            .map_err(|_| integrity("promoted_executor_index_digest_invalid"))?;
        let record = read_record(tidex_home, &digest)?;
        if !seen_ids.insert(record.descriptor.executor_id.clone()) {
            return Err(integrity("promoted_executor_duplicate_executor_id"));
        }
        if let Some(op) = &record.operation {
            if !seen_ops.insert(op.clone()) {
                return Err(integrity("promoted_executor_duplicate_operation"));
            }
        }
        out.push(record);
    }
    out.sort_by(|a, b| a.descriptor.executor_id.cmp(&b.descriptor.executor_id));
    Ok(out)
}

pub fn load_promoted_executor_descriptors(
    tidex_home: &Path,
) -> BrainResult<Vec<ExecutorDescriptor>> {
    Ok(load_promoted_executor_records(tidex_home)?
        .into_iter()
        .map(|r| r.descriptor)
        .collect())
}

pub fn promoted_operation_executor_id(
    tidex_home: &Path,
    operation: &str,
) -> BrainResult<Option<String>> {
    for record in load_promoted_executor_records(tidex_home)? {
        if record.operation.as_deref() == Some(operation) {
            return Ok(Some(record.descriptor.executor_id));
        }
    }
    Ok(None)
}

pub struct PersistPromotedOutcome {
    pub reference: PrivateFileReference,
    pub idempotent: bool,
}

/// Persist a sealed promoted-executor record. Caller must have already enforced
/// UPG readiness and descriptor policy (PromotionAuthority).
pub fn persist_promoted_executor_record(
    tidex_home: &Path,
    record: &PromotedExecutorRecord,
) -> BrainResult<PersistPromotedOutcome> {
    if record.schema != PROMOTED_RECORD_SCHEMA {
        return Err(invalid("promoted_executor_record_schema_invalid"));
    }
    if record_digest(record)? != record.record_sha256 {
        return Err(integrity("promoted_executor_record_digest_mismatch"));
    }
    record.descriptor.validate()?;
    if let Some(op) = &record.operation {
        validate_operation_token(op)?;
    }
    ensure_promoted_layout(tidex_home)?;

    with_private_authority_lock(tidex_home, &lock_path(tidex_home), || {
        let existing_records = load_promoted_executor_records(tidex_home)?;
        for prior in &existing_records {
            if prior.record_sha256 == record.record_sha256 {
                let path = record_path(tidex_home, &record.record_sha256);
                let bytes = serde_json::to_vec(record)?;
                let file_digest = write_or_verify_immutable(tidex_home, &path, &bytes)?;
                return Ok(PersistPromotedOutcome {
                    reference: PrivateFileReference::new(path, file_digest),
                    idempotent: true,
                });
            }
            if prior.descriptor.executor_id == record.descriptor.executor_id
                && prior.descriptor.descriptor_sha256 != record.descriptor.descriptor_sha256
            {
                return Err(integrity("promoted_executor_id_collision"));
            }
            if let (Some(a), Some(b)) = (&prior.operation, &record.operation) {
                if a == b && prior.descriptor.executor_id != record.descriptor.executor_id {
                    return Err(integrity("promoted_executor_operation_collision"));
                }
            }
        }

        let path = record_path(tidex_home, &record.record_sha256);
        let bytes = serde_json::to_vec(record)?;
        let file_digest = write_or_verify_immutable(tidex_home, &path, &bytes)?;
        let mut index = load_index(tidex_home)?;
        index.records.insert(record.record_sha256.to_string());
        persist_index(tidex_home, &index)?;
        Ok(PersistPromotedOutcome {
            reference: PrivateFileReference::new(path, file_digest),
            idempotent: false,
        })
    })
}
