//! Local TIDE-X Advanced Laboratory.
//!
//! The laboratory is an operator surface over existing TIDE-X authorities. It
//! never evaluates arbitrary shell commands and it never turns a laboratory
//! result into production activation authority. Every executable recipe maps to
//! an allow-listed `tidex` command and its exact inputs/results are persisted.

use crate::digest::Sha256Digest;
use crate::error::{BrainError, BrainResult};
use crate::executor_registry::{executor_catalog, executor_for_lab_recipe};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_SCAN_ENTRIES: usize = 100_000;
const MAX_SCAN_DEPTH: usize = 8;
const MAX_DATASET_BYTES: usize = 128 * 1024 * 1024;
const MAX_HTTP_BODY_BYTES: usize = 128 * 1024 * 1024;
const MAX_RESULT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ASSETS: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LabRecipeCategory {
    Acquisition,
    Discovery,
    Learning,
    Benchmark,
    Receiver,
    Materialization,
    Evaluation,
    Governance,
    Lifecycle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LabExecutable {
    Tidex,
    SiblingBinary { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabRecipe {
    pub id: String,
    pub title: String,
    pub category: LabRecipeCategory,
    pub description: String,
    pub executable: LabExecutable,
    pub asset_count: usize,
    pub argv_template: Vec<String>,
    pub production_activation: bool,
}

fn recipe(
    id: &str,
    title: &str,
    category: LabRecipeCategory,
    description: &str,
    asset_count: usize,
    argv: &[&str],
    production_activation: bool,
) -> LabRecipe {
    LabRecipe {
        id: id.into(),
        title: title.into(),
        category,
        description: description.into(),
        executable: LabExecutable::Tidex,
        asset_count,
        argv_template: argv.iter().map(|value| (*value).to_string()).collect(),
        production_activation,
    }
}

fn sibling_recipe(
    id: &str,
    title: &str,
    category: LabRecipeCategory,
    description: &str,
    binary: &str,
    asset_count: usize,
    argv: &[&str],
) -> LabRecipe {
    let mut value = recipe(id, title, category, description, asset_count, argv, false);
    value.executable = LabExecutable::SiblingBinary {
        name: binary.to_string(),
    };
    value
}

/// Canonical operator recipes. Each one delegates to an existing TIDE-X CLI
/// authority instead of reimplementing its semantics in the laboratory.
pub fn recipe_catalog() -> Vec<LabRecipe> {
    vec![
        recipe("acquisition.capture", "Acquire workspace", LabRecipeCategory::Acquisition, "Descriptor-bound source acquisition into the private vault.", 0, &["acquire"], false),
        recipe("knowledge.plan", "Plan epistemic transition", LabRecipeCategory::Learning, "Authenticate a persisted KnowledgeEngine state and derive the next admissible invocation or terminal decision.", 1, &["knowledge", "plan", "{0}"], false),
        recipe("residency.decide", "Decide capability residency", LabRecipeCategory::Learning, "Run ResidencyDecisionAuthority over an authenticated precommit reference and persist the resulting decision.", 1, &["residency", "decide", "{0}"], false),
        recipe("numerical.evolve", "Run numerical evolution campaign", LabRecipeCategory::Learning, "Execute one or more ordered NumericalEvolution revisions under an explicit sealed governance policy; never promotes implicitly.", 1, &["numerical", "evolve", "{0}"], false),
        recipe("analysis.tomography", "Run skill-field tomography", LabRecipeCategory::Discovery, "Execute the canonical BrainEngine analysis chain, including tomography, identifiability and structural diagnostics, over authenticated observations.", 1, &["analysis", "tomography", "{0}"], false),
        recipe("analysis.protected_map", "Build protected cortex map", LabRecipeCategory::Discovery, "Build and persist a protected-cortex map from authenticated sensitivity artifacts without using task labels.", 1, &["analysis", "protected-map", "{0}"], false),
        recipe("analysis.pythagoras", "Analyze geometry / topology", LabRecipeCategory::Discovery, "Run Pythagoras staircase correction or persistent topology analysis from a typed request.", 1, &["analysis", "geometry", "{0}"], false),
        sibling_recipe("analysis.brain", "Analyze skill fields", LabRecipeCategory::Discovery, "Run the canonical BrainEngine analysis over authenticated observations.", "cerebro-tidex", 1, &["analyze", "{0}"]),
        sibling_recipe("runtime.sleep", "Consolidation / sleep", LabRecipeCategory::Learning, "Run the canonical evidence-bound sleep/consolidation transaction.", "cerebro-tidex", 0, &["sleep"]),
        sibling_recipe("learning.autonomous_plan", "Autonomous learning plan", LabRecipeCategory::Learning, "Plan an adaptive learning campaign from a typed LearningTarget.", "autonomous-learning-plan", 1, &["{0}"]),
        sibling_recipe("cross_model.discovery_cycle", "Multi-LLM discovery cycle", LabRecipeCategory::Discovery, "Execute real behavioral comparison across explicitly configured LLM runtimes. Requires the cross-model-plasticity build feature.", "plasticity-daemon", 2, &["once", "{0}", "{1}"]),
        sibling_recipe("lab.direct_runner", "Direct model laboratory runner", LabRecipeCategory::Discovery, "Execute a typed cross-model laboratory request using the canonical HF runtime and analysis components.", "tidex-lab-runner", 1, &["{0}"]),
        recipe("discovery.capabilities", "Capability discovery", LabRecipeCategory::Discovery, "Run canonical capability discovery from an explicit request.", 1, &["discover", "capabilities", "{0}"], false),
        recipe("benchmark.response", "Receiver response benchmark", LabRecipeCategory::Benchmark, "Benchmark declared receiver functional responses.", 1, &["benchmark", "response", "{0}"], false),
        recipe("benchmark.receiver_basis", "Receiver basis benchmark", LabRecipeCategory::Benchmark, "Benchmark the receiver basis under the declared protocol.", 1, &["benchmark", "receiver-basis", "{0}"], false),
        recipe("benchmark.portability", "Receiver compilation benchmark", LabRecipeCategory::Benchmark, "Leave-one-capability-out receiver compilation benchmark.", 1, &["benchmark", "portability", "{0}"], false),
        recipe("receiver.profile", "Profile receiver", LabRecipeCategory::Receiver, "Authenticate and profile a physical receiver checkpoint.", 1, &["receiver", "profile", "{0}"], false),
        recipe("receiver.normalize_sharded", "Normalize sharded SafeTensors", LabRecipeCategory::Receiver, "Normalize an authenticated HF sharded checkpoint into one retained SafeTensors authority.", 1, &["receiver", "normalize-sharded", "{0}"], false),
        recipe("receiver.freeze_compiler", "Freeze receiver compiler", LabRecipeCategory::Receiver, "Freeze calibration, protection, risk and maps before held-out target compilation.", 1, &["receiver", "freeze-compiler", "{0}"], false),
        recipe("receiver.verify_frozen", "Verify frozen compiler", LabRecipeCategory::Receiver, "Replay-authenticate a frozen receiver compiler.", 1, &["receiver", "verify-frozen-compiler", "{0}"], false),
        recipe("compile.universal", "Compile held-out capability", LabRecipeCategory::Receiver, "Compile an authenticated CapabilityIR with a frozen receiver compiler.", 1, &["compile", "universal", "{0}"], false),
        recipe("compile.universal_plan", "Plan universal shadow", LabRecipeCategory::Receiver, "Create a receiver-bound shadow materialization plan.", 1, &["compile", "universal-plan", "{0}"], false),
        recipe("materialize.shadow_dense", "Materialize dense shadow", LabRecipeCategory::Materialization, "Replay a universal shadow plan into a dense candidate representation bound to the receiver layout.", 3, &["materialize", "dense", "{0}", "{1}", "{2}"], false),
        recipe("materialize.shadow_low_rank", "Materialize low-rank shadow", LabRecipeCategory::Materialization, "Replay a universal shadow plan into a bounded low-rank candidate under explicit policy.", 4, &["materialize", "low-rank", "{0}", "{1}", "{2}", "{3}"], false),
        recipe("materialize.shadow_sparse", "Materialize sparse shadow", LabRecipeCategory::Materialization, "Replay a universal shadow plan into a bounded sparse candidate under explicit policy.", 4, &["materialize", "sparse", "{0}", "{1}", "{2}", "{3}"], false),
        recipe("materialize.shadow_steering", "Materialize activation-steering shadow", LabRecipeCategory::Materialization, "Replay a universal shadow plan into a runtime steering candidate; no production hook is installed.", 5, &["materialize", "steering", "{0}", "{1}", "{2}", "{3}", "{4}"], false),
        recipe("materialize.compiled", "Materialize compiled checkpoint", LabRecipeCategory::Materialization, "Physically materialize a candidate-only checkpoint using the canonical actuator.", 1, &["materialize", "compiled", "{0}"], false),
        recipe("materialize.verify", "Verify compiled checkpoint", LabRecipeCategory::Materialization, "Replay-authenticate physical checkpoint arithmetic.", 1, &["materialize", "verify-compiled", "{0}"], false),
        recipe("selection.backend", "Select materialization backend", LabRecipeCategory::Evaluation, "Rank measured candidate backends under the canonical selection policy.", 1, &["select", "backend", "{0}"], false),
        recipe("evaluation.shadow", "Run isolated shadow evaluation", LabRecipeCategory::Evaluation, "Run an authenticated evaluator and bundle through isolated execution.", 2, &["shadow", "run", "{0}", "{1}"], false),
        recipe("evidence.universality", "Measure universality", LabRecipeCategory::Evaluation, "Reduce held-out trials across capabilities, receivers, families and seeds.", 1, &["measure", "universality", "{0}"], false),
        recipe("promotion.readiness", "Universal promotion readiness", LabRecipeCategory::Governance, "Fail-closed readiness gate; does not activate production.", 1, &["gate", "promotion", "{0}"], false),
        recipe("adapter.import", "Import adapter", LabRecipeCategory::Lifecycle, "Import an authenticated adapter candidate into AdapterBank.", 1, &["adapter-bank", "import", "{0}"], false),
        recipe("adapter.compose", "Compose adapters", LabRecipeCategory::Lifecycle, "Create exact ordered adapter composition.", 1, &["adapter-bank", "compose", "{0}"], false),
        recipe("adapter.materialize", "Materialize adapter candidate", LabRecipeCategory::Lifecycle, "Create a candidate-only adapter materialization.", 1, &["adapter-bank", "materialize", "{0}"], false),
        recipe("adapter.authorize", "Authorize governed promotion", LabRecipeCategory::Governance, "Re-authenticate sealed governance witnesses and mint a current-state promotion permit.", 1, &["adapter-bank", "authorize", "{0}"], false),
        recipe("adapter.activate", "Activate adapter", LabRecipeCategory::Lifecycle, "Production activation. Requires a previously authenticated governed authorization.", 1, &["adapter-bank", "activate", "{0}"], true),
        recipe("adapter.revoke", "Revoke adapter", LabRecipeCategory::Lifecycle, "Sticky governed revocation of an adapter and dependent compositions.", 1, &["adapter-bank", "revoke", "{0}"], true),
        recipe("adapter.rollback", "Rollback adapter bank", LabRecipeCategory::Lifecycle, "Publish a new forward revision representing rollback.", 1, &["adapter-bank", "rollback", "{0}"], true),
        recipe("adapter.status", "Verify AdapterBank history", LabRecipeCategory::Lifecycle, "Verify the complete hash-linked AdapterBank history.", 0, &["adapter-bank", "status"], false),
    ]
}

fn find_recipe(id: &str) -> BrainResult<LabRecipe> {
    recipe_catalog()
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| BrainError::Invalid("lab_recipe_unknown".into()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalModelLayout {
    HuggingFaceSingleSafetensors,
    HuggingFaceShardedSafetensors,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LocalModelCandidate {
    pub model_id: Sha256Digest,
    pub root: PathBuf,
    pub layout: LocalModelLayout,
    pub config: PathBuf,
    pub tokenizer: PathBuf,
    pub weights: PathBuf,
    pub architecture: Option<String>,
}

fn future_runtime_root() -> BrainResult<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let future_root = manifest_dir
        .parent()
        .ok_or_else(|| BrainError::Invalid("future_root_unavailable".into()))?;
    Ok(future_root.join("cerebro3-runtime"))
}

fn default_lab_home() -> BrainResult<PathBuf> {
    Ok(future_runtime_root()?.join("tidex"))
}

fn default_hf_hub_root() -> BrainResult<PathBuf> {
    Ok(future_runtime_root()?.join("llms/huggingface/hub"))
}

fn default_mechinterp_python() -> BrainResult<PathBuf> {
    Ok(future_runtime_root()?.join("python/cerebro3-mechinterp/bin/python"))
}

/// Discover local HF-style model snapshots. Discovery is read-only and never
/// treats a found directory as authenticated runtime evidence.
pub fn configured_lab_home() -> BrainResult<PathBuf> {
    let home = if let Some(configured) = std::env::var_os("TIDEX_HOME") {
        PathBuf::from(configured)
    } else {
        default_lab_home()?
    };
    if !home.is_absolute() {
        return Err(BrainError::Invalid("lab_home_must_be_absolute".into()));
    }
    ensure_private_dir(&home)?;
    Ok(home.canonicalize()?)
}

pub fn discover_local_models(root: &Path) -> BrainResult<Vec<LocalModelCandidate>> {
    if !root.is_absolute() {
        return Err(BrainError::Invalid(
            "lab_model_scan_root_must_be_absolute".into(),
        ));
    }
    let metadata = fs::symlink_metadata(root)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(BrainError::Invalid("lab_model_scan_root_invalid".into()));
    }
    let root = root.canonicalize()?;
    let mut queue = VecDeque::from([(root.clone(), 0usize)]);
    let mut visited = 0usize;
    let mut found = Vec::new();
    while let Some((dir, depth)) = queue.pop_front() {
        visited = visited
            .checked_add(1)
            .ok_or_else(|| BrainError::Invalid("lab_model_scan_overflow".into()))?;
        if visited > MAX_SCAN_ENTRIES {
            return Err(BrainError::Invalid("lab_model_scan_limit".into()));
        }
        let config = dir.join("config.json");
        let tokenizer = dir.join("tokenizer.json");
        let single = dir.join("model.safetensors");
        let sharded = dir.join("model.safetensors.index.json");
        if config.is_file() && tokenizer.is_file() && (single.is_file() || sharded.is_file()) {
            let (layout, weights) = if single.is_file() {
                (LocalModelLayout::HuggingFaceSingleSafetensors, single)
            } else {
                (LocalModelLayout::HuggingFaceShardedSafetensors, sharded)
            };
            let architecture = fs::read(&config)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                .and_then(|value| {
                    value
                        .get("architectures")
                        .and_then(|v| v.as_array())
                        .and_then(|v| v.first())
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                        .or_else(|| {
                            value
                                .get("model_type")
                                .and_then(|v| v.as_str())
                                .map(str::to_string)
                        })
                });
            let identity = Sha256Digest::digest_domain(
                b"CEREBRO:TIDEX:LAB-MODEL-CANDIDATE:v1\0",
                &serde_json::to_vec(&(dir.as_os_str(), &layout, &config, &tokenizer, &weights))?,
            );
            found.push(LocalModelCandidate {
                model_id: identity,
                root: dir.clone(),
                layout,
                config,
                tokenizer,
                weights,
                architecture,
            });
        }
        if depth >= MAX_SCAN_DEPTH {
            continue;
        }
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() && !kind.is_symlink() {
                queue.push_back((entry.path(), depth + 1));
            }
        }
    }
    found.sort_by(|a, b| a.root.cmp(&b.root));
    found.dedup_by(|a, b| a.root == b.root);
    Ok(found)
}

pub fn catalog_local_models(
    tidex_home: &Path,
    root: &Path,
) -> BrainResult<Vec<LocalModelCandidate>> {
    let models = discover_local_models(root)?;
    let catalog = tidex_home.join("lab/models/by-sha");
    ensure_private_dir(&catalog)?;
    for model in &models {
        let bytes = serde_json::to_vec(model)?;
        let path = catalog.join(format!("{}.json", model.model_id));
        if path.exists() {
            let existing = fs::read(&path)?;
            if existing != bytes {
                return Err(BrainError::Integrity("lab_model_catalog_collision".into()));
            }
        } else {
            write_private_new(&path, &bytes)?;
        }
    }
    Ok(models)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LabDatasetFormat {
    Json,
    Jsonl,
    Csv,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabDatasetManifest {
    pub schema: String,
    pub name: String,
    pub format: LabDatasetFormat,
    pub content_sha256: Sha256Digest,
    pub bytes: u64,
    pub artifact: PathBuf,
    pub generated: bool,
    pub independent_evidence: bool,
}

fn validate_dataset_name(name: &str) -> BrainResult<()> {
    if name.is_empty()
        || name.len() > 128
        || name != name.trim()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(BrainError::Invalid("lab_dataset_name_invalid".into()));
    }
    Ok(())
}

fn ensure_private_dir(path: &Path) -> BrainResult<()> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_dir() || meta.permissions().mode() & 0o077 != 0 {
        return Err(BrainError::Integrity("lab_directory_not_private".into()));
    }
    Ok(())
}

fn write_private_new(path: &Path, bytes: &[u8]) -> BrainResult<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn import_dataset_bytes(
    tidex_home: &Path,
    name: &str,
    format: LabDatasetFormat,
    bytes: &[u8],
    generated: bool,
) -> BrainResult<LabDatasetManifest> {
    validate_dataset_name(name)?;
    if bytes.is_empty() || bytes.len() > MAX_DATASET_BYTES {
        return Err(BrainError::Invalid("lab_dataset_size_invalid".into()));
    }
    match format {
        LabDatasetFormat::Json => {
            let _: serde_json::Value = serde_json::from_slice(bytes)
                .map_err(|_| BrainError::Invalid("lab_dataset_json_invalid".into()))?;
        }
        LabDatasetFormat::Jsonl => {
            for line in bytes
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
            {
                let _: serde_json::Value = serde_json::from_slice(line)
                    .map_err(|_| BrainError::Invalid("lab_dataset_jsonl_invalid".into()))?;
            }
        }
        LabDatasetFormat::Csv | LabDatasetFormat::Text => {
            std::str::from_utf8(bytes)
                .map_err(|_| BrainError::Invalid("lab_dataset_text_encoding_invalid".into()))?;
        }
    }
    let root = tidex_home.join("lab/datasets/by-sha");
    ensure_private_dir(&root)?;
    let content_sha256 = Sha256Digest::digest_bytes(bytes);
    let artifact = root.join(format!("{}.data", content_sha256));
    if artifact.exists() {
        let existing = fs::read(&artifact)?;
        if Sha256Digest::digest_bytes(&existing) != content_sha256 {
            return Err(BrainError::Integrity(
                "lab_dataset_existing_digest_mismatch".into(),
            ));
        }
    } else {
        write_private_new(&artifact, bytes)?;
    }
    let manifest = LabDatasetManifest {
        schema: "cerebro.tidex.lab_dataset/v1".into(),
        name: name.into(),
        format,
        content_sha256: content_sha256.clone(),
        bytes: u64::try_from(bytes.len())
            .map_err(|_| BrainError::Invalid("lab_dataset_size_overflow".into()))?,
        artifact,
        generated,
        independent_evidence: !generated,
    };
    let manifests = tidex_home.join("lab/datasets/manifests");
    ensure_private_dir(&manifests)?;
    let manifest_bytes = serde_json::to_vec(&manifest)?;
    let path = manifests.join(format!("{}.json", content_sha256));
    if !path.exists() {
        write_private_new(&path, &manifest_bytes)?;
    }
    Ok(manifest)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabRunRequest {
    pub schema: String,
    pub recipe_id: String,
    #[serde(default)]
    pub assets: Vec<PathBuf>,
    #[serde(default)]
    pub selected_model_ids: Vec<Sha256Digest>,
    #[serde(default)]
    pub dataset_sha256: Option<Sha256Digest>,
    #[serde(default)]
    pub maximum_runtime_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabRunReceipt {
    pub schema: String,
    pub run_id: Sha256Digest,
    pub recipe_id: String,
    #[serde(default)]
    pub executor_id: Option<String>,
    pub argv: Vec<String>,
    pub selected_model_ids: Vec<Sha256Digest>,
    pub dataset_sha256: Option<Sha256Digest>,
    pub source_tree_sha256: Sha256Digest,
    pub exit_code: i32,
    pub stdout_sha256: Sha256Digest,
    pub stderr_sha256: Sha256Digest,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub succeeded: bool,
    pub production_activation_recipe: bool,
    pub authorizes_production: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabRunView {
    pub receipt: LabRunReceipt,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabJobEvidenceReceipt {
    pub schema: String,
    pub job_id: Sha256Digest,
    pub request_sha256: Option<Sha256Digest>,
    pub executor_id: Option<String>,
    pub executor_descriptor_sha256: Option<Sha256Digest>,
    pub state: LabJobState,
    pub run_id: Option<Sha256Digest>,
    pub stdout_sha256: Option<Sha256Digest>,
    pub stderr_sha256: Option<Sha256Digest>,
    pub succeeded: bool,
    pub authorizes_production: bool,
    pub evidence_sha256: Sha256Digest,
}

fn build_lab_job_evidence_receipt(record: &LabJobRecord) -> BrainResult<LabJobEvidenceReceipt> {
    let run = record.run.as_ref();
    let executor_id = executor_id_for_operation(&record.operation)
        .map(str::to_string)
        .or_else(|| run.and_then(|value| value.receipt.executor_id.clone()));
    let executor_descriptor_sha256 = match executor_id.as_deref() {
        Some(id) => crate::executor_registry::executor_by_id(id)
            .ok()
            .map(|descriptor| descriptor.descriptor_sha256),
        None => None,
    };
    let mut receipt = LabJobEvidenceReceipt {
        schema: "cerebro.tidex.lab_job_evidence_receipt/v1".into(),
        job_id: record.job_id.clone(),
        request_sha256: record.request_sha256.clone(),
        executor_id,
        executor_descriptor_sha256,
        state: record.state.clone(),
        run_id: run.map(|value| value.receipt.run_id.clone()),
        stdout_sha256: run.map(|value| value.receipt.stdout_sha256.clone()),
        stderr_sha256: run.map(|value| value.receipt.stderr_sha256.clone()),
        succeeded: matches!(record.state, LabJobState::Completed)
            && run.is_some_and(|value| value.receipt.succeeded),
        authorizes_production: false,
        evidence_sha256: Sha256Digest::zero(),
    };
    receipt.evidence_sha256 = Sha256Digest::digest_domain(
        b"CEREBRO:TIDEX:LAB-JOB-EVIDENCE:v1\0",
        &serde_json::to_vec(&(
            &receipt.schema,
            &receipt.job_id,
            &receipt.request_sha256,
            &receipt.executor_id,
            &receipt.executor_descriptor_sha256,
            &receipt.state,
            &receipt.run_id,
            &receipt.stdout_sha256,
            &receipt.stderr_sha256,
            receipt.succeeded,
            receipt.authorizes_production,
        ))?,
    );
    Ok(receipt)
}

fn executor_id_for_operation(operation: &str) -> Option<&'static str> {
    match operation {
        "behavioral_discovery" => Some("cross_model.discovery"),
        "probe_runtime" => Some("cross_model.probe_runtime"),
        "behavioral_evaluation" => Some("cross_model.evaluate"),
        "extract_capability" => Some("cross_model.extract_steering"),
        "deep_instrumentation" => Some("cross_model.nnsight"),
        "sparse_autoencoder_analysis" => Some("cross_model.sae"),
        "counterfactual_analysis" => Some("cross_model.counterfactual"),
        "calibrate_alignment" => Some("cross_model.align"),
        "activation_transfer_experiment" => Some("cross_model.transfer_steering"),
        "generate_behavioral_dataset" => Some("cross_model.evaluate"),
        _ => None,
    }
}

fn lab_run_view(receipt: LabRunReceipt) -> BrainResult<LabRunView> {
    let stdout_bytes = fs::read(&receipt.stdout)?;
    let stderr_bytes = fs::read(&receipt.stderr)?;
    if stdout_bytes.len() as u64 > MAX_RESULT_BYTES || stderr_bytes.len() as u64 > MAX_RESULT_BYTES
    {
        return Err(BrainError::Invalid("lab_run_output_limit".into()));
    }
    let stdout = String::from_utf8(stdout_bytes)
        .map_err(|_| BrainError::Invalid("lab_run_stdout_not_utf8".into()))?;
    let stderr = String::from_utf8(stderr_bytes)
        .map_err(|_| BrainError::Invalid("lab_run_stderr_not_utf8".into()))?;
    Ok(LabRunView {
        receipt,
        stdout,
        stderr,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LabJobState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

static LAB_JOB_CANCELLATIONS: OnceLock<Mutex<std::collections::BTreeMap<String, Arc<AtomicBool>>>> =
    OnceLock::new();

fn cancellation_registry() -> &'static Mutex<std::collections::BTreeMap<String, Arc<AtomicBool>>> {
    LAB_JOB_CANCELLATIONS.get_or_init(|| Mutex::new(std::collections::BTreeMap::new()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LabJobRecord {
    pub schema: String,
    pub job_id: Sha256Digest,
    #[serde(default)]
    pub request_sha256: Option<Sha256Digest>,
    #[serde(default)]
    pub evidence_receipt: Option<LabJobEvidenceReceipt>,
    pub state: LabJobState,
    pub operation: String,
    pub submitted_unix_ns: u128,
    pub run: Option<LabRunView>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
enum LabJobRequest {
    BehavioralDiscovery(BehavioralDiscoveryWorkflowRequest),
    Direct(LabDirectWorkflowRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabRuntimeAccessProfile {
    pub behavioral_inference: bool,
    pub internal_activations: bool,
    pub activation_intervention: bool,
    pub deep_instrumentation: bool,
    pub sparse_autoencoder_analysis: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabModelRuntimeProfile {
    pub schema: String,
    pub model_id: Sha256Digest,
    pub source_job_id: Sha256Digest,
    pub source_run_id: Sha256Digest,
    pub runtime_metadata_sha256: String,
    pub runtime_architecture: String,
    pub parameter_count: u64,
    pub num_layers: usize,
    pub embedding_dim: usize,
    pub access: LabRuntimeAccessProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LabModelRuntimeStatus {
    pub schema: String,
    pub model_id: Sha256Digest,
    pub state: String,
    pub job_id: Option<Sha256Digest>,
    pub profile: Option<LabModelRuntimeProfile>,
    pub error: Option<String>,
}

fn runtime_profile_from_probe_job(
    record: &LabJobRecord,
) -> BrainResult<Option<LabModelRuntimeProfile>> {
    if record.operation != "probe_runtime" || record.state != LabJobState::Completed {
        return Ok(None);
    }
    let run = match &record.run {
        Some(value) if value.receipt.selected_model_ids.len() == 1 => value,
        _ => return Ok(None),
    };
    let value: serde_json::Value = serde_json::from_str(&run.stdout)
        .map_err(|_| BrainError::Invalid("lab_runtime_probe_output_not_json".into()))?;
    if value.get("schema").and_then(serde_json::Value::as_str)
        != Some("cerebro.tidex.lab_runtime_probe/v1")
    {
        return Ok(None);
    }
    let model = value
        .get("model")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| BrainError::Invalid("lab_runtime_probe_model_missing".into()))?;
    let access = value
        .get("access")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| BrainError::Invalid("lab_runtime_probe_access_missing".into()))?;
    let read_bool = |name: &str| -> BrainResult<bool> {
        access
            .get(name)
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| BrainError::Invalid(format!("lab_runtime_probe_access_missing:{name}")))
    };
    Ok(Some(LabModelRuntimeProfile {
        schema: "cerebro.tidex.lab_model_runtime_profile/v1".into(),
        model_id: run.receipt.selected_model_ids[0].clone(),
        source_job_id: record.job_id.clone(),
        source_run_id: run.receipt.run_id.clone(),
        runtime_metadata_sha256: model
            .get("runtime_metadata_sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| BrainError::Invalid("lab_runtime_probe_metadata_missing".into()))?
            .to_string(),
        runtime_architecture: model
            .get("runtime_architecture")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| BrainError::Invalid("lab_runtime_probe_architecture_missing".into()))?
            .to_string(),
        parameter_count: model
            .get("parameter_count")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                BrainError::Invalid("lab_runtime_probe_parameter_count_missing".into())
            })?,
        num_layers: model
            .get("num_layers")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| BrainError::Invalid("lab_runtime_probe_layers_missing".into()))?,
        embedding_dim: model
            .get("embedding_dim")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| BrainError::Invalid("lab_runtime_probe_embedding_missing".into()))?,
        access: LabRuntimeAccessProfile {
            behavioral_inference: read_bool("behavioral_inference")?,
            internal_activations: read_bool("internal_activations")?,
            activation_intervention: read_bool("activation_intervention")?,
            deep_instrumentation: read_bool("deep_instrumentation")?,
            sparse_autoencoder_analysis: read_bool("sparse_autoencoder_analysis")?,
        },
    }))
}

pub fn list_runtime_profiles(tidex_home: &Path) -> BrainResult<Vec<LabModelRuntimeProfile>> {
    let cataloged_models = list_catalog_models(tidex_home)?
        .into_iter()
        .map(|model| model.model_id)
        .collect::<std::collections::BTreeSet<_>>();
    let mut latest: std::collections::BTreeMap<Sha256Digest, LabModelRuntimeProfile> =
        std::collections::BTreeMap::new();
    for record in list_job_records(tidex_home)? {
        if let Some(profile) = runtime_profile_from_probe_job(&record)? {
            if cataloged_models.contains(&profile.model_id) {
                latest.entry(profile.model_id.clone()).or_insert(profile);
            }
        }
    }
    Ok(latest.into_values().collect())
}

pub fn list_runtime_statuses(tidex_home: &Path) -> BrainResult<Vec<LabModelRuntimeStatus>> {
    let models = list_catalog_models(tidex_home)?;
    let mut latest_jobs = std::collections::BTreeMap::<Sha256Digest, LabJobRecord>::new();
    for record in list_job_records(tidex_home)? {
        if record.operation != "probe_runtime" {
            continue;
        }
        let Some(run) = record.run.as_ref() else {
            continue;
        };
        if run.receipt.selected_model_ids.len() != 1 {
            continue;
        }
        let model_id = run.receipt.selected_model_ids[0].clone();
        latest_jobs.entry(model_id).or_insert(record);
    }
    models
        .into_iter()
        .map(|model| {
            let job = latest_jobs.get(&model.model_id);
            let profile = match job {
                Some(record) => runtime_profile_from_probe_job(record)?,
                None => None,
            };
            Ok(LabModelRuntimeStatus {
                schema: "cerebro.tidex.lab_model_runtime_status/v1".into(),
                model_id: model.model_id,
                state: job
                    .map(|record| match record.state {
                        LabJobState::Queued => "queued",
                        LabJobState::Running => "running",
                        LabJobState::Completed => "completed",
                        LabJobState::Failed => "failed",
                        LabJobState::Cancelled => "cancelled",
                    })
                    .unwrap_or("missing")
                    .to_string(),
                job_id: job.map(|record| record.job_id.clone()),
                profile,
                error: job.and_then(|record| record.error.clone()),
            })
        })
        .collect()
}

fn job_status_path(tidex_home: &Path, job_id: &Sha256Digest) -> PathBuf {
    tidex_home
        .join("lab/jobs/by-sha")
        .join(job_id.as_str())
        .join("status.json")
}

fn persist_job_record(tidex_home: &Path, record: &LabJobRecord) -> BrainResult<()> {
    let root = tidex_home
        .join("lab/jobs/by-sha")
        .join(record.job_id.as_str());
    ensure_private_dir(&root)?;
    let bytes = serde_json::to_vec(record)?;
    let target = root.join("status.json");
    let temp = root.join(format!("status-{}.tmp", now_nanos()?));
    write_private_new(&temp, &bytes)?;
    fs::rename(&temp, &target)?;
    Ok(())
}

pub fn load_job_record(tidex_home: &Path, job_id: &Sha256Digest) -> BrainResult<LabJobRecord> {
    let bytes = fs::read(job_status_path(tidex_home, job_id))
        .map_err(|_| BrainError::Invalid("lab_job_not_found".into()))?;
    let record: LabJobRecord = serde_json::from_slice(&bytes)?;
    if record.job_id != *job_id || record.schema != "cerebro.tidex.lab_job/v1" {
        return Err(BrainError::Integrity("lab_job_identity_invalid".into()));
    }
    Ok(record)
}

pub fn list_job_records(tidex_home: &Path) -> BrainResult<Vec<LabJobRecord>> {
    let root = tidex_home.join("lab/jobs/by-sha");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let status = entry.path().join("status.json");
        if !status.is_file() {
            continue;
        }
        let bytes = fs::read(status)?;
        let record: LabJobRecord = serde_json::from_slice(&bytes)?;
        if record.schema != "cerebro.tidex.lab_job/v1" {
            return Err(BrainError::Integrity("lab_job_schema_invalid".into()));
        }
        records.push(record);
    }
    records.sort_by_key(|record| std::cmp::Reverse(record.submitted_unix_ns));
    records.truncate(100);
    Ok(records)
}

fn start_lab_job(tidex_home: &Path, request: LabJobRequest) -> BrainResult<LabJobRecord> {
    let submitted_unix_ns = now_nanos()?;
    let operation = match &request {
        LabJobRequest::BehavioralDiscovery(_) => "behavioral_discovery",
        LabJobRequest::Direct(value) => match value.operation {
            LabDirectOperation::ProbeRuntime => "probe_runtime",
            LabDirectOperation::BehavioralEvaluation => "behavioral_evaluation",
            LabDirectOperation::ExtractCapability => "extract_capability",
            LabDirectOperation::DeepInstrumentation => "deep_instrumentation",
            LabDirectOperation::SparseAutoencoderAnalysis => "sparse_autoencoder_analysis",
            LabDirectOperation::CounterfactualAnalysis => "counterfactual_analysis",
            LabDirectOperation::GenerateBehavioralDataset => "generate_behavioral_dataset",
            LabDirectOperation::CalibrateAlignment => "calibrate_alignment",
            LabDirectOperation::ActivationTransferExperiment => "activation_transfer_experiment",
        },
    }
    .to_string();
    let request_digest = match &request {
        LabJobRequest::BehavioralDiscovery(value) => {
            Sha256Digest::digest_bytes(&serde_json::to_vec(value)?)
        }
        LabJobRequest::Direct(value) => Sha256Digest::digest_bytes(&serde_json::to_vec(value)?),
    };
    if let Some(existing) = list_job_records(tidex_home)?.into_iter().find(|record| {
        record.request_sha256.as_ref() == Some(&request_digest)
            && matches!(record.state, LabJobState::Queued | LabJobState::Running)
    }) {
        return Ok(existing);
    }
    let job_id = Sha256Digest::digest_domain(
        b"CEREBRO:TIDEX:LAB-JOB:v1\0",
        &serde_json::to_vec(&(submitted_unix_ns, &operation, &request_digest))?,
    );
    let queued = LabJobRecord {
        schema: "cerebro.tidex.lab_job/v1".into(),
        job_id: job_id.clone(),
        request_sha256: Some(request_digest.clone()),
        evidence_receipt: None,
        state: LabJobState::Queued,
        operation: operation.clone(),
        submitted_unix_ns,
        run: None,
        error: None,
    };
    persist_job_record(tidex_home, &queued)?;
    let cancellation = Arc::new(AtomicBool::new(false));
    cancellation_registry()
        .lock()
        .map_err(|_| BrainError::Integrity("lab_cancellation_registry_poisoned".into()))?
        .insert(job_id.as_str().to_string(), cancellation.clone());
    let home = tidex_home.to_path_buf();
    let thread_job_id = job_id.clone();
    std::thread::Builder::new()
        .name(format!("tidex-lab-{}", &job_id.as_str()[..12]))
        .spawn(move || {
            let mut record = LabJobRecord {
                schema: "cerebro.tidex.lab_job/v1".into(),
                job_id: thread_job_id,
                request_sha256: Some(request_digest),
                evidence_receipt: None,
                state: LabJobState::Running,
                operation,
                submitted_unix_ns,
                run: None,
                error: None,
            };
            if persist_job_record(&home, &record).is_err() {
                return;
            }
            let result = match request {
                LabJobRequest::BehavioralDiscovery(value) => {
                    execute_behavioral_discovery_workflow_cancelable(
                        &home,
                        &value,
                        Some(&cancellation),
                    )
                    .and_then(lab_run_view)
                }
                LabJobRequest::Direct(value) => {
                    execute_direct_workflow_cancelable(&home, &value, Some(&cancellation))
                        .and_then(lab_run_view)
                }
            };
            match result {
                Ok(run) if run.receipt.succeeded => {
                    record.state = LabJobState::Completed;
                    record.run = Some(run);
                }
                Ok(run) => {
                    record.state = LabJobState::Failed;
                    record.error = Some(if run.stderr.trim().is_empty() {
                        format!("lab_child_exit_failed:{}", run.receipt.exit_code)
                    } else {
                        run.stderr.trim().to_string()
                    });
                    record.run = Some(run);
                }
                Err(error) => {
                    let message = error.to_string();
                    record.state = if message.contains("lab_run_cancelled") {
                        LabJobState::Cancelled
                    } else {
                        LabJobState::Failed
                    };
                    record.error = Some(message);
                }
            }
            if let Ok(receipt) = build_lab_job_evidence_receipt(&record) {
                record.evidence_receipt = Some(receipt);
            }
            let _ = persist_job_record(&home, &record);
            if let Ok(mut registry) = cancellation_registry().lock() {
                registry.remove(record.job_id.as_str());
            }
        })?;
    Ok(queued)
}

pub fn cancel_lab_job(tidex_home: &Path, job_id: &Sha256Digest) -> BrainResult<LabJobRecord> {
    let mut record = load_job_record(tidex_home, job_id)?;
    match record.state {
        LabJobState::Queued | LabJobState::Running => {
            let flag = cancellation_registry()
                .lock()
                .map_err(|_| BrainError::Integrity("lab_cancellation_registry_poisoned".into()))?
                .get(job_id.as_str())
                .cloned()
                .ok_or_else(|| BrainError::Invalid("lab_job_not_active_in_this_process".into()))?;
            flag.store(true, Ordering::SeqCst);
            record.error = Some("cancellation_requested".into());
            persist_job_record(tidex_home, &record)?;
            Ok(record)
        }
        _ => Err(BrainError::Invalid("lab_job_not_cancellable".into())),
    }
}

fn recover_incomplete_lab_jobs(tidex_home: &Path) -> BrainResult<usize> {
    let mut recovered = 0usize;
    for mut record in list_job_records(tidex_home)? {
        let inconsistent_completed = matches!(record.state, LabJobState::Completed)
            && record
                .run
                .as_ref()
                .is_some_and(|run| !run.receipt.succeeded);
        if matches!(record.state, LabJobState::Queued | LabJobState::Running)
            || inconsistent_completed
        {
            record.state = LabJobState::Failed;
            record.error = Some(if inconsistent_completed {
                "recovered_inconsistent_completed_failed_run".into()
            } else {
                "recovered_incomplete_job_after_lab_restart".into()
            });
            record.evidence_receipt = Some(build_lab_job_evidence_receipt(&record)?);
            persist_job_record(tidex_home, &record)?;
            recovered = recovered
                .checked_add(1)
                .ok_or_else(|| BrainError::Invalid("lab_recovery_counter_overflow".into()))?;
        }
    }
    Ok(recovered)
}

fn resolve_assets(request: &LabRunRequest, recipe: &LabRecipe) -> BrainResult<Vec<PathBuf>> {
    if request.schema != "cerebro.tidex.lab_run_request/v1"
        || request.assets.len() != recipe.asset_count
        || request.assets.len() > MAX_ASSETS
    {
        return Err(BrainError::Invalid("lab_run_request_invalid".into()));
    }
    let mut out = Vec::with_capacity(request.assets.len());
    for path in &request.assets {
        if !path.is_absolute() {
            return Err(BrainError::Invalid(
                "lab_asset_path_must_be_absolute".into(),
            ));
        }
        let meta = fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() || !meta.is_file() || meta.len() == 0 {
            return Err(BrainError::Invalid("lab_asset_invalid".into()));
        }
        out.push(path.canonicalize()?);
    }
    Ok(out)
}

fn render_argv(recipe: &LabRecipe, assets: &[PathBuf]) -> BrainResult<Vec<String>> {
    recipe
        .argv_template
        .iter()
        .map(|part| {
            if part.starts_with('{') && part.ends_with('}') {
                let index = part[1..part.len() - 1]
                    .parse::<usize>()
                    .map_err(|_| BrainError::Integrity("lab_recipe_placeholder_invalid".into()))?;
                let path = assets
                    .get(index)
                    .ok_or_else(|| BrainError::Integrity("lab_recipe_asset_missing".into()))?;
                Ok(path.to_string_lossy().into_owned())
            } else {
                Ok(part.clone())
            }
        })
        .collect()
}

pub fn execute_lab_run(tidex_home: &Path, request: &LabRunRequest) -> BrainResult<LabRunReceipt> {
    execute_lab_run_cancelable(tidex_home, request, None)
}

fn execute_lab_run_cancelable(
    tidex_home: &Path,
    request: &LabRunRequest,
    cancellation: Option<&AtomicBool>,
) -> BrainResult<LabRunReceipt> {
    let recipe = find_recipe(&request.recipe_id)?;
    let assets = resolve_assets(request, &recipe)?;
    let argv = render_argv(&recipe, &assets)?;
    let source_tree_sha256 = Sha256Digest::parse(env!("TIDEX_SOURCE_TREE_DIGEST"))?;
    let run_id = Sha256Digest::digest_domain(
        b"CEREBRO:TIDEX:LAB-RUN:v1\0",
        &serde_json::to_vec(&(
            &request.recipe_id,
            &argv,
            &request.selected_model_ids,
            &request.dataset_sha256,
            now_nanos()?,
            &source_tree_sha256,
        ))?,
    );
    let run_root = tidex_home.join("lab/runs/by-sha").join(run_id.as_str());
    ensure_private_dir(&run_root)?;
    let stdout_path = run_root.join("stdout.json");
    let stderr_path = run_root.join("stderr.txt");
    let stdout_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stdout_path)?;
    stdout_file.set_permissions(fs::Permissions::from_mode(0o600))?;
    let stderr_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stderr_path)?;
    stderr_file.set_permissions(fs::Permissions::from_mode(0o600))?;
    let current_executable = std::env::current_exe()?;
    let executable = match &recipe.executable {
        LabExecutable::Tidex => current_executable.clone(),
        LabExecutable::SiblingBinary { name } => {
            if name.is_empty()
                || name.len() > 128
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            {
                return Err(BrainError::Integrity("lab_recipe_binary_invalid".into()));
            }
            let path = current_executable
                .parent()
                .ok_or_else(|| BrainError::Integrity("lab_executable_parent_missing".into()))?
                .join(name);
            let meta = fs::symlink_metadata(&path)
                .map_err(|_| BrainError::Invalid("lab_recipe_binary_not_built".into()))?;
            if meta.file_type().is_symlink() || !meta.is_file() {
                return Err(BrainError::Integrity("lab_recipe_binary_invalid".into()));
            }
            path
        }
    };
    let maximum_runtime_seconds = request.maximum_runtime_seconds.unwrap_or(1_800);
    if !(1..=86_400).contains(&maximum_runtime_seconds) {
        return Err(BrainError::Invalid("lab_run_timeout_invalid".into()));
    }
    let mut child = Command::new(executable)
        .args(&argv)
        .env("TIDEX_LAB_CHILD", "1")
        .env("TIDEX_HOME", tidex_home)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()?;
    let started = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(maximum_runtime_seconds);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if cancellation.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(BrainError::Invalid("lab_run_cancelled".into()));
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(BrainError::Invalid("lab_run_timeout".into()));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let stdout_meta = fs::metadata(&stdout_path)?;
    let stderr_meta = fs::metadata(&stderr_path)?;
    if stdout_meta.len() > MAX_RESULT_BYTES || stderr_meta.len() > MAX_RESULT_BYTES {
        return Err(BrainError::Invalid("lab_run_output_limit".into()));
    }
    let stdout_sha256 = Sha256Digest::digest_bytes(&fs::read(&stdout_path)?);
    let stderr_sha256 = Sha256Digest::digest_bytes(&fs::read(&stderr_path)?);
    let exit_code = status.code().unwrap_or(-1);
    let executor_id = executor_for_lab_recipe(&recipe.id)?.map(|descriptor| descriptor.executor_id);
    let receipt = LabRunReceipt {
        schema: "cerebro.tidex.lab_run_receipt/v1".into(),
        run_id: run_id.clone(),
        recipe_id: recipe.id,
        executor_id,
        argv,
        selected_model_ids: request.selected_model_ids.clone(),
        dataset_sha256: request.dataset_sha256.clone(),
        source_tree_sha256,
        exit_code,
        stdout_sha256,
        stderr_sha256,
        stdout: stdout_path,
        stderr: stderr_path,
        succeeded: status.success(),
        production_activation_recipe: recipe.production_activation,
        authorizes_production: false,
    };
    let receipt_bytes = serde_json::to_vec(&receipt)?;
    write_private_new(&run_root.join("receipt.json"), &receipt_bytes)?;
    Ok(receipt)
}

fn now_nanos() -> BrainResult<u128> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| BrainError::Invalid("lab_system_clock_invalid".into()))?
        .as_nanos())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BehavioralDiscoveryWorkflowRequest {
    pub schema: String,
    pub model_ids: Vec<Sha256Digest>,
    pub dataset_sha256: Sha256Digest,
    #[serde(default = "default_max_new_tokens")]
    pub max_new_tokens: usize,
    #[serde(default)]
    pub seed: u64,
}

fn default_max_new_tokens() -> usize {
    128
}

fn load_catalog_model(tidex_home: &Path, id: &Sha256Digest) -> BrainResult<LocalModelCandidate> {
    let path = tidex_home
        .join("lab/models/by-sha")
        .join(format!("{}.json", id));
    let bytes =
        fs::read(&path).map_err(|_| BrainError::Invalid("lab_model_not_cataloged".into()))?;
    let model: LocalModelCandidate = serde_json::from_slice(&bytes)?;
    if &model.model_id != id {
        return Err(BrainError::Integrity(
            "lab_model_catalog_identity_mismatch".into(),
        ));
    }
    Ok(model)
}

fn load_dataset_manifest(tidex_home: &Path, id: &Sha256Digest) -> BrainResult<LabDatasetManifest> {
    let path = tidex_home
        .join("lab/datasets/manifests")
        .join(format!("{}.json", id));
    let bytes =
        fs::read(&path).map_err(|_| BrainError::Invalid("lab_dataset_not_cataloged".into()))?;
    let manifest: LabDatasetManifest = serde_json::from_slice(&bytes)?;
    if &manifest.content_sha256 != id {
        return Err(BrainError::Integrity(
            "lab_dataset_manifest_identity_mismatch".into(),
        ));
    }
    let artifact = fs::read(&manifest.artifact)?;
    if Sha256Digest::digest_bytes(&artifact) != *id {
        return Err(BrainError::Integrity(
            "lab_dataset_artifact_digest_mismatch".into(),
        ));
    }
    Ok(manifest)
}

pub fn list_catalog_models(tidex_home: &Path) -> BrainResult<Vec<LocalModelCandidate>> {
    let root = tidex_home.join("lab/models/by-sha");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let bytes = fs::read(entry.path())?;
        let model: LocalModelCandidate = serde_json::from_slice(&bytes)?;
        out.push(model);
    }
    out.sort_by(|a, b| a.root.cmp(&b.root));
    Ok(out)
}

pub fn list_datasets(tidex_home: &Path) -> BrainResult<Vec<LabDatasetManifest>> {
    let root = tidex_home.join("lab/datasets/manifests");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let bytes = fs::read(entry.path())?;
        let manifest: LabDatasetManifest = serde_json::from_slice(&bytes)?;
        out.push(manifest);
    }
    out.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.content_sha256.cmp(&b.content_sha256))
    });
    Ok(out)
}

