//! Evidence-preserving model contracts for cross-model operations.
//!
//! Behavioral inference, internal activations, and physical intervention are
//! distinct authorities. A backend may expose one without exposing the others;
//! unsupported authorities fail closed.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Device {
    Cpu,
    Cuda(usize),
    Metal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DType {
    F32,
    F64,
    BF16,
    F16,
    I32,
    I64,
    U8,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tensor {
    pub data: Vec<f64>,
    pub shape: Vec<usize>,
    pub device: Device,
    pub dtype: DType,
    pub layer_index: usize,
}

impl Tensor {
    pub fn new(
        data: Vec<f64>,
        shape: Vec<usize>,
        device: Device,
        dtype: DType,
        layer_index: usize,
    ) -> Self {
        Self {
            data,
            shape,
            device,
            dtype,
            layer_index,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.shape.is_empty() {
            return Err("tensor_shape_empty".into());
        }
        let expected = self.shape.iter().try_fold(1usize, |acc, value| {
            acc.checked_mul(*value)
                .ok_or_else(|| "tensor_shape_overflow".to_string())
        })?;
        if expected != self.data.len() || self.data.iter().any(|value| !value.is_finite()) {
            return Err("tensor_invalid".into());
        }
        Ok(())
    }

    pub fn numel(&self) -> usize {
        self.data.len()
    }
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    pub fn reshape(&self, new_shape: Vec<usize>) -> Option<Self> {
        let new_size = new_shape
            .iter()
            .try_fold(1usize, |acc, value| acc.checked_mul(*value))?;
        (new_size == self.data.len()).then(|| Self {
            data: self.data.clone(),
            shape: new_shape,
            device: self.device,
            dtype: self.dtype,
            layer_index: self.layer_index,
        })
    }

    pub fn l2_norm(&self) -> f64 {
        self.data
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt()
    }

    pub fn normalize(&self) -> Result<Self, String> {
        self.validate()?;
        let norm = self.l2_norm();
        if norm <= f64::EPSILON {
            return Err("tensor_zero_norm".into());
        }
        Ok(Self {
            data: self.data.iter().map(|value| value / norm).collect(),
            shape: self.shape.clone(),
            device: self.device,
            dtype: self.dtype,
            layer_index: self.layer_index,
        })
    }

    pub fn dot(&self, other: &Self) -> Option<f64> {
        if self.data.len() != other.data.len() {
            return None;
        }
        Some(self.data.iter().zip(&other.data).map(|(a, b)| a * b).sum())
    }

    pub fn cosine_similarity(&self, other: &Self) -> Option<f64> {
        let dot = self.dot(other)?;
        let a = self.l2_norm();
        let b = other.l2_norm();
        if a <= f64::EPSILON || b <= f64::EPSILON {
            return None;
        }
        Some((dot / (a * b)).clamp(-1.0, 1.0))
    }
}

impl fmt::Debug for Tensor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tensor")
            .field("shape", &self.shape)
            .field("device", &self.device)
            .field("dtype", &self.dtype)
            .field("layer_index", &self.layer_index)
            .field("numel", &self.data.len())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationOutput {
    pub model: String,
    pub text: String,
    /// SHA-256 of text bytes only.
    pub response_sha256: String,
    /// Domain-separated commitment to model/runtime identity, prompt, generation
    /// policy, active interventions, response and token accounting.
    pub execution_sha256: String,
    /// Domain-separated commitment to the exact active intervention set.
    pub active_interventions_sha256: String,
    pub active_intervention_count: usize,
    pub total_duration_ns: Option<u64>,
    pub load_duration_ns: Option<u64>,
    pub prompt_eval_count: Option<u64>,
    pub eval_count: Option<u64>,
    pub done_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelAccess {
    BehavioralInference,
    InternalActivations,
    ActivationIntervention,
    DeepInstrumentation,
    SparseAutoencoderAnalysis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeepInstrumentationRequest {
    pub module_path: String,
    pub prompt: String,
    pub token_from_end: usize,
}

impl DeepInstrumentationRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.module_path.is_empty()
            || self.module_path.len() > 4096
            || self.prompt.trim().is_empty()
            || self.prompt.len() > 1_048_576
            || self.token_from_end > 65_535
            || self.module_path.split('.').any(|part| {
                part.is_empty()
                    || part.starts_with('_')
                    || !part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            })
        {
            return Err("deep_instrumentation_request_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeepInstrumentationEvidence {
    pub schema: String,
    pub model: String,
    pub runtime_metadata_sha256: String,
    pub module_path: String,
    pub token_from_end: usize,
    pub values: Vec<f64>,
    pub values_sha256: String,
    pub backend: String,
    pub backend_version: String,
    pub evidence_sha256: String,
}

impl DeepInstrumentationEvidence {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "cerebro.cross_model.deep_instrumentation/v1"
            || self.model.trim().is_empty()
            || !is_sha256(&self.runtime_metadata_sha256)
            || self.module_path.trim().is_empty()
            || self.values.is_empty()
            || self.values.len() > 1_048_576
            || self.values.iter().any(|value| !value.is_finite())
            || !is_sha256(&self.values_sha256)
            || self.backend != "nnsight"
            || self.backend_version.trim().is_empty()
            || !is_sha256(&self.evidence_sha256)
        {
            return Err("deep_instrumentation_evidence_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SparseAutoencoderRequest {
    pub module_path: String,
    pub prompt: String,
    pub token_from_end: usize,
    pub release: String,
    pub sae_id: String,
    pub top_k: usize,
}

impl SparseAutoencoderRequest {
    pub fn validate(&self) -> Result<(), String> {
        DeepInstrumentationRequest {
            module_path: self.module_path.clone(),
            prompt: self.prompt.clone(),
            token_from_end: self.token_from_end,
        }
        .validate()?;
        if self.release.trim().is_empty()
            || self.release.len() > 4096
            || self.sae_id.trim().is_empty()
            || self.sae_id.len() > 4096
            || self.top_k == 0
            || self.top_k > 4096
        {
            return Err("sparse_autoencoder_request_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SparseFeature {
    pub index: usize,
    pub activation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SparseAutoencoderEvidence {
    pub schema: String,
    pub model: String,
    pub runtime_metadata_sha256: String,
    pub module_path: String,
    pub token_from_end: usize,
    pub release: String,
    pub sae_id: String,
    pub sae_lens_version: String,
    pub input_sha256: String,
    pub feature_count: usize,
    pub active_feature_count: usize,
    pub top_features: Vec<SparseFeature>,
    pub reconstruction_relative_error: f64,
    pub evidence_sha256: String,
}

impl SparseAutoencoderEvidence {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "cerebro.cross_model.sparse_autoencoder_evidence/v1"
            || self.model.trim().is_empty()
            || !is_sha256(&self.runtime_metadata_sha256)
            || self.module_path.trim().is_empty()
            || self.release.trim().is_empty()
            || self.sae_id.trim().is_empty()
            || self.sae_lens_version.trim().is_empty()
            || !is_sha256(&self.input_sha256)
            || self.feature_count == 0
            || self.active_feature_count > self.feature_count
            || self.top_features.is_empty()
            || self.top_features.len() > self.feature_count
            || self.top_features.iter().any(|feature| {
                feature.index >= self.feature_count || !feature.activation.is_finite()
            })
            || !self.reconstruction_relative_error.is_finite()
            || self.reconstruction_relative_error < 0.0
            || !is_sha256(&self.evidence_sha256)
        {
            return Err("sparse_autoencoder_evidence_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureFamily {
    Llama,
    Mistral,
    Qwen,
    Gpt,
    Claude,
    Other(String),
}

impl ArchitectureFamily {
    pub fn from_runtime_architecture(value: &str) -> Self {
        let normalized = value.trim().to_ascii_lowercase();
        if normalized.starts_with("llama") {
            Self::Llama
        } else if normalized.starts_with("mistral") || normalized.starts_with("mixtral") {
            Self::Mistral
        } else if normalized.starts_with("qwen") {
            Self::Qwen
        } else if normalized.starts_with("gpt") {
            Self::Gpt
        } else if normalized.starts_with("claude") {
            Self::Claude
        } else {
            Self::Other(normalized)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelConfig {
    pub name: String,
    pub runtime_model: String,
    pub family: ArchitectureFamily,
    pub runtime_architecture: String,
    pub embedding_dim: usize,
    pub hidden_dim: usize,
    pub num_layers: usize,
    pub num_heads: usize,
    pub vocab_size: usize,
    pub max_sequence_length: usize,
    pub parameter_count: u64,
    pub quantization: Option<String>,
    pub runtime_metadata_sha256: String,
    pub tensor_names: Vec<String>,
}

impl ModelConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.runtime_model.trim().is_empty()
            || self.runtime_architecture.trim().is_empty()
            || self.embedding_dim == 0
            || self.hidden_dim == 0
            || self.num_layers == 0
            || self.num_heads == 0
            || self.vocab_size == 0
            || self.max_sequence_length == 0
            || self.parameter_count == 0
            || !is_sha256(&self.runtime_metadata_sha256)
            || self.tensor_names.is_empty()
            || self.tensor_names.iter().any(|name| name.trim().is_empty())
        {
            return Err("model_config_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityEvidenceKind {
    BehavioralVerified,
    InternalActivation,
    WeightDelta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityMetadata {
    pub name: String,
    pub source_model: String,
    pub source_layer: Option<usize>,
    pub domain: String,
    pub confidence: f64,
    pub evidence_kind: CapabilityEvidenceKind,
    pub evidence_sha256: String,
    pub created_at: String,
}

impl CapabilityMetadata {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.source_model.trim().is_empty()
            || self.domain.trim().is_empty()
            || !self.confidence.is_finite()
            || !(0.0..=1.0).contains(&self.confidence)
            || !is_sha256(&self.evidence_sha256)
            || self.created_at.trim().is_empty()
        {
            return Err("capability_metadata_invalid".into());
        }
        if self.evidence_kind == CapabilityEvidenceKind::BehavioralVerified
            && self.source_layer.is_some()
        {
            return Err("behavioral_evidence_cannot_claim_source_layer".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SteeringVector {
    pub vector: Tensor,
    pub metadata: CapabilityMetadata,
    pub strength: f64,
}

impl SteeringVector {
    pub fn new(
        vector: Tensor,
        metadata: CapabilityMetadata,
        strength: f64,
    ) -> Result<Self, String> {
        vector.validate()?;
        metadata.validate()?;
        if metadata.evidence_kind != CapabilityEvidenceKind::InternalActivation {
            return Err("steering_requires_internal_activation_evidence".into());
        }
        if metadata.source_layer != Some(vector.layer_index) {
            return Err("steering_layer_binding_mismatch".into());
        }
        if !strength.is_finite() || strength <= 0.0 {
            return Err("steering_strength_invalid".into());
        }
        Ok(Self {
            vector,
            metadata,
            strength,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentMethod {
    CalibratedLinearRidge,
    IdentityVerified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlignmentResult {
    pub source_vector: Tensor,
    pub target_vector: Tensor,
    pub alignment_score: f64,
    pub normalized_residual: f64,
    pub calibration_sha256: String,
    pub method: AlignmentMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActivationInterventionReceipt {
    pub schema: String,
    pub model: String,
    pub runtime_metadata_sha256: String,
    pub layer_index: usize,
    pub steering_sha256: String,
    pub strength: f64,
    pub applied_at: String,
}

impl ActivationInterventionReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "cerebro.cross_model.activation_intervention_receipt/v1"
            || self.model.trim().is_empty()
            || !is_sha256(&self.runtime_metadata_sha256)
            || !is_sha256(&self.steering_sha256)
            || !self.strength.is_finite()
            || self.strength <= 0.0
            || chrono::DateTime::parse_from_rfc3339(&self.applied_at).is_err()
        {
            return Err("activation_intervention_receipt_invalid".into());
        }
        Ok(())
    }
}

pub trait LLMModel: Send + Sync {
    fn config(&self) -> &ModelConfig;
    fn generate(
        &self,
        prompt: &str,
    ) -> Result<GenerationOutput, Box<dyn std::error::Error + Send + Sync>>;

    fn name(&self) -> &str {
        &self.config().name
    }
    fn embedding_dim(&self) -> usize {
        self.config().embedding_dim
    }
    fn num_layers(&self) -> usize {
        self.config().num_layers
    }
    fn hidden_dim(&self) -> usize {
        self.config().hidden_dim
    }
    fn num_heads(&self) -> usize {
        self.config().num_heads
    }
    fn architecture_family(&self) -> ArchitectureFamily {
        self.config().family.clone()
    }

    fn supports(&self, access: ModelAccess) -> bool {
        matches!(access, ModelAccess::BehavioralInference)
    }

    fn extract_layer_activations(
        &self,
        _layer_index: usize,
        _input: &str,
    ) -> Result<Tensor, Box<dyn std::error::Error + Send + Sync>> {
        Err(format!(
            "internal_activations_unavailable:{}",
            self.config().runtime_model
        )
        .into())
    }

    fn apply_steering(
        &self,
        _layer_index: usize,
        _steering: &Tensor,
        _strength: f64,
    ) -> Result<ActivationInterventionReceipt, Box<dyn std::error::Error + Send + Sync>> {
        Err(format!(
            "activation_intervention_unavailable:{}",
            self.config().runtime_model
        )
        .into())
    }

    fn clear_activation_interventions(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err(format!(
            "activation_intervention_clear_unavailable:{}",
            self.config().runtime_model
        )
        .into())
    }

    fn deep_instrumentation(
        &self,
        _request: &DeepInstrumentationRequest,
    ) -> Result<DeepInstrumentationEvidence, Box<dyn std::error::Error + Send + Sync>> {
        Err(format!(
            "deep_instrumentation_unavailable:{}",
            self.config().runtime_model
        )
        .into())
    }

    fn sparse_autoencoder_analysis(
        &self,
        _request: &SparseAutoencoderRequest,
    ) -> Result<SparseAutoencoderEvidence, Box<dyn std::error::Error + Send + Sync>> {
        Err(format!(
            "sparse_autoencoder_analysis_unavailable:{}",
            self.config().runtime_model
        )
        .into())
    }
}

pub(crate) fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
