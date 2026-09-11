//! Production-binary integration: physical profile -> alternative plan ->
//! existing actuator -> standalone checkpoint -> read-only arithmetic replay.
//! Small numerical fixtures test interoperability, not LLM capability transfer.
use serde::{de::DeserializeOwned, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};
use tidex::capability::acquisition_contract::{
    AcquisitionBudget, AcquisitionRequest, AcquisitionScope, NoisePolicy, RequestedResidency,
    SystemEnvelope,
};
use tidex::capability::capability_ir::{
    CapabilityIr, IrNode, OperationalCapabilityContract, OperatorIrTransition, OutputBinding,
    PrimitiveSet, StateIrAnchor, TypedPort, ValueReference,
};
use tidex::foundation::contracts::ProtectedCortex;
use tidex::foundation::identity::{
    AcquisitionId, ArchitectureId, CapabilityId, CapabilityNodeId, ModelId, PortId, PrimitiveId,
    TensorId,
};
use tidex::materialization::low_rank_shadow_materializer::LowRankShadowPolicy;
use tidex::materialization::materialization_pipeline::{
    CompiledMaterializationSource, PhysicalMaterializationBackend, PhysicalMaterializationOutcome,
    PhysicalMaterializationReceipt, PhysicalMaterializationRequest,
};
use tidex::materialization::sparse_shadow_materializer::SparseShadowPolicy;
use tidex::materialization::universal_capability_compiler::{
    UniversalCapabilityCompilationRequest, UniversalCapabilityPlanningRequest,
    UniversalCapabilityShadowPlanReceipt,
};
use tidex::receiver::checkpoint_adapter::{
    InspectedReceiverArtifacts, PhysicalPlanningProfileRequest,
};
use tidex::receiver::model_adaptation::{ReceiverModelProfileInput, ReceiverModelProfileReceipt};
use tidex::receiver::receiver_compiler::{
    freeze_receiver_compiler, FrozenReceiverCompilerInput, ReceiverCalibrationSet,
    ReceiverCompilerPolicy, ReceiverProposalMethod,
};
use tidex::receiver::receiver_profile::{
    CapabilityModality, CapabilityRequirements, MaterializationStrategy,
};
use tidex::receiver::weight_actuator::read_model_tensor_f32;

