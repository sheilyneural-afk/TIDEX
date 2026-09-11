use cerebro_tidex::cross_model::discovery::{evaluate_model, BehavioralBenchmark};
use cerebro_tidex::cross_model::models::{
    DeepInstrumentationRequest, HfTransformersModel, HfTransformersRuntimeConfig,
    SparseAutoencoderRequest,
};
use cerebro_tidex::cross_model::{
    CounterfactualAnalyzer, CounterfactualScenario, CrossModelAligner, ExtractionLevel,
    HierarchicalSteeringExtractor, LLMModel,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

const MAX_REQUEST_BYTES: u64 = 128 * 1024 * 1024;

const HF_RUNTIME_PROBE_SCRIPT: &str = r#"
import json, os, sys, time, hashlib, importlib.metadata
from pathlib import Path

os.environ.setdefault("TOKENIZERS_PARALLELISM", "false")
os.environ.setdefault("TRANSFORMERS_NO_TF", "1")
os.environ.setdefault("USE_TF", "0")
os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("PYTHONNOUSERSITE", "1")

def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()

def importable(name: str) -> str | None:
    try:
        __import__(name)
        dist = "sae-lens" if name == "sae_lens" else name
        return importlib.metadata.version(dist)
    except Exception:
        return None

def get_int(obj, *names):
    for name in names:
        value = obj.get(name) if isinstance(obj, dict) else getattr(obj, name, None)
        if value is not None:
            try:
                value = int(value)
                if value > 0:
                    return value
            except Exception:
                pass
    return 0

def layer_count(model):
    paths = [
        ("model", "layers"),
        ("transformer", "h"),
        ("gpt_neox", "layers"),
        ("bert", "encoder", "layer"),
        ("roberta", "encoder", "layer"),
        ("encoder", "layer"),
    ]
    for path in paths:
        cur = model
        ok = True
        for part in path:
            cur = getattr(cur, part, None)
            if cur is None:
                ok = False
                break
        if ok:
            try:
                return len(cur)
            except Exception:
                pass
    return 0

def main():
    req = json.loads(sys.stdin.buffer.read())
    snapshot = Path(req["model_dir"])
    checkpoint = Path(req["checkpoint_path"])
    config_path = Path(req["config_path"])
    tokenizer_path = Path(req["tokenizer_path"])
    notes = []
    access = {
        "behavioral_inference": False,
        "internal_activations": False,
        "activation_intervention": False,
        "deep_instrumentation": False,
        "sparse_autoencoder_analysis": False,
    }
    raw = json.loads(config_path.read_text())
    model_type = str(raw.get("model_type", "unknown"))
    architectures = list(raw.get("architectures") or [])
    hidden_size = get_int(raw, "hidden_size", "n_embd", "d_model")
    intermediate_size = get_int(raw, "intermediate_size", "n_inner", "ffn_hidden_size")
    if intermediate_size == 0 and hidden_size > 0 and model_type in {"gpt2", "gpt_bigcode"}:
        intermediate_size = hidden_size * 4
        notes.append("intermediate_size_derived_from_gpt2_default")
    num_layers = get_int(raw, "num_hidden_layers", "n_layer", "num_layers")
    num_heads = get_int(raw, "num_attention_heads", "n_head", "num_heads")
    vocab_size = get_int(raw, "vocab_size")
    max_positions = get_int(raw, "max_position_embeddings", "n_positions", "seq_length", "max_sequence_length")
    config_sha256 = sha256_file(config_path)
    tokenizer_sha256 = sha256_file(tokenizer_path)
    checkpoint_sha256 = sha256_file(checkpoint)
    nnsight_version = importable("nnsight")
    sae_lens_version = importable("sae_lens")
    parameter_count = 0
    runtime_model_sha256 = checkpoint_sha256
    loaded_class = None
    load_duration_ns = None
    causal_architectures = ("causallm", "lmheadmodel")
    encoder_only_types = {"bert", "roberta", "distilbert", "sentence_bert"}
    declared_causal = model_type not in encoder_only_types and any(
        causal in str(arch).lower() for arch in architectures for causal in causal_architectures
    )
    if checkpoint.name == "model.safetensors.index.json":
        notes.append("sharded_checkpoint_detected_probe_metadata_only_normalize_for_execution")
    else:
        try:
            import torch
            from transformers import AutoConfig, AutoModel, AutoModelForCausalLM, AutoTokenizer
            torch.set_num_threads(max(1, min(int(req.get("threads", 1)), 256)))
            started = time.monotonic_ns()
            cfg = AutoConfig.from_pretrained(str(snapshot), local_files_only=True, trust_remote_code=False)
            tokenizer = AutoTokenizer.from_pretrained(str(snapshot), local_files_only=True, trust_remote_code=False)
            if declared_causal:
                try:
                    model = AutoModelForCausalLM.from_pretrained(str(snapshot), local_files_only=True, trust_remote_code=False, torch_dtype=torch.float32, low_cpu_mem_usage=False)
                    model.eval()
                    loaded_class = type(model).__name__
                    parameter_count = sum(int(p.numel()) for p in model.parameters())
                    layers = layer_count(model)
                    access["behavioral_inference"] = True
                    access["internal_activations"] = layers > 0
                    access["activation_intervention"] = layers > 0
                    access["deep_instrumentation"] = nnsight_version is not None
                    access["sparse_autoencoder_analysis"] = nnsight_version is not None and sae_lens_version is not None
                    if layers == 0:
                        notes.append("causal_model_loaded_without_supported_layer_path")
                except Exception as exc:
                    notes.append("causal_load_failed:" + type(exc).__name__ + ":" + str(exc)[:512])
            else:
                notes.append("non_causal_architecture_no_current_generative_workflows")
            if loaded_class is None:
                try:
                    model = AutoModel.from_pretrained(str(snapshot), local_files_only=True, trust_remote_code=False, torch_dtype=torch.float32, low_cpu_mem_usage=False)
                    model.eval()
                    loaded_class = type(model).__name__
                    parameter_count = sum(int(p.numel()) for p in model.parameters())
                    notes.append("generic_model_loaded_probe_only")
                except Exception as generic_exc:
                    notes.append("generic_load_failed:" + type(generic_exc).__name__ + ":" + str(generic_exc)[:512])
            load_duration_ns = time.monotonic_ns() - started
        except Exception as outer:
            notes.append("transformers_probe_failed:" + type(outer).__name__ + ":" + str(outer)[:512])
    model = {
        "name": req.get("name", "lab-probe"),
        "runtime_model": runtime_model_sha256,
        "family": "qwen" if model_type.startswith("qwen") else ("llama" if model_type.startswith("llama") else ("gpt" if model_type.startswith("gpt") else {"other": model_type})),
        "runtime_architecture": model_type,
        "embedding_dim": hidden_size,
        "hidden_dim": intermediate_size,
        "num_layers": num_layers,
        "num_heads": num_heads,
        "vocab_size": vocab_size,
        "max_sequence_length": max_positions,
        "parameter_count": parameter_count,
        "quantization": None,
        "runtime_metadata_sha256": hashlib.sha256(json.dumps([config_sha256, tokenizer_sha256, checkpoint_sha256, model_type, architectures, loaded_class, notes], sort_keys=True, separators=(",", ":")).encode()).hexdigest(),
        "tensor_names": ["profile_only"],
    }
    print(json.dumps({
        "schema": "cerebro.tidex.lab_runtime_probe/v1",
        "model": model,
        "access": access,
        "probe": {
            "checkpoint_kind": checkpoint.name,
            "loaded_class": loaded_class,
            "load_duration_ns": load_duration_ns,
            "nnsight_version": nnsight_version,
            "sae_lens_version": sae_lens_version,
            "notes": notes,
        },
    }, sort_keys=True))

if __name__ == "__main__":
    main()
"#;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum LabRunnerRequest {
    ProbeRuntime {
        model: Box<HfTransformersRuntimeConfig>,
    },
    BehavioralEvaluation {
        model: Box<HfTransformersRuntimeConfig>,
        benchmark: BehavioralBenchmark,
    },
    ExtractCapability {
        model: Box<HfTransformersRuntimeConfig>,
        capability_name: String,
        domain: String,
        level: ExtractionLevel,
        positive_examples: Vec<String>,
        negative_examples: Vec<String>,
    },
    DeepInstrumentation {
        model: Box<HfTransformersRuntimeConfig>,
        request: DeepInstrumentationRequest,
    },
    SparseAutoencoderAnalysis {
        model: Box<HfTransformersRuntimeConfig>,
        request: SparseAutoencoderRequest,
    },
    CounterfactualAnalysis {
        model: Box<HfTransformersRuntimeConfig>,
        scenarios: Vec<CounterfactualScenario>,
        activation_layers: Vec<usize>,
    },
    GenerateBehavioralDataset {
        model: Box<HfTransformersRuntimeConfig>,
        benchmark_id: String,
        domain: String,
        objective: String,
        probe_count: usize,
    },
    CalibrateAlignment {
        source: Box<HfTransformersRuntimeConfig>,
        target: Box<HfTransformersRuntimeConfig>,
        source_layer: usize,
        target_layer: usize,
        training_prompts: Vec<String>,
        validation_prompts: Vec<String>,
    },
    ActivationTransferExperiment {
        source: Box<HfTransformersRuntimeConfig>,
        target: Box<HfTransformersRuntimeConfig>,
        capability_name: String,
        domain: String,
        extraction_level: ExtractionLevel,
        positive_examples: Vec<String>,
        negative_examples: Vec<String>,
        source_layer: usize,
        target_layer: usize,
        calibration_prompts: Vec<String>,
        validation_prompts: Vec<String>,
        strength: f64,
        evaluation: BehavioralBenchmark,
    },
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ActivationTransferReport {
    schema: String,
    source_model: String,
    target_model: String,
    capability_name: String,
    extraction: cerebro_tidex::cross_model::ExtractionResult,
    calibration: cerebro_tidex::cross_model::extraction::AlignmentCalibration,
    alignment: cerebro_tidex::cross_model::AlignmentResult,
    intervention: cerebro_tidex::cross_model::ActivationInterventionReceipt,
    baseline: cerebro_tidex::cross_model::discovery::ModelEvaluation,
    intervened: cerebro_tidex::cross_model::discovery::ModelEvaluation,
    restored: cerebro_tidex::cross_model::discovery::ModelEvaluation,
    score_delta: f64,
    restore_delta: f64,
    behavioral_improvement_observed: bool,
}

fn read_request(path: &Path) -> Result<LabRunnerRequest, Box<dyn std::error::Error + Send + Sync>> {
    if !path.is_absolute() {
        return Err("lab_runner_request_path_must_be_absolute".into());
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_REQUEST_BYTES
    {
        return Err("lab_runner_request_file_invalid".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_REQUEST_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_REQUEST_BYTES {
        return Err("lab_runner_request_too_large".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn probe_runtime_profile(
    config: &HfTransformersRuntimeConfig,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let payload = serde_json::to_vec(config)?;
    let mut child = Command::new(&config.python_executable)
        .arg("-c")
        .arg(HF_RUNTIME_PROBE_SCRIPT)
        .env("PYTHONNOUSERSITE", "1")
        .env("TOKENIZERS_PARALLELISM", "false")
        .env("TRANSFORMERS_NO_TF", "1")
        .env("USE_TF", "0")
        .env("HF_HUB_OFFLINE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("lab_probe_stdin_missing")?
        .write_all(&payload)?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!(
            "lab_runtime_probe_failed:{}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    if output.stdout.len() > 8 * 1024 * 1024 || output.stderr.len() > 8 * 1024 * 1024 {
        return Err("lab_runtime_probe_output_too_large".into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn spawn(
    config: HfTransformersRuntimeConfig,
) -> Result<HfTransformersModel, Box<dyn std::error::Error + Send + Sync>> {
    HfTransformersModel::spawn(config)
}

fn run(
    request: LabRunnerRequest,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    match request {
        LabRunnerRequest::ProbeRuntime { model } => probe_runtime_profile(&model),
        LabRunnerRequest::BehavioralEvaluation { model, benchmark } => {
            let model = spawn(*model)?;
            Ok(serde_json::to_value(evaluate_model(&model, &benchmark)?)?)
        }
        LabRunnerRequest::ExtractCapability {
            model,
            capability_name,
            domain,
            level,
            positive_examples,
            negative_examples,
        } => {
            let model = spawn(*model)?;
            let extractor = HierarchicalSteeringExtractor::default();
            Ok(serde_json::to_value(extractor.extract(
                &model,
                &capability_name,
                &domain,
                level,
                &positive_examples,
                &negative_examples,
            )?)?)
        }
        LabRunnerRequest::DeepInstrumentation { model, request } => {
            let model = spawn(*model)?;
            Ok(serde_json::to_value(model.deep_instrumentation(&request)?)?)
        }
        LabRunnerRequest::SparseAutoencoderAnalysis { model, request } => {
            let model = spawn(*model)?;
            Ok(serde_json::to_value(
                model.sparse_autoencoder_analysis(&request)?,
            )?)
        }
        LabRunnerRequest::CounterfactualAnalysis {
            model,
            scenarios,
            activation_layers,
        } => {
            let model = spawn(*model)?;
            let analyzer = CounterfactualAnalyzer::default();
            Ok(serde_json::to_value(analyzer.analyze(
                &model,
                &scenarios,
                &activation_layers,
            )?)?)
        }
        LabRunnerRequest::GenerateBehavioralDataset {
            model,
            benchmark_id,
            domain,
            objective,
            probe_count,
        } => {
            if benchmark_id.trim().is_empty()
                || domain.trim().is_empty()
                || objective.trim().is_empty()
                || benchmark_id.len() > 256
                || domain.len() > 256
                || objective.len() > 16_384
                || !(2..=128).contains(&probe_count)
            {
                return Err("generated_benchmark_request_invalid".into());
            }
            let model = spawn(*model)?;
            let prompt = format!(
                "Create exactly {probe_count} deterministic behavioral benchmark probes for the objective below. Return ONLY one JSON object, with no markdown and no commentary. The JSON MUST satisfy this exact schema: {{\"schema\":\"cerebro.cross_model.behavioral_benchmark/v1\",\"benchmark_id\":{benchmark_id:?},\"domain\":{domain:?},\"probes\":[{{\"probe_id\":\"p1\",\"prompt\":\"...\",\"verifier\":{{\"kind\":\"exact_text\",\"expected\":\"...\",\"trim\":true,\"case_sensitive\":true}},\"weight\":1.0}}],\"minimum_mean_gap\":0.0,\"significance_alpha\":0.05}}. Use deterministic verifiers only: exact_text, contains_all, numeric, or json_pointer_equals. Do not include unverifiable subjective grading. Objective: {objective}"
            );
            let generation = model.generate(&prompt)?;
            let benchmark: BehavioralBenchmark = serde_json::from_str(&generation.text)
                .map_err(|_| "generated_benchmark_not_exact_json")?;
            benchmark.validate()?;
            if benchmark.benchmark_id != benchmark_id
                || benchmark.domain != domain
                || benchmark.probes.len() != probe_count
            {
                return Err("generated_benchmark_contract_mismatch".into());
            }
            Ok(serde_json::json!({
                "schema":"cerebro.tidex.lab_generated_benchmark/v1",
                "generator_model":model.name(),
                "runtime_metadata_sha256":model.config().runtime_metadata_sha256,
                "generation_execution_sha256":generation.execution_sha256,
                "generation_response_sha256":generation.response_sha256,
                "independent_evidence":false,
                "benchmark":benchmark
            }))
        }
        LabRunnerRequest::CalibrateAlignment {
            source,
            target,
            source_layer,
            target_layer,
            training_prompts,
            validation_prompts,
        } => {
            let source = spawn(*source)?;
            let target = spawn(*target)?;
            let aligner = CrossModelAligner::default();
            Ok(serde_json::to_value(aligner.calibrate_from_prompts(
                &source,
                &target,
                source_layer,
                target_layer,
                &training_prompts,
                &validation_prompts,
            )?)?)
        }
        LabRunnerRequest::ActivationTransferExperiment {
            source,
            target,
            capability_name,
            domain,
            extraction_level,
            positive_examples,
            negative_examples,
            source_layer,
            target_layer,
            calibration_prompts,
            validation_prompts,
            strength,
            evaluation,
        } => {
            if !strength.is_finite() || strength <= 0.0 {
                return Err("activation_transfer_strength_invalid".into());
            }
            let source = spawn(*source)?;
            let target = spawn(*target)?;
            let baseline = evaluate_model(&target, &evaluation)?;
            let extractor = HierarchicalSteeringExtractor::default();
            let extraction = extractor.extract(
                &source,
                &capability_name,
                &domain,
                extraction_level,
                &positive_examples,
                &negative_examples,
            )?;
            if extraction.steering_vector.vector.layer_index != source_layer {
                return Err("activation_transfer_extraction_source_layer_mismatch".into());
            }
            let aligner = CrossModelAligner::default();
            let calibration = aligner.calibrate_from_prompts(
                &source,
                &target,
                source_layer,
                target_layer,
                &calibration_prompts,
                &validation_prompts,
            )?;
            let alignment = aligner.align(&extraction.steering_vector.vector, &calibration)?;
            let intervention =
                target.apply_steering(target_layer, &alignment.target_vector, strength)?;
            let intervened = match evaluate_model(&target, &evaluation) {
                Ok(value) => value,
                Err(error) => {
                    let _ = target.clear_activation_interventions();
                    return Err(error);
                }
            };
            target.clear_activation_interventions()?;
            let restored = evaluate_model(&target, &evaluation)?;
            let score_delta = intervened.weighted_score - baseline.weighted_score;
            let restore_delta = restored.weighted_score - baseline.weighted_score;
            Ok(serde_json::to_value(ActivationTransferReport {
                schema: "cerebro.tidex.lab_activation_transfer/v1".into(),
                source_model: source.name().into(),
                target_model: target.name().into(),
                capability_name,
                extraction,
                calibration,
                alignment,
                intervention,
                baseline,
                intervened,
                restored,
                score_delta,
                restore_delta,
                behavioral_improvement_observed: score_delta > 0.0,
            })?)
        }
    }
}

fn main() {
    let result = (|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = std::env::args()
            .nth(1)
            .ok_or("usage: tidex-lab-runner <absolute-request.json>")?;
        let request = read_request(Path::new(&path))?;
        let value = run(request)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
        Ok(())
    })();
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
