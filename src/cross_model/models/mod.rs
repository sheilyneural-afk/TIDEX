//! Real behavioral model runtimes for cross-model evidence acquisition.
//!
//! The Ollama backend is used only for behavior that it actually exposes.
//! Hidden activations and model mutation remain unavailable through this
//! backend and therefore fail closed through `LLMModel` defaults.

pub mod candle_llama;
pub mod candle_mistral;
pub mod llama;
pub mod mistral;
pub mod qwen;
pub mod traits;

pub use candle_llama::CandleLlamaModel;
pub use candle_mistral::CandleMistralModel;
pub use llama::LlamaModel;
pub use mistral::MistralModel;
pub use qwen::QwenModel;
pub use traits::*;

use crate::foundation::digest::Sha256Digest;
use reqwest::blocking::Client;
use rustix::event::{poll, PollFd, PollFlags, Timespec};
use rustix::fs::{fcntl_getfl, fcntl_setfl, OFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const HF_WORKER_SOURCE: &str = include_str!("../runtime/hf_worker.py");
const HF_RUNTIME_LOCK: &str = include_str!("../runtime/hf-runtime.lock.txt");
const HF_RUNTIME_PYTHON_MINOR: &str = "3.12";

fn pep503_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_sep = false;
    for ch in name.chars() {
        if matches!(ch, '-' | '_' | '.') {
            if !prev_sep {
                out.push('-');
                prev_sep = true;
            }
        } else {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            prev_sep = false;
        }
    }
    out
}

fn hf_runtime_lock_packages() -> Result<BTreeMap<String, &'static str>, Box<dyn Error + Send + Sync>>
{
    let mut packages = BTreeMap::new();
    for raw in HF_RUNTIME_LOCK.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
            continue;
        }
        let spec = line
            .split_whitespace()
            .next()
            .ok_or("hf_runtime_lock_line_invalid")?;
        let Some((name, version)) = spec.split_once("==") else {
            return Err("hf_runtime_lock_line_invalid".into());
        };
        if version.is_empty() {
            return Err("hf_runtime_lock_version_empty".into());
        }
        let key = pep503_name(name);
        if key.is_empty() {
            return Err("hf_runtime_lock_name_invalid".into());
        }
        if packages.insert(key.clone(), version).is_some() {
            return Err(format!("hf_runtime_lock_duplicate:{key}").into());
        }
    }
    if packages.is_empty() {
        return Err("hf_runtime_lock_empty".into());
    }
    Ok(packages)
}

fn hf_runtime_lock_version(package: &str) -> Result<&'static str, Box<dyn Error + Send + Sync>> {
    let wanted = pep503_name(package);
    hf_runtime_lock_packages()?
        .get(&wanted)
        .copied()
        .ok_or_else(|| format!("hf_runtime_lock_package_missing:{package}").into())
}

fn certified_runtime_identity_sha256() -> Result<String, Box<dyn Error + Send + Sync>> {
    let packages = hf_runtime_lock_packages()?;
    let payload = serde_json::json!({
        "implementation": "cpython",
        "packages": packages,
        "python_minor": HF_RUNTIME_PYTHON_MINOR,
    });
    Ok(Sha256Digest::digest_domain(
        b"CEREBRO:TIDEX:HF-RUNTIME-IDENTITY:v1\0",
        &serde_json::to_vec(&payload)?,
    )
    .into_string())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationPolicy {
    pub temperature: f64,
    pub top_p: f64,
    pub max_tokens: usize,
    pub seed: u64,
    pub keep_alive: String,
    pub request_timeout_seconds: u64,
    pub think: bool,
}

impl Default for GenerationPolicy {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            top_p: 1.0,
            max_tokens: 256,
            seed: 0,
            keep_alive: "10m".into(),
            request_timeout_seconds: 180,
            think: false,
        }
    }
}

