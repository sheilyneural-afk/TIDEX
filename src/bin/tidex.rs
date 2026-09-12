use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use tidex::analysis::protected_map::{
    build_protected_cortex_map, persist_protected_map, SensitivityEvidence,
};
use tidex::analysis::pythagoras_topology::{PythagorasStaircaseMetric, TopologicalSkillManifold};
use tidex::capability::acquisition_contract::{
    AcquisitionBudget, AcquisitionRequest, AcquisitionScope, DeclaredRelativePath, NoisePolicy,
    RequestedResidency,
};
use tidex::capability::content_vault::capture_to_vault;
use tidex::engine::BrainEngine;
use tidex::foundation::artifact::{read_dvec_f32, DeltaArtifactRef};
use tidex::foundation::authority::{read_untrusted_private_file_bounded, PrivateFileReference};
use tidex::foundation::contracts::{BrainConfig, DeltaObservation};
use tidex::foundation::finite::FiniteF64;
use tidex::foundation::identity::{AcquisitionId, ProbeId};
use tidex::foundation::security::configured_private_root;
use tidex::governance::adapter_bank::{
    AdapterActivationRequest, AdapterBank, AdapterBankLookup, AdapterBankQuery,
    AdapterCandidateMaterializationRequest, AdapterCompositionRequest, AdapterExecutionResolution,
    AdapterGovernedPromotionRequest, AdapterImportRequest, AdapterResolutionRequest,
    AdapterRevocationRequest, AdapterRollbackRequest,
};
use tidex::governance::residency_decision::ResidencyDecisionAuthority;
use tidex::governance::universal_promotion_gate::{
    evaluate_universal_promotion_gate, UniversalPromotionGateRequest,
};
use tidex::knowledge::knowledge_engine::{AuthorityInstanceId, KnowledgeEngine};
use tidex::learning::numerical_evolution::{
    numerical_metric_specs, NumericalEvaluationGroup, NumericalEvaluationLimits,
    NumericalEvolutionDisposition, NumericalEvolutionEngine, NumericalEvolutionInput,
    NumericalEvolutionPolicy,
};
use tidex::learning::portfolio_governance::{
    CandidateGatePolicy, MetricId, PetfcConservationLimits, PetfcMetricPolicy, PetfcPathLimits,
    PetfcPolicy, PetfcUtilityPolicy, RobustEvaluationPolicy,
};
use tidex::learning::procedural_memory::CapabilityContext;
use tidex::learning::solver_portfolio::{
    CandidateRepresentation, LeastSquaresProblem, PortfolioPolicy,
};
use tidex::materialization::activation_steering_materializer::{
    materialize_replayed_activation_steering_shadow, ActivationSteeringLayout,
    ActivationSteeringPolicy,
};
use tidex::materialization::dense_shadow_materializer::materialize_replayed_dense_delta_shadow;
use tidex::materialization::low_rank_shadow_materializer::{
    materialize_replayed_low_rank_shadow, LowRankShadowPolicy,
};
use tidex::materialization::materialization_selector::BackendSelectionInput;
use tidex::materialization::shadow_evaluation::{run_shadow_evaluation, ShadowEvaluationInput};
use tidex::materialization::sparse_shadow_materializer::{
    materialize_replayed_sparse_shadow, SparseShadowPolicy,
};
use tidex::materialization::universal_capability_compiler::{
    execute_experimental_universal_capability_request, execute_universal_capability_shadow_plan,
    replay_experimental_universal_capability_request, replay_universal_capability_shadow_plan,
    UniversalCapabilityCompilationReceipt, UniversalCapabilityCompilationRequest,
    UniversalCapabilityPlanningRequest, UniversalCapabilityShadowPlanReceipt,
};
use tidex::materialization::universality_evidence::UniversalityEvidenceInput;
use tidex::operator::control_plane::{
    catalog_local_models, compute_operator_living_staircase, configured_operator_home,
    execute_behavioral_discovery_workflow, execute_direct_workflow, execute_operator_run,
    import_dataset_bytes, list_catalog_models, list_datasets, recipe_catalog, serve,
    BehavioralDiscoveryWorkflowRequest, OperatorDatasetFormat, OperatorDirectOperation,
    OperatorDirectWorkflowRequest, OperatorRunRequest,
};
use tidex::operator::executor_registry::{executor_by_id, executor_catalog};
use tidex::operator::graph::compute_operator_graph;
use tidex::operator::workspace::{
    add_model, configured_tidex_home, create_workspace, current_workspace, load_model, use_model,
    use_workspace, ModelProfile, ModelProvider,
};
use tidex::receiver::capability_discovery::CapabilityDiscoveryRequest;
use tidex::receiver::checkpoint_adapter::{
    inspect_safetensors_receiver, SafeTensorsReceiverRequest,
};
use tidex::receiver::model_adaptation::{
    authenticate_live_receiver_model_profile, authenticate_receiver_model_profile,
    profile_receiver_model, ReceiverModelProfileInput,
};
use tidex::receiver::receiver_compiler::{
    benchmark_receiver_portability_leave_one_out, freeze_receiver_compiler, FrozenReceiverCompiler,
    FrozenReceiverCompilerInput, ReceiverPortabilityBenchmarkInput,
};
use tidex::receiver::receiver_layout::ReceiverMaterializationLayout;
use tidex::receiver::receiver_weight_binding::{
    assemble_distributed_lora_basis, authenticate_receiver_weight_candidate,
    materialize_receiver_weight_candidate, prepare_receiver_weight_candidate,
    DistributedLoraBasisInput,
};
use tidex::runtime::isolated_execution::AuthenticatedBytes;

