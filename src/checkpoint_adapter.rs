//! Read-only, content-authenticated adapter for real SafeTensors checkpoints.
//!
//! It inspects headers and exact file bytes without loading tensor payloads.
//! Unsupported or ambiguous encodings are rejected rather than guessed.

use crate::architecture_families::{fingerprint_architecture, ArchitectureFamilyFingerprint};
use crate::authority::PrivateFileReference;
use crate::block_tomography::{BlockShapeSpec, ParameterBlockLayout, ParameterLayoutArtifact};
use crate::digest::Sha256Digest;
use crate::error::{BrainError, BrainResult};
use crate::identity::{ArchitectureId, ModelId, TensorId};
use crate::model_adaptation::authenticate_live_receiver_model_profile;
use crate::receiver_layout::{
    FloatingScalarType, ReceiverMaterializationLayout, ReceiverScalarEncoding,
    ReceiverTensorPartitioning, ReceiverTensorPhysicalSpec,
};
use crate::receiver_profile::{
    CapabilityModality, MaterializationStrategy, ReceiverArchitecture, ReceiverProfile,
    ReceiverRegion,
};
use crate::receiver_profiler::ReceiverSnapshotBinding;
use crate::weight_actuator::{inspect_model_safetensors, ModelTensorSpec};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_CHECKPOINT_FILES: usize = 16_384;
const MAX_AUXILIARY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CHECKPOINT_BYTES: u64 = 1 << 50;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SafeTensorsReceiverRequest {
    pub schema: String,
    pub model_id: ModelId,
    pub architecture_id: ArchitectureId,
    pub architecture: ReceiverArchitecture,
    pub modalities: BTreeSet<CapabilityModality>,
    pub supports_persistent_state: bool,
    pub checkpoint_files: Vec<PathBuf>,
    pub configuration_file: PathBuf,
    pub tokenizer_file: PathBuf,
    pub supported_strategies: BTreeSet<MaterializationStrategy>,
    #[serde(default)]
    pub materialization_tensors: BTreeSet<TensorId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InspectedReceiverArtifacts {
    pub schema: String,
    pub profile: ReceiverProfile,
    pub layout: ReceiverMaterializationLayout,
    pub snapshot: ReceiverSnapshotBinding,
    pub architecture_fingerprint: ArchitectureFamilyFingerprint,
    pub checkpoint_file_sha256: BTreeMap<PathBuf, Sha256Digest>,
    pub manifest_sha256: Sha256Digest,
}

fn confined(root: &Path, relative: &Path) -> BrainResult<PathBuf> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(BrainError::Invalid("checkpoint_path_not_relative".into()));
    }
    let canonical_root = root.canonicalize()?;
    let path = canonical_root.join(relative).canonicalize()?;
    if !path.metadata()?.is_file() || !confined_file_target_allowed(&canonical_root, &path) {
        return Err(BrainError::Invalid(
            "checkpoint_path_not_confined_file".into(),
        ));
    }
    Ok(path)
}

/// Hugging Face snapshots are immutable directory views whose regular entries
/// are symlinks into the sibling repository-local `blobs/` CAS. Permit only
/// that exact escape from a `snapshots/<revision>` root; arbitrary symlink
/// targets remain rejected after canonicalization.
fn confined_file_target_allowed(canonical_root: &Path, path: &Path) -> bool {
    if path.starts_with(canonical_root) {
        return true;
    }
    let Some(snapshots_root) = canonical_root.parent() else {
        return false;
    };
    if snapshots_root.file_name().and_then(|value| value.to_str()) != Some("snapshots") {
        return false;
    }
    let Some(repository_root) = snapshots_root.parent() else {
        return false;
    };
    let Ok(blobs_root) = repository_root.join("blobs").canonicalize() else {
        return false;
    };
    blobs_root.is_dir() && path.starts_with(blobs_root)
}

fn auxiliary_bytes(path: &Path, maximum: u64) -> BrainResult<Vec<u8>> {
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() as u64 > maximum {
        return Err(BrainError::Invalid(
            "checkpoint_auxiliary_size_invalid".into(),
        ));
    }
    Ok(bytes)
}

fn encoding(dtype: &str) -> BrainResult<(ReceiverScalarEncoding, u64)> {
    match dtype {
        "F64" => Ok((
            ReceiverScalarEncoding::Floating {
                scalar_type: FloatingScalarType::Float64,
            },
            8,
        )),
        "F32" => Ok((
            ReceiverScalarEncoding::Floating {
                scalar_type: FloatingScalarType::Float32,
            },
            4,
        )),
        "BF16" => Ok((
            ReceiverScalarEncoding::Floating {
                scalar_type: FloatingScalarType::Bfloat16,
            },
            2,
        )),
        "F16" => Ok((
            ReceiverScalarEncoding::Floating {
                scalar_type: FloatingScalarType::Float16,
            },
            2,
        )),
        "F8_E4M3" | "F8_E4M3FN" => Ok((
            ReceiverScalarEncoding::Floating {
                scalar_type: FloatingScalarType::Float8E4m3,
            },
            1,
        )),
        "F8_E5M2" => Ok((
            ReceiverScalarEncoding::Floating {
                scalar_type: FloatingScalarType::Float8E5m2,
            },
            1,
        )),
        _ => Err(BrainError::Invalid(format!(
            "checkpoint_dtype_requires_explicit_quantization_contract:{dtype}"
        ))),
    }
}

