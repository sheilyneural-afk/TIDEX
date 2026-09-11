//! A single physical materialization boundary for both TIDE-X compiler frontends.
//!
//! Numerical planning is not execution evidence. Alternative encodings may reach
//! the physical actuator only if their reconstructed f32 delta is bit-exact to
//! the source compiler's delta. Lossy shadow experiments remain available through
//! their existing APIs, but cannot silently inherit the source's validation.
//! This module never activates models and creates no second filesystem authority.

use crate::analysis::block_tomography::{BlockShapeSpec, ParameterBlockLayout};
use crate::foundation::artifact::{read_dvec_f32, ArtifactWriteAuthority, DeltaArtifactRef};
use crate::foundation::authority::{
    ensure_private_parent, open_existing_private_file, root_relative_path,
    write_or_verify_immutable, PrivateFileReference,
};
use crate::foundation::digest::Sha256Digest;
use crate::foundation::error::{BrainError, BrainResult};
use crate::foundation::identity::{CapabilityId, TensorId};
use crate::foundation::security::verify_internal_private_root;
use crate::materialization::low_rank_shadow_materializer::{
    factor_dense_delta_verified, LowRankShadowPolicy, VerifiedLowRankFactors,
};
use crate::materialization::sparse_shadow_materializer::{
    sparsify, SparseCoordinate, SparseShadowPolicy,
};
use crate::materialization::universal_capability_compiler::{
    replay_universal_capability_shadow_plan, UniversalCapabilityPlanningRequest,
    UniversalCapabilityShadowPlanReceipt,
};
use crate::receiver::checkpoint_adapter::{
    planning_profile_from_physical, PhysicalPlanningProfileRequest,
};
use crate::receiver::model_adaptation::{
    authenticate_live_receiver_model_profile, ReceiverModelProfile,
};
use crate::receiver::receiver_layout::ReceiverMaterializationLayout;
use crate::receiver::receiver_profile::MaterializationStrategy;
use crate::receiver::receiver_weight_binding::{
    authenticate_receiver_weight_candidate, ReceiverResponseProtocol, ReceiverWeightBasis,
    ReceiverWeightRequest,
};
use crate::receiver::weight_actuator::{
    authenticate_weight_materialization_receipt, materialize_dense_delta_checkpoint,
    parameter_layout_for_tensors, receiver_delta_dtype_supported, WeightMaterializationReceipt,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const MAX_RECORD_BYTES: u64 = 128 * 1024 * 1024;
const MAX_COMPRESSED_ELEMENTS: u64 = 16 * 1024 * 1024;
const REQUEST_SCHEMA: &str = "cerebro.tidex.physical_materialization_request/v1";
const RECEIPT_SCHEMA: &str = "cerebro.tidex.physical_materialization_receipt/v1";

fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}
fn integrity(code: &str) -> BrainError {
    BrainError::Integrity(code.into())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledMaterializationSource {
    MeasuredReceiver {
        candidate: PrivateFileReference,
    },
    UniversalPlan {
        request: Box<UniversalCapabilityPlanningRequest>,
        receipt: Box<UniversalCapabilityShadowPlanReceipt>,
        layout: Box<ReceiverMaterializationLayout>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PhysicalMaterializationBackend {
    Dense,
    LowRank { policy: LowRankShadowPolicy },
    Sparse { policy: SparseShadowPolicy },
}
impl PhysicalMaterializationBackend {
    fn strategy(&self) -> MaterializationStrategy {
        match self {
            Self::Dense => MaterializationStrategy::DenseDelta,
            Self::LowRank { .. } => MaterializationStrategy::LowRank,
            Self::Sparse { .. } => MaterializationStrategy::SparseDelta,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PhysicalMaterializationRequest {
    pub schema: String,
    pub physical_profile: PrivateFileReference,
    pub source: CompiledMaterializationSource,
    pub backend: PhysicalMaterializationBackend,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NamedLowRankFactors {
    pub tensor_id: TensorId,
    pub factors: VerifiedLowRankFactors,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PhysicalDeltaEncoding {
    Dense {
        parameter_count: u64,
    },
    LowRank {
        tensors: Vec<NamedLowRankFactors>,
    },
    Sparse {
        parameter_count: u64,
        coordinates: Vec<SparseCoordinate>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PhysicalMaterializationReceipt {
    pub schema: String,
    pub request: PrivateFileReference,
    pub compiler_source_sha256: Sha256Digest,
    pub capability_id: CapabilityId,
    pub physical_profile: PrivateFileReference,
    pub encoding: PhysicalDeltaEncoding,
    pub delta: DeltaArtifactRef,
    pub output_path: PathBuf,
    pub materialization: WeightMaterializationReceipt,
    pub maximum_source_f32_rounding_error: f64,
    pub reconstruction_preserves_f32_delta_exactly: bool,
    pub model_execution_verified: bool,
    pub authorizes_promotion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PhysicalMaterializationOutcome {
    pub receipt: PhysicalMaterializationReceipt,
    pub receipt_reference: PrivateFileReference,
}

struct Derived {
    profile: ReceiverModelProfile,
    capability_id: CapabilityId,
    layout: ParameterBlockLayout,
    existing_delta: Option<DeltaArtifactRef>,
    values: Option<Vec<f32>>,
    encoding: PhysicalDeltaEncoding,
    rounding_error: f64,
}

fn read_record<T: serde::de::DeserializeOwned>(
    root: &Path,
    reference: &PrivateFileReference,
) -> BrainResult<T> {
    Ok(serde_json::from_slice(
        &reference.read_verified_bounded(root, MAX_RECORD_BYTES)?,
    )?)
}

fn derive(root: &Path, input: &PhysicalMaterializationRequest) -> BrainResult<Derived> {
    if input.schema != REQUEST_SCHEMA {
        return Err(invalid("physical_materialization_request_invalid"));
    }
    root_relative_path(root, &input.output_path)?;
    let profile = authenticate_live_receiver_model_profile(root, &input.physical_profile)?;
    let (capability_id, layout, existing_delta, source_values) = match &input.source {
        CompiledMaterializationSource::MeasuredReceiver { candidate } => {
            let candidate = authenticate_receiver_weight_candidate(root, candidate)?;
            if !candidate.blockers.is_empty() || !candidate.numerical.allowed {
                return Err(invalid("physical_materialization_source_blocked"));
            }
            let delta = candidate
                .dense_delta
                .ok_or_else(|| invalid("physical_materialization_delta_missing"))?;
            let request: ReceiverWeightRequest = read_record(root, &candidate.request)?;
            let basis: ReceiverWeightBasis = read_record(root, &request.basis)?;
            if basis.base_model_sha256 != profile.checkpoint.sha256 {
                return Err(integrity("physical_materialization_base_mismatch"));
            }
            let protocol: serde_json::Value = read_record(root, &request.protocol)?;
            if protocol.get("schema").and_then(|v| v.as_str())
                == Some("cerebro.tidex.receiver_response_protocol/v1")
            {
                let protocol: ReceiverResponseProtocol = serde_json::from_value(protocol)?;
                if protocol.model_config_sha256 != profile.config.sha256
                    || protocol.tokenizer_sha256 != profile.tokenizer.sha256
                {
                    return Err(integrity("physical_materialization_tokenizer_or_config_mismatch"));
                }
            }
            // Dense measured candidates keep the original streaming path without
            // imposing the small numerical compressor's allocation budget.
            let values = if matches!(input.backend, PhysicalMaterializationBackend::Dense) {
                None
            } else {
                if delta.parameter_count > MAX_COMPRESSED_ELEMENTS {
                    return Err(invalid("physical_compression_resource_limit"));
                }
                Some(
                    read_dvec_f32(root, &delta)?
                        .into_iter()
                        .map(f64::from)
                        .collect::<Vec<_>>(),
                )
            };
            (candidate.target_capability_id, basis.layout, Some(delta), values)
        }
        CompiledMaterializationSource::UniversalPlan {
            request,
            receipt,
            layout,
        } => {
            if request.requested_strategy != input.backend.strategy() {
                return Err(invalid("physical_materialization_strategy_mismatch"));
            }
            replay_universal_capability_shadow_plan(request, receipt)?;
            let physical = planning_profile_from_physical(
                root,
                &PhysicalPlanningProfileRequest {
                    schema: "cerebro.tidex.physical_planning_profile_request/v1".into(),
                    physical_profile: input.physical_profile.clone(),
                    modalities: request.receiver_profile.modalities.clone(),
                    supports_persistent_state: request.receiver_profile.supports_persistent_state,
                },
            )?;
            if physical.profile != request.receiver_profile
                || physical.snapshot != request.receiver_snapshot
                || &physical.layout != layout.as_ref()
            {
                return Err(integrity("physical_materialization_plan_profile_mismatch"));
            }
            let target = &receipt
                .shadow_plan
                .compilation_receipt
                .compilation
                .receiver
                .target_delta;
            if target.len() as u64 > MAX_COMPRESSED_ELEMENTS {
                return Err(invalid("physical_materialization_resource_limit"));
            }
            let affected = request.affected_regions.iter().collect::<BTreeSet<_>>();
            let mut shapes = Vec::new();
            let mut values = Vec::new();
            for block in &layout.geometry.layout.blocks {
                let id = TensorId::parse(&block.name)?;
                let start = usize::try_from(block.offset)
                    .map_err(|_| invalid("physical_materialization_offset_overflow"))?;
                let end = start
                    .checked_add(block.count)
                    .ok_or_else(|| invalid("physical_materialization_range_overflow"))?;
                let part = target
                    .get(start..end)
                    .ok_or_else(|| integrity("physical_materialization_range_invalid"))?;
                if affected.contains(&id) {
                    shapes.push(BlockShapeSpec {
                        name: block.name.clone(),
                        shape: block.shape.clone(),
                        count: block.count,
                    });
                    values.extend_from_slice(part);
                } else if part.iter().any(|v| *v != 0.0) {
                    return Err(integrity("physical_materialization_nonzero_outside_plan"));
                }
            }
            (
                request.compilation.capability_ir.capability_id().clone(),
                ParameterBlockLayout::from_shapes(&shapes)?,
                None,
                Some(values),
            )
        }
    };
    layout.validate()?;
    let ids = layout
        .blocks
        .iter()
        .map(|b| TensorId::parse(&b.name))
        .collect::<BrainResult<Vec<_>>>()?;
    if parameter_layout_for_tensors(&profile.inventory, &ids)? != layout {
        return Err(integrity("physical_materialization_layout_mismatch"));
    }
    for id in &ids {
        let spec = profile
            .inventory
            .tensors
            .iter()
            .find(|t| &t.tensor_id == id)
            .ok_or_else(|| integrity("physical_materialization_tensor_missing"))?;
        if !receiver_delta_dtype_supported(&spec.dtype) {
            return Err(invalid("physical_materialization_dtype_unsupported"));
        }
    }
    let Some(source_values) = source_values else {
        return Ok(Derived {
            profile,
            capability_id,
            encoding: PhysicalDeltaEncoding::Dense {
                parameter_count: layout.total_parameter_count,
            },
            layout,
            existing_delta,
            values: None,
            rounding_error: 0.0,
        });
    };
    if source_values.len() as u64 != layout.total_parameter_count {
        return Err(integrity("physical_materialization_source_shape"));
    }
    let mut rounding_error = 0.0_f64;
    let original = source_values
        .iter()
        .map(|v| {
            let rounded = *v as f32;
            if !v.is_finite() || !rounded.is_finite() || (*v != 0.0 && rounded == 0.0) {
                return Err(invalid("physical_materialization_f32_conversion_invalid"));
            }
            rounding_error = rounding_error.max((*v - f64::from(rounded)).abs());
            Ok(rounded)
        })
        .collect::<BrainResult<Vec<_>>>()?;
    let values64 = original.iter().copied().map(f64::from).collect::<Vec<_>>();
    let (reconstructed, encoding) = match &input.backend {
        PhysicalMaterializationBackend::Dense => (
            original.clone(),
            PhysicalDeltaEncoding::Dense {
                parameter_count: layout.total_parameter_count,
            },
        ),
        PhysicalMaterializationBackend::LowRank { policy } => {
            let mut reconstructed = Vec::with_capacity(original.len());
            let mut tensors = Vec::new();
            for block in &layout.blocks {
                if block.shape.len() != 2 {
                    return Err(invalid("physical_low_rank_requires_matrix"));
                }
                let start = usize::try_from(block.offset)
                    .map_err(|_| invalid("physical_materialization_offset_overflow"))?;
                let factors = factor_dense_delta_verified(
                    block.shape[0],
                    block.shape[1],
                    &values64[start..start + block.count],
                    policy,
                )?;
                reconstructed.extend(factors.materialize_dense()?.into_iter().map(|v| v as f32));
                tensors.push(NamedLowRankFactors {
                    tensor_id: TensorId::parse(&block.name)?,
                    factors,
                });
            }
            (reconstructed, PhysicalDeltaEncoding::LowRank { tensors })
        }
        PhysicalMaterializationBackend::Sparse { policy } => {
            policy.validate()?;
            let budget = policy
                .maximum_nonzero_count
                .min((original.len() as f64 * policy.maximum_density).floor() as usize);
            if budget == 0 {
                return Err(invalid("physical_sparse_budget_empty"));
            }
            let (coordinates, _, _, _) = sparsify(&values64, budget, policy)?;
            let reduction = 1.0 - (coordinates.len() as f64 * 16.0) / (original.len() as f64 * 8.0);
            if reduction < policy.minimum_storage_reduction_ratio {
                return Err(invalid("physical_sparse_storage_not_admissible"));
            }
            let mut reconstructed = vec![0.0_f32; original.len()];
            for coordinate in &coordinates {
                let index = usize::try_from(coordinate.flat_index)
                    .map_err(|_| invalid("physical_sparse_index_overflow"))?;
                *reconstructed
                    .get_mut(index)
                    .ok_or_else(|| invalid("physical_sparse_index_out_of_range"))? =
                    coordinate.value as f32;
            }
            (
                reconstructed,
                PhysicalDeltaEncoding::Sparse {
                    parameter_count: layout.total_parameter_count,
                    coordinates,
                },
            )
        }
    };
    if reconstructed.len() != original.len()
        || reconstructed
            .iter()
            .zip(&original)
            .any(|(a, b)| a.to_bits() != b.to_bits())
    {
        return Err(invalid("physical_backend_changes_compiled_delta_requires_fresh_validation"));
    }
    Ok(Derived {
        profile,
        capability_id,
        layout,
        existing_delta,
        values: Some(reconstructed),
        encoding,
        rounding_error,
    })
}

fn record_path(root: &Path, category: &str, sha: &Sha256Digest) -> PathBuf {
    root.join("state/materialization_pipeline")
        .join(category)
        .join("by-sha")
        .join(format!("{sha}.json"))
}
fn persist_record<T: Serialize>(
    root: &Path,
    category: &str,
    record: &T,
) -> BrainResult<PrivateFileReference> {
    let bytes = serde_json::to_vec(record)?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(invalid("physical_materialization_record_limit"));
    }
    let sha = Sha256Digest::digest_bytes(&bytes);
    let path = record_path(root, category, &sha);
    write_or_verify_immutable(root, &path, &bytes)?;
    Ok(PrivateFileReference::new(path, sha))
}

pub fn materialize_compiled_checkpoint(
    root: &Path,
    input: &PhysicalMaterializationRequest,
) -> BrainResult<PhysicalMaterializationOutcome> {
    let root = verify_internal_private_root(root)?;
    let derived = derive(&root, input)?;
    let delta = match derived.existing_delta {
        Some(delta) => delta,
        None => ArtifactWriteAuthority::for_internal_root(&root)?.create_content_addressed_dvec(
            derived
                .values
                .as_ref()
                .ok_or_else(|| integrity("physical_materialization_values_missing"))?,
        )?,
    };
    let request = persist_record(&root, "requests", input)?;
    ensure_private_parent(&root, &input.output_path)?;
    let materialization = materialize_dense_delta_checkpoint(
        &root,
        &derived.profile.checkpoint.path,
        &derived.profile.checkpoint.sha256,
        &derived.layout,
        &delta,
        &input.output_path,
    )?;
    let receipt = PhysicalMaterializationReceipt {
        schema: RECEIPT_SCHEMA.into(),
        request,
        compiler_source_sha256: Sha256Digest::parse(env!("TIDEX_SOURCE_TREE_DIGEST"))?,
        capability_id: derived.capability_id,
        physical_profile: input.physical_profile.clone(),
        encoding: derived.encoding,
        delta,
        output_path: input.output_path.clone(),
        materialization,
        maximum_source_f32_rounding_error: derived.rounding_error,
        reconstruction_preserves_f32_delta_exactly: true,
        model_execution_verified: false,
        authorizes_promotion: false,
    };
    let receipt_reference = persist_record(&root, "receipts", &receipt)?;
    authenticate_compiled_checkpoint(&root, &receipt_reference)?;
    Ok(PhysicalMaterializationOutcome {
        receipt,
        receipt_reference,
    })
}

/// Read-only replay: recompiles the original source, reconstructs the chosen
/// encoding, authenticates the actual delta and verifies the output arithmetic.
pub fn authenticate_compiled_checkpoint(
    root: &Path,
    reference: &PrivateFileReference,
) -> BrainResult<PhysicalMaterializationReceipt> {
    let root = verify_internal_private_root(root)?;
    if reference.path != record_path(&root, "receipts", &reference.sha256) {
        return Err(integrity("physical_materialization_receipt_path_invalid"));
    }
    let bytes = reference.read_verified_bounded(&root, MAX_RECORD_BYTES)?;
    let receipt: PhysicalMaterializationReceipt = serde_json::from_slice(&bytes)?;
    if receipt.schema != RECEIPT_SCHEMA
        || serde_json::to_vec(&receipt)? != bytes
        || receipt.compiler_source_sha256 != Sha256Digest::parse(env!("TIDEX_SOURCE_TREE_DIGEST"))?
        || receipt.model_execution_verified
        || receipt.authorizes_promotion
        || !receipt.reconstruction_preserves_f32_delta_exactly
        || receipt.request.path != record_path(&root, "requests", &receipt.request.sha256)
        || receipt.delta.path
            != root
                .join("artifacts/deltas/by-sha")
                .join(format!("{}.dvec", receipt.delta.sha256))
    {
        return Err(integrity("physical_materialization_receipt_contract_invalid"));
    }
    let request: PhysicalMaterializationRequest = read_record(&root, &receipt.request)?;
    let derived = derive(&root, &request)?;
    if receipt.capability_id != derived.capability_id
        || receipt.physical_profile != request.physical_profile
        || receipt.output_path != request.output_path
        || receipt.encoding != derived.encoding
        || receipt.maximum_source_f32_rounding_error.to_bits() != derived.rounding_error.to_bits()
        || derived
            .existing_delta
            .as_ref()
            .is_some_and(|delta| delta != &receipt.delta)
    {
        return Err(integrity("physical_materialization_replay_mismatch"));
    }
    if let Some(expected) = derived.values {
        if receipt.delta.parameter_count != expected.len() as u64 {
            return Err(integrity("physical_materialization_delta_count"));
        }
        let actual = read_dvec_f32(&root, &receipt.delta)?;
        if actual
            .iter()
            .zip(expected)
            .any(|(a, b)| a.to_bits() != b.to_bits())
        {
            return Err(integrity("physical_materialization_delta_replay_mismatch"));
        }
    }
    drop(open_existing_private_file(&root, &receipt.output_path)?);
    authenticate_weight_materialization_receipt(
        &root,
        &derived.profile.checkpoint.path,
        &derived.layout,
        &receipt.delta,
        &receipt.output_path,
        &receipt.materialization,
    )?;
    Ok(receipt)
}