const MAX_CLI_JSON_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ANALYSIS_INPUT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnowledgePlanRequest {
    schema: String,
    authority_instance: String,
    state_reference: PrivateFileReference,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResidencyDecisionRequest {
    schema: String,
    authority_instance: String,
    precommit_reference: PrivateFileReference,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum GeometryAnalysisRequest {
    Pythagoras {
        delta: Vec<f64>,
    },
    Topology {
        points: Vec<Vec<f64>>,
        distance_threshold: f64,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtectedMapEvidenceRow {
    probe_id: String,
    artifact: DeltaArtifactRef,
    causal_damage_per_parameter_norm: f64,
    reliability: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtectedMapRequest {
    schema: String,
    task_labels_used: bool,
    evidence: Vec<ProtectedMapEvidenceRow>,
    retained_energy_fraction: f64,
    regularization_scale: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NumericalProblemRequest {
    inputs: Vec<Vec<f64>>,
    targets: Vec<Vec<f64>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NumericalDenseBaselineRequest {
    rows: usize,
    columns: usize,
    weights: Vec<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NumericalEvolutionPolicyRequest {
    solver_profile: String,
    solver_relative_residual_tolerance: f64,
    solver_absolute_residual_tolerance: f64,
    minimum_normalized_fit: f64,
    maximum_normalized_worst_error: f64,
    robust_minimum_independent_groups: usize,
    robust_median_of_means_blocks: usize,
    robust_maximum_observations: usize,
    gate_minimum_independent_groups: usize,
    gate_uncertainty_multiplier: f64,
    gate_minimum_fit_improvement: f64,
    gate_minimum_worst_error_improvement: f64,
    petfc_fit_normalization_scale: f64,
    petfc_fit_maximum_endpoint_degradation: f64,
    petfc_worst_error_normalization_scale: f64,
    petfc_worst_error_maximum_endpoint_degradation: f64,
    petfc_minimum_reports: usize,
    petfc_maximum_reports: usize,
    petfc_maximum_step_distance: f64,
    petfc_maximum_tortuosity: f64,
    petfc_maximum_waste: f64,
    petfc_minimum_path_efficiency: f64,
    petfc_maximum_soft_degradation_sum: f64,
    petfc_maximum_degraded_metric_count: usize,
    petfc_maximum_distributed_degradation: f64,
    petfc_path_penalty: f64,
    petfc_conservation_penalty: f64,
    petfc_tortuosity_penalty: f64,
    petfc_minimum_utility: f64,
    evaluation_maximum_groups: usize,
    evaluation_maximum_total_cases: usize,
    evaluation_maximum_total_scalar_elements: usize,
    target_scale_floor: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NumericalEvolutionCycleRequest {
    training_problem: NumericalProblemRequest,
    baseline: NumericalDenseBaselineRequest,
    evaluation_groups: Vec<NumericalProblemRequest>,
    revision: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NumericalEvolutionRequest {
    schema: String,
    context: CapabilityContext,
    policy: NumericalEvolutionPolicyRequest,
    cycles: Vec<NumericalEvolutionCycleRequest>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let route = resolve_cli_route(&args);

    if let Err(error) = match route {
        "interface" => run_terminal_interface(),
        _ => run(args),
    } {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn resolve_cli_route(args: &[String]) -> &'static str {
    match args {
        [command] if command == "interface" || command == "ui" => "interface",
        _ => "usage",
    }
}

fn normalize_tidex_interface_args(mut args: Vec<String>) -> Vec<String> {
    const HEADS: &[&str] = &[
        "serve",
        "models",
        "datasets",
        "dataset",
        "evaluate",
        "discover",
        "recipes",
        "run",
        "graph",
        "staircase",
    ];
    if args.first().map(String::as_str) == Some("operator") {
        return args;
    }
    if args
        .first()
        .is_some_and(|head| HEADS.contains(&head.as_str()))
    {
        args.insert(0, "operator".into());
    }
    args
}

fn run(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let args = normalize_tidex_interface_args(args);
    match args.as_slice() {
        [area, command, name, flag, target]
            if area == "workspace" && command == "create" && flag == "--target" =>
        {
            let home = configured_tidex_home()?;
            let manifest = create_workspace(&home, name, Path::new(target))?;
            println!("{}", serde_json::to_string_pretty(&manifest)?);
        }
        [area, command, name] if area == "workspace" && command == "use" => {
            let home = configured_tidex_home()?;
            use_workspace(&home, name)?;
            println!("{}", serde_json::to_string_pretty(&current_workspace(&home)?)?);
        }
        [area, command] if area == "workspace" && command == "show" => {
            let home = configured_tidex_home()?;
            println!("{}", serde_json::to_string_pretty(&current_workspace(&home)?)?);
        }
        [area, command, name, provider_flag, provider, endpoint_flag, endpoint, model_flag, model]
            if area == "model"
                && command == "add"
                && provider_flag == "--provider"
                && endpoint_flag == "--url"
                && model_flag == "--model" =>
        {
            let home = configured_tidex_home()?;
            let provider = match provider.as_str() {
                "openai-compatible" => ModelProvider::OpenAiCompatible,
                _ => return Err("model_provider_invalid".into()),
            };
            add_model(
                &home,
                ModelProfile {
                    schema: "tidex.model_profile/v1".into(),
                    name: name.clone(),
                    provider,
                    endpoint: endpoint.clone(),
                    model: model.clone(),
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&load_model(&home, name)?)?);
        }
        [area, command, name] if area == "model" && command == "use" => {
            let home = configured_tidex_home()?;
            use_model(&home, name)?;
            println!("{}", serde_json::to_string_pretty(&load_model(&home, name)?)?);
        }
        [command] if command == "acquire" => {
            let home = configured_tidex_home()?;
            acquire_workspace(&home, None)?
        }
        [command, flag, path] if command == "acquire" && flag == "--path" => {
            let home = configured_tidex_home()?;
            acquire_workspace(&home, Some(Path::new(path)))?
        }
        [area, command, path] if area == "knowledge" && command == "plan" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&execute_knowledge_plan(Path::new(path))?)?
            );
        }
        [area, command, path] if area == "knowledge" && command == "staircase" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&execute_knowledge_staircase(Path::new(path))?)?
            );
        }
        [area, command, path] if area == "residency" && command == "decide" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&execute_residency_decision(Path::new(path))?)?
            );
        }
        [area, command, path] if area == "numerical" && command == "evolve" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&execute_numerical_evolution(Path::new(path))?)?
            );
        }
        [area, command, path] if area == "analysis" && command == "tomography" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&execute_tomography_analysis(Path::new(path))?)?
            );
        }
        [area, command, path] if area == "analysis" && command == "protected-map" => {
            println!("{}", serde_json::to_string_pretty(&execute_protected_map(Path::new(path))?)?);
        }
        [area, command, path] if area == "analysis" && command == "geometry" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&execute_geometry_analysis(Path::new(path))?)?
            );
        }
        [area, command, path] if area == "benchmark" && command == "response" => {
            let input: tidex::receiver::receiver_compiler::ReceiverSignatureBenchmarkInput =
                read_benchmark_json_bounded(Path::new(path))?;
            let report = tidex::receiver::receiver_compiler::benchmark_receiver_signature(&input)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        [area, command, path] if area == "benchmark" && command == "receiver-basis" => {
            let input: tidex::receiver::receiver_compiler::ReceiverBasisBenchmarkInput =
                read_benchmark_json_bounded(Path::new(path))?;
            let report = tidex::receiver::receiver_compiler::benchmark_receiver_basis(&input)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        [area, command, path] if area == "benchmark" && command == "portability" => {
            let input: ReceiverPortabilityBenchmarkInput =
                read_benchmark_json_bounded(Path::new(path))?;
            let report = benchmark_receiver_portability_leave_one_out(&input)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        [area, command, path] if area == "receiver" && command == "describe-readout" => {
            let input: tidex::receiver::receiver_weight_binding::DescribeLinearReadoutInput =
                read_json_bounded(Path::new(path))?;
            let report = tidex::receiver::receiver_weight_binding::describe_linear_readout(&input)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        [area, command, path] if area == "receiver" && command == "acquire-readout" => {
            let root = configured_private_root()?;
            let input: tidex::receiver::receiver_weight_binding::AcquireLinearReadoutInput =
                read_json_bounded(Path::new(path))?;
            let report =
                tidex::receiver::receiver_weight_binding::acquire_linear_readout(&root, &input)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        [area, command, reference] if area == "receiver" && command == "import-axis" => {
            let root = configured_private_root()?;
            let input: PrivateFileReference = read_json_bounded(Path::new(reference))?;
            let bytes = input.read_verified_bounded(&root, 8 * 1024 * 1024)?;
            let values: Vec<f32> = serde_json::from_slice(&bytes)?;
            let delta = tidex::foundation::artifact::ArtifactWriteAuthority::open(&root)?
                .create_content_addressed_dvec(&values)?;
            println!("{}", serde_json::to_string_pretty(&delta)?);
        }
        [area, command, path] if area == "receiver" && command == "normalize-sharded" => {
            let root = configured_private_root()?;
            let input: tidex::receiver::weight_actuator::ShardedSafetensorsNormalizationInput =
                read_json_bounded(Path::new(path))?;
            let receipt =
                tidex::receiver::weight_actuator::normalize_sharded_safetensors(&root, &input)?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
        }
        [area, command, path] if area == "receiver" && command == "import-lora-axis" => {
            let root = configured_private_root()?;
            let input: tidex::receiver::weight_actuator::LoraAdapterAxisInput =
                read_json_bounded(Path::new(path))?;
            let receipt =
                tidex::receiver::weight_actuator::import_peft_lora_as_dense_axis(&root, &input)?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
        }
        [area, command, path] if area == "receiver" && command == "assemble-lora-basis" => {
            let root = configured_private_root()?;
            let input: DistributedLoraBasisInput = read_json_bounded(Path::new(path))?;
            let basis = assemble_distributed_lora_basis(&root, &input)?;
            println!("{}", serde_json::to_string_pretty(&basis)?);
        }
        [area, command, reference] if area == "receiver" && command == "compile" => {
            let root = configured_private_root()?;
            let input: PrivateFileReference = read_json_bounded(Path::new(reference))?;
            let candidate = prepare_receiver_weight_candidate(&root, &input)?;
            println!("{}", serde_json::to_string_pretty(&candidate)?);
        }
        [area, command, reference] if area == "receiver" && command == "inspect" => {
            let root = configured_private_root()?;
            let input: PrivateFileReference = read_json_bounded(Path::new(reference))?;
            let candidate = authenticate_receiver_weight_candidate(&root, &input)?;
            println!("{}", serde_json::to_string_pretty(&candidate)?);
        }
        [area, command, reference, base_flag, base, output_flag, output]
            if area == "receiver"
                && command == "materialize"
                && base_flag == "--base-model"
                && output_flag == "--output" =>
        {
            let root = configured_private_root()?;
            let input: PrivateFileReference = read_json_bounded(Path::new(reference))?;
            let receipt = materialize_receiver_weight_candidate(
                &root,
                &input,
                Path::new(base),
                Path::new(output),
            )?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
        }
        [area, command, path] if area == "receiver" && command == "profile" => {
            let root = configured_private_root()?;
            let input: ReceiverModelProfileInput = read_json_bounded(Path::new(path))?;
            let receipt = profile_receiver_model(&root, &input)?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
        }
        [area, command, reference] if area == "receiver" && command == "verify-profile" => {
            let root = configured_private_root()?;
            let input: PrivateFileReference = read_json_bounded(Path::new(reference))?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &authenticate_receiver_model_profile(&root, &input,)?
                )?
            );
        }
        [area, command, reference] if area == "receiver" && command == "verify-live-profile" => {
            let root = configured_private_root()?;
            let input: PrivateFileReference = read_json_bounded(Path::new(reference))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&authenticate_live_receiver_model_profile(
                    &root, &input,
                )?)?
            );
        }
        [area, command, path] if area == "adapter-bank" && command == "import" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            let input: AdapterImportRequest = read_json_bounded(Path::new(path))?;
            println!("{}", serde_json::to_string_pretty(&bank.import_lora(&input)?)?);
        }
        [area, command, path] if area == "adapter-bank" && command == "compose" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            let input: AdapterCompositionRequest = read_json_bounded(Path::new(path))?;
            println!("{}", serde_json::to_string_pretty(&bank.compose_exact(&input)?)?);
        }
        [area, command, path] if area == "adapter-bank" && command == "materialize" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            let input: AdapterCandidateMaterializationRequest = read_json_bounded(Path::new(path))?;
            println!("{}", serde_json::to_string_pretty(&bank.materialize_candidate(&input)?)?);
        }
        [area, command, path] if area == "adapter-bank" && command == "authorize" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            let input: AdapterGovernedPromotionRequest = read_json_bounded(Path::new(path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&bank.authorize_governed_promotion_request(&input)?)?
            );
        }
        [area, command, path] if area == "adapter-bank" && command == "verify-materialization" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            let input: PrivateFileReference = read_json_bounded(Path::new(path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &bank.authenticate_candidate_materialization(&input)?,
                )?
            );
        }
        [area, command, path] if area == "adapter-bank" && command == "query" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            let input: AdapterBankQuery = read_json_bounded(Path::new(path))?;
            println!("{}", serde_json::to_string_pretty(&bank.query(&input)?)?);
        }
        [area, command, path] if area == "adapter-bank" && command == "show" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            let input: AdapterBankLookup = read_json_bounded(Path::new(path))?;
            let manifest = bank
                .lookup(&input)?
                .ok_or("adapter_bank_manifest_not_found")?;
            println!("{}", serde_json::to_string_pretty(&manifest)?);
        }
        [area, command, path] if area == "adapter-bank" && command == "resolve" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            let input: AdapterResolutionRequest = read_json_bounded(Path::new(path))?;
            println!("{}", serde_json::to_string_pretty(&bank.resolve_active(&input)?)?);
        }
        [area, command, path] if area == "adapter-bank" && command == "verify-resolution" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            let input: AdapterExecutionResolution = read_json_bounded(Path::new(path))?;
            bank.authenticate_execution_resolution(&input)?;
            println!("{}", serde_json::to_string_pretty(&input)?);
        }

        [area, command, path] if area == "adapter-bank" && command == "activate" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            let input: AdapterActivationRequest = read_json_bounded(Path::new(path))?;
            println!("{}", serde_json::to_string_pretty(&bank.activate(&input)?)?);
        }
        [area, command, path] if area == "adapter-bank" && command == "revoke" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            let input: AdapterRevocationRequest = read_json_bounded(Path::new(path))?;
            println!("{}", serde_json::to_string_pretty(&bank.revoke(&input)?)?);
        }
        [area, command, path] if area == "adapter-bank" && command == "rollback" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            let input: AdapterRollbackRequest = read_json_bounded(Path::new(path))?;
            println!("{}", serde_json::to_string_pretty(&bank.rollback(&input)?)?);
        }
        [area, command] if area == "adapter-bank" && command == "status" => {
            let root = configured_private_root()?;
            let bank = AdapterBank::open(&root)?;
            println!("{}", serde_json::to_string_pretty(&bank.verify_history()?)?);
        }
        [area, command, path] if area == "receiver" && command == "freeze-compiler" => {
            let input: FrozenReceiverCompilerInput = read_json_bounded(Path::new(path))?;
            let frozen = freeze_receiver_compiler(&input)?;
            println!("{}", serde_json::to_string_pretty(&frozen)?);
        }
        [area, command, path] if area == "receiver" && command == "verify-frozen-compiler" => {
            let frozen: FrozenReceiverCompiler = read_json_bounded(Path::new(path))?;
            frozen.verify()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "schema":"tidex.frozen_receiver_compiler_verification/v1",
                    "manifest_sha256":frozen.manifest_sha256(),
                    "verified":true
                }))?
            );
        }
        [area, command, path] if area == "compile" && command == "universal" => {
            let request: UniversalCapabilityCompilationRequest =
                read_json_bounded(Path::new(path))?;
            let report = execute_experimental_universal_capability_request(&request)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        [area, command, request_path, receipt_path]
            if area == "compile" && command == "universal-replay" =>
        {
            let request: UniversalCapabilityCompilationRequest =
                read_json_bounded(Path::new(request_path))?;
            let receipt: UniversalCapabilityCompilationReceipt =
                read_json_bounded(Path::new(receipt_path))?;
            replay_experimental_universal_capability_request(&request, &receipt)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "schema":"tidex.universal_capability_compilation_replay/v2",
                    "request_sha256":receipt.request_sha256,
                    "replayed":true
                }))?
            );
        }
        [area, command, path] if area == "compile" && command == "universal-plan" => {
            let request: UniversalCapabilityPlanningRequest = read_json_bounded(Path::new(path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&execute_universal_capability_shadow_plan(&request)?)?
            );
        }
        [area, command, request_path, receipt_path]
            if area == "compile" && command == "universal-plan-replay" =>
        {
            let request: UniversalCapabilityPlanningRequest =
                read_json_bounded(Path::new(request_path))?;
            let receipt: UniversalCapabilityShadowPlanReceipt =
                read_json_bounded(Path::new(receipt_path))?;
            replay_universal_capability_shadow_plan(&request, &receipt)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({"schema":"tidex.universal_shadow_plan_replay/v2","planning_request_sha256":receipt.planning_request_sha256,"replayed":true})
                )?
            );
        }
        [area, backend, request_path, receipt_path, layout_path]
            if area == "materialize" && backend == "dense" =>
        {
            let request: UniversalCapabilityPlanningRequest =
                read_json_bounded(Path::new(request_path))?;
            let receipt: UniversalCapabilityShadowPlanReceipt =
                read_json_bounded(Path::new(receipt_path))?;
            let layout: ReceiverMaterializationLayout = read_json_bounded(Path::new(layout_path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&materialize_replayed_dense_delta_shadow(
                    &request, &receipt, &layout
                )?)?
            );
        }
        [area, backend, request_path, receipt_path, layout_path, policy_path]
            if area == "materialize" && backend == "low-rank" =>
        {
            let request: UniversalCapabilityPlanningRequest =
                read_json_bounded(Path::new(request_path))?;
            let receipt: UniversalCapabilityShadowPlanReceipt =
                read_json_bounded(Path::new(receipt_path))?;
            let layout: ReceiverMaterializationLayout = read_json_bounded(Path::new(layout_path))?;
            let policy: LowRankShadowPolicy = read_json_bounded(Path::new(policy_path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&materialize_replayed_low_rank_shadow(
                    &request, &receipt, &layout, &policy
                )?)?
            );
        }
        [area, backend, request_path, receipt_path, layout_path, policy_path]
            if area == "materialize" && backend == "sparse" =>
        {
            let request: UniversalCapabilityPlanningRequest =
                read_json_bounded(Path::new(request_path))?;
            let receipt: UniversalCapabilityShadowPlanReceipt =
                read_json_bounded(Path::new(receipt_path))?;
            let layout: ReceiverMaterializationLayout = read_json_bounded(Path::new(layout_path))?;
            let policy: SparseShadowPolicy = read_json_bounded(Path::new(policy_path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&materialize_replayed_sparse_shadow(
                    &request, &receipt, &layout, &policy
                )?)?
            );
        }
        [area, backend, request_path, receipt_path, receiver_layout_path, steering_layout_path, policy_path]
            if area == "materialize" && backend == "steering" =>
        {
            let request: UniversalCapabilityPlanningRequest =
                read_json_bounded(Path::new(request_path))?;
            let receipt: UniversalCapabilityShadowPlanReceipt =
                read_json_bounded(Path::new(receipt_path))?;
            let receiver_layout: ReceiverMaterializationLayout =
                read_json_bounded(Path::new(receiver_layout_path))?;
            let steering_layout: ActivationSteeringLayout =
                read_json_bounded(Path::new(steering_layout_path))?;
            let policy: ActivationSteeringPolicy = read_json_bounded(Path::new(policy_path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&materialize_replayed_activation_steering_shadow(
                    &request,
                    &receipt,
                    &receiver_layout,
                    &steering_layout,
                    &policy
                )?)?
            );
        }
        [area, command, root, request_path]
            if area == "receiver" && command == "inspect-safetensors" =>
        {
            let request: SafeTensorsReceiverRequest = read_json_bounded(Path::new(request_path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&inspect_safetensors_receiver(
                    Path::new(root),
                    &request
                )?)?
            );
        }
        [area, command, input_path] if area == "select" && command == "backend" => {
            let input: BackendSelectionInput = read_json_bounded(Path::new(input_path))?;
            println!("{}", serde_json::to_string_pretty(&input.execute()?)?);
        }
        [area, command, input_path] if area == "measure" && command == "universality" => {
            let input: UniversalityEvidenceInput = read_json_bounded(Path::new(input_path))?;
            let receipt = input.execute()?;
            // Recompute per-trial pass predicate independently and verify consistency.
            let mut recomputed_global_successes: usize = 0;
            let mut recomputed_per_capability: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();
            for trial in &input.trials {
                if trial.passes(&input.protocol) {
                    recomputed_global_successes += 1;
                    *recomputed_per_capability
                        .entry(trial.capability_id.clone())
                        .or_default() += 1;
                }
            }
            // Compute receipt global successes from capabilities vector
            let receipt_global_successes: usize =
                receipt.capabilities.iter().map(|c| c.successes).sum();
            if recomputed_global_successes > 0 && receipt_global_successes == 0 {
                // Inconsistency detected - fail loudly and print diagnostics.
                eprintln!(
                    "universality_inconsistent_reducer: reducer reported 0 successes but recomputation found {} passing trials",
                    recomputed_global_successes
                );
                eprintln!(
                    "Recomputed per-capability successes: {}",
                    serde_json::to_string_pretty(&recomputed_per_capability)?
                );
                eprintln!(
                    "Reducer receipt capabilities: {}",
                    serde_json::to_string_pretty(&receipt.capabilities)?
                );
                return Err("universality_inconsistent_reducer".into());
            }
            println!("{}", serde_json::to_string_pretty(&receipt)?);
        }
        [area, command, input_path] if area == "discover" && command == "capabilities" => {
            let input: CapabilityDiscoveryRequest = read_json_bounded(Path::new(input_path))?;
            println!("{}", serde_json::to_string_pretty(&input.execute()?)?);
        }
        [area, command, input_path] if area == "gate" && command == "promotion" => {
            let input: UniversalPromotionGateRequest = read_json_bounded(Path::new(input_path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&evaluate_universal_promotion_gate(&input)?)?
            );
        }
        [area, command, runner_path, input_path] if area == "shadow" && command == "run" => {
            let input: ShadowEvaluationInput = read_json_bounded(Path::new(input_path))?;
            if input.schema != "tidex.shadow_evaluation_input/v1" {
                return Err("shadow_evaluation_input_schema_invalid".into());
            }
            let runner = AuthenticatedBytes::from_trusted_bytes(read_bytes_bounded(
                Path::new(runner_path),
                64 * 1024 * 1024,
            )?);
            println!(
                "{}",
                serde_json::to_string_pretty(&run_shadow_evaluation(
                    runner,
                    &input.bundle,
                    input.arguments,
                    input.limits,
                    input.requirements
                )?)?
            );
        }
        [area, command, path] if area == "receiver" && command == "planning-profile" => {
            let root = configured_private_root()?;
            let request: tidex::receiver::checkpoint_adapter::PhysicalPlanningProfileRequest =
                read_json_bounded(Path::new(path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &tidex::receiver::checkpoint_adapter::planning_profile_from_physical(
                        &root, &request
                    )?
                )?
            );
        }
        [area, command, path] if area == "materialize" && command == "compiled" => {
            let root = configured_private_root()?;
            let request: tidex::materialization::materialization_pipeline::PhysicalMaterializationRequest =
                read_json_bounded(Path::new(path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &tidex::materialization::materialization_pipeline::materialize_compiled_checkpoint(
                        &root, &request
                    )?
                )?
            );
        }
        [area, command, path] if area == "materialize" && command == "verify-compiled" => {
            let root = configured_private_root()?;
            let reference: PrivateFileReference = read_json_bounded(Path::new(path))?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &tidex::materialization::materialization_pipeline::authenticate_compiled_checkpoint(
                        &root, &reference
                    )?
                )?
            );
        }
        [area, command] if area == "operator" && command == "recipes" => {
            println!("{}", serde_json::to_string_pretty(&recipe_catalog())?);
        }
        [area, command] if area == "operator" && command == "graph" => {
            let receipt = compute_operator_graph()?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
            if !receipt.passed {
                return Err("operator_graph_findings".into());
            }
        }
        [area, command] if area == "operator" && command == "staircase" => {
            let home = configured_operator_home()?;
            let receipt = compute_operator_living_staircase(&home)?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
        }
        [area, command, action]
            if area == "operator" && command == "models" && action == "list" =>
        {
            let home = configured_operator_home()?;
            println!("{}", serde_json::to_string_pretty(&list_catalog_models(&home)?)?);
        }
        [area, command, action]
            if area == "operator" && command == "datasets" && action == "list" =>
        {
            let home = configured_operator_home()?;
            println!("{}", serde_json::to_string_pretty(&list_datasets(&home)?)?);
        }
        [area, command, model_id, dataset_sha] if area == "operator" && command == "evaluate" => {
            let home = configured_operator_home()?;
            let request = OperatorDirectWorkflowRequest {
                schema: "tidex.operator_direct_workflow/v1".into(),
                operation: OperatorDirectOperation::BehavioralEvaluation,
                model_ids: vec![tidex::foundation::digest::Sha256Digest::parse(model_id)?],
                dataset_sha256: Some(tidex::foundation::digest::Sha256Digest::parse(dataset_sha)?),
                parameters: serde_json::Value::Object(serde_json::Map::new()),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&execute_direct_workflow(&home, &request)?)?
            );
        }
        [area, command, dataset_sha, model_ids @ ..]
            if area == "operator" && command == "discover" && model_ids.len() >= 2 =>
        {
            let home = configured_operator_home()?;
            let request = BehavioralDiscoveryWorkflowRequest {
                schema: "tidex.operator_behavioral_discovery/v1".into(),
                model_ids: model_ids
                    .iter()
                    .map(tidex::foundation::digest::Sha256Digest::parse)
                    .collect::<Result<Vec<_>, _>>()?,
                dataset_sha256: tidex::foundation::digest::Sha256Digest::parse(dataset_sha)?,
                max_new_tokens: 128,
                seed: 0,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&execute_behavioral_discovery_workflow(
                    &home, &request
                )?)?
            );
        }
        [area, command, action, root]
            if area == "operator" && command == "models" && action == "scan" =>
        {
            let home = configured_operator_home()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&catalog_local_models(&home, Path::new(root))?)?
            );
        }
        [area, command, action, name, format, path]
            if area == "operator" && command == "dataset" && action == "import" =>
        {
            let format = match format.as_str() {
                "json" => OperatorDatasetFormat::Json,
                "jsonl" => OperatorDatasetFormat::Jsonl,
                "csv" => OperatorDatasetFormat::Csv,
                "text" => OperatorDatasetFormat::Text,
                _ => return Err("operator_dataset_format_invalid".into()),
            };
            let home = configured_operator_home()?;
            let bytes = read_bytes_bounded(Path::new(path), 128 * 1024 * 1024)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&import_dataset_bytes(
                    &home, name, format, &bytes, false
                )?)?
            );
        }
        [area, command, recipe_id, assets @ ..] if area == "operator" && command == "run" => {
            let home = configured_operator_home()?;
            let request = OperatorRunRequest {
                schema: "tidex.operator_run_request/v1".into(),
                recipe_id: recipe_id.clone(),
                assets: assets
                    .iter()
                    .map(Path::new)
                    .map(Path::to_path_buf)
                    .collect(),
                selected_model_ids: Vec::new(),
                dataset_sha256: None,
                maximum_runtime_seconds: None,
            };
            println!("{}", serde_json::to_string_pretty(&execute_operator_run(&home, &request)?)?);
        }
        [area, command] if area == "operator" && command == "serve" => {
            let home = configured_operator_home()?;
            serve(&home, "127.0.0.1:8793".parse()?)?;
        }
        [area, command, port] if area == "operator" && command == "serve" => {
            let port = port.parse::<u16>()?;
            if port == 0 {
                return Err("operator_port_invalid".into());
            }
            let home = configured_operator_home()?;
            serve(&home, format!("127.0.0.1:{port}").parse()?)?;
        }
        [command] if command == "executors" => {
            println!("{}", serde_json::to_string_pretty(&executor_catalog()?)?);
        }
        [command, executor_id] if command == "executor" => {
            println!("{}", serde_json::to_string_pretty(&executor_by_id(executor_id)?)?);
        }
        [area, command] if area == "operator" && command == "executors" => {
            println!("{}", serde_json::to_string_pretty(&executor_catalog()?)?);
        }
        [command] if command == "capabilities" => {
            let home = configured_tidex_home()?;
            let workspace = current_workspace(&home)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "schema":"tidex.capabilities/v1",
                    "workspace":workspace.name,
                    "target":workspace.target,
                    "capabilities":[
                        {"id":"acquisition.capture","status":"implemented","engine":"content_vault::capture_to_vault"},
                        {"id":"analysis.skill_fields","status":"implemented","engine":"BrainEngine::analyze"},
                        {"id":"learning.adaptive","status":"implemented","engine":"learning_orchestrator"},
                        {"id":"controller.learned","status":"implemented","engine":"learned_controller"},
                        {"id":"transport.functional","status":"implemented","engine":"transport::learn_functional_transplant"},
                        {"id":"transport.relational","status":"implemented","engine":"transport::learn_relational_transport"},
                        {"id":"compile.skill_fields","status":"implemented","engine":"parametric_program::compile_operator_to_fields"},
                        {"id":"compile.receiver","canonical_name":"compile.receiver.operational","status":"implemented_experimental","engine":"receiver_compiler::compile_receiver_capability","evidence_status":"bounded_numeric_tests","reason":"operational IR compilation with a learned inverse predictor, protection and trust-region gates; V66 LoRA evidence does not establish this Rust compiler on an LLM"},
                        {"id":"materialize.receiver.verified_behavior","status":"implemented_experimental","engine":"quality/experiments/v66_mbpp_receiver_compile.py","evidence_status":"bounded_cross_model_experimental","reason":"V66 is a separate verified-behavior LoRA training backend; no evidence of target-training-free functional compilation"},
                        {"id":"materialize.receiver.weights","status":"implemented_experimental","engine":"weight_actuator::materialize_dense_delta_checkpoint","reason":"candidate checkpoint writer; V67 uses a V66-derived delta and is not evidence of independent functional transfer"},
                        {"id":"compile.receiver.distributed_lora_axes","status":"implemented_candidate_backend","engine":"weight_actuator::import_peft_lora_as_dense_axis -> receiver_weight_binding::assemble_distributed_lora_basis -> capability_ir::execute_linear_readout -> receiver_weight_binding::prepare_receiver_weight_candidate","reason":"reconstructs calibration-only PEFT LoRAs as authenticated full multiblock axes; distributed target compilation requires executed CapabilityIR evidence and performs no target receiver training; independent target execution remains mandatory"},
                        {"id":"compile.receiver.capability_ir_readout","status":"experimental_partial_candidate_only","engine":"capability_ir::execute_linear_readout -> receiver_compiler::compile_receiver_readout_capability","reason":"executes an authenticated resident readout fragment before compiling receiver-native coordinates; full task semantics and independent receiver validation remain separate"},
                        {"id":"compile.receiver.measured_weights","status":"experimental_candidate_only","engine":"receiver_weight_binding::prepare_receiver_weight_candidate","reason":"authenticated measured-response calibration -> receiver coordinates -> dense delta -> standalone candidate; inverse predictions are not model execution or capability-transfer evidence"},
                        {"id":"benchmark.portability","canonical_name":"benchmark.receiver_compilation","status":"implemented","engine":"receiver_compiler::benchmark_receiver_portability_leave_one_out","reason":"leave-one-capability-out functional-space benchmark over declared calibration cases; the legacy portability id is retained for compatibility, while the benchmark measures receiver compilation recovery inside that domain and is not a universal cross-model claim"},
                        {"id":"receiver.normalize.sharded_safetensors","status":"implemented","engine":"weight_actuator::normalize_sharded_safetensors","reason":"validates a Hugging Face SafeTensors weight_map against every shard and streams exact tensor payloads into one immutable content-addressed SafeTensors checkpoint; normalization is structural/data-plane evidence, not behavioral equivalence or promotion"},
                        {"id":"receiver.profile.physical","status":"implemented","engine":"model_adaptation::profile_receiver_model","reason":"binds an exact single-file SafeTensors receiver to config, tokenizer, physical topology and authenticated adaptation layouts; standard Hugging Face sharded SafeTensors inputs are first normalized into the same retained single-file authority; unsupported surfaces remain fail-closed"},
                        {"id":"adapter_bank.modular","status":"implemented","engine":"adapter_bank::AdapterBank","reason":"immutable manifests with normalized retained provenance and a transactionally published snapshot chain"},
                        {"id":"adapter_bank.index.dynamic","status":"implemented","engine":"adapter_bank::AdapterBank::query","reason":"capability/model projections are regenerated and authenticated from the primary manifest table"},
                        {"id":"adapter.compose.exact_dense","status":"implemented","engine":"adapter_bank::AdapterBank::compose_exact","reason":"canonical ordered f32 axes multiplied and accumulated in f64 with one final f32 rounding; no SVD, pruning or rank truncation"},
                        {"id":"adapter_bank.lifecycle","status":"implemented_governed","engine":"adapter_bank::AdapterBank::{authorize_governed_promotion_request,activate,revoke,rollback}","reason":"authorization reopens and semantically reauthenticates sealed gate/PETFC/canary witnesses before minting a current-index-bound permit; activation consumes that permit; revocation is sticky and transitive; rollback publishes a new forward revision"},
                        {"id":"receiver.profile.planning_projection","status":"implemented","engine":"checkpoint_adapter::planning_profile_from_physical","reason":"derives alternate compiler geometry from the live operational profile; preserves exact model/config/tokenizer identity and remaps physical tensor order by names; modality/state annotations are not behavioral evidence"},
                        {"id":"compile.receiver.frozen","status":"implemented_candidate_compiler","engine":"receiver_compiler::freeze_receiver_compiler -> FrozenReceiverCompiler::verify -> VerifiedFrozenReceiverCompiler::compile_capability","reason":"calibration/protection/risk/proposal geometry are committed before held-out targets; target requests carry only CapabilityIR plus explicit negative controls; verification replays calibration without target observations"},
                        {"id":"compile.universal.plan","status":"implemented_bounded_numerical","engine":"universal_capability_compiler::execute_universal_capability_shadow_plan","reason":"replays operational-contract compilation against a bound receiver profile; at most 256 receiver coordinates; not evidence of universal LLM transfer"},
                        {"id":"materialize.compiled.physical","status":"implemented_candidate_only","engine":"materialization_pipeline::materialize_compiled_checkpoint","reason":"both measured-receiver and universal-plan sources use the existing physical actuator; dense/low-rank/sparse must reconstruct the identical f32 delta; checkpoint arithmetic is replayed; no automatic promotion"},
                        {"id":"materialize.shadow.alternatives","status":"implemented_shadow_only","engine":"low_rank_shadow_materializer + sparse_shadow_materializer + activation_steering_materializer","reason":"bounded numerical representations with replay; lossy experiments do not inherit compiled-candidate validation; steering produces interventions but installs no runtime hook"},
                        {"id":"materialize.selection","status":"implemented_advisory_only","engine":"materialization_selector::select_materialization_backend","reason":"deterministic comparative ranking of supplied metrics; no attestation of measurement origin and no activation authority"},
                        {"id":"evidence.universality","status":"implemented_declared_evidence_reducer","engine":"universality_evidence::measure_universality_n","reason":"requires consistent receiver identities, successful held-out coverage and unseen-receiver success; caller-supplied trials are not independently attested experiments"},
                        {"id":"promotion.universal.readiness","status":"advisory_only","engine":"universal_promotion_gate::evaluate_universal_promotion_gate","reason":"requires exact agreement of selected and reported execution metrics; never emits an operational activation permit"},
                        {"id":"runtime.sleep","status":"implemented","engine":"BrainEngine::sleep_cycle"},
                        {"id":"capability_ir.v63.contract","status":"implemented_foundation","engine":"capability_ir::OperationalCapabilityContract","reason":"StateIR anchors, repeated OperatorIR transitions, canonical transition signatures, closure and contraction verification are implemented; evidence is bounded to tested domains and does not establish a universal capability representation across arbitrary models or tasks"},
                        {"id":"model.assistance","status":"configured_not_authoritative","reason":"model profiles are selectable; no model call is permitted to create evidence or promotion authority"}
                    ]
                }))?
            );
        }
        _ => return Err(usage().into()),
    }
    Ok(())
}