pub fn inspect_safetensors_receiver(
    root: &Path,
    request: &SafeTensorsReceiverRequest,
) -> BrainResult<InspectedReceiverArtifacts> {
    if request.schema != "cerebro.tidex.safetensors_receiver_request/v1"
        || request.checkpoint_files.is_empty()
        || request.checkpoint_files.len() > MAX_CHECKPOINT_FILES
        || request.modalities.is_empty()
        || request.supported_strategies.is_empty()
    {
        return Err(BrainError::Invalid(
            "safetensors_receiver_request_invalid".into(),
        ));
    }
    let mut files = request.checkpoint_files.clone();
    files.sort();
    if files.windows(2).any(|p| p[0] >= p[1]) {
        return Err(BrainError::Invalid("checkpoint_files_not_unique".into()));
    }
    let mut all = BTreeMap::new();
    let mut file_digests = BTreeMap::new();
    let mut total_bytes = 0u64;
    for relative in files {
        let path = confined(root, &relative)?;
        total_bytes = total_bytes
            .checked_add(path.metadata()?.len())
            .ok_or_else(|| BrainError::Invalid("checkpoint_total_overflow".into()))?;
        if total_bytes > MAX_CHECKPOINT_BYTES {
            return Err(BrainError::Invalid("checkpoint_total_limit".into()));
        }
        // One parser/hash authority: inventory and digest describe the same consumed bytes.
        let inventory = inspect_model_safetensors(&path)?;
        for spec in inventory.tensors {
            let scalar = encoding(&spec.dtype)?.0;
            if all
                .insert(spec.tensor_id.as_str().to_string(), (spec, scalar))
                .is_some()
            {
                return Err(BrainError::Invalid("checkpoint_duplicate_tensor".into()));
            }
        }
        file_digests.insert(relative, inventory.model_sha256);
    }
    let config_bytes = auxiliary_bytes(
        &confined(root, &request.configuration_file)?,
        MAX_AUXILIARY_BYTES,
    )?;
    let tokenizer = Sha256Digest::digest_bytes(&auxiliary_bytes(
        &confined(root, &request.tokenizer_file)?,
        MAX_AUXILIARY_BYTES,
    )?);
    let snapshot_digest = Sha256Digest::digest_domain(
        b"CEREBRO:TIDEX:SAFETENSORS-SNAPSHOT:v1\0",
        &serde_json::to_vec(&file_digests)?,
    );
    assemble_inspected(
        request,
        all,
        &config_bytes,
        tokenizer,
        file_digests,
        snapshot_digest,
    )
}

