use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;
use tidex::cross_model::discovery::{evaluate_model, BehavioralBenchmark};
use tidex::cross_model::models::{
    DeepInstrumentationRequest, HfTransformersModel, HfTransformersRuntimeConfig, ModelAccess,
    SparseAutoencoderRequest,
};
use tidex::cross_model::{
    CounterfactualAnalyzer, CounterfactualScenario, CrossModelAligner, ExtractionLevel,
    HierarchicalSteeringExtractor, LLMModel,
};

const MAX_REQUEST_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum OperatorRunnerRequest {
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
    extraction: tidex::cross_model::ExtractionResult,
    calibration: tidex::cross_model::extraction::AlignmentCalibration,
    alignment: tidex::cross_model::AlignmentResult,
    intervention: tidex::cross_model::ActivationInterventionReceipt,
    baseline: tidex::cross_model::discovery::ModelEvaluation,
    intervened: tidex::cross_model::discovery::ModelEvaluation,
    restored: tidex::cross_model::discovery::ModelEvaluation,
    score_delta: f64,
    restore_delta: f64,
    behavioral_improvement_observed: bool,
}

fn read_request(
    path: &Path,
) -> Result<OperatorRunnerRequest, Box<dyn std::error::Error + Send + Sync>> {
    if !path.is_absolute() {
        return Err("operator_runner_request_path_must_be_absolute".into());
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_REQUEST_BYTES
    {
        return Err("operator_runner_request_file_invalid".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_REQUEST_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_REQUEST_BYTES {
        return Err("operator_runner_request_too_large".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn probe_runtime_profile(
    config: HfTransformersRuntimeConfig,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let model = spawn(config)?;
    Ok(serde_json::json!({
        "schema": "tidex.operator_runtime_probe/v1",
        "model": model.config(),
        "access": {
            "behavioral_inference": model.supports(ModelAccess::BehavioralInference),
            "internal_activations": model.supports(ModelAccess::InternalActivations),
            "activation_intervention": model.supports(ModelAccess::ActivationIntervention),
            "deep_instrumentation": model.supports(ModelAccess::DeepInstrumentation),
            "sparse_autoencoder_analysis": model.supports(ModelAccess::SparseAutoencoderAnalysis),
        },
        "probe": {
            "runtime_identity_sha256": model.runtime_identity_sha256(),
            "nnsight_version": model.nnsight_version(),
            "bound_sparse_dictionary": false,
            "load_duration_ns": model.worker_load_duration_ns(),
        },
    }))
}

fn spawn(
    config: HfTransformersRuntimeConfig,
) -> Result<HfTransformersModel, Box<dyn std::error::Error + Send + Sync>> {
    HfTransformersModel::spawn(config)
}

fn run(
    request: OperatorRunnerRequest,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    match request {
        OperatorRunnerRequest::ProbeRuntime { model } => probe_runtime_profile(*model),
        OperatorRunnerRequest::BehavioralEvaluation { model, benchmark } => {
            let model = spawn(*model)?;
            Ok(serde_json::to_value(evaluate_model(&model, &benchmark)?)?)
        }
        OperatorRunnerRequest::ExtractCapability {
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
        OperatorRunnerRequest::DeepInstrumentation { model, request } => {
            let model = spawn(*model)?;
            Ok(serde_json::to_value(model.deep_instrumentation(&request)?)?)
        }
        OperatorRunnerRequest::SparseAutoencoderAnalysis { model, request } => {
            let model = spawn(*model)?;
            Ok(serde_json::to_value(model.sparse_autoencoder_analysis(&request)?)?)
        }
        OperatorRunnerRequest::CounterfactualAnalysis {
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
        OperatorRunnerRequest::GenerateBehavioralDataset {
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
                "Create exactly {probe_count} deterministic behavioral benchmark probes for the objective below. Return ONLY one JSON object, with no markdown and no commentary. The JSON MUST satisfy this exact schema: {{\"schema\":\"tidex.cross_model.behavioral_benchmark/v1\",\"benchmark_id\":{benchmark_id:?},\"domain\":{domain:?},\"probes\":[{{\"probe_id\":\"p1\",\"prompt\":\"...\",\"verifier\":{{\"kind\":\"exact_text\",\"expected\":\"...\",\"trim\":true,\"case_sensitive\":true}},\"weight\":1.0}}],\"minimum_mean_gap\":0.0,\"significance_alpha\":0.05}}. Use deterministic verifiers only: exact_text, contains_all, numeric, or json_pointer_equals. Do not include unverifiable subjective grading. Objective: {objective}"
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
                "schema":"tidex.operator_generated_benchmark/v1",
                "generator_model":model.name(),
                "runtime_metadata_sha256":model.config().runtime_metadata_sha256,
                "generation_execution_sha256":generation.execution_sha256,
                "generation_response_sha256":generation.response_sha256,
                "independent_evidence":false,
                "benchmark":benchmark
            }))
        }
        OperatorRunnerRequest::CalibrateAlignment {
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
        OperatorRunnerRequest::ActivationTransferExperiment {
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
                schema: "tidex.operator_activation_transfer/v1".into(),
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
            .ok_or("usage: tidex-operator-runner <absolute-request.json>")?;
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
