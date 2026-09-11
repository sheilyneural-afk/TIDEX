//! Mistral-family behavioral runtime backed by a real Ollama model.

use super::{ArchitectureFamily, GenerationOutput, LLMModel, ModelConfig, OllamaBackend};
use std::error::Error;

#[derive(Debug, Clone)]
pub struct MistralModel {
    backend: OllamaBackend,
}

impl MistralModel {
    pub fn connect(
        endpoint: impl Into<String>,
        runtime_model: impl Into<String>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            backend: OllamaBackend::connect(
                endpoint,
                runtime_model,
                Some(ArchitectureFamily::Mistral),
            )?,
        })
    }
}

impl LLMModel for MistralModel {
    fn config(&self) -> &ModelConfig {
        self.backend.config()
    }

    fn generate(&self, prompt: &str) -> Result<GenerationOutput, Box<dyn Error + Send + Sync>> {
        self.backend.generate(prompt)
    }
}