fn assemble_inspected(
    request: &SafeTensorsReceiverRequest,
    all: BTreeMap<String, (ModelTensorSpec, ReceiverScalarEncoding)>,
    config_bytes: &[u8],
    tokenizer: Sha256Digest,
    file_digests: BTreeMap<PathBuf, Sha256Digest>,
    snapshot_digest: Sha256Digest,
) -> BrainResult<InspectedReceiverArtifacts> {
    let config = Sha256Digest::digest_bytes(config_bytes);
    let tensor_names = all.keys().cloned().collect::<Vec<_>>();
    let architecture_fingerprint = fingerprint_architecture(config_bytes, &tensor_names)?;
    if request.architecture != ReceiverArchitecture::Unknown
        && architecture_fingerprint.receiver_architecture != ReceiverArchitecture::Unknown
        && request.architecture != architecture_fingerprint.receiver_architecture
    {
        return Err(BrainError::Integrity(
            "declared_receiver_architecture_conflicts_with_checkpoint".into(),
        ));
    }
    let resolved_architecture = if request.architecture == ReceiverArchitecture::Unknown {
        architecture_fingerprint.receiver_architecture
    } else {
        request.architecture
    };
    let selected = if request.materialization_tensors.is_empty() {
        all.into_iter().collect::<Vec<_>>()
    } else {
        let mut all = all;
        let mut selected = Vec::with_capacity(request.materialization_tensors.len());
        for tensor_id in &request.materialization_tensors {
            let item = all.remove(tensor_id.as_str()).ok_or_else(|| {
                BrainError::Invalid("materialization_tensor_not_in_checkpoint".into())
            })?;
            selected.push((tensor_id.as_str().to_string(), item));
        }
        selected
    };
    let mut shapes = Vec::with_capacity(selected.len());
    let mut regions = Vec::with_capacity(selected.len());
    let mut physical = Vec::with_capacity(selected.len());
    for (name, (header, encoding)) in selected {
        let tensor_id = TensorId::parse(&name)?;
        let count = header.shape.iter().try_fold(1usize, |a, v| {
            a.checked_mul(*v)
                .ok_or_else(|| BrainError::Invalid("checkpoint_parameter_overflow".into()))
        })?;
        shapes.push(BlockShapeSpec {
            name: name.clone(),
            shape: header.shape,
            count,
        });
        regions.push(ReceiverRegion {
            tensor_id: tensor_id.clone(),
            parameter_count: u64::try_from(count)
                .map_err(|_| BrainError::Invalid("checkpoint_parameter_overflow".into()))?,
            supported_strategies: request
                .supported_strategies
                .iter()
                .copied()
                .filter(|strategy| {
                    *strategy != MaterializationStrategy::LowRank
                        || shapes.last().is_some_and(|s| s.shape.len() == 2)
                })
                .collect(),
        });
        physical.push(ReceiverTensorPhysicalSpec {
            tensor_id,
            encoding,
            partitioning: ReceiverTensorPartitioning::Replicated,
        });
    }
    let geometry = ParameterLayoutArtifact::new(ParameterBlockLayout::from_shapes(&shapes)?)?;
    let profile = ReceiverProfile {
        schema: "cerebro.tidex.receiver_profile/v1".into(),
        model_id: request.model_id.clone(),
        architecture_id: request.architecture_id.clone(),
        architecture: resolved_architecture,
        modalities: request.modalities.clone(),
        supports_persistent_state: request.supports_persistent_state,
        parameter_dimension: geometry.total_parameter_count,
        regions,
    };
    let layout = ReceiverMaterializationLayout::create(&profile, geometry, physical, vec![])?;
    let snapshot = ReceiverSnapshotBinding::create(
        &profile,
        snapshot_digest,
        config,
        tokenizer,
        layout.manifest_sha256.clone(),
    )?;
    let mut result = InspectedReceiverArtifacts {
        schema: "cerebro.tidex.inspected_receiver_artifacts/v1".into(),
        profile,
        layout,
        snapshot,
        architecture_fingerprint,
        checkpoint_file_sha256: file_digests,
        manifest_sha256: Sha256Digest::zero(),
    };
    let mut unsigned = result.clone();
    unsigned.manifest_sha256 = Sha256Digest::zero();
    result.manifest_sha256 = Sha256Digest::digest_domain(
        b"CEREBRO:TIDEX:INSPECTED-RECEIVER-ARTIFACTS:v1\0",
        &serde_json::to_vec(&unsigned)?,
    );
    Ok(result)
}

/// Project the operational physical authority into the alternative compiler's
/// layout. Modalities/state are declared planning requirements, not evidence of
/// model behavior. Model/config/tokenizer and tensor geometry are authenticated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PhysicalPlanningProfileRequest {
    pub schema: String,
    pub physical_profile: PrivateFileReference,
    pub modalities: BTreeSet<CapabilityModality>,
    pub supports_persistent_state: bool,
}

