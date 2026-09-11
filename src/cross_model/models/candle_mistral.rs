//! Real local Mistral-family inference backed by Candle SafeTensors.
//!
//! Exact checkpoint/config/tokenizer bytes are authenticated and loaded into a
//! Candle CPU model. Generation holds the model lock for one complete request
//! because the public Mistral implementation owns its KV cache internally.
//! Intermediate decoder states are private in Candle, therefore this backend
//! exposes behavioral inference only and fails closed for hidden activations.

use super::{
    load_local_safetensors_artifacts, local_model_config, ArchitectureFamily, GenerationOutput,
    GenerationPolicy, LLMModel, LocalModelGeometry, ModelConfig,
};
use candle_core::{DType as CandleDType, Device as CandleDevice, Tensor as CandleTensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::mistral::{Config as MistralConfig, Model as Mistral};
use std::error::Error;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;
use tokenizers::Tokenizer;

pub struct CandleMistralModel {
    config: ModelConfig,
    model: Mutex<Mistral>,
    tokenizer: Tokenizer,
    eos_token_ids: Vec<u32>,
    device: CandleDevice,
    policy: GenerationPolicy,
}

impl std::fmt::Debug for CandleMistralModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CandleMistralModel")
            .field("config", &self.config)
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

impl CandleMistralModel {
    pub fn from_safetensors(
        name: impl Into<String>,
        checkpoint_path: impl AsRef<Path>,
        config_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
        policy: GenerationPolicy,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        policy.validate()?;
        let name = name.into();
        let artifacts = load_local_safetensors_artifacts(
            checkpoint_path.as_ref(),
            config_path.as_ref(),
            tokenizer_path.as_ref(),
        )?;
        let parsed: MistralConfig = serde_json::from_slice(&artifacts.config_bytes)?;
        let generic: serde_json::Value = serde_json::from_slice(&artifacts.config_bytes)?;
        let runtime_architecture = generic
            .get("model_type")
            .and_then(serde_json::Value::as_str)
            .ok_or("candle_mistral_model_type_missing")?
            .to_ascii_lowercase();
        if ArchitectureFamily::from_runtime_architecture(&runtime_architecture)
            != ArchitectureFamily::Mistral
        {
            return Err(
                format!("candle_mistral_architecture_mismatch:{runtime_architecture}").into()
            );
        }
        let eos_token_ids = parse_token_ids(generic.get("eos_token_id"))?;
        let config = local_model_config(
            &name,
            &runtime_architecture,
            LocalModelGeometry {
                family: ArchitectureFamily::Mistral,
                embedding_dim: parsed.hidden_size,
                hidden_dim: parsed.intermediate_size,
                num_layers: parsed.num_hidden_layers,
                num_heads: parsed.num_attention_heads,
                vocab_size: parsed.vocab_size,
                max_sequence_length: parsed.max_position_embeddings,
            },
            &artifacts,
        )?;
        let tokenizer = Tokenizer::from_bytes(&artifacts.tokenizer_bytes)?;
        let device = CandleDevice::Cpu;
        let builder = VarBuilder::from_buffered_safetensors(
            artifacts.checkpoint_bytes,
            CandleDType::F32,
            &device,
        )?;
        let model = Mistral::new(&parsed, builder)?;
        Ok(Self {
            config,
            model: Mutex::new(model),
            tokenizer,
            eos_token_ids,
            device,
            policy,
        })
    }
}

impl LLMModel for CandleMistralModel {
    fn config(&self) -> &ModelConfig {
        &self.config
    }

    fn generate(&self, prompt: &str) -> Result<GenerationOutput, Box<dyn Error + Send + Sync>> {
        if prompt.trim().is_empty() || prompt.len() > 1_048_576 {
            return Err("generation_prompt_invalid".into());
        }
        let encoding = self.tokenizer.encode(prompt, true)?;
        let mut tokens = encoding.get_ids().to_vec();
        if tokens.is_empty()
            || tokens
                .len()
                .checked_add(self.policy.max_tokens)
                .is_none_or(|count| count > self.config.max_sequence_length)
        {
            return Err("candle_mistral_context_limit".into());
        }
        let prompt_tokens = tokens.len();
        let temperature = (self.policy.temperature > 0.0).then_some(self.policy.temperature);
        let top_p = (self.policy.top_p < 1.0).then_some(self.policy.top_p);
        let mut sampler = LogitsProcessor::new(self.policy.seed, temperature, top_p);
        let started = Instant::now();
        let mut index_pos = 0usize;
        let mut done_reason = "length";
        let mut model = self
            .model
            .lock()
            .map_err(|_| "candle_mistral_model_lock_poisoned")?;
        model.clear_kv_cache();

        for step in 0..self.policy.max_tokens {
            let input_slice = if step == 0 {
                tokens.as_slice()
            } else {
                &tokens[tokens.len() - 1..]
            };
            let input = CandleTensor::new(input_slice, &self.device)?.unsqueeze(0)?;
            let logits = model.forward(&input, index_pos)?;
            index_pos = index_pos
                .checked_add(input_slice.len())
                .ok_or("candle_mistral_position_overflow")?;
            let logits = logits.squeeze(0)?.squeeze(0)?;
            let token = sampler.sample(&logits)?;
            if self.eos_token_ids.contains(&token) {
                done_reason = "eos";
                break;
            }
            tokens.push(token);
        }
        let generated = &tokens[prompt_tokens..];
        let text = self.tokenizer.decode(generated, true)?;
        let response_sha256 = super::sha256_hex(text.as_bytes());
        let active_interventions: Vec<serde_json::Value> = Vec::new();
        let active_interventions_sha256 =
            super::active_interventions_sha256(&active_interventions)?;
        let prompt_eval_count = u64::try_from(prompt_tokens).ok();
        let eval_count = u64::try_from(generated.len()).ok();
        let done_reason = Some(done_reason.to_string());
        let execution_sha256 =
            super::generation_execution_sha256(super::GenerationExecutionEvidence {
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
            model: self.config.name.clone(),
            response_sha256,
            execution_sha256,
            active_interventions_sha256,
            active_intervention_count: 0,
            text,
            total_duration_ns: u64::try_from(started.elapsed().as_nanos()).ok(),
            load_duration_ns: None,
            prompt_eval_count,
            eval_count,
            done_reason,
        })
    }
}

fn parse_token_ids(
    value: Option<&serde_json::Value>,
) -> Result<Vec<u32>, Box<dyn Error + Send + Sync>> {
    match value {
        None | Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(serde_json::Value::Number(number)) => {
            let raw = number.as_u64().ok_or("mistral_eos_token_invalid")?;
            Ok(vec![u32::try_from(raw).map_err(|_| "mistral_eos_token_overflow")?])
        }
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_u64()
                    .ok_or_else(|| "mistral_eos_token_invalid".into())
                    .and_then(|raw| {
                        u32::try_from(raw).map_err(|_| -> Box<dyn Error + Send + Sync> {
                            "mistral_eos_token_overflow".into()
                        })
                    })
            })
            .collect(),
        Some(_) => Err("mistral_eos_token_invalid".into()),
    }
}