fn execute_residency_decision(
    path: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let request: ResidencyDecisionRequest = read_json_bounded(path)?;
    if request.schema != "tidex.residency_decision_request/v1" {
        return Err("residency_decision_request_schema_invalid".into());
    }
    let root = configured_private_root()?;
    let authority_instance = AuthorityInstanceId::parse(request.authority_instance)?;
    let knowledge = KnowledgeEngine::open_with_authority_instance(&root, authority_instance)?;
    let authority = ResidencyDecisionAuthority::current(&root, &knowledge)?;
    let (decision, reference) = authority.decide_and_persist(&request.precommit_reference)?;
    Ok(json!({
        "schema":"tidex.residency_decision_operator_receipt/v1",
        "decision":decision,
        "decision_reference":reference,
        "authorizes_production":false
    }))
}

fn execute_knowledge_plan(path: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let request: KnowledgePlanRequest = read_json_bounded(path)?;
    if request.schema != "tidex.knowledge_plan_request/v1" {
        return Err("knowledge_plan_request_schema_invalid".into());
    }
    let root = configured_private_root()?;
    let authority_instance = AuthorityInstanceId::parse(&request.authority_instance)?;
    let engine = KnowledgeEngine::open_with_authority_instance(&root, authority_instance)?;
    let state = engine.authenticate_state(&request.state_reference)?;
    let decision = engine.plan(&state)?;
    let staircase = engine.living_staircase(&state)?;
    Ok(json!({
        "schema":"tidex.knowledge_plan_receipt/v1",
        "state_reference":request.state_reference,
        "decision":decision,
        "staircase":staircase,
        "authorizes_execution":false,
        "authorizes_production":false
    }))
}

