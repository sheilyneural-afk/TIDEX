//! Production cross-model evidence daemon.
//!
//! Usage:
//!   plasticity-daemon once <runtime.json> <benchmark.json>
//!   plasticity-daemon loop <runtime.json> <benchmark.json> <interval-seconds>
//!
//! Runtime model entries are explicit. Ollama performs remote behavioral
//! inference and the certified Hugging Face worker loads authenticated local
//! SafeTensors. Neither backend grants production authority.
//!
//! Boundary: this daemon drives `PlasticityEngine` (discovery/intervention).
//! It does not load `config/plasticity.toml` or advance BCM/ELO/PI controllers;
//! those advisory numerical controllers live under `cross_model::plasticity`
//! and are accumulated by the operator plasticity advice path.

use serde::Deserialize;
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tidex::cross_model::discovery::BehavioralBenchmark;
use tidex::cross_model::models::{
    GenerationPolicy, HfTransformersModel, HfTransformersRuntimeConfig, LLMModel, OllamaModel,
};
use tidex::cross_model::plasticity_engine::{PlasticityEngine, PlasticityEngineConfig};

const MAX_CONFIG_BYTES: u64 = 8 * 1024 * 1024;
const MAX_INTERVAL_SECONDS: u64 = 86_400;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "backend", rename_all = "snake_case", deny_unknown_fields)]
enum RuntimeModelSpec {
    Ollama {
        model: String,
        endpoint: String,
        generation: GenerationPolicy,
    },
    HfTransformers {
        runtime: HfTransformersRuntimeConfig,
    },
}

impl RuntimeModelSpec {
    fn identity(&self) -> &str {
        match self {
            Self::Ollama { model, .. } => model,
            Self::HfTransformers { runtime } => &runtime.name,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.identity().trim().is_empty() {
            return Err("runtime_model_identity_invalid".into());
        }
        match self {
            Self::Ollama { endpoint, .. } => {
                if !(endpoint.starts_with("http://") || endpoint.starts_with("https://")) {
                    return Err("ollama_runtime_endpoint_invalid".into());
                }
            }
            Self::HfTransformers { runtime } => {
                runtime.validate().map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }

    fn connect(&self) -> Result<Box<dyn LLMModel>, Box<dyn Error + Send + Sync>> {
        self.validate()?;
        match self {
            Self::Ollama {
                model,
                endpoint,
                generation,
            } => Ok(Box::new(OllamaModel::connect_with_policy(
                endpoint.clone(),
                model.clone(),
                generation.clone(),
            )?)),
            Self::HfTransformers { runtime } => {
                Ok(Box::new(HfTransformersModel::spawn(runtime.clone())?))
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeConfig {
    schema: String,
    models: Vec<RuntimeModelSpec>,
    maximum_stored_capabilities: usize,
}

impl RuntimeConfig {
    fn validate(&self) -> Result<(), String> {
        if self.schema != "tidex.cross_model.runtime/v2"
            || self.models.len() < 2
            || self.models.len() > 64
            || self.maximum_stored_capabilities == 0
        {
            return Err("cross_model_runtime_config_invalid".into());
        }
        let mut unique = BTreeSet::new();
        for model in &self.models {
            model.validate()?;
            if !unique.insert(model.identity()) {
                return Err("cross_model_runtime_model_duplicate".into());
            }
        }
        Ok(())
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.as_slice() {
        [_, mode, runtime_path, benchmark_path] if mode == "once" => {
            let runtime: RuntimeConfig = read_json_bounded(Path::new(runtime_path))?;
            let benchmark: BehavioralBenchmark = read_json_bounded(Path::new(benchmark_path))?;
            execute_once(&runtime, &benchmark)?;
        }
        [_, mode, runtime_path, benchmark_path, interval] if mode == "loop" => {
            let interval = interval.parse::<u64>()?;
            if interval == 0 || interval > MAX_INTERVAL_SECONDS {
                return Err("plasticity_daemon_interval_invalid".into());
            }
            let runtime: RuntimeConfig = read_json_bounded(Path::new(runtime_path))?;
            let benchmark: BehavioralBenchmark = read_json_bounded(Path::new(benchmark_path))?;

            // Construct engine once and preserve its state between iterations.
            runtime.validate()?;
            benchmark.validate()?;
            let mut engine = PlasticityEngine::new(PlasticityEngineConfig {
                maximum_models: runtime.models.len(),
                maximum_stored_capabilities: runtime.maximum_stored_capabilities,
            })?;
            for model in &runtime.models {
                engine.register_model(model.connect()?)?;
            }

            loop {
                let report = engine.run_discovery_pipeline(&benchmark)?;
                println!("{}", serde_json::to_string(&report)?);
                std::thread::sleep(Duration::from_secs(interval));
            }
        }
        _ => {
            return Err(
                "usage: plasticity-daemon once <runtime.json> <benchmark.json> | loop <runtime.json> <benchmark.json> <interval-seconds>"
                    .into(),
            );
        }
    }
    Ok(())
}

fn execute_once(
    runtime: &RuntimeConfig,
    benchmark: &BehavioralBenchmark,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    runtime.validate()?;
    benchmark.validate()?;
    let mut engine = PlasticityEngine::new(PlasticityEngineConfig {
        maximum_models: runtime.models.len(),
        maximum_stored_capabilities: runtime.maximum_stored_capabilities,
    })?;
    for model in &runtime.models {
        engine.register_model(model.connect()?)?;
    }
    let report = engine.run_discovery_pipeline(benchmark)?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}

fn read_json_bounded<T: serde::de::DeserializeOwned>(
    path: &Path,
) -> Result<T, Box<dyn Error + Send + Sync>> {
    if !path.is_absolute() {
        return Err("daemon_input_path_must_be_absolute".into());
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_CONFIG_BYTES
    {
        return Err("daemon_input_file_invalid".into());
    }
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}