impl GenerationPolicy {
    fn validate(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.temperature.is_finite()
            || self.temperature < 0.0
            || !self.top_p.is_finite()
            || !(0.0..=1.0).contains(&self.top_p)
            || self.max_tokens == 0
            || self.max_tokens > 32_768
            || self.keep_alive.trim().is_empty()
            || self.request_timeout_seconds == 0
            || self.request_timeout_seconds > 3600
        {
            return Err("generation_policy_invalid".into());
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct OllamaBackend {
    endpoint: String,
    runtime_model: String,
    config: ModelConfig,
    client: Client,
    policy: GenerationPolicy,
}

impl std::fmt::Debug for OllamaBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OllamaBackend")
            .field("endpoint", &self.endpoint)
            .field("runtime_model", &self.runtime_model)
            .field("config", &self.config)
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

impl OllamaBackend {
    pub(crate) fn connect(
        endpoint: impl Into<String>,
        runtime_model: impl Into<String>,
        expected_family: Option<ArchitectureFamily>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Self::connect_with_policy(
            endpoint,
            runtime_model,
            expected_family,
            GenerationPolicy::default(),
        )
    }

    pub(crate) fn connect_with_policy(
        endpoint: impl Into<String>,
        runtime_model: impl Into<String>,
        expected_family: Option<ArchitectureFamily>,
        policy: GenerationPolicy,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        policy.validate()?;
        let endpoint = endpoint.into().trim_end_matches('/').to_string();
        let runtime_model = runtime_model.into();
        if !(endpoint.starts_with("http://") || endpoint.starts_with("https://"))
            || runtime_model.trim().is_empty()
        {
            return Err("ollama_runtime_configuration_invalid".into());
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(policy.request_timeout_seconds))
            .pool_idle_timeout(Duration::from_secs(90))
            .build()?;
        let show =
            Self::post_json_with(&client, &endpoint, "api/show", &json!({"model": runtime_model}))?;
        let config = Self::config_from_show(&runtime_model, &show)?;
        if let Some(expected) = expected_family {
            if config.family != expected {
                return Err(format!(
                    "model_architecture_mismatch:model={}:expected={expected:?}:observed={:?}",
                    runtime_model, config.family
                )
                .into());
            }
        }
        Ok(Self {
            endpoint,
            runtime_model,
            config,
            client,
            policy,
        })
    }

    pub(crate) fn config(&self) -> &ModelConfig {
        &self.config
    }

    pub(crate) fn generate(
        &self,
        prompt: &str,
    ) -> Result<GenerationOutput, Box<dyn Error + Send + Sync>> {
        if prompt.trim().is_empty() || prompt.len() > 1_048_576 {
            return Err("generation_prompt_invalid".into());
        }
        let payload = json!({
            "model": self.runtime_model,
            "prompt": prompt,
            "stream": false,
            "think": self.policy.think,
            "keep_alive": self.policy.keep_alive,
            "options": {
                "temperature": self.policy.temperature,
                "top_p": self.policy.top_p,
                "num_predict": self.policy.max_tokens,
                "seed": self.policy.seed
            }
        });
        let value = Self::post_json_with(&self.client, &self.endpoint, "api/generate", &payload)?;
        if let Some(error) = value.get("error").and_then(Value::as_str) {
            return Err(format!("ollama_generate_error:{error}").into());
        }
        let returned_model = value
            .get("model")
            .and_then(Value::as_str)
            .ok_or("ollama_response_model_missing")?;
        if returned_model != self.runtime_model {
            return Err("ollama_response_model_mismatch".into());
        }
        let text = value
            .get("response")
            .and_then(Value::as_str)
            .ok_or("ollama_response_missing")?
            .to_string();
        let response_sha256 = sha256_hex(text.as_bytes());
        let active_interventions: Vec<serde_json::Value> = Vec::new();
        let active_interventions_sha256 = active_interventions_sha256(&active_interventions)?;
        let prompt_eval_count = value.get("prompt_eval_count").and_then(Value::as_u64);
        let eval_count = value.get("eval_count").and_then(Value::as_u64);
        let done_reason = value
            .get("done_reason")
            .and_then(Value::as_str)
            .map(str::to_string);
        let execution_sha256 = generation_execution_sha256(GenerationExecutionEvidence {
            config: &self.config,
            prompt,
            policy: &self.policy,
            active_interventions_sha256: &active_interventions_sha256,
            active_intervention_count: 0,
            response_sha256: &response_sha256,
            prompt_eval_count,
            eval_count,
            done_reason: done_reason.as_deref(),
        })?;
        Ok(GenerationOutput {
            model: returned_model.to_string(),
            text,
            response_sha256,
            execution_sha256,
            active_interventions_sha256,
            active_intervention_count: 0,
            total_duration_ns: value.get("total_duration").and_then(Value::as_u64),
            load_duration_ns: value.get("load_duration").and_then(Value::as_u64),
            prompt_eval_count,
            eval_count,
            done_reason,
        })
    }

    fn post_json_with(
        client: &Client,
        endpoint: &str,
        path: &str,
        payload: &Value,
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let url = format!("{endpoint}/{path}");
        let response = client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(payload)
            .send()?;
        let status = response.status();
        let value: Value = response.json()?;
        if !status.is_success() {
            let detail = value.get("error").and_then(Value::as_str);
            return Err(match detail {
                Some(detail) => format!("ollama_http_error:{status}:{detail}"),
                None => format!("ollama_http_error:{status}:no_error_detail"),
            }
            .into());
        }
        Ok(value)
    }

    fn config_from_show(
        runtime_model: &str,
        show: &Value,
    ) -> Result<ModelConfig, Box<dyn Error + Send + Sync>> {
        let info = show
            .get("model_info")
            .and_then(Value::as_object)
            .ok_or("ollama_model_info_missing")?;
        let architecture = info
            .get("general.architecture")
            .and_then(Value::as_str)
            .ok_or("ollama_architecture_missing")?
            .to_ascii_lowercase();
        let family = ArchitectureFamily::from_runtime_architecture(&architecture);
        let key = |suffix: &str| format!("{architecture}.{suffix}");
        let read_usize = |suffix: &str| -> Result<usize, Box<dyn Error + Send + Sync>> {
            let full = key(suffix);
            info.get(&full)
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .ok_or_else(|| format!("ollama_model_metadata_missing:{full}").into())
        };
        let embedding_dim = read_usize("embedding_length")?;
        let hidden_dim = read_usize("feed_forward_length")?;
        let num_layers = read_usize("block_count")?;
        let num_heads = read_usize("attention.head_count")?;
        let max_sequence_length = read_usize("context_length")?;
        let parameter_count = info
            .get("general.parameter_count")
            .and_then(Value::as_u64)
            .ok_or("ollama_parameter_count_missing")?;
        let tensors = show
            .get("tensors")
            .and_then(Value::as_array)
            .ok_or("ollama_tensor_manifest_missing")?;
        let tensor_names = tensors
            .iter()
            .map(|tensor| {
                tensor
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .ok_or("ollama_tensor_name_missing")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let vocab_size = tensors
            .iter()
            .find_map(|tensor| {
                let name = tensor.get("name")?.as_str()?;
                if name != "output.weight" && name != "token_embd.weight" {
                    return None;
                }
                tensor
                    .get("shape")?
                    .as_array()?
                    .iter()
                    .filter_map(Value::as_u64)
                    .filter_map(|value| usize::try_from(value).ok())
                    .filter(|value| *value > embedding_dim)
                    .max()
            })
            .ok_or("ollama_vocab_size_missing")?;
        let quantization = show
            .pointer("/details/quantization_level")
            .and_then(Value::as_str)
            .map(str::to_string);
        let config = ModelConfig {
            name: runtime_model.to_string(),
            runtime_model: runtime_model.to_string(),
            family,
            runtime_architecture: architecture,
            embedding_dim,
            hidden_dim,
            num_layers,
            num_heads,
            vocab_size,
            max_sequence_length,
            parameter_count,
            quantization,
            runtime_metadata_sha256: sha256_hex(&serde_json::to_vec(show)?),
            tensor_names,
        };
        config.validate()?;
        Ok(config)
    }
}

#[derive(Debug, Clone)]
pub struct OllamaModel {
    backend: OllamaBackend,
}

impl OllamaModel {
    pub fn connect(
        endpoint: impl Into<String>,
        runtime_model: impl Into<String>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            backend: OllamaBackend::connect(endpoint, runtime_model, None)?,
        })
    }

    pub fn connect_with_policy(
        endpoint: impl Into<String>,
        runtime_model: impl Into<String>,
        policy: GenerationPolicy,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            backend: OllamaBackend::connect_with_policy(endpoint, runtime_model, None, policy)?,
        })
    }
}

impl LLMModel for OllamaModel {
    fn config(&self) -> &ModelConfig {
        self.backend.config()
    }