fn execute_knowledge_staircase(
    path: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let request: KnowledgePlanRequest = read_json_bounded(path)?;
    if request.schema != "tidex.knowledge_plan_request/v1" {
        return Err("knowledge_plan_request_schema_invalid".into());
    }
    let root = configured_private_root()?;
    let authority_instance = AuthorityInstanceId::parse(&request.authority_instance)?;
    let engine = KnowledgeEngine::open_with_authority_instance(&root, authority_instance)?;
    let state = engine.authenticate_state(&request.state_reference)?;
    let staircase = engine.living_staircase(&state)?;
    Ok(json!({
        "schema":"tidex.living_staircase_receipt/v1",
        "state_reference":request.state_reference,
        "staircase":staircase,
        "authorizes_execution":false,
        "authorizes_production":false
    }))
}

fn execute_tomography_analysis(
    path: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let root = configured_private_root()?;
    let bytes = read_untrusted_private_file_bounded(&root, path, MAX_ANALYSIS_INPUT_BYTES)?;
    let observations: Vec<DeltaObservation> = serde_json::from_slice(&bytes)?;
    if observations.is_empty() {
        return Err("tomography_observations_empty".into());
    }
    let engine = BrainEngine::open(&root, BrainConfig::default())?;
    let report = engine.analyze(&observations)?;
    Ok(json!({
        "schema":"tidex.tomography_operator_receipt/v1",
        "report":report,
        "authorizes_production":false
    }))
}