fn configured_hf_python_candidate() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("TIDEX_HF_PYTHON") {
        return Some(PathBuf::from(value));
    }

    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/models.toml");
    let Ok(contents) = std::fs::read_to_string(&config_path) else {
        return None;
    };

    for line in contents.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("python_executable") {
            continue;
        }
        let Some(rest) = trimmed.split_once('=') else {
            continue;
        };
        let candidate = rest.1.trim().trim_matches('"').trim_matches('\'');
        if !candidate.is_empty() {
            return Some(PathBuf::from(candidate));
        }
    }

    default_mechinterp_python().ok()
}

fn usable_python_path(candidate: PathBuf) -> BrainResult<Option<PathBuf>> {
    let selected = if candidate.is_absolute() {
        candidate
    } else {
        candidate.canonicalize()?
    };
    if !selected.exists() {
        return Ok(None);
    }
    let resolved = selected.canonicalize()?;
    if resolved.is_file() {
        Ok(Some(selected))
    } else {
        Ok(None)
    }
}

fn find_python3() -> BrainResult<PathBuf> {
    if let Some(candidate) = configured_hf_python_candidate() {
        if let Some(selected) = usable_python_path(candidate)? {
            return Ok(selected);
        }
    }

    let path_env =
        std::env::var_os("PATH").ok_or_else(|| BrainError::Invalid("path_unavailable".into()))?;
    for dir in std::env::split_paths(&path_env) {
        let candidate = dir.join("python3");
        if let Some(selected) = usable_python_path(candidate)? {
            return Ok(selected);
        }
    }
    Err(BrainError::Invalid("python3_not_found".into()))
}