pub fn planning_profile_from_physical(
    private_root: &Path,
    request: &PhysicalPlanningProfileRequest,
) -> BrainResult<InspectedReceiverArtifacts> {
    if request.schema != "cerebro.tidex.physical_planning_profile_request/v1" {
        return Err(BrainError::Invalid(
            "physical_planning_profile_request_invalid".into(),
        ));
    }
    let source = authenticate_live_receiver_model_profile(private_root, &request.physical_profile)?;
    let config = auxiliary_bytes(&source.config.path, MAX_AUXILIARY_BYTES)?;
    if Sha256Digest::digest_bytes(&config) != source.config.sha256 {
        return Err(BrainError::Integrity(
            "physical_planning_config_changed".into(),
        ));
    }
    let mut all = BTreeMap::new();
    for spec in source.inventory.tensors {
        let scalar = encoding(&spec.dtype)?.0;
        all.insert(spec.tensor_id.as_str().to_string(), (spec, scalar));
    }
    let declarations = SafeTensorsReceiverRequest {
        schema: "cerebro.tidex.safetensors_receiver_request/v1".into(),
        model_id: source.model_id,
        architecture_id: source.architecture_id,
        architecture: ReceiverArchitecture::Unknown,
        modalities: request.modalities.clone(),
        supports_persistent_state: request.supports_persistent_state,
        checkpoint_files: vec![],
        configuration_file: source.config.path,
        tokenizer_file: source.tokenizer.path,
        supported_strategies: BTreeSet::from([
            MaterializationStrategy::DenseDelta,
            MaterializationStrategy::LowRank,
            MaterializationStrategy::SparseDelta,
        ]),
        materialization_tensors: BTreeSet::new(),
    };
    assemble_inspected(
        &declarations,
        all,
        &config,
        source.tokenizer.sha256,
        BTreeMap::from([(source.checkpoint.path, source.checkpoint.sha256.clone())]),
        source.checkpoint.sha256,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn inspects_exact_safetensors_geometry_and_rejects_ambiguous_dtype() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("tidex-safetensors-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("config.json"), b"{}").unwrap();
        fs::write(root.join("tokenizer.json"), b"{}").unwrap();
        let header = br#"{"layer.weight":{"dtype":"F32","shape":[2,2],"data_offsets":[0,16]}}"#;
        let mut model = File::create(root.join("model.safetensors")).unwrap();
        model
            .write_all(&(header.len() as u64).to_le_bytes())
            .unwrap();
        model.write_all(header).unwrap();
        model.write_all(&[0u8; 16]).unwrap();
        let request = SafeTensorsReceiverRequest {
            schema: "cerebro.tidex.safetensors_receiver_request/v1".into(),
            model_id: ModelId::parse("receiver.fixture").unwrap(),
            architecture_id: ArchitectureId::parse("transformer.fixture").unwrap(),
            architecture: ReceiverArchitecture::Transformer,
            modalities: BTreeSet::from([CapabilityModality::Text]),
            supports_persistent_state: false,
            checkpoint_files: vec!["model.safetensors".into()],
            configuration_file: "config.json".into(),
            tokenizer_file: "tokenizer.json".into(),
            supported_strategies: BTreeSet::from([MaterializationStrategy::DenseDelta]),
            materialization_tensors: BTreeSet::new(),
        };
        let inspected = inspect_safetensors_receiver(&root, &request).unwrap();
        assert_eq!(inspected.profile.parameter_dimension, 4);
        assert_eq!(inspected.layout.geometry.layout.blocks[0].shape, vec![2, 2]);
        assert_ne!(
            inspected.snapshot.model_snapshot_sha256,
            Sha256Digest::zero()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn accepts_only_repository_local_huggingface_blob_symlinks() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let repo = std::env::temp_dir().join(format!(
            "models--fixture--hf-{}-{nonce}",
            std::process::id()
        ));
        let snapshot = repo.join("snapshots/revision");
        let blobs = repo.join("blobs");
        fs::create_dir_all(&snapshot).unwrap();
        fs::create_dir_all(&blobs).unwrap();
        let header =
            br#"{"encoder.layer.0.weight":{"dtype":"F32","shape":[2,2],"data_offsets":[0,16]}}"#;
        let mut model = (header.len() as u64).to_le_bytes().to_vec();
        model.extend_from_slice(header);
        model.extend_from_slice(&[0u8; 16]);
        fs::write(blobs.join("model"), model).unwrap();
        fs::write(
            blobs.join("config"),
            br#"{"model_type":"bert","architectures":["BertModel"]}"#,
        )
        .unwrap();
        fs::write(blobs.join("tokenizer"), b"{}").unwrap();
        symlink("../../blobs/model", snapshot.join("model.safetensors")).unwrap();
        symlink("../../blobs/config", snapshot.join("config.json")).unwrap();
        symlink("../../blobs/tokenizer", snapshot.join("tokenizer.json")).unwrap();
        let request = SafeTensorsReceiverRequest {
            schema: "cerebro.tidex.safetensors_receiver_request/v1".into(),
            model_id: ModelId::parse("receiver.hf.fixture").unwrap(),
            architecture_id: ArchitectureId::parse("bert.fixture").unwrap(),
            architecture: ReceiverArchitecture::Unknown,
            modalities: BTreeSet::from([CapabilityModality::Text]),
            supports_persistent_state: false,
            checkpoint_files: vec!["model.safetensors".into()],
            configuration_file: "config.json".into(),
            tokenizer_file: "tokenizer.json".into(),
            supported_strategies: BTreeSet::from([MaterializationStrategy::DenseDelta]),
            materialization_tensors: BTreeSet::new(),
        };
        let inspected = inspect_safetensors_receiver(&snapshot, &request).unwrap();
        assert_eq!(inspected.profile.parameter_dimension, 4);
        assert_eq!(
            inspected.architecture_fingerprint.model_family,
            crate::architecture_families::ModelFamily::EncoderTransformer
        );
        let outside = repo.with_extension("outside");
        fs::write(&outside, b"{}").unwrap();
        fs::remove_file(snapshot.join("config.json")).unwrap();
        symlink(&outside, snapshot.join("config.json")).unwrap();
        assert!(inspect_safetensors_receiver(&snapshot, &request).is_err());
        fs::remove_dir_all(repo).unwrap();
        fs::remove_file(outside).unwrap();
    }
}