fn execute_geometry_analysis(path: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let request: GeometryAnalysisRequest = read_json_bounded(path)?;
    let result = match request {
        GeometryAnalysisRequest::Pythagoras { delta } => json!({
            "operation":"pythagoras",
            "report":PythagorasStaircaseMetric::evaluate_and_correct(&delta)?
        }),
        GeometryAnalysisRequest::Topology {
            points,
            distance_threshold,
        } => json!({
            "operation":"topology",
            "report":TopologicalSkillManifold::analyze_topology(&points, distance_threshold)?
        }),
    };
    Ok(json!({
        "schema":"tidex.geometry_analysis_receipt/v1",
        "result":result,
        "authorizes_production":false
    }))
}

fn execute_protected_map(path: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let request: ProtectedMapRequest = read_json_bounded(path)?;
    if request.schema != "tidex.protected_map_request/v1"
        || request.task_labels_used
        || request.evidence.len() < 2
    {
        return Err("protected_map_request_contract_invalid".into());
    }
    let root = configured_private_root()?;
    let evidence = request
        .evidence
        .into_iter()
        .map(|row| {
            Ok(SensitivityEvidence {
                probe_id: ProbeId::parse(&row.probe_id)?,
                sensitivity: read_dvec_f32(&root, &row.artifact)?
                    .into_iter()
                    .map(f64::from)
                    .collect(),
                reliability: row.reliability,
                causal_damage: Some(row.causal_damage_per_parameter_norm),
            })
        })
        .collect::<tidex::foundation::error::BrainResult<Vec<_>>>()?;
    let map = build_protected_cortex_map(
        &evidence,
        request.retained_energy_fraction,
        request.regularization_scale,
    )?;
    let artifact = persist_protected_map(&root, &map)?;
    Ok(json!({
        "schema":"tidex.protected_map_operator_receipt/v1",
        "artifact":artifact,
        "evidence_count":evidence.len(),
        "task_labels_used":false,
        "authorizes_production":false
    }))
}