pub fn execute_behavioral_discovery_workflow(
    tidex_home: &Path,
    request: &BehavioralDiscoveryWorkflowRequest,
) -> BrainResult<LabRunReceipt> {
    execute_behavioral_discovery_workflow_cancelable(tidex_home, request, None)
}

fn execute_behavioral_discovery_workflow_cancelable(
    tidex_home: &Path,
    request: &BehavioralDiscoveryWorkflowRequest,
    cancellation: Option<&AtomicBool>,
) -> BrainResult<LabRunReceipt> {
    if request.schema != "cerebro.tidex.lab_behavioral_discovery/v1"
        || request.model_ids.len() < 2
        || request.model_ids.len() > 64
        || request.max_new_tokens == 0
        || request.max_new_tokens > 4096
    {
        return Err(BrainError::Invalid(
            "lab_behavioral_discovery_request_invalid".into(),
        ));
    }
    let mut unique = std::collections::BTreeSet::new();
    if request
        .model_ids
        .iter()
        .any(|id| !unique.insert(id.clone()))
    {
        return Err(BrainError::Invalid(
            "lab_behavioral_discovery_model_duplicate".into(),
        ));
    }
    let dataset = load_dataset_manifest(tidex_home, &request.dataset_sha256)?;
    if dataset.format != LabDatasetFormat::Json {
        return Err(BrainError::Invalid(
            "behavioral_benchmark_requires_json_dataset".into(),
        ));
    }
    let benchmark_bytes = fs::read(&dataset.artifact)?;
    let benchmark: serde_json::Value = serde_json::from_slice(&benchmark_bytes)?;
    if benchmark.get("schema").and_then(|v| v.as_str())
        != Some("cerebro.cross_model.behavioral_benchmark/v1")
    {
        return Err(BrainError::Invalid(
            "dataset_is_not_behavioral_benchmark".into(),
        ));
    }
    let python = find_python3()?;
    let mut models = Vec::with_capacity(request.model_ids.len());
    for id in &request.model_ids {
        let model = load_catalog_model(tidex_home, id)?;
        if model.layout != LocalModelLayout::HuggingFaceSingleSafetensors {
            return Err(BrainError::Invalid(
                "lab_hf_workflow_requires_single_safetensors".into(),
            ));
        }
        let name = model
            .root
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| BrainError::Invalid("lab_model_name_unavailable".into()))?;
        models.push(serde_json::json!({
            "backend":"hf_transformers",
            "runtime":{
                "name": format!("lab-{}", &id.as_str()[..12]),
                "python_executable": python,
                "model_dir": model.root,
                "checkpoint_path": model.weights,
                "config_path": model.config,
                "tokenizer_path": model.tokenizer,
                "threads": 1,
                "generation":{
                    "temperature":0.0,
                    "top_p":1.0,
                    "max_tokens":request.max_new_tokens,
                    "seed":request.seed,
                    "keep_alive":"1m",
                    "request_timeout_seconds":600,
                    "think":false
                },
                "require_nnsight":false,
                "require_sae_lens":false
            },
            "catalog_label": name
        }));
        // RuntimeConfig denies unknown fields, so remove the human-only label.
        models
            .last_mut()
            .and_then(|v| v.as_object_mut())
            .map(|o| o.remove("catalog_label"));
    }
    let runtime = serde_json::json!({
        "schema":"cerebro.cross_model.runtime/v2",
        "models":models,
        "maximum_stored_capabilities":100000
    });
    let workflow_root = tidex_home.join("lab/workflow-inputs");
    ensure_private_dir(&workflow_root)?;
    let runtime_bytes = serde_json::to_vec(&runtime)?;
    let runtime_sha = Sha256Digest::digest_bytes(&runtime_bytes);
    let runtime_path = workflow_root.join(format!("runtime-{}.json", runtime_sha));
    if !runtime_path.exists() {
        write_private_new(&runtime_path, &runtime_bytes)?;
    }
    execute_lab_run_cancelable(
        tidex_home,
        &LabRunRequest {
            schema: "cerebro.tidex.lab_run_request/v1".into(),
            recipe_id: "cross_model.discovery_cycle".into(),
            assets: vec![runtime_path, dataset.artifact],
            selected_model_ids: request.model_ids.clone(),
            dataset_sha256: Some(request.dataset_sha256.clone()),
            maximum_runtime_seconds: Some(1_800),
        },
        cancellation,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LabDirectOperation {
    ProbeRuntime,
    BehavioralEvaluation,
    ExtractCapability,
    DeepInstrumentation,
    SparseAutoencoderAnalysis,
    CounterfactualAnalysis,
    GenerateBehavioralDataset,
    CalibrateAlignment,
    ActivationTransferExperiment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LabDirectWorkflowRequest {
    pub schema: String,
    pub operation: LabDirectOperation,
    pub model_ids: Vec<Sha256Digest>,
    #[serde(default)]
    pub dataset_sha256: Option<Sha256Digest>,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabInfo {
    pub schema: String,
    pub lab_home: PathBuf,
    pub default_model_scan_root: Option<PathBuf>,
    /// Interpreter path selected from TIDEX_HF_PYTHON/config/default. This is
    /// the exact path executed so virtualenv symlinks keep their environment.
    pub hf_python: Option<PathBuf>,
    pub configured_hf_python: Option<PathBuf>,
    pub resolved_hf_python: Option<PathBuf>,
    pub nnsight_available: bool,
    pub sae_lens_available: bool,
}

fn python_module_available(python: &Path, module: &str) -> bool {
    Command::new(python)
        .arg("-c")
        .arg(format!("__import__({module:?})"))
        .env("PYTHONNOUSERSITE", "1")
        .env("TRANSFORMERS_NO_TF", "1")
        .env("USE_TF", "0")
        .env("HF_HUB_OFFLINE", "1")
        .env("TOKENIZERS_PARALLELISM", "false")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub fn lab_info(tidex_home: &Path) -> BrainResult<LabInfo> {
    let default_model_scan_root = default_hf_hub_root().ok().filter(|path| path.is_dir());
    let configured_hf_python = configured_hf_python_candidate();
    let hf_python = find_python3().ok();
    let resolved_hf_python = hf_python
        .as_deref()
        .and_then(|python| python.canonicalize().ok());
    let nnsight_available = hf_python
        .as_deref()
        .is_some_and(|python| python_module_available(python, "nnsight"));
    let sae_lens_available = hf_python
        .as_deref()
        .is_some_and(|python| python_module_available(python, "sae_lens"));
    Ok(LabInfo {
        schema: "cerebro.tidex.lab_info/v1".into(),
        lab_home: tidex_home.to_path_buf(),
        default_model_scan_root,
        hf_python,
        configured_hf_python,
        resolved_hf_python,
        nnsight_available,
        sae_lens_available,
    })
}

fn hf_runtime_for_model(
    tidex_home: &Path,
    id: &Sha256Digest,
    require_nnsight: bool,
    require_sae_lens: bool,
    max_tokens: usize,
    allow_sharded_probe: bool,
) -> BrainResult<serde_json::Value> {
    let model = load_catalog_model(tidex_home, id)?;
    if model.layout != LocalModelLayout::HuggingFaceSingleSafetensors
        && !(allow_sharded_probe && model.layout == LocalModelLayout::HuggingFaceShardedSafetensors)
    {
        return Err(BrainError::Invalid(
            "lab_hf_workflow_requires_single_safetensors".into(),
        ));
    }
    let python = find_python3()?;
    Ok(serde_json::json!({
        "name": format!("lab-{}", &id.as_str()[..12]),
        "python_executable": python,
        "model_dir": model.root,
        "checkpoint_path": model.weights,
        "config_path": model.config,
        "tokenizer_path": model.tokenizer,
        "threads": 1,
        "generation":{
            "temperature":0.0,
            "top_p":1.0,
            "max_tokens":max_tokens,
            "seed":0,
            "keep_alive":"1m",
            "request_timeout_seconds":600,
            "think":false
        },
        "require_nnsight":require_nnsight,
        "require_sae_lens":require_sae_lens
    }))
}

pub fn execute_direct_workflow(
    tidex_home: &Path,
    request: &LabDirectWorkflowRequest,
) -> BrainResult<LabRunReceipt> {
    execute_direct_workflow_cancelable(tidex_home, request, None)
}

fn execute_direct_workflow_cancelable(
    tidex_home: &Path,
    request: &LabDirectWorkflowRequest,
    cancellation: Option<&AtomicBool>,
) -> BrainResult<LabRunReceipt> {
    if request.schema != "cerebro.tidex.lab_direct_workflow/v1" {
        return Err(BrainError::Invalid(
            "lab_direct_workflow_schema_invalid".into(),
        ));
    }
    let expected_models = match request.operation {
        LabDirectOperation::ProbeRuntime
        | LabDirectOperation::BehavioralEvaluation
        | LabDirectOperation::ExtractCapability
        | LabDirectOperation::DeepInstrumentation
        | LabDirectOperation::SparseAutoencoderAnalysis
        | LabDirectOperation::CounterfactualAnalysis
        | LabDirectOperation::GenerateBehavioralDataset => 1,
        LabDirectOperation::CalibrateAlignment
        | LabDirectOperation::ActivationTransferExperiment => 2,
    };
    if request.model_ids.len() != expected_models {
        return Err(BrainError::Invalid(
            "lab_direct_workflow_model_count_invalid".into(),
        ));
    }
    let mut unique = std::collections::BTreeSet::new();
    if request
        .model_ids
        .iter()
        .any(|id| !unique.insert(id.clone()))
    {
        return Err(BrainError::Invalid(
            "lab_direct_workflow_model_duplicate".into(),
        ));
    }
    let parameters = request.parameters.as_object().cloned().unwrap_or_default();
    let mut payload = serde_json::Map::new();
    for (key, value) in parameters {
        if matches!(
            key.as_str(),
            "operation" | "model" | "source" | "target" | "benchmark" | "evaluation"
        ) {
            return Err(BrainError::Invalid(
                "lab_direct_workflow_reserved_parameter".into(),
            ));
        }
        payload.insert(key, value);
    }
    let op_name = match request.operation {
        LabDirectOperation::ProbeRuntime => "probe_runtime",
        LabDirectOperation::BehavioralEvaluation => "behavioral_evaluation",
        LabDirectOperation::ExtractCapability => "extract_capability",
        LabDirectOperation::DeepInstrumentation => "deep_instrumentation",
        LabDirectOperation::SparseAutoencoderAnalysis => "sparse_autoencoder_analysis",
        LabDirectOperation::CounterfactualAnalysis => "counterfactual_analysis",
        LabDirectOperation::GenerateBehavioralDataset => "generate_behavioral_dataset",
        LabDirectOperation::CalibrateAlignment => "calibrate_alignment",
        LabDirectOperation::ActivationTransferExperiment => "activation_transfer_experiment",
    };
    payload.insert(
        "operation".into(),
        serde_json::Value::String(op_name.into()),
    );
    let requires_nnsight = matches!(
        request.operation,
        LabDirectOperation::DeepInstrumentation | LabDirectOperation::SparseAutoencoderAnalysis
    );
    let requires_sae = matches!(
        request.operation,
        LabDirectOperation::SparseAutoencoderAnalysis
    );
    if expected_models == 1 {
        let max_tokens = if matches!(
            request.operation,
            LabDirectOperation::GenerateBehavioralDataset
        ) {
            512
        } else {
            128
        };
        payload.insert(
            "model".into(),
            hf_runtime_for_model(
                tidex_home,
                &request.model_ids[0],
                requires_nnsight,
                requires_sae,
                max_tokens,
                matches!(request.operation, LabDirectOperation::ProbeRuntime),
            )?,
        );
    } else {
        payload.insert(
            "source".into(),
            hf_runtime_for_model(tidex_home, &request.model_ids[0], false, false, 128, false)?,
        );
        payload.insert(
            "target".into(),
            hf_runtime_for_model(tidex_home, &request.model_ids[1], false, false, 128, false)?,
        );
    }
    if matches!(
        request.operation,
        LabDirectOperation::BehavioralEvaluation | LabDirectOperation::ActivationTransferExperiment
    ) {
        let dataset_id = request
            .dataset_sha256
            .as_ref()
            .ok_or_else(|| BrainError::Invalid("lab_direct_workflow_dataset_required".into()))?;
        let dataset = load_dataset_manifest(tidex_home, dataset_id)?;
        if dataset.format != LabDatasetFormat::Json {
            return Err(BrainError::Invalid(
                "lab_direct_workflow_benchmark_must_be_json".into(),
            ));
        }
        let bytes = fs::read(&dataset.artifact)?;
        let benchmark: serde_json::Value = serde_json::from_slice(&bytes)?;
        if benchmark.get("schema").and_then(|v| v.as_str())
            != Some("cerebro.cross_model.behavioral_benchmark/v1")
        {
            return Err(BrainError::Invalid(
                "dataset_is_not_behavioral_benchmark".into(),
            ));
        }
        let key = if matches!(request.operation, LabDirectOperation::BehavioralEvaluation) {
            "benchmark"
        } else {
            "evaluation"
        };
        payload.insert(key.into(), benchmark);
    }
    let request_value = serde_json::Value::Object(payload);
    let bytes = serde_json::to_vec(&request_value)?;
    let digest = Sha256Digest::digest_bytes(&bytes);
    let root = tidex_home.join("lab/workflow-inputs");
    ensure_private_dir(&root)?;
    let path = root.join(format!("direct-{}.json", digest));
    if !path.exists() {
        write_private_new(&path, &bytes)?;
    }
    execute_lab_run_cancelable(
        tidex_home,
        &LabRunRequest {
            schema: "cerebro.tidex.lab_run_request/v1".into(),
            recipe_id: "lab.direct_runner".into(),
            assets: vec![path],
            selected_model_ids: request.model_ids.clone(),
            dataset_sha256: request.dataset_sha256.clone(),
            maximum_runtime_seconds: Some(
                if matches!(
                    request.operation,
                    LabDirectOperation::GenerateBehavioralDataset
                ) {
                    300
                } else {
                    1_800
                },
            ),
        },
        cancellation,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LabPlasticityAdvice {
    pub schema: String,
    pub available: bool,
    pub source_jobs: usize,
    pub elo_leaderboard: Vec<(String, f64)>,
    pub elo_entities: Vec<LabEloEntityAdvice>,
    pub routing_decisions: Vec<serde_json::Value>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LabEloEntityAdvice {
    pub entity: String,
    pub rating: f64,
    pub comparisons: usize,
    pub last_evidence_sha256: Option<String>,
    pub last_update: String,
}

#[cfg(feature = "cross-model-plasticity")]
pub fn compute_lab_plasticity_advice(tidex_home: &Path) -> BrainResult<LabPlasticityAdvice> {
    use crate::cross_model::plasticity::{ELOSystem, RoutingObservation, RoutingPlasticity};

    let mut elo = ELOSystem::default();
    let mut routing = RoutingPlasticity::default();
    let mut known_models = std::collections::BTreeSet::<String>::new();
    let mut by_domain = std::collections::BTreeMap::<
        String,
        std::collections::BTreeMap<String, RoutingObservation>,
    >::new();
    let mut source_jobs = 0usize;
    let mut notes = Vec::new();

    for record in list_job_records(tidex_home)? {
        if record.state != LabJobState::Completed {
            continue;
        }
        let run = match record.run.as_ref() {
            Some(run) => run,
            None => continue,
        };
        let value = match serde_json::from_str::<serde_json::Value>(&run.stdout) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let evaluations = if value.get("schema").and_then(serde_json::Value::as_str)
            == Some("cerebro.cross_model.discovery_cycle/v1")
        {
            value
                .get("evaluations")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default()
        } else if value.get("schema").and_then(serde_json::Value::as_str)
            == Some("cerebro.cross_model.model_evaluation/v1")
        {
            vec![value]
        } else {
            Vec::new()
        };
        if evaluations.is_empty() {
            continue;
        }
        source_jobs = source_jobs
            .checked_add(1)
            .ok_or_else(|| BrainError::Invalid("lab_plasticity_source_counter_overflow".into()))?;
        let mut parsed = Vec::new();
        for evaluation in evaluations {
            let model = evaluation
                .get("model")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let score = evaluation
                .get("weighted_score")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(f64::NAN);
            let evidence = evaluation
                .get("evidence_sha256")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let benchmark = evaluation
                .get("benchmark_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown_benchmark")
                .to_string();
            let sample_size = evaluation
                .get("observations")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            if model.is_empty()
                || !score.is_finite()
                || !(0.0..=1.0).contains(&score)
                || !Sha256Digest::is_valid_str(&evidence)
                || sample_size == 0
            {
                notes.push(format!("ignored_evaluation:{}", benchmark));
                continue;
            }
            if known_models.insert(model.clone()) {
                let _ = elo.initialize_rating(&model);
            }
            by_domain
                .entry(benchmark.clone())
                .or_default()
                .entry(model.clone())
                .or_insert(RoutingObservation {
                    model: model.clone(),
                    score,
                    sample_size,
                    evidence_sha256: evidence.clone(),
                });
            parsed.push((model, score, evidence));
        }
        for left in 0..parsed.len() {
            for right in left + 1..parsed.len() {
                let first_observed =
                    (0.5 + (parsed[left].1 - parsed[right].1) / 2.0).clamp(0.0, 1.0);
                let evidence = Sha256Digest::digest_domain(
                    b"CEREBRO:TIDEX:LAB-ELO-PAIR:v1\0",
                    &serde_json::to_vec(&(
                        &record.job_id,
                        &parsed[left].0,
                        &parsed[left].2,
                        &parsed[right].0,
                        &parsed[right].2,
                    ))?,
                );
                let _ = elo.update_observed(
                    &parsed[left].0,
                    &parsed[right].0,
                    first_observed,
                    evidence.as_str(),
                );
            }
        }
    }

    let mut routing_decisions = Vec::new();
    for (benchmark, observations_by_model) in by_domain {
        if observations_by_model.len() < 2 {
            continue;
        }
        let scope = format!("benchmark:{benchmark}");
        let observations = observations_by_model.into_values().collect::<Vec<_>>();
        match routing.route_capability(&scope, &observations) {
            Ok(decision) => routing_decisions.push(serde_json::to_value(decision)?),
            Err(error) => notes.push(format!("routing_ignored:{scope}:{error}")),
        }
    }

    let elo_entities = elo
        .get_leaderboard()
        .into_iter()
        .filter_map(|(entity, rating)| {
            elo.get_state(&entity).map(|state| LabEloEntityAdvice {
                entity,
                rating,
                comparisons: state.comparisons,
                last_evidence_sha256: state.last_evidence_sha256.clone(),
                last_update: state.last_update.clone(),
            })
        })
        .collect();

    Ok(LabPlasticityAdvice {
        schema: "cerebro.tidex.lab_plasticity_advice/v1".into(),
        available: true,
        source_jobs,
        elo_leaderboard: elo.get_leaderboard(),
        elo_entities,
        routing_decisions,
        notes,
    })
}

#[cfg(not(feature = "cross-model-plasticity"))]
pub fn compute_lab_plasticity_advice(_tidex_home: &Path) -> BrainResult<LabPlasticityAdvice> {
    Ok(LabPlasticityAdvice {
        schema: "cerebro.tidex.lab_plasticity_advice/v1".into(),
        available: false,
        source_jobs: 0,
        elo_leaderboard: Vec::new(),
        elo_entities: Vec::new(),
        routing_decisions: Vec::new(),
        notes: vec!["cross-model-plasticity feature disabled".into()],
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelScanRequest {
    root: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DatasetImportRequest {
    name: String,
    format: LabDatasetFormat,
    content: String,
    #[serde(default)]
    generated: bool,
}

fn http_response(status: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\n\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    out
}

fn parse_request(stream: &mut TcpStream) -> BrainResult<(String, String, Vec<u8>)> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() > MAX_HTTP_BODY_BYTES + 64 * 1024 {
            return Err(BrainError::Invalid("lab_http_request_too_large".into()));
        }
        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let header_end = header_end + 4;
            let header = std::str::from_utf8(&bytes[..header_end])
                .map_err(|_| BrainError::Invalid("lab_http_header_invalid".into()))?;
            let content_length = header
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length:"))
                .or_else(|| {
                    header
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                })
                .map(str::trim)
                .map(str::parse::<usize>)
                .transpose()
                .map_err(|_| BrainError::Invalid("lab_http_content_length_invalid".into()))?
                .unwrap_or(0);
            if content_length > MAX_HTTP_BODY_BYTES {
                return Err(BrainError::Invalid("lab_http_body_too_large".into()));
            }
            if bytes.len() >= header_end + content_length {
                break;
            }
        }
    }
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| BrainError::Invalid("lab_http_header_incomplete".into()))?
        + 4;
    let header = std::str::from_utf8(&bytes[..header_end])
        .map_err(|_| BrainError::Invalid("lab_http_header_invalid".into()))?;
    let mut first = header.lines().next().unwrap_or_default().split_whitespace();
    let method = first.next().unwrap_or_default().to_string();
    let path = first.next().unwrap_or_default().to_string();
    let version = first.next();
    if version != Some("HTTP/1.1") || first.next().is_some() {
        return Err(BrainError::Invalid("lab_http_request_line_invalid".into()));
    }
    Ok((method, path, bytes[header_end..].to_vec()))
}

fn json_response<T: Serialize>(value: &T) -> BrainResult<Vec<u8>> {
    Ok(http_response(
        "200 OK",
        "application/json; charset=utf-8",
        &serde_json::to_vec(value)?,
    ))
}

fn json_response_status<T: Serialize>(status: &str, value: &T) -> BrainResult<Vec<u8>> {
    Ok(http_response(
        status,
        "application/json; charset=utf-8",
        &serde_json::to_vec(value)?,
    ))
}

fn handle_http(tidex_home: &Path, stream: &mut TcpStream) -> BrainResult<()> {
    let (method, path, body) = parse_request(stream)?;
    let response = match (method.as_str(), path.as_str()) {
        ("GET", "/") => http_response("200 OK", "text/html; charset=utf-8", LAB_HTML.as_bytes()),
        ("GET", "/api/info") => json_response(&lab_info(tidex_home)?)?,
        ("GET", "/api/recipes") => json_response(&recipe_catalog())?,
        ("GET", "/api/executors") => json_response(&executor_catalog()?)?,
        ("GET", "/api/models") => json_response(&list_catalog_models(tidex_home)?)?,
        ("GET", "/api/model-profiles") => json_response(&list_runtime_profiles(tidex_home)?)?,
        ("GET", "/api/model-runtime-statuses") => {
            json_response(&list_runtime_statuses(tidex_home)?)?
        }
        ("GET", "/api/datasets") => json_response(&list_datasets(tidex_home)?)?,
        ("GET", "/api/jobs") => json_response(&list_job_records(tidex_home)?)?,
        ("GET", "/api/plasticity") => json_response(&compute_lab_plasticity_advice(tidex_home)?)?,
        ("POST", "/api/models/scan") => {
            let request: ModelScanRequest = serde_json::from_slice(&body)?;
            json_response(&catalog_local_models(tidex_home, &request.root)?)?
        }
        ("POST", "/api/datasets/import") => {
            let request: DatasetImportRequest = serde_json::from_slice(&body)?;
            json_response(&import_dataset_bytes(
                tidex_home,
                &request.name,
                request.format,
                request.content.as_bytes(),
                request.generated,
            )?)?
        }
        ("POST", "/api/runs") => {
            let request: LabRunRequest = serde_json::from_slice(&body)?;
            json_response(&lab_run_view(execute_lab_run(tidex_home, &request)?)?)?
        }
        ("POST", "/api/workflows/behavioral-discovery") => {
            let request: BehavioralDiscoveryWorkflowRequest = serde_json::from_slice(&body)?;
            json_response_status(
                "202 Accepted",
                &start_lab_job(tidex_home, LabJobRequest::BehavioralDiscovery(request))?,
            )?
        }
        ("POST", "/api/workflows/direct") => {
            let request: LabDirectWorkflowRequest = serde_json::from_slice(&body)?;
            json_response_status(
                "202 Accepted",
                &start_lab_job(tidex_home, LabJobRequest::Direct(request))?,
            )?
        }
        ("POST", dynamic_path)
            if dynamic_path.starts_with("/api/jobs/") && dynamic_path.ends_with("/cancel") =>
        {
            let raw = dynamic_path
                .trim_start_matches("/api/jobs/")
                .trim_end_matches("/cancel");
            let job_id = Sha256Digest::parse(raw)
                .map_err(|_| BrainError::Invalid("lab_job_id_invalid".into()))?;
            json_response(&cancel_lab_job(tidex_home, &job_id)?)?
        }
        ("GET", dynamic_path) if dynamic_path.starts_with("/api/jobs/") => {
            let raw = dynamic_path.trim_start_matches("/api/jobs/");
            let job_id = Sha256Digest::parse(raw)
                .map_err(|_| BrainError::Invalid("lab_job_id_invalid".into()))?;
            json_response(&load_job_record(tidex_home, &job_id)?)?
        }
        _ => http_response(
            "404 Not Found",
            "application/json; charset=utf-8",
            br#"{"error":"not_found"}"#,
        ),
    };
    stream.write_all(&response)?;
    stream.flush()?;
    Ok(())
}

/// Serve the local laboratory. Binding to non-loopback addresses is rejected.
pub fn serve_lab(tidex_home: &Path, address: SocketAddr) -> BrainResult<()> {
    if !matches!(address.ip(), IpAddr::V4(ip) if ip.is_loopback())
        && !matches!(address.ip(), IpAddr::V6(ip) if ip.is_loopback())
    {
        return Err(BrainError::Invalid("lab_server_requires_loopback".into()));
    }
    ensure_private_dir(&tidex_home.join("lab"))?;
    let recovered = recover_incomplete_lab_jobs(tidex_home)?;
    if recovered > 0 {
        eprintln!("TIDE-X Lab recovered {recovered} incomplete/inconsistent jobs");
    }
    let listener = TcpListener::bind(address)?;
    eprintln!("TIDE-X Advanced Laboratory: http://{address}");
    for incoming in listener.incoming() {
        let mut stream = incoming?;
        if !stream.peer_addr()?.ip().is_loopback() {
            continue;
        }
        if let Err(error) = handle_http(tidex_home, &mut stream) {
            let body = serde_json::to_vec(&serde_json::json!({"error": error.to_string()}))?;
            let _ = stream.write_all(&http_response(
                "400 Bad Request",
                "application/json; charset=utf-8",
                &body,
            ));
        }
    }
    Ok(())
}

const LAB_HTML: &str = r#"<!doctype html>
<html lang="es"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>TIDE-X Control de ejecución</title>
<style>
:root{color-scheme:dark;font-family:Inter,ui-sans-serif,system-ui;background:#071019;color:#eaf2f8}*{box-sizing:border-box}body{margin:0;background:linear-gradient(180deg,#071019,#0a121c 45%,#071019)}header{position:sticky;top:0;z-index:5;display:flex;justify-content:space-between;align-items:center;padding:18px 28px;background:#08111beF;border-bottom:1px solid #213141;backdrop-filter:blur(12px)}h1{font-size:20px;margin:0}.sub{color:#91a4b6;font-size:13px;margin-top:4px}.status{font:12px ui-monospace,monospace;color:#7de2a2}.shell{max-width:1500px;margin:auto;padding:22px}.steps{display:grid;grid-template-columns:1fr 1.35fr 1fr;gap:18px}.panel{background:#0c1722;border:1px solid #213141;border-radius:14px;padding:18px;box-shadow:0 10px 28px #0004}.panel h2{font-size:15px;margin:0 0 12px}.panel h3{font-size:13px;margin:18px 0 8px;color:#adc2d3}.muted{color:#8398aa;font-size:12px}.model{display:flex;gap:10px;padding:10px;border:1px solid #23384a;border-radius:10px;margin:7px 0;background:#0a141e}.model input{width:auto}.model small{display:block;color:#8197a9;overflow-wrap:anywhere}.badge{display:inline-block;padding:2px 7px;border:1px solid #38536b;border-radius:999px;font-size:10px;color:#a9c7db}.ok{border-color:#2f7049;color:#7ee19e}.warn{border-color:#765b2c;color:#e7bf6d}button,input,select,textarea{width:100%;border-radius:9px;border:1px solid #2b4052;background:#07111a;color:#eaf2f8;padding:10px;margin-top:7px}button{background:#12304a;font-weight:700;cursor:pointer}button:hover{background:#17405f}button.primary{background:#17613e}button.primary:hover{background:#1b7650}button:disabled{opacity:.45;cursor:not-allowed}textarea{min-height:95px;resize:vertical;font-family:ui-monospace,monospace;font-size:12px}.row{display:grid;grid-template-columns:1fr 1fr;gap:8px}.work{padding:12px;border:1px solid #2c4152;border-radius:10px;background:#0a141e}.result{margin-top:18px}.result pre{white-space:pre-wrap;word-break:break-word;max-height:520px;overflow:auto;background:#050b11;border:1px solid #1d2e3d;border-radius:10px;padding:14px;font-size:12px}.tabs{display:flex;gap:6px;margin-bottom:12px}.tabs button{width:auto;margin:0;padding:7px 11px}.hidden{display:none}.recipe-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(230px,1fr));gap:8px}.recipe{padding:10px;border:1px solid #273c4e;border-radius:9px}.recipe b{display:block;margin-bottom:5px}.metric-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:9px;margin:10px 0}.metric{border:1px solid #2a4051;border-radius:10px;padding:11px;background:#09131c}.metric .v{font-size:20px;font-weight:800;margin-top:4px}.metric .k{font-size:10px;color:#8197a9;text-transform:uppercase;letter-spacing:.06em}.scorebar{height:7px;background:#152533;border-radius:999px;overflow:hidden;margin-top:7px}.scorebar i{display:block;height:100%;background:#36a76a}.obs{border-top:1px solid #1f3344;padding:8px 0}.history{display:grid;gap:7px}.history-item{border:1px solid #293f50;border-radius:9px;padding:10px;cursor:pointer;background:#09131c}.history-item:hover{background:#0d1b27}.history-item .state{float:right}.good{color:#75dfa0}.bad{color:#ff8d8d}.running{color:#f2c46d}.raw-toggle{margin-top:10px}.raw-toggle summary{cursor:pointer;color:#8fa9bd}.section-title{font-size:13px;font-weight:800;margin:16px 0 6px}.model-order{font-size:11px;color:#78bce7;margin:4px 0}.result-card{border:1px solid #294052;border-radius:10px;padding:12px;margin:8px 0;background:#09131c}.modebar{display:flex;gap:8px;margin:0 0 14px}.modebar button{width:auto;margin:0}.modebar button.active{background:#17613e}.goal-grid{display:grid;grid-template-columns:1.4fr .8fr auto;gap:8px;align-items:end}.pipeline{display:grid;gap:8px;margin-top:12px}.pipeline-step{border:1px solid #294052;border-radius:10px;padding:11px;background:#09131c}.pipeline-step .head{display:flex;justify-content:space-between;gap:8px;align-items:center}.pipeline-step .why{color:#8197a9;font-size:11px;margin-top:5px}.pipeline-step.available{border-color:#2f7049}.pipeline-step.manual{border-color:#765b2c}.pipeline-step.blocked{border-color:#63363a}.area-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(230px,1fr));gap:9px}.area-card{border:1px solid #294052;border-radius:10px;padding:12px;background:#09131c;cursor:pointer}.area-card:hover{background:#0d1b27}.area-card b{display:block;margin-bottom:6px}.executor-row{border-top:1px solid #1f3344;padding:8px 0}.executor-row:first-child{border-top:0}.plasticity-tabs{display:flex;gap:6px;flex-wrap:wrap;margin-bottom:10px}.plasticity-tabs button{width:auto;margin:0}.plasticity-pane.hidden{display:none}.chip{display:inline-block;padding:2px 7px;border-radius:999px;border:1px solid #38536b;font-size:10px;margin:2px 3px 2px 0}.chip.op{border-color:#2f7049;color:#7ee19e}.chip.need{border-color:#765b2c;color:#e7bf6d}.chip.exp{border-color:#38536b;color:#78bce7}.chip.arch{border-color:#63363a;color:#ff9a9a}@media(max-width:1050px){.steps{grid-template-columns:1fr}.shell{padding:10px}.goal-grid{grid-template-columns:1fr}}
</style></head>
<body><header><div><h1 data-i18n="labTitle">Cerebro TIDE-X</h1><div class="sub" data-i18n="labSubtitle">Modelos → objetivos → flujo → ejecución real → evidencia</div></div><div style="display:flex;align-items:center;gap:10px"><button id="langToggle" style="width:auto;margin:0;padding:6px 10px;font-size:12px" onclick="toggleLanguage()">English</button><div id="status" class="status">inicializando…</div></div></header>
<div class="shell">
<div class="modebar"><button id="systemModeBtn" class="active" onclick="setLabMode('system')">Operación real · orquestación</button><button id="manualModeBtn" onclick="setLabMode('manual')">Ejecución manual · catálogo</button></div>
<section id="systemMode" class="panel">
  <h2>Objetivo y pipeline de ejecución</h2>
  <div class="goal-grid">
    <label class="muted">Objetivo del workflow<input id="goalText" placeholder="Ej.: transferir capacidad A → B manteniendo la integridad del receptor"></label>
    <label class="muted">Tipo de objetivo<select id="goalKind"><option value="investigate">Investigar modelo</option><option value="improve">Mejorar capacidad</option><option value="transfer" selected>Transferir capacidad</option><option value="validate">Validar candidato</option><option value="production">Preparar producción</option></select></label>
    <button class="primary" onclick="buildGoalPipeline()">Construir pipeline</button>
  </div>
  <div id="goalSummary" class="muted" style="margin-top:10px">Propósito: describir la intención del trabajo. Autoridad: sólo se usan ejecutores, workflows y artefactos ya registrados en el runtime de TIDE-X. Resultado: evidencia y decisión, no una mejora automática ni una promoción implícita.</div>
  <div id="goalPipeline" class="pipeline"></div>
</section>
<section id="manualMode" class="hidden">
<div class="panel"><h2>Autoridades y áreas del runtime</h2><div class="muted">Cada bloque corresponde a una autoridad real del sistema: modelos, datasets, evidencia, plasticidad, transferencia, evaluación, gobernanza y materialización. No existen módulos huérfanos ni acciones desacopladas del runtime ni de su workflow.</div><div id="areaGrid" class="area-grid" style="margin-top:12px"></div><div id="areaDetail" style="margin-top:12px"></div></div>
<div class="steps">
<section class="panel"><h2 data-i18n-key="modelsTitle">1 · Modelos</h2><div class="muted" data-i18n-key="modelsHelp">Selecciona uno para análisis o dos/más para comparación/transferencia.</div><div class="row"><input id="modelRoot" data-i18n-placeholder="modelRootPlaceholder" placeholder="directorio de modelos"><button data-i18n-key="scan" onclick="scanModels()">Escanear</button></div><div id="modelSelection" class="model-order">Sin modelos seleccionados.</div><div id="models"></div></section>
<section class="panel"><h2 data-i18n-key="workTitle">2 · Workflow / ejecución</h2><select id="operation" onchange="renderParams()">
<option value="behavioral_discovery_multi">Descubrir capacidades comparando varios LLM</option>
<option value="probe_runtime">Comprobar runtime y capacidades expuestas</option>
<option value="behavioral_evaluation">Evaluar capacidades de un LLM</option>
<option value="generate_behavioral_dataset">Generar benchmark de comportamiento</option>
<option value="extract_capability">Extraer representación interna de una capacidad</option>
<option value="deep_instrumentation">Instrumentación profunda con NNsight</option>
<option value="sparse_autoencoder_analysis">Analizar features SAE</option>
<option value="counterfactual_analysis">Análisis contrafactual</option>
<option value="calibrate_alignment">Calibrar alineamiento entre dos LLM</option>
<option value="activation_transfer_experiment">Experimento real de transferencia por steering A→B</option>
</select><div id="params" class="work"></div><button id="runBtn" class="primary" data-i18n-key="runWork" onclick="runWorkflow()">Ejecutar workflow</button><button id="cancelBtn" data-i18n-key="cancelJob" onclick="cancelActiveJob()" disabled>Cancelar job activo</button></section>
<section class="panel"><h2 data-i18n-key="datasetTitle">3 · Artefactos y datasets</h2><div class="muted" data-i18n-key="datasetHelp">Para benchmarks usa el schema <code>cerebro.cross_model.behavioral_benchmark/v1</code>. Los datasets generados quedan marcados como artefactos no independientes.</div><input id="datasetFile" type="file"><div class="row"><input id="dsName" placeholder="nombre del dataset"><select id="dsGenerated"><option value="false">externo / independiente</option><option value="true">generado / no independiente</option></select></div><button onclick="importFileDataset()">Importar artefacto</button><h3>Crear benchmark</h3><input id="benchId" placeholder="benchmark id"><input id="benchDomain" placeholder="dominio / capacidad"><textarea id="benchRows" placeholder="Una prueba por línea: prompt => respuesta esperada"></textarea><button onclick="createBenchmark()">Crear benchmark exacto</button><h3>Datasets disponibles</h3><div id="datasets"></div></section>
</div>
</section>
<section class="panel result"><div class="tabs"><button onclick="showTab('result')">Resultado · evidencia</button><button onclick="showTab('history')">Historial · jobs</button><button onclick="showTab('executors')">Ejecutores · contratos</button><button onclick="showTab('plasticity')">Plasticidad · aprendizaje</button><button onclick="showTab('advanced')">Operaciones avanzadas · autorías</button></div><div id="resultTab"><div id="summary"><p class="muted">Selecciona modelos, workflow y artefactos. La salida representa la evidencia del trabajo real, no una promesa de producción.</p></div><details class="raw-toggle"><summary>JSON / evidencia cruda</summary><pre id="output">Sin ejecución.</pre></details></div><div id="historyTab" class="hidden"><div id="history" class="history"></div></div><div id="executorsTab" class="hidden"><div id="executors" class="history"></div></div><div id="plasticityTab" class="hidden"><div id="plasticity" class="history"></div></div><div id="advancedTab" class="hidden"><div class="muted">Autoridades canónicas del runtime: cada receta mantiene su propósito, su contrato y su ámbito de producción. Para casos normales usa el pipeline principal y su evidencia.</div><textarea id="assets" placeholder="Una ruta absoluta por línea para recetas de runtime"></textarea><div id="recipes" class="recipe-grid"></div></div></section>
</div>
<script>
let state={info:null,models:[],profiles:[],runtimeStatuses:[],datasets:[],executors:[],recipes:[],jobs:[],plasticity:null,selectedDataset:null,generatedBenchmark:null,selectedModelOrder:[],activeJob:null,labMode:'system',goalPipeline:[]};
const $=id=>document.getElementById(id);
const esc=v=>String(v??'').replace(/[&<>\"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','\"':'&quot;',"'":'&#39;'}[c]));
const pct=v=>`${(Number(v||0)*100).toFixed(1)}%`;
const translations={
  es:{
    labTitle:'Cerebro TIDE-X',labSubtitle:'Modelos → objetivos → workflow real → evidencia y gobernanza',langButton:'English',ui:{modelsTitle:'1 · Modelos',modelsHelp:'Selecciona uno para análisis o dos o más para comparación, alineación o transferencia real.',modelRootPlaceholder:'directorio de modelos',scan:'Escanear',workTitle:'2 · Workflow / ejecución',runWork:'Ejecutar workflow',cancelJob:'Cancelar job activo',datasetTitle:'3 · Artefactos y datasets',datasetHelp:'Para benchmarks usa el schema cerebro.cross_model.behavioral_benchmark/v1. Los datasets generados quedan marcados como artefactos no independientes.'},
    states:{queued:'en cola',running:'ejecutando',completed:'completado',failed:'fallido',cancelled:'cancelado'},
    categories:{acquisition:'Adquisición',discovery:'Descubrimiento',learning:'Aprendizaje',benchmark:'Benchmark',receiver:'Receptor',materialization:'Materialización',evaluation:'Evaluación',governance:'Gobernanza',lifecycle:'Ciclo de vida'},
    operations:{behavioral_discovery:'descubrimiento conductual',probe_runtime:'comprobación de runtime',behavioral_evaluation:'evaluación conductual',extract_capability:'extracción de capacidad',deep_instrumentation:'instrumentación profunda',sparse_autoencoder_analysis:'análisis SAE',counterfactual_analysis:'análisis contrafactual',calibrate_alignment:'calibración de alineamiento',activation_transfer_experiment:'experimento de transferencia de activación',generate_behavioral_dataset:'generación de dataset conductual'},
    recipes:{
      'acquisition.capture':['Adquirir espacio de trabajo','Adquisición de fuentes vinculada al descriptor dentro del almacén privado.'],
      'knowledge.plan':['Planificar transición epistémica','Autentica un estado persistido de KnowledgeEngine y deriva la siguiente invocación admisible o una decisión terminal.'],
      'numerical.evolve':['Ejecutar campaña de evolución numérica','Ejecuta revisiones ordenadas de NumericalEvolution bajo una política de gobernanza explícita y sin promoción implícita.'],
      'analysis.tomography':['Ejecutar tomografía de campos de capacidades','Ejecuta la cadena canónica de BrainEngine, incluida tomografía, identificabilidad y diagnósticos estructurales.'],
      'analysis.protected_map':['Construir mapa de córtex protegido','Construye y persiste el mapa de protección canónico desde artefactos autenticados de sensibilidad, sin etiquetas de tarea.'],
      'analysis.pythagoras':['Analizar geometría / topología','Ejecuta corrección Pythagoras o análisis topológico persistente desde una solicitud tipada.'],
      'analysis.brain':['Analizar campos de capacidades','Ejecuta el análisis canónico de BrainEngine sobre observaciones autenticadas.'],
      'runtime.sleep':['Consolidación / reposo','Ejecuta la transacción canónica de reposo/consolidación vinculada a evidencia.'],
      'learning.autonomous_plan':['Plan de aprendizaje autónomo','Planifica una campaña de aprendizaje adaptativo desde un LearningTarget tipado.'],
      'cross_model.discovery_cycle':['Ciclo de descubrimiento multi-LLM','Ejecuta comparación conductual real entre runtimes LLM configurados explícitamente y vinculados a evidencia.'],
      'lab.direct_runner':['Ejecutor directo del runtime TIDE-X','Ejecuta una solicitud tipada del runtime cross-model usando el backend real y los componentes de análisis canónicos.'],
      'discovery.capabilities':['Descubrimiento de capacidades','Ejecuta descubrimiento canónico de capacidades desde una solicitud explícita dentro del runtime TIDE-X.'],
      'benchmark.response':['Benchmark de respuesta del receptor','Evalúa respuestas funcionales declaradas del receptor.'],
      'benchmark.receiver_basis':['Benchmark de la base del receptor','Evalúa la base del receptor bajo el protocolo declarado.'],
      'benchmark.portability':['Benchmark de compilación del receptor','Benchmark de compilación del receptor dejando una capacidad fuera.'],
      'receiver.profile':['Perfilar receptor','Autentica y perfila un checkpoint físico receptor.'],
      'receiver.normalize_sharded':['Normalizar SafeTensors fragmentados','Normaliza un checkpoint HF fragmentado autenticado en una única autoridad SafeTensors retenida.'],
      'receiver.freeze_compiler':['Congelar compilador del receptor','Congela calibración, protección, riesgo y mapas antes de compilar objetivos reservados.'],
      'receiver.verify_frozen':['Verificar compilador congelado','Reautentica por replay un compilador de receptor congelado.'],
      'compile.universal':['Compilar capacidad reservada','Compila un CapabilityIR autenticado con un compilador de receptor congelado.'],
      'compile.universal_plan':['Planificar candidato universal en sombra','Crea un plan de materialización en sombra vinculado al receptor.'],
      'materialize.compiled':['Materializar checkpoint compilado','Materializa físicamente un checkpoint candidato usando el actuador canónico.'],
      'materialize.verify':['Verificar checkpoint compilado','Reautentica por replay la aritmética física del checkpoint.'],
      'selection.backend':['Seleccionar backend de materialización','Ordena backends candidatos medidos bajo la política canónica de selección.'],
      'evaluation.shadow':['Ejecutar evaluación aislada en sombra','Ejecuta un evaluador autenticado y su bundle mediante ejecución aislada.'],
      'evidence.universality':['Medir universalidad','Reduce ensayos reservados entre capacidades, receptores, familias y semillas.'],
      'promotion.readiness':['Verificar preparación para promoción universal','Gate fail-closed de preparación; no activa producción.'],
      'adapter.import':['Importar adaptador','Importa un candidato de adaptador autenticado en AdapterBank.'],
      'adapter.compose':['Componer adaptadores','Crea una composición exacta y ordenada de adaptadores.'],
      'adapter.materialize':['Materializar candidato de adaptador','Crea una materialización de adaptador sólo como candidato.'],
      'adapter.authorize':['Autorizar promoción gobernada','Reautentica testigos de gobernanza sellados y emite un permiso de promoción para el estado actual.'],
      'adapter.activate':['Activar adaptador','Activación en producción. Requiere una autorización gobernada previamente autenticada.'],
      'adapter.revoke':['Revocar adaptador','Revocación gobernada persistente de un adaptador y sus composiciones dependientes.'],
      'adapter.rollback':['Revertir banco de adaptadores','Publica una nueva revisión hacia delante que representa el rollback.'],
      'adapter.status':['Verificar historial de AdapterBank','Verifica el historial completo enlazado por hash de AdapterBank.']
    }
  },
  en:{labTitle:'TIDE-X Brain Control',labSubtitle:'Models → goals → workflow → real execution → evidence and governance',langButton:'Español',ui:{modelsTitle:'1 · Models',modelsHelp:'Select one for analysis or two or more for real comparison, alignment or transfer.',modelRootPlaceholder:'models directory',scan:'Scan',workTitle:'2 · Workflow / execution',runWork:'Run workflow',cancelJob:'Cancel active job',datasetTitle:'3 · Artifacts and datasets',datasetHelp:'For benchmarks use schema cerebro.cross_model.behavioral_benchmark/v1. Generated datasets are marked as non-independent artifacts.'},states:{queued:'queued',running:'running',completed:'completed',failed:'failed',cancelled:'cancelled'},categories:{acquisition:'Acquisition',discovery:'Discovery',learning:'Learning',benchmark:'Benchmark',receiver:'Receiver',materialization:'Materialization',evaluation:'Evaluation',governance:'Governance',lifecycle:'Lifecycle'},operations:{behavioral_discovery:'behavioral discovery',probe_runtime:'runtime probe',behavioral_evaluation:'behavioral evaluation',extract_capability:'capability extraction',deep_instrumentation:'deep instrumentation',sparse_autoencoder_analysis:'SAE analysis',counterfactual_analysis:'counterfactual analysis',calibrate_alignment:'alignment calibration',activation_transfer_experiment:'activation transfer experiment',generate_behavioral_dataset:'behavioral dataset generation'},recipes:{}}
};
let currentLanguage=localStorage.getItem('tidex.lab.language')==='en'?'en':'es';
const trState=v=>translations[currentLanguage].states[v]||v;
const trCategory=v=>translations[currentLanguage].categories[String(v).toLowerCase()]||v;
const trOperation=v=>translations[currentLanguage].operations[v]||v;
function trRecipe(x){const es=translations.es.recipes[x.id];return currentLanguage==='es'&&es?{title:es[0],description:es[1]}:{title:x.title,description:x.description}}
function renderStatus(){if(!state.info)return;$('status').textContent=`TIDE-X ${state.info.lab_home} · Python ${state.info.hf_python||(currentLanguage==='es'?'no encontrado':'not found')} · ${currentLanguage==='es'?'resuelve':'resolves'} ${state.info.resolved_hf_python||(currentLanguage==='es'?'no resuelto':'unresolved')} · NNsight ${state.info.nnsight_available?'OK':'NO'} · SAE ${state.info.sae_lens_available?'OK':'NO'}`}
function applyLanguage(){const lang=translations[currentLanguage];document.documentElement.lang=currentLanguage;document.title=lang.labTitle;document.querySelector('[data-i18n="labTitle"]').textContent=lang.labTitle;document.querySelector('[data-i18n="labSubtitle"]').textContent=lang.labSubtitle;document.querySelectorAll('[data-i18n-key]').forEach(el=>{const value=lang.ui[el.dataset.i18nKey];if(value)el.textContent=value});document.querySelectorAll('[data-i18n-placeholder]').forEach(el=>{const value=lang.ui[el.dataset.i18nPlaceholder];if(value)el.placeholder=value});$('langToggle').textContent=lang.langButton;renderStatus();renderModels();renderDatasets();renderExecutors();renderPlasticity();renderAreas();if(state.goalPipeline.length)renderGoalPipeline();renderParams();loadRecipes();loadJobs()}
function toggleLanguage(){currentLanguage=currentLanguage==='es'?'en':'es';localStorage.setItem('tidex.lab.language',currentLanguage);applyLanguage()}
const executorOperationMap={
  'cross_model.probe_runtime':'probe_runtime','cross_model.evaluate':'behavioral_evaluation','cross_model.discovery':'behavioral_discovery_multi','cross_model.extract_steering':'extract_capability','cross_model.nnsight':'deep_instrumentation','cross_model.sae':'sparse_autoencoder_analysis','cross_model.counterfactual':'counterfactual_analysis','cross_model.align':'calibrate_alignment','cross_model.transfer_steering':'activation_transfer_experiment'
};
const labAreas=[
  {id:'research',title:'Investigación',description:'Benchmarks, discovery, representaciones, NNsight, SAE, tomografía, topología y mapas protegidos.',match:e=>e.authority==='evidence_producer' || e.executor_id.startsWith('tomography.') || e.executor_id.startsWith('protected.') || e.executor_id.startsWith('pythagoras.')},
  {id:'plasticity',title:'Plasticidad',description:'Aprendizaje adaptativo, señales plásticas, ELO y routing ligados a evidencia.',match:e=>e.executor_id.startsWith('plasticity.') || e.executor_id.startsWith('learning.') || e.executor_id==='numerical.evolve' || e.executor_id==='procedural.memory'},
  {id:'transfer',title:'Transferencia',description:'Descubrimiento, extracción, alineamiento, compilación e intervención entre modelos.',match:e=>e.executor_id.startsWith('cross_model.') || e.executor_id.startsWith('receiver.')},
  {id:'evaluation',title:'Evaluación',description:'Shadow, selección, universalidad, preservación y readiness.',match:e=>e.executor_id.startsWith('shadow.') || e.executor_id.startsWith('universality.') || e.executor_id.startsWith('promotion.') || e.executor_id==='materialize.selector'},
  {id:'production',title:'Producción',description:'Materialización física, AdapterBank, autorización, activación, revocación y rollback.',match:e=>e.executor_id.startsWith('adapter.') || e.executor_id==='adapter.bank' || e.executor_id.startsWith('materialize.') || e.effect_class==='physical_materialization' || e.effect_class==='production_lifecycle'},
  {id:'learning',title:'Aprendizaje',description:'KnowledgeEngine, NumericalEvolution, memoria procedural, consolidación y ciclo adaptativo.',match:e=>e.executor_id.startsWith('knowledge.') || e.executor_id.startsWith('learning.') || e.executor_id.startsWith('brain.sleep') || e.executor_id==='numerical.evolve' || e.executor_id==='procedural.memory'},
  {id:'governance',title:'Gobernanza',description:'Gates, autoridad, evidencia, autorización y ciclo de vida productivo.',match:e=>e.authority==='readiness_gate' || e.authority==='lifecycle_authority' || e.effect_class==='governance_readiness' || e.executor_id==='adapter.authorize'}
];
function executorStateClass(e){const m=e.maturity||e.state;if(m==='production_lifecycle')return'op';if(m==='operational')return'op';if(m==='operational_candidate')return'exp';if(m==='operational_advisory')return'need';if(m==='needs_workflow')return'need';return'arch'}
function executorStateLabel(e){return {production_lifecycle:'ciclo de vida producción',operational:'operativo',operational_candidate:'candidato operativo',operational_advisory:'advisory implementado',needs_workflow:'implementado · workflow pendiente'}[e.maturity]||e.maturity||e.state}
function runtimeLabel(e){return {runnable_now:'ejecutable ahora',runnable_with_typed_input:'ejecutable con input tipado',advisory_from_evidence:'advisory con evidencia',candidate_only:'candidato ejecutable',requires_workflow:'requiere workflow',requires_backend_or_artifact:'requiere backend/artefacto',blocked_by_policy:'bloqueado por política'}[e.runtime_status]||e.runtime_status||'desconocido'}
function workflowLabel(e){return {lab_ready:'Lab',cli_ready:'CLI',engine_ready:'Engine',internal_ready:'Interno',needs_operator_workflow:'workflow pendiente'}[e.workflow_status]||e.workflow_status||'—'}
function executorCanRun(e){return Boolean(executorOperationMap[e.executor_id]||e.lab_recipe_id)}
function executorActionable(e){return e.actionable_now===true}
function executorButtonLabel(e){if(executorCanRun(e))return 'Abrir operación';if(e.workflow_status==='internal_ready')return 'Ver contrato interno';if(e.runtime_status==='advisory_from_evidence')return 'Ver señal advisory';return 'Ver contrato'}
function setLabMode(mode){state.labMode=mode;$('systemMode').classList.toggle('hidden',mode!=='system');$('manualMode').classList.toggle('hidden',mode!=='manual');$('systemModeBtn').classList.toggle('active',mode==='system');$('manualModeBtn').classList.toggle('active',mode==='manual');if(mode==='manual')renderAreas()}
function renderAreas(){const box=$('areaGrid');if(!box)return;box.innerHTML=labAreas.map(area=>{const xs=state.executors.filter(area.match),registered=xs.filter(executorActionable).length,runnable=xs.filter(executorCanRun).length,prod=xs.filter(e=>e.production_authority).length;return `<div class="area-card" onclick="openArea('${area.id}')"><b>${esc(area.title)}</b><div class="muted">${esc(area.description)}</div><div style="margin-top:8px"><span class="chip op">${registered} operativos</span><span class="chip">${runnable} ejecución directa</span><span class="chip">${registered} acciones UI</span><span class="chip">${xs.length} contratos</span>${prod?`<span class="chip op">${prod} autoridad prod</span>`:''}</div></div>`}).join('')}
function executorDetail(e){return `<div class="executor-row"><span class="chip ${executorStateClass(e)}">${esc(executorStateLabel(e))}</span><span class="chip">${esc(runtimeLabel(e))}</span><span class="chip">${esc(workflowLabel(e))}</span><span class="chip">${esc(e.evidence_status||'—')}</span>${e.production_authority?'<span class="chip op">autoridad producción</span>':'<span class="chip">sin autoridad prod</span>'}<b>${esc(e.title)}</b><div class="muted">${esc(e.executor_id)} · ${esc(e.module_path)}</div><div class="muted">requiere: ${esc((e.requires||[]).join(', ')||'—')} · produce: ${esc((e.produces||[]).join(', ')||'—')}</div><div class="why">${esc(e.notes||'')}</div>${executorActionable(e)?`<button style="width:auto" onclick="activateExecutor('${e.executor_id}')">${esc(executorButtonLabel(e))}</button>`:''}<details><summary>contrato</summary><pre>${esc(JSON.stringify(e,null,2))}</pre></details></div>`}
function openArea(id){const area=labAreas.find(x=>x.id===id);if(!area)return;const xs=state.executors.filter(area.match);$('areaDetail').innerHTML=`<div class="result-card"><b>${esc(area.title)}</b><div class="muted">${esc(area.description)}</div>${xs.map(executorDetail).join('')}</div>`}
function activateExecutor(id){const e=state.executors.find(x=>x.executor_id===id);if(!e)return;setLabMode('manual');const op=executorOperationMap[id];if(op){$('operation').value=op;renderParams();$('operation').scrollIntoView({behavior:'smooth',block:'center'});return}if(e.lab_recipe_id){showTab('advanced');const card=document.querySelector(`[data-recipe-id="${CSS.escape(e.lab_recipe_id)}"]`);if(card)card.scrollIntoView({behavior:'smooth',block:'center'});return}const area=labAreas.find(a=>a.match(e));openArea(area?.id||'research')}
const pipelineSpecs={
 investigate:['cross_model.probe_runtime','cross_model.evaluate','cross_model.nnsight','cross_model.sae','cross_model.counterfactual','tomography.skill_fields','protected.map','pythagoras.geometry'],
 improve:['cross_model.evaluate','cross_model.discovery','learning.active_aperture','numerical.evolve','cross_model.extract_steering','cross_model.transfer_steering','shadow.evaluate','promotion.readiness'],
 transfer:['cross_model.evaluate','cross_model.discovery','cross_model.extract_steering','cross_model.align','cross_model.transfer_steering','receiver.freeze_compiler','receiver.compile_universal','materialize.compiled','shadow.evaluate','universality.reduce','promotion.readiness'],
 validate:['cross_model.evaluate','materialize.selector','shadow.evaluate','universality.reduce','promotion.readiness'],
 production:['materialize.compiled','adapter.import','adapter.compose','adapter.materialize','adapter.authorize','adapter.activate','adapter.revoke','adapter.rollback','adapter.status','adapter.bank','shadow.evaluate','universality.reduce','promotion.readiness']
};
function buildGoalPipeline(){const objective=$('goalText').value.trim(),kind=$('goalKind').value;const ids=pipelineSpecs[kind]||[];state.goalPipeline=ids.map(id=>state.executors.find(e=>e.executor_id===id)).filter(Boolean);$('goalSummary').textContent=objective?`Objetivo: ${objective}`:'Pipeline basado en el tipo seleccionado; escribe el objetivo para dejar explícita la intención del experimento.';renderGoalPipeline()}
function renderGoalPipeline(){const box=$('goalPipeline');if(!box)return;box.innerHTML=state.goalPipeline.map((e,i)=>{const runnable=executorCanRun(e),actionable=executorActionable(e),cls=runnable?'available':actionable?'manual':'blocked';const mode=runnable?'ejecutable':actionable?runtimeLabel(e):'bloqueado';return `<div class="pipeline-step ${cls}"><div class="head"><div><span class="chip">${i+1}</span><b>${esc(e.title)}</b></div><span class="chip ${executorStateClass(e)}">${esc(mode)}</span></div><div class="muted">${esc(e.executor_id)} · ${esc(executorStateLabel(e))} · ${esc(e.effect_class)} · autoridad ${esc(e.authority)}</div><div class="why">${esc(e.notes||'')}</div>${actionable?`<button style="width:auto" onclick="activateExecutor('${e.executor_id}')">${esc(executorButtonLabel(e))}</button>`:''}</div>`}).join('')||'<div class="muted">No hay ejecutores registrados para este pipeline.</div>'}
const metric=(k,v,cls='')=>`<div class="metric ${cls}"><div class="k">${esc(k)}</div><div class="v">${esc(v)}</div></div>`;
function raw(v){$('output').textContent=typeof v==='string'?v:JSON.stringify(v,null,2)}
function parseRunStdout(job){if(!job?.run?.stdout)return null;try{return JSON.parse(job.run.stdout)}catch{return null}}
function renderEvaluation(x){const obs=x.observations||[];return `<div class="metric-grid">${metric('modelo',x.model||'-')}${metric('score',pct(x.weighted_score),x.weighted_score>0?'good':'')}${metric('probes',obs.length)}${metric('evidence',(x.evidence_sha256||'').slice(0,12)+'…')}</div><div class="section-title">Probes</div>${obs.map(o=>`<div class="obs"><b>${esc(o.probe_id)}</b> · score ${pct(o.score)} · tokens ${esc(o.eval_count)}<div class="muted">${esc(o.response_text||'')}</div></div>`).join('')}`}
function renderDiscovery(x){const ev=x.evaluations||[],g=x.gaps||[],p=x.proposals||[];return `<div class="metric-grid">${metric('modelos',ev.length)}${metric('gaps',g.length,g.length?'good':'')}${metric('propuestas',p.length,p.length?'good':'')}${metric('benchmark',x.benchmark_id||'-')}</div><div class="section-title">Modelos</div>${ev.map(e=>`<div class="result-card"><b>${esc(e.model)}</b> · ${pct(e.weighted_score)}<div class="scorebar"><i style="width:${Math.max(0,Math.min(100,Number(e.weighted_score||0)*100))}%"></i></div></div>`).join('')}<div class="section-title">Gaps</div>${g.length?g.map(v=>`<div class="result-card"><pre>${esc(JSON.stringify(v,null,2))}</pre></div>`).join(''):'<div class="muted">No se detectaron gaps con este benchmark.</div>'}<div class="section-title">Propuestas</div>${p.length?p.map(v=>`<div class="result-card"><pre>${esc(JSON.stringify(v,null,2))}</pre></div>`).join(''):'<div class="muted">No se generaron propuestas.</div>'}`}
function renderTransfer(x){const b=x.baseline?.weighted_score||0,i=x.intervened?.weighted_score||0,r=x.restored?.weighted_score||0;return `<div class="metric-grid">${metric('baseline',pct(b))}${metric('intervenido',pct(i),i>b?'good':i<b?'bad':'')}${metric('restaurado',pct(r))}${metric('delta',Number(x.score_delta||0).toFixed(4),x.score_delta>0?'good':x.score_delta<0?'bad':'')}${metric('restore Δ',Number(x.restore_delta||0).toFixed(4),Math.abs(x.restore_delta||0)<1e-9?'good':'warn')}${metric('mejora observada',x.behavioral_improvement_observed?'SÍ':'NO',x.behavioral_improvement_observed?'good':'')}</div><div class="result-card"><b>${esc(x.source_model)} → ${esc(x.target_model)}</b><div class="muted">capacidad: ${esc(x.capability_name)}</div></div><div class="section-title">Alineamiento</div><div class="metric-grid">${metric('alignment score',pct(x.alignment?.alignment_score))}${metric('residual',Number(x.alignment?.normalized_residual||0).toFixed(6))}${metric('strength',x.intervention?.strength??'-')}</div>`}
function renderRuntimeProbe(x){const a=x.access||{},m=x.model||{};return `<div class="metric-grid">${metric('arquitectura',m.runtime_architecture||'-')}${metric('parámetros',Number(m.parameter_count||0).toLocaleString())}${metric('capas',m.num_layers??'-')}${metric('contexto',m.max_sequence_length??'-')}</div><div class="section-title">Accesos comprobados</div><div class="metric-grid">${Object.entries(a).map(([k,v])=>metric(k,v?'SÍ':'NO',v?'good':'bad')).join('')}</div><div class="result-card"><b>${esc(m.name||'-')}</b><div class="muted">runtime metadata: ${esc(m.runtime_metadata_sha256||'')}</div></div>`}
function renderGeneratedBenchmark(x){state.generatedBenchmark=x.benchmark||null;const probes=x.benchmark?.probes||[];return `<div class="metric-grid">${metric('generator',x.generator_model||'-')}${metric('probes',probes.length)}${metric('dominio',x.benchmark?.domain||'-')}${metric('independiente',x.independent_evidence?'SÍ':'NO',x.independent_evidence?'good':'warn')}</div><div class="result-card"><b>${esc(x.benchmark?.benchmark_id||'benchmark generado')}</b><div class="muted">${esc(x.generation_response_sha256||'')}</div></div><div class="section-title">Probes generados</div>${probes.slice(0,50).map(p=>`<div class="obs"><b>${esc(p.probe_id)}</b><div>${esc(p.prompt)}</div><div class="muted">verifier: ${esc(JSON.stringify(p.verifier))}</div></div>`).join('')}<button class="primary" onclick="saveGeneratedBenchmark()">Guardar benchmark generado</button>`}
async function saveGeneratedBenchmark(){try{if(!state.generatedBenchmark)throw Error('No hay benchmark generado');const name=state.generatedBenchmark.benchmark_id||'generated-benchmark';const v=await jpost('/api/datasets/import',{name,format:'json',content:JSON.stringify(state.generatedBenchmark),generated:true});state.selectedDataset=v.content_sha256;await reload();const radio=document.querySelector(`input[name=ds][value="${v.content_sha256}"]`);if(radio)radio.checked=true;out(v)}catch(e){out(e.message)}}
function renderGeneric(x){if(Array.isArray(x))return `<div class="metric-grid">${metric('resultados',x.length)}</div>${x.slice(0,50).map(v=>`<div class="result-card"><pre>${esc(JSON.stringify(v,null,2))}</pre></div>`).join('')}`;if(x?.steering_vector)return `<div class="metric-grid">${metric('capacidad',x.steering_vector?.metadata?.name||'-')}${metric('confianza',pct(x.steering_vector?.metadata?.confidence))}${metric('capa',x.steering_vector?.metadata?.source_layer??'-')}${metric('componentes',(x.components||[]).length)}</div>`;if(x?.calibration_sha256)return `<div class="metric-grid">${metric('source',x.source_model||'-')}${metric('target',x.target_model||'-')}${metric('source layer',x.source_layer)}${metric('target layer',x.target_layer)}${metric('calibration',String(x.calibration_sha256).slice(0,12)+'…')}</div>`;return `<div class="result-card"><pre>${esc(JSON.stringify(x,null,2))}</pre></div>`}
function renderJob(job){raw(job);const box=$('summary');const stateLabel=currentLanguage==='es'?'estado':'state',operationLabel=currentLanguage==='es'?'operación':'operation';if(job.state==='queued'||job.state==='running'){box.innerHTML=`<div class="metric-grid">${metric(stateLabel,trState(job.state),'running')}${metric(operationLabel,trOperation(job.operation))}${metric('job',job.job_id.slice(0,12)+'…')}</div><div class="muted">${currentLanguage==='es'?'El trabajo sigue ejecutándose. El resultado persistirá aunque cierres esta pestaña.':'The job is still running. Its result will persist even if you close this tab.'}</div>`;return}if(job.state==='failed'){box.innerHTML=`<div class="metric-grid">${metric(stateLabel,trState(job.state),'bad')}${metric(operationLabel,trOperation(job.operation))}</div><div class="result-card bad">${esc(job.error||(currentLanguage==='es'?'error desconocido':'unknown error'))}</div>`;return}const x=parseRunStdout(job);if(!x){box.innerHTML=`<div class="metric-grid">${metric(stateLabel,trState(job.state),'good')}${metric(operationLabel,trOperation(job.operation))}</div><div class="muted">${currentLanguage==='es'?'La ejecución terminó, pero la salida no es JSON estructurado.':'Execution finished, but the output is not structured JSON.'}</div>`;return}if(x.schema==='cerebro.cross_model.discovery_cycle/v1')box.innerHTML=renderDiscovery(x);else if(x.schema==='cerebro.cross_model.model_evaluation/v1')box.innerHTML=renderEvaluation(x);else if(x.schema==='cerebro.tidex.lab_activation_transfer/v1')box.innerHTML=renderTransfer(x);else if(x.schema==='cerebro.tidex.lab_runtime_probe/v1')box.innerHTML=renderRuntimeProbe(x);else if(x.schema==='cerebro.tidex.lab_generated_benchmark/v1')box.innerHTML=renderGeneratedBenchmark(x);else box.innerHTML=renderGeneric(x)}
const out=v=>{raw(v);if(v&&typeof v==='object'&&v.state)renderJob(v);else $('summary').innerHTML=`<div class="result-card"><pre>${esc(typeof v==='string'?v:JSON.stringify(v,null,2))}</pre></div>`};
async function jget(url){const r=await fetch(url);const t=await r.text();if(!r.ok)throw Error(t);return JSON.parse(t)}
async function jpost(url,body){const r=await fetch(url,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)});const t=await r.text();let v;try{v=JSON.parse(t)}catch{v=t}if(!r.ok)throw Error(typeof v==='string'?v:JSON.stringify(v));return v}
function selectedModels(){return state.selectedModelOrder.slice()}
function toggleModel(id,checked){if(checked){if(!state.selectedModelOrder.includes(id))state.selectedModelOrder.push(id)}else{state.selectedModelOrder=state.selectedModelOrder.filter(x=>x!==id)}updateModelSelection()}
function updateModelSelection(){const ids=selectedModels();const labels=ids.map((id,i)=>{const m=state.models.find(x=>x.model_id===id);return `${i===0?'SOURCE / #1':i===1?'TARGET / #2':'#'+(i+1)}: ${m?.architecture||id.slice(0,12)}`});$('modelSelection').textContent=labels.length?labels.join(' · '):'Sin modelos seleccionados.'}
function lines(id){return $(id).value.split('\n').map(s=>s.trim()).filter(Boolean)}
function num(id,d){const v=Number($(id)?.value);return Number.isFinite(v)?v:d}
function profileFor(id){return state.profiles.find(p=>p.model_id===id)||null}
function runtimeStatusFor(id){return state.runtimeStatuses.find(p=>p.model_id===id)||null}
function accessBadge(label,ok){return `<span class="badge ${ok?'ok':'warn'}">${label} ${ok?'✓':'×'}</span>`}
function renderModels(){const box=$('models');box.innerHTML=state.models.map(m=>{const p=profileFor(m.model_id);const st=runtimeStatusFor(m.model_id);let caps;if(p){caps=`<br>${accessBadge('INF',p.access.behavioral_inference)} ${accessBadge('ACT',p.access.internal_activations)} ${accessBadge('STEER',p.access.activation_intervention)} ${accessBadge('NN',p.access.deep_instrumentation)} ${accessBadge('SAE',p.access.sparse_autoencoder_analysis)}`}else if(st&&st.state!=='missing'){caps=`<br><span class="badge ${st.state==='failed'||st.state==='cancelled'?'warn':'ok'}">runtime ${esc(st.state)}</span>${st.error?` <span class="badge warn" title="${esc(st.error)}">error</span>`:''}`}else{caps='<br><span class="badge warn">runtime sin comprobar</span>'}return `<label class="model"><input class="modelSel" type="checkbox" value="${m.model_id}" ${state.selectedModelOrder.includes(m.model_id)?'checked':''} onchange="toggleModel('${m.model_id}',this.checked)"><span><b>${m.architecture||'arquitectura desconocida'}</b> <span class="badge ${m.layout==='hugging_face_single_safetensors'?'ok':'warn'}">${m.layout}</span>${caps}<small>${m.root}</small></span></label>`}).join('')||'<p class="muted">No hay modelos catalogados.</p>';state.selectedModelOrder=state.selectedModelOrder.filter(id=>state.models.some(m=>m.model_id===id));updateModelSelection()}
function renderDatasets(){const box=$('datasets');box.innerHTML=state.datasets.map(d=>`<label class="model"><input type="radio" name="ds" value="${d.content_sha256}" onchange="state.selectedDataset=this.value"><span><b>${d.name}</b> <span class="badge ${d.independent_evidence?'ok':'warn'}">${d.independent_evidence?'independiente':'generado'}</span><small>${d.format} · ${d.bytes} bytes</small></span></label>`).join('')||'<p class="muted">Sin datasets.</p>'}
async function reload(){state.models=await jget('/api/models');state.profiles=await jget('/api/model-profiles');state.runtimeStatuses=await jget('/api/model-runtime-statuses');state.datasets=await jget('/api/datasets');state.executors=await jget('/api/executors');state.plasticity=await jget('/api/plasticity');renderModels();renderDatasets();renderExecutors();renderPlasticity();renderAreas();if(state.goalPipeline.length)buildGoalPipeline()}
async function scanModels(){try{state.models=await jpost('/api/models/scan',{root:$('modelRoot').value});renderModels();out({modelos_encontrados:state.models.length})}catch(e){out(e.message)}}
async function importFileDataset(){try{const f=$('datasetFile').files[0];if(!f)throw Error('Selecciona un archivo');const content=await f.text();const name=$('dsName').value.trim()||f.name.replace(/[^A-Za-z0-9_.-]/g,'_');const ext=f.name.split('.').pop().toLowerCase();const format=['json','jsonl','csv'].includes(ext)?ext:'text';const v=await jpost('/api/datasets/import',{name,format,content,generated:$('dsGenerated').value==='true'});state.selectedDataset=v.content_sha256;await reload();const radio=document.querySelector(`input[name=ds][value="${v.content_sha256}"]`);if(radio)radio.checked=true;out(v)}catch(e){out(e.message)}}
async function createBenchmark(){try{const id=$('benchId').value.trim();const domain=$('benchDomain').value.trim();const rows=$('benchRows').value.split('\n').map(x=>x.trim()).filter(Boolean);if(!id||!domain||rows.length<2)throw Error('Indica benchmark id, dominio y al menos 2 pruebas');const probes=rows.map((row,i)=>{const parts=row.split('=>');if(parts.length<2)throw Error(`Línea ${i+1}: usa prompt => respuesta`);const prompt=parts.shift().trim(),expected=parts.join('=>').trim();if(!prompt||!expected)throw Error(`Línea ${i+1}: prompt/respuesta vacíos`);return{probe_id:`p${i+1}`,prompt,verifier:{kind:'exact_text',expected,trim:true,case_sensitive:true},weight:1.0}});const content=JSON.stringify({schema:'cerebro.cross_model.behavioral_benchmark/v1',benchmark_id:id,domain,probes,minimum_mean_gap:0.0,significance_alpha:0.05});const v=await jpost('/api/datasets/import',{name:id,format:'json',content,generated:false});state.selectedDataset=v.content_sha256;await reload();const radio=document.querySelector(`input[name=ds][value="${v.content_sha256}"]`);if(radio)radio.checked=true;out(v)}catch(e){out(e.message)}}
function input(id,label,value=''){return `<label class="muted">${label}<input id="${id}" value="${value}"></label>`}
function area(id,label,ph=''){return `<label class="muted">${label}<textarea id="${id}" placeholder="${ph}"></textarea></label>`}
function renderParams(){const op=$('operation').value;let h='<p class="muted">';
if(op==='behavioral_discovery_multi')h+='Compara 2–64 modelos con un benchmark y descubre gaps/propuestas reales.</p>';
if(op==='probe_runtime')h+='Abre realmente el checkpoint con el backend HF y devuelve qué accesos soporta el runtime actual.</p>';
if(op==='behavioral_evaluation')h+='Ejecuta un benchmark real sobre un único LLM.</p>';
if(op==='generate_behavioral_dataset')h+='El LLM propone un benchmark verificable. Rust exige JSON exacto y lo valida; siempre queda marcado como generado/no independiente.</p>'+input('genBenchId','benchmark id','generated.benchmark')+input('genDomain','dominio','general')+input('genProbeCount','número de probes','8')+area('genObjective','objetivo del benchmark','Describe con precisión qué capacidad quieres medir y qué tipo de casos debe cubrir.');
if(op==='extract_capability')h+='Necesita un modelo HF con activaciones internas.</p>'+input('capName','capability name','capability.test')+input('domain','dominio','general')+input('layer','capa','0')+area('positive','ejemplos positivos, uno por línea')+area('negative','ejemplos negativos, uno por línea');
if(op==='deep_instrumentation')h+='Requiere NNsight disponible en el Python seleccionado por TIDEX_HF_PYTHON.</p>'+input('modulePath','module path','model.layers.0')+input('tokenFromEnd','token desde el final','0')+area('prompt','prompt');
if(op==='sparse_autoencoder_analysis')h+='Requiere NNsight + SAE Lens.</p>'+input('modulePath','module path','model.layers.0')+input('tokenFromEnd','token desde el final','0')+input('release','SAE release')+input('saeId','SAE id')+input('topK','top K','32')+area('prompt','prompt');
if(op==='counterfactual_analysis')h+='Ejecuta original/perturbado y mide efecto; pega escenarios JSON.</p>'+area('scenarios','escenarios JSON','[{"scenario_id":"x","original_input":"...","perturbed_input":"...","perturbation_type":"caller_defined","verifier":{"kind":"exact_text","expected":"...","trim":true,"case_sensitive":true}}]')+input('activationLayers','capas de activación separadas por coma','');
if(op==='calibrate_alignment')h+='Primer modelo seleccionado = source; segundo = target.</p>'+input('sourceLayer','source layer','0')+input('targetLayer','target layer','0')+area('trainingPrompts','prompts de calibración, uno por línea')+area('validationPrompts','prompts held-out, uno por línea');
if(op==='activation_transfer_experiment')h+='Pipeline real: baseline B → extracción A → alineamiento → steering B → evaluación → clear → restauración.</p>'+input('capName','capability name','capability.test')+input('domain','dominio','general')+input('sourceLayer','source layer','0')+input('targetLayer','target layer','0')+input('strength','strength','1.0')+area('positive','ejemplos positivos, uno por línea')+area('negative','ejemplos negativos, uno por línea')+area('trainingPrompts','prompts calibración, uno por línea')+area('validationPrompts','prompts validación, uno por línea');
$('params').innerHTML=h}
function parameters(op){if(op==='generate_behavioral_dataset')return{benchmark_id:$('genBenchId').value,domain:$('genDomain').value,objective:$('genObjective').value,probe_count:num('genProbeCount',8)};if(op==='extract_capability')return{capability_name:$('capName').value,domain:$('domain').value,level:{kind:'single_layer',layer:num('layer',0)},positive_examples:lines('positive'),negative_examples:lines('negative')};if(op==='deep_instrumentation')return{request:{module_path:$('modulePath').value,prompt:$('prompt').value,token_from_end:num('tokenFromEnd',0)}};if(op==='sparse_autoencoder_analysis')return{request:{module_path:$('modulePath').value,prompt:$('prompt').value,token_from_end:num('tokenFromEnd',0),release:$('release').value,sae_id:$('saeId').value,top_k:num('topK',32)}};if(op==='counterfactual_analysis')return{scenarios:JSON.parse($('scenarios').value),activation_layers:$('activationLayers').value.split(',').map(x=>x.trim()).filter(Boolean).map(Number)};if(op==='calibrate_alignment')return{source_layer:num('sourceLayer',0),target_layer:num('targetLayer',0),training_prompts:lines('trainingPrompts'),validation_prompts:lines('validationPrompts')};if(op==='activation_transfer_experiment')return{capability_name:$('capName').value,domain:$('domain').value,extraction_level:{kind:'single_layer',layer:num('sourceLayer',0)},positive_examples:lines('positive'),negative_examples:lines('negative'),source_layer:num('sourceLayer',0),target_layer:num('targetLayer',0),calibration_prompts:lines('trainingPrompts'),validation_prompts:lines('validationPrompts'),strength:num('strength',1)};return{}}
function ensureCompatible(op,models){const needs={behavioral_evaluation:['behavioral_inference'],behavioral_discovery_multi:['behavioral_inference'],extract_capability:['internal_activations'],deep_instrumentation:['deep_instrumentation'],sparse_autoencoder_analysis:['sparse_autoencoder_analysis'],counterfactual_analysis:['behavioral_inference'],calibrate_alignment:['internal_activations'],activation_transfer_experiment:['behavioral_inference','internal_activations','activation_intervention'],generate_behavioral_dataset:['behavioral_inference']};if(op==='probe_runtime')return;for(const id of models){const p=profileFor(id);if(!p)throw Error('Primero ejecuta Comprobar runtime para '+id.slice(0,12));for(const k of (needs[op]||[])){if(!p.access[k])throw Error(`Modelo ${id.slice(0,12)} no soporta ${k}`)}}}
async function cancelActiveJob(){try{if(!state.activeJob)throw Error('No hay job activo');const v=await jpost('/api/jobs/'+state.activeJob+'/cancel',{});renderJob(v);await loadJobs()}catch(e){out(e.message)}}
async function pollJob(job){state.activeJob=job.job_id;$('cancelBtn').disabled=false;renderJob(job);for(;;){await new Promise(r=>setTimeout(r,1000));const v=await jget('/api/jobs/'+job.job_id);renderJob(v);if(v.state==='completed'||v.state==='failed'||v.state==='cancelled'){if(v.operation==='probe_runtime'&&v.state==='completed')await reload();await loadJobs();state.activeJob=null;$('cancelBtn').disabled=true;return v}}}
async function runWorkflow(){try{$('runBtn').disabled=true;$('runBtn').textContent='Ejecutando…';const op=$('operation').value,models=selectedModels();ensureCompatible(op,models);let job;if(op==='behavioral_discovery_multi'){if(models.length<2)throw Error('Selecciona al menos 2 modelos');if(!state.selectedDataset)throw Error('Selecciona un benchmark');job=await jpost('/api/workflows/behavioral-discovery',{schema:'cerebro.tidex.lab_behavioral_discovery/v1',model_ids:models,dataset_sha256:state.selectedDataset,max_new_tokens:128,seed:0})}else{job=await jpost('/api/workflows/direct',{schema:'cerebro.tidex.lab_direct_workflow/v1',operation:op,model_ids:models,dataset_sha256:state.selectedDataset,parameters:parameters(op)})}await pollJob(job)}catch(e){out(e.message)}finally{$('runBtn').disabled=false;$('runBtn').textContent='Ejecutar trabajo'}}
function renderPlasticity(){const p=state.plasticity;if(!p){$('plasticity').innerHTML='<p class="muted">Sin señales plásticas calculadas.</p>';return}const jobs=(state.jobs||[]);const related=jobs.filter(j=>['behavioral_discovery','behavioral_evaluation','extract_capability','calibrate_alignment','activation_transfer_experiment'].includes(j.operation));const transfers=related.map(j=>({job:j,data:parseRunStdout(j)})).filter(x=>x.data?.schema==='cerebro.tidex.lab_activation_transfer/v1');const last=transfers[0]?.data;const leaders=((p.elo_entities||[]).length?p.elo_entities:(p.elo_leaderboard||[]).map(([entity,rating])=>({entity,rating,comparisons:0,last_evidence_sha256:null,last_update:null}))).map(x=>`<div class="result-card"><b>${esc(x.entity)}</b><div class="scorebar"><i style="width:${Math.max(0,Math.min(100,(Number(x.rating)-100)/29))}%"></i></div><div class="muted">rating ${Number(x.rating).toFixed(2)} · comparaciones ${Number(x.comparisons||0)}${x.last_evidence_sha256?' · evidencia '+esc(String(x.last_evidence_sha256).slice(0,12))+'…':''}</div></div>`).join('')||'<p class="muted">Sin pares suficientes para ELO.</p>';const routes=(p.routing_decisions||[]).map(r=>`<div class="result-card"><b>${esc(String(r.capability||'').startsWith('benchmark:')?'benchmark '+String(r.capability).slice(10):String(r.capability||''))} → ${esc(r.target_model)}</b><div class="muted">routing ${Number(r.routing_score||0).toFixed(4)} · medido ${Number(r.measured_score||0).toFixed(4)} · incertidumbre ${Number(r.uncertainty_bonus||0).toFixed(4)} · evidencia ${esc(String(r.evidence_sha256||'').slice(0,12))}…</div></div>`).join('')||'<p class="muted">Sin observaciones comparativas suficientes para routing.</p>';const experiments=related.map(j=>`<div class="history-item" onclick="openJob('${j.job_id}')"><span class="state ${j.state==='completed'?'good':(j.state==='failed'||j.state==='cancelled')?'bad':'running'}">${esc(trState(j.state))}</span><b>${esc(trOperation(j.operation))}</b><div class="muted">${esc(j.job_id.slice(0,16))}…${j.evidence_receipt?.evidence_sha256?' · evidence '+esc(j.evidence_receipt.evidence_sha256.slice(0,12))+'…':''}</div></div>`).join('')||'<p class="muted">Sin experimentos plásticos registrados.</p>';const learning=state.executors.filter(e=>e.executor_id.startsWith('plasticity.')||e.executor_id.startsWith('learning.')||e.executor_id==='numerical.evolve'||e.executor_id==='procedural.memory').map(e=>`<div class="executor-row"><span class="chip ${executorStateClass(e)}">${esc(executorStateLabel(e))}</span><b>${esc(e.title)}</b><div class="muted">${esc(e.executor_id)} · ${esc(e.notes||'')}</div>${executorCanRun(e)?`<button style="width:auto" onclick="activateExecutor('${e.executor_id}')">Abrir operación</button>`:`<details><summary>Contrato interno</summary><pre>${esc(JSON.stringify(e,null,2))}</pre></details>`}</div>`).join('');$('plasticity').innerHTML=`<div class="plasticity-tabs"><button onclick="switchPlasticityPane('state')">Estado</button><button onclick="switchPlasticityPane('experiments')">Experimentos</button><button onclick="switchPlasticityPane('elo')">Entidades / ELO</button><button onclick="switchPlasticityPane('routes')">Rutas</button><button onclick="switchPlasticityPane('learning')">Aprendizaje</button></div><div id="plasticity-state" class="plasticity-pane"><div class="metric-grid">${metric('disponible',p.available?'SÍ':'NO',p.available?'good':'bad')}${metric('jobs fuente',p.source_jobs||0)}${metric('ELO entities',(p.elo_leaderboard||[]).length)}${metric('rutas',(p.routing_decisions||[]).length)}${last?metric('último baseline',pct(last.baseline?.weighted_score)):''}${last?metric('último candidato',pct(last.intervened?.weighted_score),last.score_delta>0?'good':last.score_delta<0?'bad':''):''}${last?metric('delta',Number(last.score_delta||0).toFixed(4),last.score_delta>0?'good':last.score_delta<0?'bad':''):''}</div><div class="section-title">Notas de evidencia</div>${(p.notes||[]).map(n=>`<div class="muted">${esc(n)}</div>`).join('')||'<div class="muted">Sin notas.</div>'}</div><div id="plasticity-experiments" class="plasticity-pane hidden">${experiments}</div><div id="plasticity-elo" class="plasticity-pane hidden">${leaders}</div><div id="plasticity-routes" class="plasticity-pane hidden">${routes}</div><div id="plasticity-learning" class="plasticity-pane hidden">${learning||'<p class="muted">No hay ejecutores de aprendizaje registrados.</p>'}</div>`}
function switchPlasticityPane(name){document.querySelectorAll('.plasticity-pane').forEach(x=>x.classList.add('hidden'));const pane=$('plasticity-'+name);if(pane)pane.classList.remove('hidden')}
function renderExecutors(){const total=state.executors.length,operational=state.executors.filter(e=>e.state==='operational').length,implemented=state.executors.filter(e=>e.implementation_status==='implemented').length,actionable=state.executors.filter(executorActionable).length,runnable=state.executors.filter(executorCanRun).length,prod=state.executors.filter(e=>e.production_authority).length;const grouped={};for(const e of state.executors){(grouped[e.maturity||'unknown'] ||= []).push(e)}const order=['production_lifecycle','operational','operational_candidate','operational_advisory','needs_workflow','experimental','unknown'];$('executors').innerHTML=`<div class="metric-grid">${metric('registrados',total)}${metric('operativos contrato',operational,'good')}${metric('implementados',implemented,'good')}${metric('accionables',actionable,'good')}${metric('ejecutables UI/CLI',runnable)}${metric('autoridad producción',prod,prod===1?'good':'warn')}</div><div class="muted">Operativo aquí significa contrato implementado y rastreable. Madurez, evidencia y autoridad se muestran por separado para no mezclar candidatos/advisory con producción.</div>`+order.filter(k=>grouped[k]?.length).map(k=>`<div class="section-title">${esc(({production_lifecycle:'ciclo de vida producción',operational:'operativo',operational_candidate:'operativo candidato',operational_advisory:'operativo advisory',needs_workflow:'necesita workflow',experimental:'experimental',unknown:'desconocido'}[k]||k))} · ${grouped[k].length}</div>`+grouped[k].map(executorDetail).join('')).join('')||'<p class="muted">Sin ejecutores.</p>'}
function showTab(t){$('resultTab').classList.toggle('hidden',t!=='result');$('historyTab').classList.toggle('hidden',t!=='history');$('executorsTab').classList.toggle('hidden',t!=='executors');$('plasticityTab').classList.toggle('hidden',t!=='plasticity');$('advancedTab').classList.toggle('hidden',t!=='advanced');if(t==='history')loadJobs();if(t==='executors')renderExecutors();if(t==='plasticity')renderPlasticity()}
async function loadJobs(){try{const xs=await jget('/api/jobs');state.jobs=Array.isArray(xs)?xs:[];$('history').innerHTML=state.jobs.map(j=>`<div class="history-item" onclick="openJob('${j.job_id}')"><span class="state ${j.state==='completed'?'good':(j.state==='failed'||j.state==='cancelled')?'bad':'running'}">${esc(trState(j.state))}</span><b>${esc(trOperation(j.operation))}</b><div class="muted">${esc(j.job_id.slice(0,16))}… · ${new Date(Number(j.submitted_unix_ns||0)/1e6).toLocaleString()}${j.evidence_receipt?.evidence_sha256?' · evidence '+esc(j.evidence_receipt.evidence_sha256.slice(0,12))+'…':''}</div></div>`).join('')||`<p class="muted">${currentLanguage==='es'?'Sin ejecuciones todavía.':'No executions yet.'}</p>`;renderPlasticity()}catch(e){$('history').textContent=e.message}}
async function openJob(id){try{const j=await jget('/api/jobs/'+id);showTab('result');renderJob(j)}catch(e){out(e.message)}}
async function loadRecipes(){const xs=await jget('/api/recipes');state.recipes=Array.isArray(xs)?xs:[];$('recipes').innerHTML=state.recipes.map(x=>{const r=trRecipe(x);return `<div class="recipe" data-recipe-id="${esc(x.id)}"><b>${esc(r.title)}</b><span class="badge">${esc(trCategory(x.category))}</span><p class="muted">${esc(r.description)}</p><button onclick='runRaw(${JSON.stringify(JSON.stringify(x.id))})'>${currentLanguage==='es'?'Ejecutar':'Run'}</button></div>`}).join('');renderAreas();if(state.goalPipeline.length)renderGoalPipeline()}
async function runRaw(encoded){try{const id=JSON.parse(encoded),assets=$('assets').value.split('\n').map(s=>s.trim()).filter(Boolean);out(await jpost('/api/runs',{schema:'cerebro.tidex.lab_run_request/v1',recipe_id:id,assets,selected_model_ids:[],dataset_sha256:null}))}catch(e){out(e.message)}}
(async()=>{try{state.info=await jget('/api/info');renderStatus();if(state.info.default_model_scan_root)$('modelRoot').value=state.info.default_model_scan_root;await reload();if(!state.models.length&&state.info.default_model_scan_root)await scanModels();await loadRecipes();await loadJobs();renderParams();setLabMode('system');buildGoalPipeline();applyLanguage()}catch(e){out(e.message);$('status').textContent=currentLanguage==='es'?'error de inicialización':'initialization error'}})();
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipes_are_unique_and_never_claim_authority() {
        let recipes = recipe_catalog();
        let ids = recipes
            .iter()
            .map(|item| &item.id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), recipes.len());
        assert!(recipes.iter().all(|item| item.asset_count <= MAX_ASSETS));
    }

    #[test]
    fn non_loopback_server_is_rejected_before_bind() {
        let root = std::env::temp_dir().join(format!("tidex-lab-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let address: SocketAddr = "0.0.0.0:0".parse().unwrap();
        assert!(serve_lab(&root, address).is_err());
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn configured_hf_python_candidate_prefers_explicit_override() {
        let previous = std::env::var_os("TIDEX_HF_PYTHON");
        std::env::set_var("TIDEX_HF_PYTHON", "/tmp/override-python");
        let chosen = configured_hf_python_candidate().unwrap();
        assert_eq!(chosen, PathBuf::from("/tmp/override-python"));

        match previous {
            Some(value) => std::env::set_var("TIDEX_HF_PYTHON", value),
            None => std::env::remove_var("TIDEX_HF_PYTHON"),
        }
    }
}