    fn generate(&self, prompt: &str) -> Result<GenerationOutput, Box<dyn Error + Send + Sync>> {
        self.backend.generate(prompt)
    }
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub(crate) fn active_interventions_sha256<T: serde::Serialize>(
    interventions: &T,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let payload = serde_json::to_vec(interventions)?;
    let mut framed = b"CEREBRO:CROSS-MODEL:ACTIVE-INTERVENTIONS:v1\0".to_vec();
    framed.extend_from_slice(&payload);
    Ok(sha256_hex(&framed))
}

pub(crate) struct GenerationExecutionEvidence<'a> {
    pub config: &'a ModelConfig,
    pub prompt: &'a str,
    pub policy: &'a GenerationPolicy,
    pub active_interventions_sha256: &'a str,
    pub active_intervention_count: usize,
    pub response_sha256: &'a str,
    pub prompt_eval_count: Option<u64>,
    pub eval_count: Option<u64>,
    pub done_reason: Option<&'a str>,
}

pub(crate) fn generation_execution_sha256(
    evidence: GenerationExecutionEvidence<'_>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    if !is_sha256(evidence.active_interventions_sha256)
        || !is_sha256(evidence.response_sha256)
        || evidence.prompt.trim().is_empty()
    {
        return Err("generation_execution_evidence_invalid".into());
    }
    let commitment = serde_json::json!({
        "schema": "cerebro.cross_model.generation_execution/v1",
        "model": evidence.config.name,
        "runtime_model": evidence.config.runtime_model,
        "runtime_metadata_sha256": evidence.config.runtime_metadata_sha256,
        "prompt_sha256": sha256_hex(evidence.prompt.as_bytes()),
        "generation_policy": evidence.policy,
        "active_interventions_sha256": evidence.active_interventions_sha256,
        "active_intervention_count": evidence.active_intervention_count,
        "response_sha256": evidence.response_sha256,
        "prompt_eval_count": evidence.prompt_eval_count,
        "eval_count": evidence.eval_count,
        "done_reason": evidence.done_reason,
    });
    let payload = serde_json::to_vec(&commitment)?;
    let mut framed = b"CEREBRO:CROSS-MODEL:GENERATION-EXECUTION:v1\0".to_vec();
    framed.extend_from_slice(&payload);
    Ok(sha256_hex(&framed))
}

pub(crate) struct LocalSafetensorsArtifacts {
    pub checkpoint_bytes: Vec<u8>,
    pub config_bytes: Vec<u8>,
    pub tokenizer_bytes: Vec<u8>,
    pub inventory: crate::receiver::weight_actuator::ModelParameterInventory,
}

pub(crate) fn load_local_safetensors_artifacts(
    checkpoint_path: &Path,
    config_path: &Path,
    tokenizer_path: &Path,
) -> Result<LocalSafetensorsArtifacts, Box<dyn Error + Send + Sync>> {
    const MAX_CHECKPOINT_BYTES: u64 = 64 * 1024 * 1024 * 1024;
    const MAX_CONFIG_BYTES: u64 = 16 * 1024 * 1024;
    const MAX_TOKENIZER_BYTES: u64 = 256 * 1024 * 1024;

    fn read_regular_bounded(
        path: &Path,
        maximum: u64,
        label: &str,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        if !path.is_absolute() {
            return Err(format!("{label}_path_must_be_absolute").into());
        }
        let canonical = std::fs::canonicalize(path)?;
        let metadata = std::fs::metadata(&canonical)?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > maximum {
            return Err(format!("{label}_file_invalid").into());
        }
        let bytes = std::fs::read(&canonical)?;
        if u64::try_from(bytes.len()).ok() != Some(metadata.len()) {
            return Err(format!("{label}_size_changed_during_read").into());
        }
        Ok(bytes)
    }

    let inventory = crate::receiver::weight_actuator::inspect_model_safetensors(checkpoint_path)?;
    let checkpoint_bytes =
        read_regular_bounded(checkpoint_path, MAX_CHECKPOINT_BYTES, "checkpoint")?;
    let checkpoint_sha256 = sha256_hex(&checkpoint_bytes);
    if checkpoint_sha256.as_str() != inventory.model_sha256.as_ref() {
        return Err("checkpoint_bytes_changed_after_inventory".into());
    }
    let config_bytes = read_regular_bounded(config_path, MAX_CONFIG_BYTES, "model_config")?;
    let tokenizer_bytes = read_regular_bounded(tokenizer_path, MAX_TOKENIZER_BYTES, "tokenizer")?;
    Ok(LocalSafetensorsArtifacts {
        checkpoint_bytes,
        config_bytes,
        tokenizer_bytes,
        inventory,
    })
}

pub(crate) struct LocalModelGeometry {
    pub family: ArchitectureFamily,
    pub embedding_dim: usize,
    pub hidden_dim: usize,
    pub num_layers: usize,
    pub num_heads: usize,
    pub vocab_size: usize,
    pub max_sequence_length: usize,
}

pub(crate) fn local_model_config(
    name: &str,
    runtime_architecture: &str,
    geometry: LocalModelGeometry,
    artifacts: &LocalSafetensorsArtifacts,
) -> Result<ModelConfig, Box<dyn Error + Send + Sync>> {
    if name.trim().is_empty() {
        return Err("local_model_name_invalid".into());
    }
    let tensor_names = artifacts
        .inventory
        .tensors
        .iter()
        .map(|tensor| tensor.tensor_id.to_string())
        .collect::<Vec<_>>();
    let runtime_metadata_sha256 = sha256_hex(&serde_json::to_vec(&(
        artifacts.inventory.model_sha256.to_string(),
        sha256_hex(&artifacts.config_bytes),
        sha256_hex(&artifacts.tokenizer_bytes),
        runtime_architecture,
    ))?);
    let config = ModelConfig {
        name: name.to_string(),
        runtime_model: artifacts.inventory.model_sha256.to_string(),
        family: geometry.family,
        runtime_architecture: runtime_architecture.to_string(),
        embedding_dim: geometry.embedding_dim,
        hidden_dim: geometry.hidden_dim,
        num_layers: geometry.num_layers,
        num_heads: geometry.num_heads,
        vocab_size: geometry.vocab_size,
        max_sequence_length: geometry.max_sequence_length,
        parameter_count: artifacts.inventory.total_parameter_count,
        quantization: None,
        runtime_metadata_sha256,
        tensor_names,
    };
    config.validate()?;
    Ok(config)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HfTransformersRuntimeConfig {
    pub name: String,
    pub python_executable: PathBuf,
    pub model_dir: PathBuf,
    pub checkpoint_path: PathBuf,
    pub config_path: PathBuf,
    pub tokenizer_path: PathBuf,
    pub threads: usize,
    pub generation: GenerationPolicy,
    pub require_nnsight: bool,
    pub require_sae_lens: bool,
}

impl HfTransformersRuntimeConfig {
    pub fn validate(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.generation.validate()?;
        if self.name.trim().is_empty()
            || !(1..=256).contains(&self.threads)
            || [
                &self.python_executable,
                &self.model_dir,
                &self.checkpoint_path,
                &self.config_path,
                &self.tokenizer_path,
            ]
            .iter()
            .any(|path| !path.is_absolute())
        {
            return Err("hf_runtime_config_invalid".into());
        }
        let python = std::fs::canonicalize(&self.python_executable)?;
        let model_dir = std::fs::canonicalize(&self.model_dir)?;
        if !python.is_file() || !model_dir.is_dir() {
            return Err("hf_runtime_authority_file_invalid".into());
        }
        for (path, name) in [
            (&self.checkpoint_path, "model.safetensors"),
            (&self.config_path, "config.json"),
            (&self.tokenizer_path, "tokenizer.json"),
        ] {
            let expected = self.model_dir.join(name);
            if !expected.exists() || !path.exists() || !expected.same_file(path)? {
                return Err(format!("hf_runtime_snapshot_binding_invalid:{name}").into());
            }
        }
        Ok(())
    }
}

trait SameFile {
    fn same_file(&self, other: &Path) -> std::io::Result<bool>;
}

impl SameFile for PathBuf {
    fn same_file(&self, other: &Path) -> std::io::Result<bool> {
        use std::os::unix::fs::MetadataExt;
        let left = std::fs::metadata(self)?;
        let right = std::fs::metadata(other)?;
        Ok(left.dev() == right.dev() && left.ino() == right.ino())
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HfWorkerHello {
    schema: String,
    model_type: String,
    architectures: Vec<String>,
    hidden_size: usize,
    intermediate_size: usize,
    num_hidden_layers: usize,
    num_attention_heads: usize,
    vocab_size: usize,
    max_position_embeddings: usize,
    parameter_count: u64,
    checkpoint_sha256: String,
    config_sha256: String,
    tokenizer_sha256: String,
    load_duration_ns: u64,
    activation_semantics: String,
    steering_semantics: String,
    runtime_identity_sha256: String,
    manifest_sha256: String,
}

impl HfWorkerHello {
    fn validate(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let expected_identity = certified_runtime_identity_sha256()?;
        if self.schema != "cerebro.tidex.hf_worker_hello/v1"
            || self.model_type.trim().is_empty()
            || self.hidden_size == 0
            || self.intermediate_size == 0
            || self.num_hidden_layers == 0
            || self.num_attention_heads == 0
            || self.vocab_size == 0
            || self.max_position_embeddings == 0
            || self.parameter_count == 0
            || !is_sha256(&self.checkpoint_sha256)
            || !is_sha256(&self.config_sha256)
            || !is_sha256(&self.tokenizer_sha256)
            || !is_sha256(&self.manifest_sha256)
            || !is_sha256(&self.runtime_identity_sha256)
            || self.activation_semantics != "decoder_layer_output_last_prompt_token/v1"
            || self.steering_semantics != "decoder_layer_output_additive_broadcast/v1"
        {
            return Err("hf_worker_hello_invalid".into());
        }
        if self.runtime_identity_sha256 != expected_identity {
            return Err(format!(
                "hf_worker_runtime_identity_mismatch:expected={expected_identity}:got={}",
                self.runtime_identity_sha256
            )
            .into());
        }
        if self
            .architectures
            .iter()
            .any(|value| value.trim().is_empty())
        {
            return Err("hf_worker_architecture_list_invalid".into());
        }
        let mut value = serde_json::to_value(self)?;
        let object = value.as_object_mut().ok_or("hf_worker_hello_not_object")?;
        object.remove("manifest_sha256");
        if sha256_hex(&serde_json::to_vec(&value)?) != self.manifest_sha256 {
            return Err("hf_worker_hello_manifest_mismatch".into());
        }
        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HfWorkerResponse {
    schema: String,
    request_id: String,
    operation: String,
    ok: bool,
    #[serde(default)]
    payload: Option<Value>,
    #[serde(default)]
    error: Option<String>,
    response_sha256: String,
}

const MAX_HF_PROTOCOL_LINE: usize = 32 * 1024 * 1024;
const MAX_HF_STDERR_BYTES: usize = 256 * 1024;

fn drain_hf_stderr(stderr: &mut ChildStderr, retained: &mut Vec<u8>) -> std::io::Result<()> {
    let mut buffer = [0_u8; 8192];
    loop {
        match stderr.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(count) => {
                let remaining = MAX_HF_STDERR_BYTES.saturating_sub(retained.len());
                retained.extend_from_slice(&buffer[..count.min(remaining)]);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

fn duration_timespec(duration: Duration) -> Timespec {
    Timespec {
        tv_sec: duration.as_secs() as i64,
        tv_nsec: i64::from(duration.subsec_nanos()),
    }
}

fn read_hf_protocol_line(
    stdout: &mut BufReader<ChildStdout>,
    stderr: &mut ChildStderr,
    retained_stderr: &mut Vec<u8>,
    timeout: Duration,
) -> Result<Option<Vec<u8>>, Box<dyn Error + Send + Sync>> {
    let deadline = Instant::now() + timeout;
    let mut line = Vec::with_capacity(4096);
    loop {
        drain_hf_stderr(stderr, retained_stderr)?;
        match stdout.fill_buf() {
            Ok([]) => {
                return if line.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(line))
                };
            }
            Ok(buffer) => {
                let newline = buffer.iter().position(|byte| *byte == b'\n');
                let consume = newline.map_or(buffer.len(), |index| index + 1);
                let next_len = line
                    .len()
                    .checked_add(consume)
                    .ok_or("hf_worker_protocol_line_length_overflow")?;
                if next_len > MAX_HF_PROTOCOL_LINE {
                    return Err("hf_worker_protocol_line_too_large".into());
                }
                line.extend_from_slice(&buffer[..consume]);
                stdout.consume(consume);
                if newline.is_some() {
                    return Ok(Some(line));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }

        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("hf_worker_timeout")?;
        let mut fds = [
            PollFd::new(stdout.get_ref(), PollFlags::IN | PollFlags::HUP | PollFlags::ERR),
            PollFd::new(stderr, PollFlags::IN | PollFlags::HUP | PollFlags::ERR),
        ];
        if poll(&mut fds, Some(&duration_timespec(remaining)))? == 0 {
            return Err("hf_worker_timeout".into());
        }
        if fds.iter().any(|fd| fd.revents().contains(PollFlags::NVAL)) {
            return Err("hf_worker_poll_invalid_fd".into());
        }
    }
}

fn sha256_open_file(file: &mut File) -> Result<String, Box<dyn Error + Send + Sync>> {
    file.seek(SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(format!("{:x}", hasher.finalize()))
}

struct HfWorker {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: ChildStderr,
    stderr_bytes: Vec<u8>,
    request_timeout: Duration,
    next_request: u64,
}

impl HfWorker {
    fn terminate(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn stderr_detail(&self) -> String {
        String::from_utf8_lossy(&self.stderr_bytes)
            .trim()
            .to_string()
    }

    fn receive_line(&mut self, context: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        match read_hf_protocol_line(
            &mut self.stdout,
            &mut self.stderr,
            &mut self.stderr_bytes,
            self.request_timeout,
        ) {
            Ok(Some(line)) => Ok(line),
            Ok(None) => {
                let status = self.child.try_wait()?;
                Err(format!(
                    "hf_worker_closed_pipe:{context}:{status:?}:stderr={}",
                    self.stderr_detail()
                )
                .into())
            }
            Err(error) => {
                self.terminate();
                Err(format!(
                    "hf_worker_protocol_failure:{context}:{error}:stderr={}",
                    self.stderr_detail()
                )
                .into())
            }
        }
    }

    fn transact(
        &mut self,
        operation: &str,
        payload: Value,
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
        self.next_request = self
            .next_request
            .checked_add(1)
            .ok_or("hf_worker_request_counter_overflow")?;
        let request_id = format!("rust-{:016x}", self.next_request);
        let request = json!({
            "schema": "cerebro.cross_model.hf_worker_request/v1",
            "request_id": request_id,
            "operation": operation,
            "payload": payload,
        });
        let bytes = serde_json::to_vec(&request)?;
        if bytes.len() > MAX_HF_PROTOCOL_LINE {
            return Err("hf_worker_request_too_large".into());
        }
        if let Err(error) = self
            .stdin
            .write_all(&bytes)
            .and_then(|()| self.stdin.write_all(b"\n"))
            .and_then(|()| self.stdin.flush())
        {
            self.terminate();
            return Err(format!("hf_worker_request_write_failed:{error}").into());
        }

        let line = self.receive_line(operation)?;
        let response: HfWorkerResponse = serde_json::from_slice(&line)?;
        if response.schema != "cerebro.cross_model.hf_worker_response/v1"
            || response.request_id != request_id
            || response.operation != operation
            || !is_sha256(&response.response_sha256)
        {
            self.terminate();
            return Err("hf_worker_response_binding_invalid".into());
        }
        let commitment = if response.ok {
            let payload = response
                .payload
                .as_ref()
                .ok_or("hf_worker_success_payload_missing")?;
            if response.error.is_some() {
                return Err("hf_worker_success_contains_error".into());
            }
            json!({
                "schema": response.schema,
                "request_id": response.request_id,
                "operation": response.operation,
                "ok": true,
                "payload": payload,
            })
        } else {
            let error = response
                .error
                .as_ref()
                .ok_or("hf_worker_failure_reason_missing")?;
            if response.payload.is_some() {
                return Err("hf_worker_failure_contains_payload".into());
            }
            json!({
                "schema": response.schema,
                "request_id": response.request_id,
                "operation": response.operation,
                "ok": false,
                "error": error,
            })
        };
        if sha256_hex(&serde_json::to_vec(&commitment)?) != response.response_sha256 {
            return Err("hf_worker_response_sha256_mismatch".into());
        }
        if response.ok {
            response
                .payload
                .ok_or_else(|| "hf_worker_success_payload_missing".into())
        } else {
            let error = response.error.ok_or("hf_worker_failure_reason_missing")?;
            Err(format!("hf_worker_operation_failed:{operation}:{error}").into())
        }
    }
}

impl Drop for HfWorker {
    fn drop(&mut self) {
        self.terminate();
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HfGeneratePayload {
    text: String,
    prompt_eval_count: u64,
    eval_count: u64,
    total_duration_ns: u64,
    done_reason: String,
    active_steering: Vec<HfActiveSteering>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HfActiveSteering {
    layer_index: usize,
    steering_sha256: String,
    strength: f64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HfActivationPayload {
    layer_index: usize,
    values: Vec<f64>,
    vector_sha256: String,
    semantics: String,
    total_duration_ns: u64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HfSteeringPayload {
    layer_index: usize,
    steering_sha256: String,
    strength: f64,
    active_steering_count: usize,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HfDeepInstrumentationPayload {
    module_path: String,
    token_from_end: usize,
    values: Vec<f64>,
    values_sha256: String,
    backend: String,
    backend_version: String,
    total_duration_ns: u64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HfSparseFeaturePayload {
    index: usize,
    activation: f64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HfSparseAutoencoderPayload {
    module_path: String,
    token_from_end: usize,
    sae_weights_sha256: String,
    sae_config_sha256: String,
    d_in: usize,
    d_sae: usize,
    input_sha256: String,
    feature_count: usize,
    active_feature_count: usize,
    top_features: Vec<HfSparseFeaturePayload>,
    reconstruction_relative_error: f64,
    total_duration_ns: u64,
}

pub struct HfTransformersModel {
    config: ModelConfig,
    worker: Mutex<HfWorker>,
    policy: GenerationPolicy,
    worker_load_duration_ns: u64,
    nnsight_version: String,
    runtime_identity_sha256: String,
}

impl std::fmt::Debug for HfTransformersModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HfTransformersModel")
            .field("config", &self.config)
            .field("policy", &self.policy)
            .field("worker_load_duration_ns", &self.worker_load_duration_ns)
            .finish_non_exhaustive()
    }
}

impl HfTransformersModel {
    pub fn spawn(
        runtime: HfTransformersRuntimeConfig,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        runtime.validate()?;
        let inventory =
            crate::receiver::weight_actuator::inspect_model_safetensors(&runtime.checkpoint_path)?;
        let config_sha256 = crate::foundation::digest::sha256_file(&runtime.config_path)?;
        let tokenizer_sha256 = crate::foundation::digest::sha256_file(&runtime.tokenizer_path)?;
        let worker_sha256 = sha256_hex(HF_WORKER_SOURCE.as_bytes());
        let python_path = std::fs::canonicalize(&runtime.python_executable)?;
        let mut python_file = File::open(&python_path)?;
        if !python_file.metadata()?.is_file() {
            return Err("hf_python_executable_not_regular".into());
        }
        let python_sha256 = sha256_open_file(&mut python_file)?;
        let python_fd_path = format!("/proc/self/fd/{}", python_file.as_raw_fd());
        let request_timeout = Duration::from_secs(runtime.generation.request_timeout_seconds);

        let mut child = Command::new(&python_fd_path)
            .arg0(&runtime.python_executable)
            .arg("-c")
            .arg(HF_WORKER_SOURCE)
            .arg("--model-dir")
            .arg(&runtime.model_dir)
            .arg("--checkpoint")
            .arg(&runtime.checkpoint_path)
            .arg("--config")
            .arg(&runtime.config_path)
            .arg("--tokenizer")
            .arg(&runtime.tokenizer_path)
            .arg("--threads")
            .arg(runtime.threads.to_string())
            .env_clear()
            .env("HF_HUB_OFFLINE", "1")
            .env("PYTHONHASHSEED", "0")
            .env("PYTHONNOUSERSITE", "1")
            .env("TOKENIZERS_PARALLELISM", "false")
            .env("TRANSFORMERS_NO_TF", "1")
            .env("USE_TF", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        drop(python_file);
        let stdin = child.stdin.take().ok_or("hf_worker_stdin_missing")?;
        let stdout = child.stdout.take().ok_or("hf_worker_stdout_missing")?;
        let stderr = child.stderr.take().ok_or("hf_worker_stderr_missing")?;
        fcntl_setfl(&stdout, fcntl_getfl(&stdout)? | OFlags::NONBLOCK)?;
        fcntl_setfl(&stderr, fcntl_getfl(&stderr)? | OFlags::NONBLOCK)?;
        let mut worker = HfWorker {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            stderr,
            stderr_bytes: Vec::new(),
            request_timeout,
            next_request: 0,
        };
        let hello_line = worker.receive_line("hello")?;
        if hello_line.len() > 8 * 1024 * 1024 {
            return Err("hf_worker_hello_oversize".into());
        }
        let hello: HfWorkerHello = serde_json::from_slice(&hello_line)?;
        hello.validate()?;
        if hello.checkpoint_sha256.as_str() != inventory.model_sha256.as_ref()
            || hello.config_sha256.as_str() != config_sha256.as_ref()
            || hello.tokenizer_sha256.as_str() != tokenizer_sha256.as_ref()
            || hello.parameter_count != inventory.total_parameter_count
        {
            worker.terminate();
            return Err("hf_worker_model_identity_mismatch".into());
        }
        let family = ArchitectureFamily::from_runtime_architecture(&hello.model_type);
        let nnsight_version = hf_runtime_lock_version("nnsight")?.to_string();
        let runtime_identity_sha256 = hello.runtime_identity_sha256.clone();
        let tensor_names = inventory
            .tensors
            .iter()
            .map(|tensor| tensor.tensor_id.to_string())
            .collect::<Vec<_>>();
        let lock_sha256 = sha256_hex(HF_RUNTIME_LOCK.as_bytes());
        let runtime_metadata_sha256 = sha256_hex(&serde_json::to_vec(&(
            hello.manifest_sha256.as_str(),
            inventory.model_sha256.as_ref(),
            config_sha256.as_ref(),
            tokenizer_sha256.as_ref(),
            worker_sha256.as_str(),
            python_sha256.as_str(),
            lock_sha256.as_str(),
        ))?);
        let config = ModelConfig {
            name: runtime.name,
            runtime_model: inventory.model_sha256.to_string(),
            family,
            runtime_architecture: hello.model_type,
            embedding_dim: hello.hidden_size,
            hidden_dim: hello.intermediate_size,
            num_layers: hello.num_hidden_layers,
            num_heads: hello.num_attention_heads,
            vocab_size: hello.vocab_size,
            max_sequence_length: hello.max_position_embeddings,
            parameter_count: hello.parameter_count,
            quantization: None,
            runtime_metadata_sha256,
            tensor_names,
        };
        config.validate()?;
        Ok(Self {
            config,
            worker: Mutex::new(worker),
            policy: runtime.generation,
            worker_load_duration_ns: hello.load_duration_ns,
            nnsight_version,
            runtime_identity_sha256,
        })
    }

    pub fn nnsight_version(&self) -> &str {
        &self.nnsight_version
    }

    pub fn runtime_identity_sha256(&self) -> &str {
        &self.runtime_identity_sha256
    }

    pub fn worker_load_duration_ns(&self) -> u64 {
        self.worker_load_duration_ns
    }
}

impl LLMModel for HfTransformersModel {
    fn config(&self) -> &ModelConfig {
        &self.config
    }

    fn supports(&self, access: ModelAccess) -> bool {
        match access {
            ModelAccess::BehavioralInference
            | ModelAccess::InternalActivations
            | ModelAccess::ActivationIntervention => true,
            ModelAccess::DeepInstrumentation => true,
            ModelAccess::SparseAutoencoderAnalysis => true,
        }
    }

    fn generate(&self, prompt: &str) -> Result<GenerationOutput, Box<dyn Error + Send + Sync>> {
        if prompt.trim().is_empty() || prompt.len() > 1_048_576 {
            return Err("generation_prompt_invalid".into());
        }
        let value = self
            .worker
            .lock()
            .map_err(|_| "hf_worker_lock_poisoned")?
            .transact(
                "generate",
                json!({
                    "prompt": prompt,
                    "max_new_tokens": self.policy.max_tokens,
                    "seed": self.policy.seed,
                    "temperature": self.policy.temperature,
                    "top_p": self.policy.top_p,
                }),
            )?;
        let payload: HfGeneratePayload = serde_json::from_value(value)?;
        if payload.done_reason != "eos" && payload.done_reason != "length" {
            return Err("hf_generation_done_reason_invalid".into());
        }
        for active in &payload.active_steering {
            if active.layer_index >= self.config.num_layers
                || !is_sha256(&active.steering_sha256)
                || !active.strength.is_finite()
                || active.strength <= 0.0
            {
                return Err("hf_generation_active_steering_invalid".into());
            }
        }
        let active_intervention_count = payload.active_steering.len();
        let active_interventions_sha256 = active_interventions_sha256(&payload.active_steering)?;
        let text = payload.text;
        let response_sha256 = sha256_hex(text.as_bytes());
        let prompt_eval_count = Some(payload.prompt_eval_count);
        let eval_count = Some(payload.eval_count);
        let done_reason = Some(payload.done_reason);
        let execution_sha256 = generation_execution_sha256(GenerationExecutionEvidence {
            config: &self.config,
            prompt,
            policy: &self.policy,
            active_interventions_sha256: &active_interventions_sha256,
            active_intervention_count,
            response_sha256: &response_sha256,
            prompt_eval_count,
            eval_count,
            done_reason: done_reason.as_deref(),
        })?;
        Ok(GenerationOutput {
            model: self.config.name.clone(),
            response_sha256,
            execution_sha256,
            active_interventions_sha256,
            active_intervention_count,
            text,
            total_duration_ns: Some(payload.total_duration_ns),
            load_duration_ns: Some(self.worker_load_duration_ns),
            prompt_eval_count,
            eval_count,
            done_reason,
        })
    }

    fn extract_layer_activations(
        &self,
        layer_index: usize,
        input: &str,
    ) -> Result<Tensor, Box<dyn Error + Send + Sync>> {
        if layer_index >= self.config.num_layers || input.trim().is_empty() {
            return Err("hf_activation_request_invalid".into());
        }
        let value = self
            .worker
            .lock()
            .map_err(|_| "hf_worker_lock_poisoned")?
            .transact("activation", json!({"prompt": input, "layer_index": layer_index}))?;
        let payload: HfActivationPayload = serde_json::from_value(value)?;
        if payload.layer_index != layer_index
            || payload.semantics != "decoder_layer_output_last_prompt_token/v1"
            || payload.values.len() != self.config.embedding_dim
            || payload.values.iter().any(|value| !value.is_finite())
            || payload.vector_sha256 != f64_vector_sha256(&payload.values)?
            || payload.total_duration_ns == 0
        {
            return Err("hf_activation_evidence_invalid".into());
        }
        let tensor = Tensor::new(
            payload.values,
            vec![self.config.embedding_dim],
            Device::Cpu,
            DType::F64,
            layer_index,
        );
        tensor.validate()?;
        Ok(tensor)
    }

    fn clear_activation_interventions(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let payload = self
            .worker
            .lock()
            .map_err(|_| "hf_worker_lock_poisoned")?
            .transact("clear_steering", json!({}))?;
        let count = payload
            .get("active_steering_count")
            .and_then(Value::as_u64)
            .ok_or("hf_clear_steering_count_missing")?;
        if count != 0 {
            return Err("hf_clear_steering_failed".into());
        }
        Ok(())
    }

    fn apply_steering(
        &self,
        layer_index: usize,
        steering: &Tensor,
        strength: f64,
    ) -> Result<ActivationInterventionReceipt, Box<dyn Error + Send + Sync>> {
        steering.validate()?;
        if layer_index >= self.config.num_layers
            || steering.layer_index != layer_index
            || steering.data.len() != self.config.embedding_dim
            || !strength.is_finite()
            || strength <= 0.0
        {
            return Err("hf_activation_intervention_request_invalid".into());
        }
        let steering_sha256 = f64_vector_sha256(&steering.data)?;
        let value = self
            .worker
            .lock()
            .map_err(|_| "hf_worker_lock_poisoned")?
            .transact(
                "set_steering",
                json!({
                    "layer_index": layer_index,
                    "values": steering.data,
                    "strength": strength,
                }),
            )?;
        let payload: HfSteeringPayload = serde_json::from_value(value)?;
        if payload.layer_index != layer_index
            || payload.steering_sha256 != steering_sha256
            || payload.strength.to_bits() != strength.to_bits()
            || payload.active_steering_count == 0
        {
            return Err("hf_activation_intervention_receipt_mismatch".into());
        }
        let receipt = ActivationInterventionReceipt {
            schema: "cerebro.cross_model.activation_intervention_receipt/v1".into(),
            model: self.config.name.clone(),
            runtime_metadata_sha256: self.config.runtime_metadata_sha256.clone(),
            layer_index,
            steering_sha256,
            strength,
            applied_at: chrono::Utc::now().to_rfc3339(),
        };
        receipt.validate()?;
        Ok(receipt)
    }

    fn deep_instrumentation(
        &self,
        request: &DeepInstrumentationRequest,
    ) -> Result<DeepInstrumentationEvidence, Box<dyn Error + Send + Sync>> {
        request.validate()?;
        let value = self
            .worker
            .lock()
            .map_err(|_| "hf_worker_lock_poisoned")?
            .transact(
                "deep_instrumentation",
                json!({
                    "prompt": request.prompt,
                    "module_path": request.module_path,
                    "token_from_end": request.token_from_end,
                }),
            )?;
        let payload: HfDeepInstrumentationPayload = serde_json::from_value(value)?;
        if payload.module_path != request.module_path
            || payload.token_from_end != request.token_from_end
            || payload.values.is_empty()
            || payload.values.iter().any(|v| !v.is_finite())
            || payload.values_sha256 != f64_vector_sha256(&payload.values)?
            || payload.backend != "nnsight"
            || payload.backend_version != self.nnsight_version
            || payload.total_duration_ns == 0
        {
            return Err("hf_deep_instrumentation_payload_invalid".into());
        }
        let mut evidence = DeepInstrumentationEvidence {
            schema: "cerebro.cross_model.deep_instrumentation/v1".into(),
            model: self.config.name.clone(),
            runtime_metadata_sha256: self.config.runtime_metadata_sha256.clone(),
            module_path: payload.module_path,
            token_from_end: payload.token_from_end,
            values: payload.values,
            values_sha256: payload.values_sha256,
            backend: payload.backend,
            backend_version: payload.backend_version,
            evidence_sha256: String::new(),
        };
        evidence.evidence_sha256 = sha256_hex(&serde_json::to_vec(&(
            &evidence.schema,
            &evidence.model,
            &evidence.runtime_metadata_sha256,
            &evidence.module_path,
            evidence.token_from_end,
            &evidence.values_sha256,
            &evidence.backend,
            &evidence.backend_version,
        ))?);
        evidence.validate()?;
        Ok(evidence)
    }

    fn sparse_autoencoder_analysis(
        &self,
        request: &SparseAutoencoderRequest,
    ) -> Result<SparseAutoencoderEvidence, Box<dyn Error + Send + Sync>> {
        request.validate()?;
        let expected_weights = request.sae_dir.join("sae.safetensors");
        let expected_config = request.sae_dir.join("config.json");
        if !request.sae_dir.is_dir()
            || !expected_weights.same_file(&request.weights_path)?
            || !expected_config.same_file(&request.config_path)?
        {
            return Err("hf_sparse_autoencoder_snapshot_binding_invalid".into());
        }
        let weights_sha256 = crate::foundation::digest::sha256_file(&request.weights_path)?;
        let config_sha256 = crate::foundation::digest::sha256_file(&request.config_path)?;
        let value = self
            .worker
            .lock()
            .map_err(|_| "hf_worker_lock_poisoned")?
            .transact(
                "sae_analysis",
                json!({
                    "prompt": request.prompt,
                    "module_path": request.module_path,
                    "token_from_end": request.token_from_end,
                    "sae_dir": request.sae_dir,
                    "weights_path": request.weights_path,
                    "config_path": request.config_path,
                    "top_k": request.top_k,
                }),
            )?;
        let payload: HfSparseAutoencoderPayload = serde_json::from_value(value)?;
        if payload.module_path != request.module_path
            || payload.token_from_end != request.token_from_end
            || payload.sae_weights_sha256 != weights_sha256.as_str()
            || payload.sae_config_sha256 != config_sha256.as_str()
            || payload.d_in == 0
            || payload.d_sae == 0
            || payload.feature_count != payload.d_sae
            || !is_sha256(&payload.input_sha256)
            || payload.active_feature_count > payload.feature_count
            || payload.top_features.is_empty()
            || payload.top_features.len() > request.top_k
            || !payload.reconstruction_relative_error.is_finite()
            || payload.reconstruction_relative_error < 0.0
            || payload.total_duration_ns == 0
        {
            return Err("hf_sparse_autoencoder_payload_invalid".into());
        }
        let top_features = payload
            .top_features
            .into_iter()
            .map(|feature| SparseFeature {
                index: feature.index,
                activation: feature.activation,
            })
            .collect::<Vec<_>>();
        let mut evidence = SparseAutoencoderEvidence {
            schema: "cerebro.tidex.sparse_autoencoder_evidence/v1".into(),
            model: self.config.name.clone(),
            runtime_metadata_sha256: self.config.runtime_metadata_sha256.clone(),
            module_path: payload.module_path,
            token_from_end: payload.token_from_end,
            sae_weights_sha256: payload.sae_weights_sha256,
            sae_config_sha256: payload.sae_config_sha256,
            d_in: payload.d_in,
            d_sae: payload.d_sae,
            input_sha256: payload.input_sha256,
            feature_count: payload.feature_count,
            active_feature_count: payload.active_feature_count,
            top_features,
            reconstruction_relative_error: payload.reconstruction_relative_error,
            evidence_sha256: String::new(),
        };
        evidence.evidence_sha256 = sha256_hex(&serde_json::to_vec(&(
            &evidence.schema,
            &evidence.model,
            &evidence.runtime_metadata_sha256,
            &evidence.module_path,
            evidence.token_from_end,
            &evidence.sae_weights_sha256,
            &evidence.sae_config_sha256,
            evidence.d_in,
            evidence.d_sae,
            &evidence.input_sha256,
            evidence.feature_count,
            evidence.active_feature_count,
            &evidence.top_features,
            evidence.reconstruction_relative_error,
        ))?);
        evidence.validate()?;
        Ok(evidence)
    }
}

impl Drop for HfTransformersModel {
    fn drop(&mut self) {
        if let Ok(mut worker) = self.worker.lock() {
            let shutdown = worker.transact("shutdown", json!({}));
            if shutdown.is_err() {
                let _ = worker.child.kill();
            }
            let _ = worker.child.wait();
        }
    }
}

pub(crate) fn f64_vector_sha256(values: &[f64]) -> Result<String, Box<dyn Error + Send + Sync>> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err("f64_vector_invalid".into());
    }
    let len = u64::try_from(values.len()).map_err(|_| "f64_vector_length_overflow")?;
    let mut hasher = Sha256::new();
    hasher.update(b"CEREBRO:CROSS-MODEL:F64-VECTOR:v1\0");
    hasher.update(len.to_le_bytes());
    for value in values {
        hasher.update(value.to_le_bytes());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod evidence_tests {
    use super::*;

    #[test]
    fn generation_execution_digest_changes_with_active_intervention_set() {
        let config = ModelConfig {
            name: "fixture".into(),
            runtime_model: "runtime-fixture".into(),
            family: ArchitectureFamily::Llama,
            runtime_architecture: "llama".into(),
            embedding_dim: 8,
            hidden_dim: 16,
            num_layers: 2,
            num_heads: 2,
            vocab_size: 32,
            max_sequence_length: 128,
            parameter_count: 64,
            quantization: None,
            runtime_metadata_sha256: sha256_hex(b"runtime-metadata"),
            tensor_names: vec!["layer.weight".into()],
        };
        let policy = GenerationPolicy::default();
        let empty = Vec::<serde_json::Value>::new();
        let active = vec![serde_json::json!({
            "layer_index": 1,
            "steering_sha256": sha256_hex(b"steering"),
            "strength": 0.5
        })];
        let empty_sha = active_interventions_sha256(&empty).unwrap();
        let active_sha = active_interventions_sha256(&active).unwrap();
        assert_ne!(empty_sha, active_sha);
        let response_sha = sha256_hex(b"same text");
        let digest = |interventions: &str, count| {
            generation_execution_sha256(GenerationExecutionEvidence {
                config: &config,
                prompt: "same prompt",
                policy: &policy,
                active_interventions_sha256: interventions,
                active_intervention_count: count,
                response_sha256: &response_sha,
                prompt_eval_count: Some(2),
                eval_count: Some(2),
                done_reason: Some("length"),
            })
            .unwrap()
        };
        assert_ne!(digest(&empty_sha, 0), digest(&active_sha, 1));
    }

    #[test]
    fn hf_runtime_lock_certifies_worker_packages_and_excludes_sae_lens() {
        for package in [
            "torch",
            "transformers",
            "safetensors",
            "accelerate",
            "huggingface_hub",
            "tokenizers",
            "nnsight",
        ] {
            let version = hf_runtime_lock_version(package).unwrap();
            assert!(!version.is_empty(), "{package}");
        }
        assert!(hf_runtime_lock_version("torch").unwrap().contains("+cpu"));
        assert!(HF_RUNTIME_LOCK.contains("--hash=sha256:"));
        assert!(hf_runtime_lock_version("sae-lens").is_err());
        assert!(hf_runtime_lock_version("sae_lens").is_err());
        assert_eq!(pep503_name("HuggingFace.Hub"), "huggingface-hub");
        assert_eq!(pep503_name("huggingface_hub"), "huggingface-hub");
        let identity = certified_runtime_identity_sha256().unwrap();
        assert_eq!(identity.len(), 64);
        assert!(identity.bytes().all(|byte| byte.is_ascii_hexdigit()));
        let payload = serde_json::json!({
            "implementation": "cpython",
            "packages": hf_runtime_lock_packages().unwrap(),
            "python_minor": HF_RUNTIME_PYTHON_MINOR,
        });
        assert_eq!(serde_json::to_vec(&payload).unwrap().len(), 1402);
        assert_eq!(identity, "6f878b9a0c499cbca153d263337b8116bc4861a3ca2002307a78bbfb21c2722b");
        assert!(HF_RUNTIME_LOCK
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .all(|line| {
                let spec = line.split_whitespace().next().unwrap_or("");
                let name = spec.split_once("==").map(|(name, _)| name).unwrap_or("");
                pep503_name(name) != "sae-lens"
            }));
    }
}