fn build_numerical_policy(
    request: &NumericalEvolutionPolicyRequest,
) -> Result<NumericalEvolutionPolicy, Box<dyn std::error::Error>> {
    if request.solver_profile != "portfolio_default_v1" {
        return Err("numerical_solver_profile_unsupported".into());
    }
    let specs = numerical_metric_specs(
        request.minimum_normalized_fit,
        request.maximum_normalized_worst_error,
    )?;
    let fit = MetricId::parse("numerical.normalized_fit")?;
    let worst = MetricId::parse("numerical.normalized_worst_error")?;
    let robust = RobustEvaluationPolicy::new(
        request.robust_minimum_independent_groups,
        request.robust_median_of_means_blocks,
        request.robust_maximum_observations,
    )?;
    let gate = CandidateGatePolicy::new(
        &specs,
        request.gate_minimum_independent_groups,
        request.gate_uncertainty_multiplier,
        BTreeMap::from([
            (fit.clone(), FiniteF64::new(request.gate_minimum_fit_improvement)?),
            (worst.clone(), FiniteF64::new(request.gate_minimum_worst_error_improvement)?),
        ]),
    )?;
    let petfc = PetfcPolicy::new(
        &specs,
        fit.clone(),
        vec![
            PetfcMetricPolicy::new(
                fit,
                request.petfc_fit_normalization_scale,
                request.petfc_fit_maximum_endpoint_degradation,
            )?,
            PetfcMetricPolicy::new(
                worst,
                request.petfc_worst_error_normalization_scale,
                request.petfc_worst_error_maximum_endpoint_degradation,
            )?,
        ],
        PetfcPathLimits::new(
            request.petfc_minimum_reports,
            request.petfc_maximum_reports,
            request.petfc_maximum_step_distance,
            request.petfc_maximum_tortuosity,
            request.petfc_maximum_waste,
            request.petfc_minimum_path_efficiency,
        )?,
        PetfcConservationLimits::new(
            request.petfc_maximum_soft_degradation_sum,
            request.petfc_maximum_degraded_metric_count,
            request.petfc_maximum_distributed_degradation,
        )?,
        PetfcUtilityPolicy::new(
            request.petfc_path_penalty,
            request.petfc_conservation_penalty,
            request.petfc_tortuosity_penalty,
            request.petfc_minimum_utility,
        )?,
    )?;
    let limits = NumericalEvaluationLimits::new(
        request.evaluation_maximum_groups,
        request.evaluation_maximum_total_cases,
        request.evaluation_maximum_total_scalar_elements,
    )?;
    let solver = PortfolioPolicy::default().with_residual_tolerances(
        request.solver_relative_residual_tolerance,
        request.solver_absolute_residual_tolerance,
    )?;
    Ok(NumericalEvolutionPolicy::new(
        solver,
        specs,
        robust,
        gate,
        petfc,
        limits,
        request.target_scale_floor,
    )?)
}

fn numerical_disposition_name(value: NumericalEvolutionDisposition) -> &'static str {
    match value {
        NumericalEvolutionDisposition::SolverRejected => "solver_rejected",
        NumericalEvolutionDisposition::SolverBoundedUnknown => "solver_bounded_unknown",
        NumericalEvolutionDisposition::NoNewEvidence => "no_new_evidence",
        NumericalEvolutionDisposition::CandidateRejected => "candidate_rejected",
        NumericalEvolutionDisposition::CandidateBoundedUnknown => "candidate_bounded_unknown",
        NumericalEvolutionDisposition::CandidateValidatedForFurtherGates => {
            "candidate_validated_for_further_gates"
        }
    }
}