struct Fixture {
    root: PathBuf,
    source: PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
        let _ = fs::remove_dir_all(&self.source);
    }
}
impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir()
            .join(format!("tidex-convergence-cli-{}-{nonce}", std::process::id()));
        let source = root.with_extension("source");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(&source).unwrap();
        fs::write(source.join("normalize.rs"), b"pub fn normalize(s: [f64; 2]) -> [f64; 2] { let n = s[0].hypot(s[1]); [s[0]/n, s[1]/n] }\n").unwrap();
        Self { root, source }
    }
    fn put(&self, name: &str, value: &impl Serialize) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        path
    }
    fn command(&self, area: &str, command: &str, path: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tidex"))
            .args([area, command])
            .arg(path)
            .env("TIDEX_PRIVATE_ROOT", &self.root)
            .output()
            .unwrap()
    }
    fn run<T: DeserializeOwned>(&self, area: &str, command: &str, path: &Path) -> T {
        let out = self.command(area, command, path);
        assert!(
            out.status.success(),
            "{area} {command}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
}

fn write_base(path: &Path) {
    // Deliberately non-lexical physical order; projection must remap by tensor ID.
    let mut header = serde_json::to_vec(&serde_json::json!({
        "model.matrix.weight":{"dtype":"F32","shape":[4,4],"data_offsets":[0,64]},
        "model.keep.weight":{"dtype":"F32","shape":[2],"data_offsets":[64,72]}
    }))
    .unwrap();
    while !header.len().is_multiple_of(8) {
        header.push(b' ');
    }
    let mut bytes = (header.len() as u64).to_le_bytes().to_vec();
    bytes.extend(header);
    for v in [0.0_f32; 16].into_iter().chain([7.5, 9.75]) {
        bytes.extend(v.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
}

fn planning_request(
    f: &Fixture,
    profile: &InspectedReceiverArtifacts,
) -> UniversalCapabilityPlanningRequest {
    let acquisition = AcquisitionRequest::new(
        AcquisitionId::parse("convergence-normalization").unwrap(),
        AcquisitionScope::WholeProject,
        RequestedResidency::BestVerified,
        NoisePolicy::ExplicitOnly,
        AcquisitionBudget {
            max_files: 8,
            max_total_bytes: 1 << 20,
        },
        vec![],
    )
    .unwrap();
    let envelope = SystemEnvelope::capture(&f.source, &acquisition).unwrap();
    let ir = CapabilityIr::new(
        CapabilityId::parse("normalize.control:v1").unwrap(),
        &envelope,
        PrimitiveSet::tidex_core_v1().unwrap(),
        vec![TypedPort::tensor_f64(PortId::parse("state").unwrap(), vec![2, 1]).unwrap()],
        vec![IrNode::new(
            CapabilityNodeId::parse("node.normalize").unwrap(),
            PrimitiveId::parse("tensor.normalize").unwrap(),
            vec![ValueReference::Input {
                name: PortId::parse("state").unwrap(),
            }],
            TypedPort::tensor_f64(PortId::parse("normalized").unwrap(), vec![2, 1]).unwrap(),
            vec!["normalize.rs".into()],
        )
        .unwrap()],
        vec![OutputBinding::new(
            TypedPort::tensor_f64(PortId::parse("result").unwrap(), vec![2, 1]).unwrap(),
            ValueReference::NodeOutput {
                node_id: CapabilityNodeId::parse("node.normalize").unwrap(),
            },
        )
        .unwrap()],
    )
    .unwrap();
    let mut operational = OperationalCapabilityContract {
        schema: "cerebro.tidex.operational_capability/v1".into(),
        capability_id: ir.capability_id().clone(),
        capability_ir_sha256: ir.manifest_digest().clone(),
        state_dimension: 2,
        anchors: vec![
            StateIrAnchor {
                anchor_id: "raw-x".into(),
                state: vec![2.0, 0.0],
            },
            StateIrAnchor {
                anchor_id: "unit-x".into(),
                state: vec![1.0, 0.0],
            },
            StateIrAnchor {
                anchor_id: "raw-y".into(),
                state: vec![0.0, 2.0],
            },
            StateIrAnchor {
                anchor_id: "unit-y".into(),
                state: vec![0.0, 1.0],
            },
        ],
        transitions: vec![
            OperatorIrTransition {
                operator_id: "normalize".into(),
                source_anchor_id: "raw-x".into(),
                target_anchor_id: "unit-x".into(),
                observed_next_state: vec![1.0, 0.0],
                pre_target_error: 1.0,
                post_target_error: 0.0,
            },
            OperatorIrTransition {
                operator_id: "normalize".into(),
                source_anchor_id: "raw-y".into(),
                target_anchor_id: "unit-y".into(),
                observed_next_state: vec![0.0, 1.0],
                pre_target_error: 1.0,
                post_target_error: 0.0,
            },
        ],
        maximum_closure_error: 1e-5,
        maximum_contraction_ratio: 1e-5,
    };
    operational
        .anchors
        .sort_by(|a, b| a.anchor_id.cmp(&b.anchor_id));
    let query = operational.canonical_transition_signature(&ir).unwrap();
    let functions = vec![
        vec![0.8, 0.1, 0.1, 1.2],
        vec![1.0, 0.0, 1.0, 0.0],
        vec![0.0, 1.0, 0.0, 1.0],
        vec![1.0, 1.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0, 1.0],
        vec![1.0, 0.5, 0.5, 1.0],
        vec![0.2, 1.0, 1.0, 0.2],
        vec![1.2, -0.2, 0.4, 0.8],
    ];
    let target = [1.0, 2.0, 3.0, 4.0]
        .into_iter()
        .flat_map(|a| [0.5, -1.0, 2.0, 0.25].map(|b| a * b))
        .collect::<Vec<_>>();
    let qnorm = query.iter().map(|v| v * v).sum::<f64>();
    let solutions = functions
        .iter()
        .map(|s: &Vec<f64>| {
            let coefficient = s.iter().zip(&query).map(|(a, b)| a * b).sum::<f64>() / qnorm;
            let mut out = vec![0.0; 2]; // protected tensor receives no update
            out.extend((0..16).map(|i| {
                s.get(i).copied().unwrap_or(0.0)
                    + (target[i] - query.get(i).copied().unwrap_or(0.0)) * coefficient
            }));
            out
        })
        .collect();
    let requirement = CapabilityRequirements {
        schema: "cerebro.tidex.capability_requirements/v1".into(),
        capability_id: ir.capability_id().clone(),
        capability_ir_sha256: ir.manifest_digest().clone(),
        required_modalities: BTreeSet::from([CapabilityModality::Text]),
        requires_persistent_state: false,
        minimum_receiver_parameter_dimension: 18,
        acceptable_strategies: BTreeSet::from([
            MaterializationStrategy::DenseDelta,
            MaterializationStrategy::LowRank,
            MaterializationStrategy::SparseDelta,
        ]),
    };
    let wrong_functional_signatures = vec![functions[1].clone(), functions[2].clone()];
    let frozen_receiver_compiler = freeze_receiver_compiler(&FrozenReceiverCompilerInput {
        schema: "cerebro.tidex.frozen_receiver_compiler_input/v1".into(),
        calibration_capability_ids: (0..functions.len())
            .map(|index| {
                CapabilityId::parse(format!("convergence.calibration.{index}:v1")).unwrap()
            })
            .collect(),
        calibration: ReceiverCalibrationSet {
            receiver_snapshot_binding_sha256: Some(profile.snapshot.manifest_digest().clone()),
            receiver_solutions: solutions,
            wrong_functional_signatures: Vec::new(),
            functional_signatures: functions,
        },
        protected_cortex: ProtectedCortex {
            parameter_importance: vec![0.0; 18],
            directions: vec![],
            max_damage_ratio: 0.01,
        },
        risk_metric_rows: (0..18)
            .map(|i| (0..18).map(|j| f64::from(i == j)).collect())
            .collect(),
        policy: ReceiverCompilerPolicy {
            schema: "cerebro.tidex.receiver_compiler_policy/v1".into(),
            ridge: 1e-10,
            minimum_decoder_loo_r2: 0.999,
            minimum_encoder_loo_r2: 0.999,
            minimum_decoder_loo_cosine: 0.999,
            maximum_functional_relative_error: 1e-4,
            minimum_identity_margin: 0.05,
            maximum_quadratic_cost: 1e9,
        },
        proposal_method: ReceiverProposalMethod::DecodeThenProject,
    })
    .unwrap();
    UniversalCapabilityPlanningRequest {
        schema: "cerebro.tidex.universal_capability_planning_request/v2".into(),
        compilation: UniversalCapabilityCompilationRequest {
            schema: "cerebro.tidex.universal_capability_compilation_request/v2".into(),
            system_envelope: envelope,
            capability_ir: ir,
            operational_contract: operational,
            frozen_receiver_compiler,
            wrong_functional_signatures,
        },
        receiver_profile: profile.profile.clone(),
        receiver_snapshot: profile.snapshot.clone(),
        capability_requirements: requirement,
        requested_strategy: MaterializationStrategy::DenseDelta,
        affected_regions: vec![TensorId::parse("model.matrix.weight").unwrap()],
    }
}

#[test]
fn both_architectures_produce_replayed_physical_checkpoints_through_production_cli() {
    let f = Fixture::new();
    let base = f.root.join("base.safetensors");
    write_base(&base);
    let config = f.root.join("config.json");
    let tokenizer = f.root.join("tokenizer.json");
    fs::write(&config, b"{\"model_type\":\"numerical-control\"}").unwrap();
    fs::write(&tokenizer, b"{}").unwrap();
    let prof_input = f.put(
        "profile-input.json",
        &ReceiverModelProfileInput {
            schema: "cerebro.tidex.receiver_model_profile_input/v1".into(),
            model_id: ModelId::parse("numerical.receiver").unwrap(),
            architecture_id: ArchitectureId::parse("numerical.matrix").unwrap(),
            source_revision: None,
            checkpoint_path: base.clone(),
            config_path: config,
            tokenizer_path: tokenizer,
        },
    );
    let physical: ReceiverModelProfileReceipt = f.run("receiver", "profile", &prof_input);
    let projection_input = f.put(
        "projection-input.json",
        &PhysicalPlanningProfileRequest {
            schema: "cerebro.tidex.physical_planning_profile_request/v1".into(),
            physical_profile: physical.profile_reference.clone(),
            modalities: BTreeSet::from([CapabilityModality::Text]),
            supports_persistent_state: false,
        },
    );
    let planning: InspectedReceiverArtifacts =
        f.run("receiver", "planning-profile", &projection_input);
    assert_eq!(planning.snapshot.model_snapshot_sha256, physical.profile.checkpoint.sha256);
    assert_eq!(planning.layout.geometry.layout.blocks[0].name, "model.keep.weight");
    let mut request = planning_request(&f, &planning);
    let before = fs::read(&base).unwrap();
    let mut outputs = Vec::new();
    for (label, strategy, backend) in [
        (
            "dense",
            MaterializationStrategy::DenseDelta,
            PhysicalMaterializationBackend::Dense,
        ),
        (
            "low-rank",
            MaterializationStrategy::LowRank,
            PhysicalMaterializationBackend::LowRank {
                policy: LowRankShadowPolicy {
                    schema: "cerebro.tidex.low_rank_shadow_policy/v1".into(),
                    maximum_rank: 1,
                    relative_reconstruction_tolerance: 1e-6,
                    absolute_reconstruction_tolerance: 1e-6,
                    minimum_parameter_reduction_ratio: 0.1,
                    maximum_svd_sweeps: 100,
                },
            },
        ),
    ] {
        request.requested_strategy = strategy;
        let plan_input = f.put(&format!("{label}-plan.json"), &request);
        let plan: UniversalCapabilityShadowPlanReceipt =
            f.run("compile", "universal-plan", &plan_input);
        let input = PhysicalMaterializationRequest {
            schema: "cerebro.tidex.physical_materialization_request/v1".into(),
            physical_profile: physical.profile_reference.clone(),
            source: CompiledMaterializationSource::UniversalPlan {
                request: Box::new(request.clone()),
                receipt: Box::new(plan),
                layout: Box::new(planning.layout.clone()),
            },
            backend,
            output_path: f.root.join(format!("{label}.safetensors")),
        };
        let materialize_input = f.put(&format!("{label}-input.json"), &input);
        let outcome: PhysicalMaterializationOutcome =
            f.run("materialize", "compiled", &materialize_input);
        let replay_input = f.put(&format!("{label}-reference.json"), &outcome.receipt_reference);
        let replay: PhysicalMaterializationReceipt =
            f.run("materialize", "verify-compiled", &replay_input);
        assert_eq!(replay, outcome.receipt);
        assert!(!replay.authorizes_promotion);
        assert!(!replay.model_execution_verified);
        let weights = read_model_tensor_f32(
            &input.output_path,
            &TensorId::parse("model.matrix.weight").unwrap(),
        )
        .unwrap();
        // Execute an independent matrix-vector readout from the resulting weights.
        let input_vector = [2.0_f32, 1.0, -0.5, 0.0];
        let observed = weights
            .chunks_exact(4)
            .map(|row| {
                row.iter()
                    .zip(input_vector)
                    .map(|(w, x)| w * x)
                    .sum::<f32>()
            })
            .collect::<Vec<_>>();
        for (actual, expected) in observed.iter().zip([-1.0_f32, -2.0, -3.0, -4.0]) {
            assert!((*actual - expected).abs() < 1e-5);
        }
        assert!(weights.iter().any(|v| *v != 0.0));
        assert_eq!(
            read_model_tensor_f32(
                &input.output_path,
                &TensorId::parse("model.keep.weight").unwrap()
            )
            .unwrap(),
            vec![7.5, 9.75]
        );
        assert_eq!(fs::read(&base).unwrap(), before);
        assert!(!f
            .command("materialize", "compiled", &materialize_input)
            .status
            .success()); // no overwrite
        outputs.push(weights);
        // Alter a produced checkpoint; a caller cannot bless it by replaying the original receipt.
        let mut corrupted = fs::read(&input.output_path).unwrap();
        let n = corrupted.len();
        corrupted[n - 1] ^= 1;
        fs::write(&input.output_path, corrupted).unwrap();
        assert!(!f
            .command("materialize", "verify-compiled", &replay_input)
            .status
            .success());
    }
    assert_eq!(outputs[0], outputs[1]);
    request.requested_strategy = MaterializationStrategy::SparseDelta;
    let plan: UniversalCapabilityShadowPlanReceipt =
        f.run("compile", "universal-plan", &f.put("lossy-plan.json", &request));
    let lossy = PhysicalMaterializationRequest {
        schema: "cerebro.tidex.physical_materialization_request/v1".into(),
        physical_profile: physical.profile_reference,
        source: CompiledMaterializationSource::UniversalPlan {
            request: Box::new(request),
            receipt: Box::new(plan),
            layout: Box::new(planning.layout),
        },
        backend: PhysicalMaterializationBackend::Sparse {
            policy: SparseShadowPolicy {
                schema: "cerebro.tidex.sparse_shadow_policy/v1".into(),
                maximum_nonzero_count: 8,
                maximum_density: 0.5,
                absolute_zero_threshold: 0.0,
                relative_reconstruction_tolerance: 1.0,
                absolute_reconstruction_tolerance: 100.0,
                minimum_storage_reduction_ratio: 0.0,
            },
        },
        output_path: f.root.join("lossy.safetensors"),
    };
    let error = f.command("materialize", "compiled", &f.put("lossy-input.json", &lossy));
    assert!(!error.status.success());
    assert!(String::from_utf8_lossy(&error.stderr).contains("requires_fresh_validation"));
    assert!(!lossy.output_path.exists());
}