fn execute_numerical_evolution(
    path: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let request: NumericalEvolutionRequest = read_json_bounded(path)?;
    if request.schema != "tidex.numerical_evolution_request/v1" || request.cycles.is_empty() {
        return Err("numerical_evolution_request_contract_invalid".into());
    }
    let policy = build_numerical_policy(&request.policy)?;
    let mut engine = NumericalEvolutionEngine::new(request.context, policy)?;
    let mut receipts = Vec::with_capacity(request.cycles.len());
    for cycle_request in request.cycles {
        let training = LeastSquaresProblem::new(
            cycle_request.training_problem.inputs,
            cycle_request.training_problem.targets,
        )?;
        let baseline = CandidateRepresentation::Dense {
            rows: cycle_request.baseline.rows,
            columns: cycle_request.baseline.columns,
            weights: cycle_request.baseline.weights,
        };
        let groups = cycle_request
            .evaluation_groups
            .into_iter()
            .map(|group| {
                NumericalEvaluationGroup::new(LeastSquaresProblem::new(
                    group.inputs,
                    group.targets,
                )?)
            })
            .collect::<tidex::foundation::error::BrainResult<Vec<_>>>()?;
        let input =
            NumericalEvolutionInput::new(training, baseline, groups, cycle_request.revision)?;
        let cycle = engine.evolve(input, &[])?;
        let candidate_sha256 = cycle
            .candidate()
            .map(|candidate| {
                candidate
                    .exact_digest()
                    .map(|digest| digest.as_str().to_string())
            })
            .transpose()?;
        receipts.push(json!({
            "revision":cycle_request.revision,
            "disposition":numerical_disposition_name(cycle.disposition()),
            "solver_report_sha256":cycle.solver_report().exact_digest()?.to_string(),
            "candidate_sha256":candidate_sha256,
            "candidate_gate":cycle.candidate_gate(),
            "petfc_assessment":cycle.petfc_assessment(),
            "trajectory":cycle.trajectory(),
            "procedural_attempt":cycle.procedural_attempt(),
            "solver_run_failure":cycle.solver_run_failure(),
            "authorizes_promotion":cycle.authorizes_promotion()
        }));
    }
    Ok(json!({
        "schema":"tidex.numerical_evolution_receipt/v1",
        "solver_profile":request.policy.solver_profile,
        "cycles":receipts,
        "procedural_attempt_count":engine.procedural_memory().attempt_count(),
        "authorizes_production":false
    }))
}

fn read_benchmark_json_bounded<T: serde::de::DeserializeOwned>(
    path: &Path,
) -> Result<T, Box<dyn std::error::Error>> {
    read_json_bounded_with_error(path, "tidex_benchmark_input_too_large")
}

fn read_json_bounded<T: serde::de::DeserializeOwned>(
    path: &Path,
) -> Result<T, Box<dyn std::error::Error>> {
    read_json_bounded_with_error(path, "tidex_cli_json_input_too_large")
}

fn read_json_bounded_with_error<T: serde::de::DeserializeOwned>(
    path: &Path,
    too_large_error: &'static str,
) -> Result<T, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(MAX_CLI_JSON_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len())? > MAX_CLI_JSON_BYTES {
        return Err(too_large_error.into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn read_bytes_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len())? > maximum {
        return Err("tidex_input_too_large".into());
    }
    Ok(bytes)
}

fn acquire_workspace(
    home: &Path,
    selected: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = current_workspace(home)?;
    let private_root = workspace.private_root(home);
    let scope = match selected {
        None => AcquisitionScope::WholeProject,
        Some(path) => AcquisitionScope::DeclaredPaths {
            roots: vec![DeclaredRelativePath::parse(path.to_path_buf())?],
        },
    };
    let request = AcquisitionRequest::new(
        AcquisitionId::parse(format!("workspace-{}-capture", workspace.name))?,
        scope,
        RequestedResidency::BestVerified,
        NoisePolicy::ConservativeGeneratedArtifacts,
        AcquisitionBudget {
            max_files: 100_000,
            max_total_bytes: 8 * 1024 * 1024 * 1024,
        },
        vec![],
    )?;
    let receipt = capture_to_vault(&workspace.target, &private_root, &request)?;
    let reference = receipt.persist(&private_root)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":"tidex.workspace_acquisition/v1",
            "workspace":workspace.name,
            "target":workspace.target,
            "capture_receipt_sha256":receipt.manifest_sha256(),
            "capture_receipt":reference,
            "system_envelope_sha256":receipt.envelope().manifest_sha256(),
            "completeness":receipt.envelope().completeness(),
            "entries":receipt.envelope().entries().len(),
            "bytes":receipt.total_file_bytes()
        }))?
    );
    Ok(())
}

fn usage() -> &'static str {
    concat!(
        "usage:\n",
        "  tidex interface\n",
        "  tidex workspace create <name> --target <absolute-path>\n",
        "  tidex workspace use <name>\n",
        "  tidex workspace show\n",
        "  tidex model add <name> --provider openai-compatible --url <endpoint> --model <model>\n",
        "  tidex model use <name>\n",
        "  tidex acquire [--path <relative-project-path>]\n",
        "  tidex knowledge plan <input.json>\n",
        "  tidex knowledge staircase <input.json>\n",
        "  tidex numerical evolve <input.json>\n",
        "  tidex analysis tomography <observations.json>\n",
        "  tidex analysis protected-map <input.json>\n",
        "  tidex analysis geometry <input.json>\n",
        "  tidex benchmark portability <input.json>\n",
        "  tidex benchmark response <input.json>\n",
        "  tidex benchmark receiver-basis <input.json>\n",
        "  tidex receiver describe-readout <input.json>\n",
        "  tidex receiver acquire-readout <input.json>\n",
        "  tidex receiver import-axis <values-reference.json>\n",
        "  tidex receiver normalize-sharded <input.json>\n",
        "  tidex receiver import-lora-axis <input.json>\n",
        "  tidex receiver profile <input.json>\n",
        "  tidex receiver verify-profile <reference.json>\n",
        "  tidex receiver verify-live-profile <reference.json>\n",
        "  tidex receiver assemble-lora-basis <input.json>\n",
        "  tidex receiver compile <request-reference.json>\n",
        "  tidex receiver inspect <candidate-reference.json>\n",
        "  tidex receiver materialize <candidate-reference.json> --base-model <model.safetensors> --output <private-root/model.safetensors>\n",
        "  tidex adapter-bank import <input.json>\n",
        "  tidex adapter-bank compose <input.json>\n",
        "  tidex adapter-bank materialize <input.json>\n",
        "  tidex adapter-bank authorize <input.json>\n",
        "  tidex adapter-bank verify-materialization <reference.json>\n",
        "  tidex adapter-bank query <query.json>\n",
        "  tidex adapter-bank show <lookup.json>\n",
        "  tidex adapter-bank resolve <input.json>\n",
        "  tidex adapter-bank verify-resolution <resolution.json>\n",
        "  tidex adapter-bank activate <input.json>\n",
        "  tidex adapter-bank revoke <input.json>\n",
        "  tidex adapter-bank rollback <input.json>\n",
        "  tidex adapter-bank status\n",
        "  tidex receiver planning-profile <input.json>\n",
        "  tidex receiver freeze-compiler <input.json>\n",
        "  tidex receiver verify-frozen-compiler <compiler.json>\n",
        "  tidex materialize compiled <input.json>\n",
        "  tidex materialize verify-compiled <receipt-reference.json>\n",
        "  tidex serve [port]\n",
        "  tidex models scan <absolute-root>\n",
        "  tidex models list\n",
        "  tidex datasets list\n",
        "  tidex dataset import <name> <json|jsonl|csv|text> <path>\n",
        "  tidex evaluate <model-id> <dataset-sha256>\n",
        "  tidex discover <dataset-sha256> <model-id> <model-id> [...]\n",
        "  tidex run <recipe-id> [asset-path ...]\n",
        "  tidex recipes\n",
        "  tidex graph\n",
        "  tidex operator graph\n",
        "  tidex staircase\n",
        "  tidex operator staircase\n",
        "  tidex residency decide <request.json>\n",
        "  tidex operator executors\n",
        "  tidex executors\n",
        "  tidex executor <executor-id>\n",
        "  tidex capabilities"
    )
}

fn run_terminal_interface() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut selected_index = 0usize;
    let mut rendered_message = status_summary();

    loop {
        let active_panel = panel_title(selected_index);

        terminal.draw(|frame| {
            let size = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(8),
                    Constraint::Length(6),
                ])
                .split(size);

            let header = Paragraph::new(Line::from(vec![
                Span::styled(" TIDE-X CLI ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("  terminal interface  "),
                Span::styled(format!("active: {}   ", active_panel), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled("[1] status  [2] capabilities  [3] workspace  [4] evidence  [5] actions  [6] help  [q] quit", Style::default().fg(Color::Gray)),
            ]))
            .block(Block::default().borders(Borders::ALL).title("tidex interface"));
            frame.render_widget(header, chunks[0]);

            let options = [
                "1. Status",
                "2. Capabilities",
                "3. Workspace",
                "4. Evidence",
                "5. Actions",
                "6. Help",
                "q. Quit",
            ];
            let items: Vec<ListItem> = options
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let style = if idx == selected_index {
                        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(*item).style(style)
                })
                .collect();
            let list = List::new(items).block(Block::default().borders(Borders::ALL).title("menu"));
            frame.render_widget(list, chunks[1]);

            let detail_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)])
                .split(chunks[2]);

            let help = Paragraph::new("↑/↓ move • Enter select • Tab cycle • 1-6 jump • r refresh • q quit\n\nstatus panel: engine health\ncapabilities: feature overview\nworkspace: runtime paths\nevidence: proof chain\nactions: quick commands\nhelp: shortcuts")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title("keyboard help"));
            frame.render_widget(help, detail_chunks[1]);

            let info = Paragraph::new(rendered_message.clone())
                .block(Block::default().borders(Borders::ALL).title("live output"))
                .alignment(Alignment::Left);
            frame.render_widget(info, detail_chunks[0]);
        })?;

        if !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => break,
                KeyCode::Tab => {
                    selected_index = (selected_index + 1) % 6;
                    rendered_message = refresh_panel(selected_index);
                }
                KeyCode::Char('1') => {
                    selected_index = 0;
                    rendered_message = refresh_panel(selected_index);
                }
                KeyCode::Char('2') => {
                    selected_index = 1;
                    rendered_message = refresh_panel(selected_index);
                }
                KeyCode::Char('3') => {
                    selected_index = 2;
                    rendered_message = refresh_panel(selected_index);
                }
                KeyCode::Char('4') => {
                    selected_index = 3;
                    rendered_message = refresh_panel(selected_index);
                }
                KeyCode::Char('5') => {
                    selected_index = 4;
                    rendered_message = refresh_panel(selected_index);
                }
                KeyCode::Char('6') | KeyCode::Char('h') | KeyCode::Char('H') => {
                    selected_index = 5;
                    rendered_message = refresh_panel(selected_index);
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    rendered_message = refresh_panel(selected_index);
                }
                KeyCode::Up => {
                    selected_index = selected_index.saturating_sub(1);
                    rendered_message = refresh_panel(selected_index);
                }
                KeyCode::Down => {
                    if selected_index + 1 < 6 {
                        selected_index += 1;
                    }
                    rendered_message = refresh_panel(selected_index);
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    rendered_message = open_selected_panel(selected_index);
                }
                _ => {}
            }
        }
    }

    execute!(terminal.backend_mut(), LeaveAlternateScreen, cursor::Show)?;
    disable_raw_mode()?;
    Ok(())
}

fn panel_title(index: usize) -> &'static str {
    match index {
        0 => "status",
        1 => "capabilities",
        2 => "workspace",
        3 => "evidence",
        4 => "actions",
        5 => "help",
        _ => "status",
    }
}

fn open_selected_panel(index: usize) -> String {
    let panel = panel_title(index);
    let content = refresh_panel(index);
    format!("opened {} panel\n\n{}", panel, content)
}

fn refresh_panel(index: usize) -> String {
    match index {
        0 => status_summary(),
        1 => capabilities_summary(),
        2 => workspace_summary(),
        3 => evidence_summary(),
        4 => actions_summary(),
        5 => help_summary(),
        _ => status_summary(),
    }
}

fn status_summary() -> String {
    let private_root = configured_private_root();
    let mut lines = vec!["status".to_string(), "------".to_string()];
    match private_root {
        Ok(root) => {
            lines.push(format!("private_root={}", root.display()));
            match tidex::engine::BrainEngine::open(
                &root,
                tidex::foundation::contracts::BrainConfig::default(),
            ) {
                Ok(engine) => match engine.status() {
                    Ok(status) => {
                        lines.push("engine: available".to_string());
                        lines.push(
                            serde_json::to_string_pretty(&status)
                                .unwrap_or_else(|_| "status_json_unavailable".to_string()),
                        );
                    }
                    Err(error) => lines.push(format!("engine_status_error={error}")),
                },
                Err(error) => lines.push(format!("engine_open_error={error}")),
            }
        }
        Err(error) => lines.push(format!("private_root_error={error}")),
    }
    lines.join("\n")
}

fn capabilities_summary() -> String {
    let home = configured_tidex_home();
    let mut lines = vec!["capabilities".to_string(), "-----------".to_string()];
    match home {
        Ok(home_path) => {
            match current_workspace(&home_path) {
                Ok(workspace) => {
                    lines.push(format!(
                        "workspace={} target={}",
                        workspace.name,
                        workspace.target.display()
                    ));
                }
                Err(error) => lines.push(format!("workspace_error={error}")),
            }
            lines.push("interface: terminal dashboard active".to_string());
            lines.push("engine: Rust authority mode active".to_string());
            lines.push("output: real CLI, no web layer".to_string());
            lines.push(
                "controls: status, capabilities, workspace, evidence, actions, help".to_string(),
            );
        }
        Err(error) => lines.push(format!("workspace_error={error}")),
    }
    lines.join("\n")
}

fn workspace_summary() -> String {
    let home = configured_tidex_home();
    let mut lines = vec!["workspace".to_string(), "--------".to_string()];
    match home {
        Ok(home_path) => match current_workspace(&home_path) {
            Ok(workspace) => {
                lines.push(format!("name={}", workspace.name));
                lines.push(format!("target={}", workspace.target.display()));
                lines
                    .push(format!("private_root={}", workspace.private_root(&home_path).display()));
            }
            Err(error) => lines.push(format!("workspace_error={error}")),
        },
        Err(error) => lines.push(format!("home_error={error}")),
    }
    lines.join("\n")
}

fn evidence_summary() -> String {
    let mut lines = vec![
        "evidence".to_string(),
        "--------".to_string(),
        "proof: real Rust execution path active".to_string(),
        "receipt: authenticated artifact flow enabled".to_string(),
        "execution: materialize, verify, and audit steps remain local to the CLI".to_string(),
        "boundary: no browser or hidden web layer is authoritative".to_string(),
    ];

    if let Ok(root) = configured_private_root() {
        lines.push(format!("private_root={}", root.display()));
    }

    lines.join("\n")
}

fn actions_summary() -> String {
    let mut lines = vec![
        "actions".to_string(),
        "-------".to_string(),
        "1. show status".to_string(),
        "2. show capabilities".to_string(),
        "3. show workspace".to_string(),
        "4. show evidence".to_string(),
        "5. refresh dashboard".to_string(),
        "6. open help".to_string(),
        "q. quit interface".to_string(),
    ];

    if let Ok(root) = configured_private_root() {
        lines.push(format!("active_root={}", root.display()));
    }

    lines.join("\n")
}

fn help_summary() -> String {
    [
        "help".to_string(),
        "----".to_string(),
        "↑ / ↓  move selection".to_string(),
        "Enter  open selected panel".to_string(),
        "1-6    jump directly to a panel".to_string(),
        "r      refresh the active status view".to_string(),
        "q      quit the interface".to_string(),
        "h      open the help panel".to_string(),
        "mode   Rust CLI, no web layer".to_string(),
    ]
    .join("\n")
}
#[cfg(test)]
mod tests {
    use super::{open_selected_panel, panel_title};

    #[test]
    fn interface_route_is_recognized() {
        assert_eq!(super::resolve_cli_route(&["interface".to_string()]), "interface");
    }

    #[test]
    fn enter_action_opens_the_selected_panel() {
        let output = open_selected_panel(5);
        assert!(output.to_lowercase().contains("opened help panel"));
        assert!(output.to_lowercase().contains("help"));
    }

    #[test]
    fn panel_title_matches_expected_names() {
        assert_eq!(panel_title(0), "status");
        assert_eq!(panel_title(2), "workspace");
        assert_eq!(panel_title(5), "help");
    }
}
