//! TIDE-X production operator control plane.
//!
//! The control plane is an operator surface over existing TIDE-X authorities. It
//! never evaluates arbitrary shell commands and it never turns a control plane
//! result into production activation authority. Every executable recipe maps to
//! an allow-listed `tidex` command and its exact inputs/results are persisted.

use crate::foundation::authority::replace_private_file_atomic;
use crate::foundation::digest::Sha256Digest;
use crate::foundation::error::{BrainError, BrainResult};
use crate::operator::executor_registry::{
    executor_catalog, executor_for_operator_recipe, executor_id_for_direct_operation,
};
use crate::operator::graph::compute_operator_graph;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_SCAN_ENTRIES: usize = 100_000;
const MAX_SCAN_DEPTH: usize = 8;
const MAX_DATASET_BYTES: usize = 128 * 1024 * 1024;
const MAX_HTTP_BODY_BYTES: usize = 128 * 1024 * 1024;
const MAX_RESULT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_MODEL_CONFIG_BYTES: u64 = 1024 * 1024;
const MAX_ASSETS: usize = 8;
const MAX_LISTED_JOBS: usize = 100;
const MAX_HTTP_CONNECTIONS: usize = 32;
const MAX_CONCURRENT_OPERATOR_JOBS: usize = 4;
const HTTP_WRITE_TIMEOUT: Duration = Duration::from_secs(30);

static OPERATOR_HTTP_INFLIGHT: AtomicUsize = AtomicUsize::new(0);

const DIRECT_PARAMETER_ALLOWLIST: &[&str] = &[
    "capability_name",
    "domain",
    "level",
    "positive_examples",
    "negative_examples",
    "request",
    "scenarios",
    "activation_layers",
    "source_layer",
    "target_layer",
    "training_prompts",
    "validation_prompts",
    "benchmark_id",
    "objective",
    "probe_count",
    "extraction_level",
    "calibration_prompts",
    "strength",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorRecipeCategory {
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
pub enum OperatorExecutable {
    Tidex,
    SiblingBinary { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatorRecipe {
    pub id: String,
    pub title: String,
    pub category: OperatorRecipeCategory,
    pub description: String,
    pub executable: OperatorExecutable,
    pub asset_count: usize,
    pub argv_template: Vec<String>,
    pub production_activation: bool,
}

fn recipe(
    id: &str,
    title: &str,
    category: OperatorRecipeCategory,
    description: &str,
    asset_count: usize,
    argv: &[&str],
    production_activation: bool,
) -> OperatorRecipe {
    OperatorRecipe {
        id: id.into(),
        title: title.into(),
        category,
        description: description.into(),
        executable: OperatorExecutable::Tidex,
        asset_count,
        argv_template: argv.iter().map(|value| (*value).to_string()).collect(),
        production_activation,
    }
}

fn sibling_recipe(
    id: &str,
    title: &str,
    category: OperatorRecipeCategory,
    description: &str,
    binary: &str,
    asset_count: usize,
    argv: &[&str],
) -> OperatorRecipe {
    let mut value = recipe(id, title, category, description, asset_count, argv, false);
    value.executable = OperatorExecutable::SiblingBinary {
        name: binary.to_string(),
    };
    value
}

/// Canonical operator recipes. Each one delegates to an existing TIDE-X CLI
/// authority instead of reimplementing its semantics in the control plane.
pub fn recipe_catalog() -> Vec<OperatorRecipe> {
    vec![
        recipe(
            "acquisition.capture",
            "Acquire workspace",
            OperatorRecipeCategory::Acquisition,
            "Descriptor-bound source acquisition into the private vault.",
            0,
            &["acquire"],
            false,
        ),
        recipe(
            "knowledge.plan",
            "Plan epistemic transition",
            OperatorRecipeCategory::Learning,
            "Authenticate a persisted KnowledgeEngine state and derive the next admissible invocation or terminal decision.",
            1,
            &["knowledge", "plan", "{0}"],
            false,
        ),
        recipe(
            "residency.decide",
            "Decide capability residency",
            OperatorRecipeCategory::Learning,
            "Run ResidencyDecisionAuthority over an authenticated precommit reference and persist the resulting decision.",
            1,
            &["residency", "decide", "{0}"],
            false,
        ),
        recipe(
            "numerical.evolve",
            "Run numerical evolution campaign",
            OperatorRecipeCategory::Learning,
            "Execute one or more ordered NumericalEvolution revisions under an explicit sealed governance policy; never promotes implicitly.",
            1,
            &["numerical", "evolve", "{0}"],
            false,
        ),
        recipe(
            "analysis.tomography",
            "Run skill-field tomography",
            OperatorRecipeCategory::Discovery,
            "Execute the canonical BrainEngine analysis chain, including tomography, identifiability and structural diagnostics, over authenticated observations.",
            1,
            &["analysis", "tomography", "{0}"],
            false,
        ),
        recipe(
            "analysis.protected_map",
            "Build protected cortex map",
            OperatorRecipeCategory::Discovery,
            "Build and persist a protected-cortex map from authenticated sensitivity artifacts without using task labels.",
            1,
            &["analysis", "protected-map", "{0}"],
            false,
        ),
        recipe(
            "analysis.pythagoras",
            "Analyze geometry / topology",
            OperatorRecipeCategory::Discovery,
            "Run Pythagoras staircase correction or persistent topology analysis from a typed request.",
            1,
            &["analysis", "geometry", "{0}"],
            false,
        ),
        sibling_recipe(
            "analysis.brain",
            "Analyze skill fields",
            OperatorRecipeCategory::Discovery,
            "Run the canonical BrainEngine analysis over authenticated observations.",
            "tidex-engine",
            1,
            &["analyze", "{0}"],
        ),
        sibling_recipe(
            "runtime.sleep",
            "Consolidation / sleep",
            OperatorRecipeCategory::Learning,
            "Run the canonical evidence-bound sleep/consolidation transaction.",
            "tidex-engine",
            0,
            &["sleep"],
        ),
        sibling_recipe(
            "learning.autonomous_plan",
            "Autonomous learning plan",
            OperatorRecipeCategory::Learning,
            "Plan an adaptive learning campaign from a typed LearningTarget.",
            "autonomous-learning-plan",
            1,
            &["{0}"],
        ),
        sibling_recipe(
            "cross_model.discovery_cycle",
            "Multi-LLM discovery cycle",
            OperatorRecipeCategory::Discovery,
            "Execute real behavioral comparison across explicitly configured LLM runtimes. Requires the cross-model-plasticity build feature.",
            "plasticity-daemon",
            2,
            &["once", "{0}", "{1}"],
        ),
        sibling_recipe(
            "operator.direct_runner",
            "Direct model control plane runner",
            OperatorRecipeCategory::Discovery,
            "Execute a typed cross-model control plane request using the canonical HF runtime and analysis components.",
            "tidex-operator-runner",
            1,
            &["{0}"],
        ),
        recipe(
            "discovery.capabilities",
            "Capability discovery",
            OperatorRecipeCategory::Discovery,
            "Run canonical capability discovery from an explicit request.",
            1,
            &["discover", "capabilities", "{0}"],
            false,
        ),
        recipe(
            "benchmark.response",
            "Receiver response benchmark",
            OperatorRecipeCategory::Benchmark,
            "Benchmark declared receiver functional responses.",
            1,
            &["benchmark", "response", "{0}"],
            false,
        ),
        recipe(
            "benchmark.receiver_basis",
            "Receiver basis benchmark",
            OperatorRecipeCategory::Benchmark,
            "Benchmark the receiver basis under the declared protocol.",
            1,
            &["benchmark", "receiver-basis", "{0}"],
            false,
        ),
        recipe(
            "benchmark.portability",
            "Receiver compilation benchmark",
            OperatorRecipeCategory::Benchmark,
            "Leave-one-capability-out receiver compilation benchmark.",
            1,
            &["benchmark", "portability", "{0}"],
            false,
        ),
        recipe(
            "receiver.profile",
            "Profile receiver",
            OperatorRecipeCategory::Receiver,
            "Authenticate and profile a physical receiver checkpoint.",
            1,
            &["receiver", "profile", "{0}"],
            false,
        ),
        recipe(
            "receiver.normalize_sharded",
            "Normalize sharded SafeTensors",
            OperatorRecipeCategory::Receiver,
            "Normalize an authenticated HF sharded checkpoint into one retained SafeTensors authority.",
            1,
            &["receiver", "normalize-sharded", "{0}"],
            false,
        ),
        recipe(
            "receiver.freeze_compiler",
            "Freeze receiver compiler",
            OperatorRecipeCategory::Receiver,
            "Freeze calibration, protection, risk and maps before held-out target compilation.",
            1,
            &["receiver", "freeze-compiler", "{0}"],
            false,
        ),
        recipe(
            "receiver.verify_frozen",
            "Verify frozen compiler",
            OperatorRecipeCategory::Receiver,
            "Replay-authenticate a frozen receiver compiler.",
            1,
            &["receiver", "verify-frozen-compiler", "{0}"],
            false,
        ),
        recipe(
            "compile.universal",
            "Compile held-out capability",
            OperatorRecipeCategory::Receiver,
            "Compile an authenticated CapabilityIR with a frozen receiver compiler.",
            1,
            &["compile", "universal", "{0}"],
            false,
        ),
        recipe(
            "compile.universal_plan",
            "Plan universal shadow",
            OperatorRecipeCategory::Receiver,
            "Create a receiver-bound shadow materialization plan.",
            1,
            &["compile", "universal-plan", "{0}"],
            false,
        ),
        recipe(
            "materialize.shadow_dense",
            "Materialize dense shadow",
            OperatorRecipeCategory::Materialization,
            "Replay a universal shadow plan into a dense candidate representation bound to the receiver layout.",
            3,
            &["materialize", "dense", "{0}", "{1}", "{2}"],
            false,
        ),
        recipe(
            "materialize.shadow_low_rank",
            "Materialize low-rank shadow",
            OperatorRecipeCategory::Materialization,
            "Replay a universal shadow plan into a bounded low-rank candidate under explicit policy.",
            4,
            &["materialize", "low-rank", "{0}", "{1}", "{2}", "{3}"],
            false,
        ),
        recipe(
            "materialize.shadow_sparse",
            "Materialize sparse shadow",
            OperatorRecipeCategory::Materialization,
            "Replay a universal shadow plan into a bounded sparse candidate under explicit policy.",
            4,
            &["materialize", "sparse", "{0}", "{1}", "{2}", "{3}"],
            false,
        ),
        recipe(
            "materialize.shadow_steering",
            "Materialize activation-steering shadow",
            OperatorRecipeCategory::Materialization,
            "Replay a universal shadow plan into a runtime steering candidate; no production hook is installed.",
            5,
            &["materialize", "steering", "{0}", "{1}", "{2}", "{3}", "{4}"],
            false,
        ),
        recipe(
            "materialize.compiled",
            "Materialize compiled checkpoint",
            OperatorRecipeCategory::Materialization,
            "Physically materialize a candidate-only checkpoint using the canonical actuator.",
            1,
            &["materialize", "compiled", "{0}"],
            false,
        ),
        recipe(
            "materialize.verify",
            "Verify compiled checkpoint",
            OperatorRecipeCategory::Materialization,
            "Replay-authenticate physical checkpoint arithmetic.",
            1,
            &["materialize", "verify-compiled", "{0}"],
            false,
        ),
        recipe(
            "selection.backend",
            "Select materialization backend",
            OperatorRecipeCategory::Evaluation,
            "Rank measured candidate backends under the canonical selection policy.",
            1,
            &["select", "backend", "{0}"],
            false,
        ),
        recipe(
            "evaluation.shadow",
            "Run isolated shadow evaluation",
            OperatorRecipeCategory::Evaluation,
            "Run an authenticated evaluator and bundle through isolated execution.",
            2,
            &["shadow", "run", "{0}", "{1}"],
            false,
        ),
        recipe(
            "evidence.universality",
            "Measure universality",
            OperatorRecipeCategory::Evaluation,
            "Reduce held-out trials across capabilities, receivers, families and seeds.",
            1,
            &["measure", "universality", "{0}"],
            false,
        ),
        recipe(
            "promotion.readiness",
            "Universal promotion readiness",
            OperatorRecipeCategory::Governance,
            "Fail-closed readiness gate; does not activate production.",
            1,
            &["gate", "promotion", "{0}"],
            false,
        ),
        recipe(
            "adapter.import",
            "Import adapter",
            OperatorRecipeCategory::Lifecycle,
            "Import an authenticated adapter candidate into AdapterBank.",
            1,
            &["adapter-bank", "import", "{0}"],
            false,
        ),
        recipe(
            "adapter.compose",
            "Compose adapters",
            OperatorRecipeCategory::Lifecycle,
            "Create exact ordered adapter composition.",
            1,
            &["adapter-bank", "compose", "{0}"],
            false,
        ),
        recipe(
            "adapter.materialize",
            "Materialize adapter candidate",
            OperatorRecipeCategory::Lifecycle,
            "Create a candidate-only adapter materialization.",
            1,
            &["adapter-bank", "materialize", "{0}"],
            false,
        ),
        recipe(
            "adapter.authorize",
            "Authorize governed promotion",
            OperatorRecipeCategory::Governance,
            "Re-authenticate sealed governance witnesses and mint a current-state promotion permit.",
            1,
            &["adapter-bank", "authorize", "{0}"],
            false,
        ),
        recipe(
            "adapter.activate",
            "Activate adapter",
            OperatorRecipeCategory::Lifecycle,
            "Production activation. Requires a previously authenticated governed authorization.",
            1,
            &["adapter-bank", "activate", "{0}"],
            true,
        ),
        recipe(
            "adapter.revoke",
            "Revoke adapter",
            OperatorRecipeCategory::Lifecycle,
            "Sticky governed revocation of an adapter and dependent compositions.",
            1,
            &["adapter-bank", "revoke", "{0}"],
            true,
        ),
        recipe(
            "adapter.rollback",
            "Rollback adapter bank",
            OperatorRecipeCategory::Lifecycle,
            "Publish a new forward revision representing rollback.",
            1,
            &["adapter-bank", "rollback", "{0}"],
            true,
        ),
        recipe(
            "adapter.status",
            "Verify AdapterBank history",
            OperatorRecipeCategory::Lifecycle,
            "Verify the complete hash-linked AdapterBank history.",
            0,
            &["adapter-bank", "status"],
            false,
        ),
    ]
}

fn find_recipe(id: &str) -> BrainResult<OperatorRecipe> {
    recipe_catalog()
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| BrainError::Invalid("operator_recipe_unknown".into()))
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
    let root = std::env::var_os("TIDEX_RUNTIME_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime"));
    if !root.is_absolute() {
        return Err(BrainError::Invalid("tidex_runtime_root_must_be_absolute".into()));
    }
    Ok(root)
}

fn default_operator_home() -> BrainResult<PathBuf> {
    Ok(future_runtime_root()?.join("tidex"))
}

fn default_hf_hub_root() -> BrainResult<PathBuf> {
    Ok(future_runtime_root()?.join("llms/huggingface/hub"))
}

fn default_mechinterp_python() -> BrainResult<PathBuf> {
    Ok(future_runtime_root()?.join("python/tidex-mechinterp/bin/python"))
}

/// Discover local HF-style model snapshots. Discovery is read-only and never
/// treats a found directory as authenticated runtime evidence.
pub fn configured_operator_home() -> BrainResult<PathBuf> {
    let home = if let Some(configured) = std::env::var_os("TIDEX_HOME") {
        PathBuf::from(configured)
    } else {
        default_operator_home()?
    };
    if !home.is_absolute() {
        return Err(BrainError::Invalid("operator_home_must_be_absolute".into()));
    }
    ensure_private_dir(&home)?;
    Ok(home.canonicalize()?)
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

fn regular_file_metadata(path: &Path) -> BrainResult<fs::Metadata> {
    let meta = fs::symlink_metadata(path)
        .map_err(|_| BrainError::Invalid("operator_file_missing".into()))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(BrainError::Invalid("operator_regular_file_required".into()));
    }
    Ok(meta)
}

fn confine_model_scan_root(root: &Path) -> BrainResult<(PathBuf, PathBuf)> {
    if !root.is_absolute() {
        return Err(BrainError::Invalid("operator_model_scan_root_must_be_absolute".into()));
    }
    let hub = default_hf_hub_root()?;
    let hub_meta = fs::symlink_metadata(&hub)
        .map_err(|_| BrainError::Invalid("operator_model_hub_missing".into()))?;
    if hub_meta.file_type().is_symlink() || !hub_meta.is_dir() {
        return Err(BrainError::Invalid("operator_model_hub_invalid".into()));
    }
    let hub = hub.canonicalize()?;
    let metadata = fs::symlink_metadata(root)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(BrainError::Invalid("operator_model_scan_root_invalid".into()));
    }
    let root = root.canonicalize()?;
    if !path_is_within(&root, &hub) {
        return Err(BrainError::Invalid("operator_model_scan_root_outside_hub".into()));
    }
    Ok((hub, root))
}

fn hub_snapshot_file(path: &Path, hub: &Path) -> BrainResult<fs::Metadata> {
    let link_meta = fs::symlink_metadata(path)
        .map_err(|_| BrainError::Invalid("operator_file_missing".into()))?;
    if link_meta.file_type().is_symlink() {
        let canonical = fs::canonicalize(path)
            .map_err(|_| BrainError::Invalid("operator_model_symlink_unresolvable".into()))?;
        if !path_is_within(&canonical, hub) {
            return Err(BrainError::Invalid("operator_model_symlink_escapes_hub".into()));
        }
        let target_meta = fs::metadata(&canonical)
            .map_err(|_| BrainError::Invalid("operator_file_missing".into()))?;
        if !target_meta.is_file() {
            return Err(BrainError::Invalid("operator_regular_file_required".into()));
        }
        return Ok(target_meta);
    }
    regular_file_metadata(path)
}

fn read_hub_snapshot_file_bounded(path: &Path, hub: &Path, max: u64) -> BrainResult<Vec<u8>> {
    let meta = hub_snapshot_file(path, hub)?;
    if meta.len() > max {
        return Err(BrainError::Invalid("operator_file_too_large".into()));
    }
    fs::read(path).map_err(|_| BrainError::Invalid("operator_file_unreadable".into()))
}

fn model_candidate_identity(
    hub: &Path,
    root: &Path,
    layout: &LocalModelLayout,
    config: &Path,
    tokenizer: &Path,
    weights: &Path,
) -> BrainResult<Sha256Digest> {
    let mut files = vec![
        ("config.json".to_string(), crate::foundation::digest::sha256_file(config)?),
        ("tokenizer.json".to_string(), crate::foundation::digest::sha256_file(tokenizer)?),
    ];
    match layout {
        LocalModelLayout::HuggingFaceSingleSafetensors => {
            hub_snapshot_file(weights, hub)?;
            files.push((
                "model.safetensors".to_string(),
                crate::foundation::digest::sha256_file(weights)?,
            ));
        }
        LocalModelLayout::HuggingFaceShardedSafetensors => {
            hub_snapshot_file(weights, hub)?;
            files.push((
                "model.safetensors.index.json".to_string(),
                crate::foundation::digest::sha256_file(weights)?,
            ));
            let mut shards = Vec::new();
            for entry in fs::read_dir(root)? {
                let entry = entry?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| BrainError::Invalid("operator_model_filename_invalid".into()))?;
                if !name.ends_with(".safetensors") {
                    continue;
                }
                let path = entry.path();
                hub_snapshot_file(&path, hub)?;
                shards.push((name, crate::foundation::digest::sha256_file(&path)?));
            }
            if shards.is_empty() {
                return Err(BrainError::Invalid("operator_model_shards_missing".into()));
            }
            shards.sort_by(|left, right| left.0.cmp(&right.0));
            files.extend(shards);
        }
    }
    Ok(Sha256Digest::digest_domain(
        b"TIDEX:OPERATOR-MODEL-CANDIDATE:v2\0",
        &serde_json::to_vec(&(layout, files))?,
    ))
}

pub fn discover_local_models(root: &Path) -> BrainResult<Vec<LocalModelCandidate>> {
    let (hub, root) = confine_model_scan_root(root)?;
    let mut queue = VecDeque::from([(root.clone(), 0usize)]);
    let mut visited = 0usize;
    let mut found = Vec::new();
    while let Some((dir, depth)) = queue.pop_front() {
        visited = visited
            .checked_add(1)
            .ok_or_else(|| BrainError::Invalid("operator_model_scan_overflow".into()))?;
        if visited > MAX_SCAN_ENTRIES {
            return Err(BrainError::Invalid("operator_model_scan_limit".into()));
        }
        let config = dir.join("config.json");
        let tokenizer = dir.join("tokenizer.json");
        let single = dir.join("model.safetensors");
        let sharded = dir.join("model.safetensors.index.json");
        let single_ok = hub_snapshot_file(&single, &hub).is_ok();
        let sharded_ok = hub_snapshot_file(&sharded, &hub).is_ok();
        if hub_snapshot_file(&config, &hub).is_ok()
            && hub_snapshot_file(&tokenizer, &hub).is_ok()
            && (single_ok || sharded_ok)
        {
            let (layout, weights) = if single_ok {
                (LocalModelLayout::HuggingFaceSingleSafetensors, single)
            } else {
                (LocalModelLayout::HuggingFaceShardedSafetensors, sharded)
            };
            let architecture =
                read_hub_snapshot_file_bounded(&config, &hub, MAX_MODEL_CONFIG_BYTES)
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
            let identity =
                model_candidate_identity(&hub, &dir, &layout, &config, &tokenizer, &weights)?;
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
    let discovered = discover_local_models(root)?;
    // Content-bound identity: collapse duplicate roots that hash to the same
    // model_id (identical config/tokenizer/weights). Prefer the lexicographically
    // smallest root so rescans are stable. Never mint aliases or path-derived ids.
    let mut by_id: std::collections::BTreeMap<String, LocalModelCandidate> =
        std::collections::BTreeMap::new();
    for model in discovered {
        let key = model.model_id.to_string();
        match by_id.get(&key) {
            None => {
                by_id.insert(key, model);
            }
            Some(existing) if model.root < existing.root => {
                by_id.insert(key, model);
            }
            Some(existing) if model.root == existing.root && model != *existing => {
                // Same root, metadata drift (e.g. architecture Option): refresh.
                by_id.insert(key, model);
            }
            _ => {}
        }
    }
    let models: Vec<LocalModelCandidate> = by_id.into_values().collect();
    let catalog = tidex_home.join("operator/models/by-sha");
    ensure_private_dir(&catalog)?;
    let mut expected = std::collections::BTreeSet::new();
    for model in &models {
        let bytes = serde_json::to_vec(model)?;
        let filename = format!("{}.json", model.model_id);
        expected.insert(filename.clone());
        let path = catalog.join(&filename);
        if path.exists() {
            let existing = fs::read(&path)?;
            if existing != bytes {
                // Same content-bound id with refreshed path binding / architecture
                // is not a collision — replace. Corrupt / mismatched id still fails.
                match serde_json::from_slice::<LocalModelCandidate>(&existing) {
                    Ok(old) if old.model_id == model.model_id => {
                        replace_private_file_atomic(tidex_home, &path, &bytes, None)?;
                    }
                    _ => {
                        return Err(BrainError::Integrity(
                            "operator_model_catalog_collision".into(),
                        ));
                    }
                }
            }
        } else {
            write_private_new(&path, &bytes)?;
        }
    }
    for entry in fs::read_dir(&catalog)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            return Err(BrainError::Integrity("operator_model_catalog_entry_not_file".into()));
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| BrainError::Integrity("operator_model_catalog_filename_invalid".into()))?;
        if !expected.contains(&name) {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(models)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorDatasetFormat {
    Json,
    Jsonl,
    Csv,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatorDatasetManifest {
    pub schema: String,
    pub name: String,
    pub format: OperatorDatasetFormat,
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
        return Err(BrainError::Invalid("operator_dataset_name_invalid".into()));
    }
    Ok(())
}

fn ensure_private_dir(path: &Path) -> BrainResult<()> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_dir() || meta.permissions().mode() & 0o077 != 0 {
        return Err(BrainError::Integrity("operator_directory_not_private".into()));
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
    format: OperatorDatasetFormat,
    bytes: &[u8],
    generated: bool,
) -> BrainResult<OperatorDatasetManifest> {
    validate_dataset_name(name)?;
    if bytes.is_empty() || bytes.len() > MAX_DATASET_BYTES {
        return Err(BrainError::Invalid("operator_dataset_size_invalid".into()));
    }
    match format {
        OperatorDatasetFormat::Json => {
            let _: serde_json::Value = serde_json::from_slice(bytes)
                .map_err(|_| BrainError::Invalid("operator_dataset_json_invalid".into()))?;
        }
        OperatorDatasetFormat::Jsonl => {
            for line in bytes
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
            {
                let _: serde_json::Value = serde_json::from_slice(line)
                    .map_err(|_| BrainError::Invalid("operator_dataset_jsonl_invalid".into()))?;
            }
        }
        OperatorDatasetFormat::Csv | OperatorDatasetFormat::Text => {
            std::str::from_utf8(bytes).map_err(|_| {
                BrainError::Invalid("operator_dataset_text_encoding_invalid".into())
            })?;
        }
    }
    let root = tidex_home.join("operator/datasets/by-sha");
    ensure_private_dir(&root)?;
    let content_sha256 = Sha256Digest::digest_bytes(bytes);
    let artifact = root.join(format!("{}.data", content_sha256));
    if artifact.exists() {
        let existing = fs::read(&artifact)?;
        if Sha256Digest::digest_bytes(&existing) != content_sha256 {
            return Err(BrainError::Integrity("operator_dataset_existing_digest_mismatch".into()));
        }
    } else {
        write_private_new(&artifact, bytes)?;
    }
    let manifest = OperatorDatasetManifest {
        schema: "tidex.operator_dataset/v1".into(),
        name: name.into(),
        format,
        content_sha256: content_sha256.clone(),
        bytes: u64::try_from(bytes.len())
            .map_err(|_| BrainError::Invalid("operator_dataset_size_overflow".into()))?,
        artifact,
        generated,
        independent_evidence: !generated,
    };
    let manifests = tidex_home.join("operator/datasets/manifests");
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
pub struct OperatorRunRequest {
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
pub struct OperatorRunReceipt {
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
pub struct OperatorRunView {
    pub receipt: OperatorRunReceipt,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatorJobEvidenceReceipt {
    pub schema: String,
    pub job_id: Sha256Digest,
    pub request_sha256: Option<Sha256Digest>,
    pub executor_id: Option<String>,
    pub executor_descriptor_sha256: Option<Sha256Digest>,
    pub state: OperatorJobState,
    pub run_id: Option<Sha256Digest>,
    pub stdout_sha256: Option<Sha256Digest>,
    pub stderr_sha256: Option<Sha256Digest>,
    pub succeeded: bool,
    pub authorizes_production: bool,
    pub evidence_sha256: Sha256Digest,
}

fn build_operator_job_evidence_receipt(
    record: &OperatorJobRecord,
) -> BrainResult<OperatorJobEvidenceReceipt> {
    let run = record.run.as_ref();
    let executor_id = executor_id_for_direct_operation(&record.operation)
        .map(str::to_string)
        .or_else(|| run.and_then(|value| value.receipt.executor_id.clone()));
    let executor_descriptor_sha256 = match executor_id.as_deref() {
        Some(id) => crate::operator::executor_registry::executor_by_id(id)
            .ok()
            .map(|descriptor| descriptor.descriptor_sha256),
        None => None,
    };
    let mut receipt = OperatorJobEvidenceReceipt {
        schema: "tidex.operator_job_evidence_receipt/v1".into(),
        job_id: record.job_id.clone(),
        request_sha256: record.request_sha256.clone(),
        executor_id,
        executor_descriptor_sha256,
        state: record.state.clone(),
        run_id: run.map(|value| value.receipt.run_id.clone()),
        stdout_sha256: run.map(|value| value.receipt.stdout_sha256.clone()),
        stderr_sha256: run.map(|value| value.receipt.stderr_sha256.clone()),
        succeeded: matches!(record.state, OperatorJobState::Completed)
            && run.is_some_and(|value| value.receipt.succeeded),
        authorizes_production: false,
        evidence_sha256: Sha256Digest::zero(),
    };
    receipt.evidence_sha256 = Sha256Digest::digest_domain(
        b"TIDEX:OPERATOR-JOB-EVIDENCE:v1\0",
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

fn verify_operator_job_evidence_receipt(
    record: &OperatorJobRecord,
    receipt: &OperatorJobEvidenceReceipt,
) -> BrainResult<()> {
    let run = record.run.as_ref();
    let expected_digest = Sha256Digest::digest_domain(
        b"TIDEX:OPERATOR-JOB-EVIDENCE:v1\0",
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
    let run_matches = match run {
        Some(run) => {
            receipt.run_id.as_ref() == Some(&run.receipt.run_id)
                && receipt.stdout_sha256.as_ref() == Some(&run.receipt.stdout_sha256)
                && receipt.stderr_sha256.as_ref() == Some(&run.receipt.stderr_sha256)
        }
        None => {
            receipt.run_id.is_none()
                && receipt.stdout_sha256.is_none()
                && receipt.stderr_sha256.is_none()
        }
    };
    if receipt.schema != "tidex.operator_job_evidence_receipt/v1"
        || receipt.job_id != record.job_id
        || receipt.request_sha256 != record.request_sha256
        || receipt.state != record.state
        || receipt.authorizes_production
        || receipt.succeeded
            != (matches!(record.state, OperatorJobState::Completed)
                && run.is_some_and(|value| value.receipt.succeeded))
        || !run_matches
        || receipt.evidence_sha256 != expected_digest
    {
        return Err(BrainError::Integrity("operator_job_evidence_receipt_invalid".into()));
    }
    Ok(())
}

fn operator_run_view(receipt: OperatorRunReceipt) -> BrainResult<OperatorRunView> {
    let stdout_bytes = fs::read(&receipt.stdout)?;
    let stderr_bytes = fs::read(&receipt.stderr)?;
    if stdout_bytes.len() as u64 > MAX_RESULT_BYTES || stderr_bytes.len() as u64 > MAX_RESULT_BYTES
    {
        return Err(BrainError::Invalid("operator_run_output_limit".into()));
    }
    if Sha256Digest::digest_bytes(&stdout_bytes) != receipt.stdout_sha256 {
        return Err(BrainError::Integrity("operator_run_stdout_digest_mismatch".into()));
    }
    if Sha256Digest::digest_bytes(&stderr_bytes) != receipt.stderr_sha256 {
        return Err(BrainError::Integrity("operator_run_stderr_digest_mismatch".into()));
    }
    let stdout = String::from_utf8(stdout_bytes)
        .map_err(|_| BrainError::Invalid("operator_run_stdout_not_utf8".into()))?;
    let stderr = String::from_utf8(stderr_bytes)
        .map_err(|_| BrainError::Invalid("operator_run_stderr_not_utf8".into()))?;
    Ok(OperatorRunView {
        receipt,
        stdout,
        stderr,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorJobState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

static OPERATOR_JOB_CANCELLATIONS: OnceLock<
    Mutex<std::collections::BTreeMap<String, Arc<AtomicBool>>>,
> = OnceLock::new();

fn cancellation_registry() -> &'static Mutex<std::collections::BTreeMap<String, Arc<AtomicBool>>> {
    OPERATOR_JOB_CANCELLATIONS.get_or_init(|| Mutex::new(std::collections::BTreeMap::new()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatorJobRecord {
    pub schema: String,
    pub job_id: Sha256Digest,
    #[serde(default)]
    pub request_sha256: Option<Sha256Digest>,
    #[serde(default)]
    pub evidence_receipt: Option<OperatorJobEvidenceReceipt>,
    pub state: OperatorJobState,
    pub operation: String,
    pub submitted_unix_ns: u128,
    pub run: Option<OperatorRunView>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
enum OperatorJobRequest {
    BehavioralDiscovery(BehavioralDiscoveryWorkflowRequest),
    Direct(OperatorDirectWorkflowRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatorRuntimeAccessProfile {
    pub behavioral_inference: bool,
    pub internal_activations: bool,
    pub activation_intervention: bool,
    pub deep_instrumentation: bool,
    pub sparse_autoencoder_analysis: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatorModelRuntimeProfile {
    pub schema: String,
    pub model_id: Sha256Digest,
    pub source_job_id: Sha256Digest,
    pub source_run_id: Sha256Digest,
    pub runtime_metadata_sha256: String,
    pub runtime_architecture: String,
    pub parameter_count: u64,
    pub num_layers: usize,
    pub embedding_dim: usize,
    pub access: OperatorRuntimeAccessProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatorModelRuntimeStatus {
    pub schema: String,
    pub model_id: Sha256Digest,
    pub state: String,
    pub job_id: Option<Sha256Digest>,
    pub profile: Option<OperatorModelRuntimeProfile>,
    pub error: Option<String>,
}

fn runtime_profile_from_probe_job(
    record: &OperatorJobRecord,
) -> BrainResult<Option<OperatorModelRuntimeProfile>> {
    if record.operation != "probe_runtime" || record.state != OperatorJobState::Completed {
        return Ok(None);
    }
    let run = match &record.run {
        Some(value) if value.receipt.selected_model_ids.len() == 1 => value,
        _ => return Ok(None),
    };
    let value: serde_json::Value = serde_json::from_str(&run.stdout)
        .map_err(|_| BrainError::Invalid("operator_runtime_probe_output_not_json".into()))?;
    if value.get("schema").and_then(serde_json::Value::as_str)
        != Some("tidex.operator_runtime_probe/v1")
    {
        return Ok(None);
    }
    let model = value
        .get("model")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| BrainError::Invalid("operator_runtime_probe_model_missing".into()))?;
    let access = value
        .get("access")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| BrainError::Invalid("operator_runtime_probe_access_missing".into()))?;
    let read_bool = |name: &str| -> BrainResult<bool> {
        access
            .get(name)
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| {
                BrainError::Invalid(format!("operator_runtime_probe_access_missing:{name}"))
            })
    };
    Ok(Some(OperatorModelRuntimeProfile {
        schema: "tidex.operator_model_runtime_profile/v1".into(),
        model_id: run.receipt.selected_model_ids[0].clone(),
        source_job_id: record.job_id.clone(),
        source_run_id: run.receipt.run_id.clone(),
        runtime_metadata_sha256: model
            .get("runtime_metadata_sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| BrainError::Invalid("operator_runtime_probe_metadata_missing".into()))?
            .to_string(),
        runtime_architecture: model
            .get("runtime_architecture")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                BrainError::Invalid("operator_runtime_probe_architecture_missing".into())
            })?
            .to_string(),
        parameter_count: model
            .get("parameter_count")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                BrainError::Invalid("operator_runtime_probe_parameter_count_missing".into())
            })?,
        num_layers: model
            .get("num_layers")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| BrainError::Invalid("operator_runtime_probe_layers_missing".into()))?,
        embedding_dim: model
            .get("embedding_dim")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| {
                BrainError::Invalid("operator_runtime_probe_embedding_missing".into())
            })?,
        access: OperatorRuntimeAccessProfile {
            behavioral_inference: read_bool("behavioral_inference")?,
            internal_activations: read_bool("internal_activations")?,
            activation_intervention: read_bool("activation_intervention")?,
            deep_instrumentation: read_bool("deep_instrumentation")?,
            sparse_autoencoder_analysis: read_bool("sparse_autoencoder_analysis")?,
        },
    }))
}

pub fn list_runtime_profiles(tidex_home: &Path) -> BrainResult<Vec<OperatorModelRuntimeProfile>> {
    let cataloged_models = list_catalog_models(tidex_home)?
        .into_iter()
        .map(|model| model.model_id)
        .collect::<std::collections::BTreeSet<_>>();
    let mut latest: std::collections::BTreeMap<Sha256Digest, OperatorModelRuntimeProfile> =
        std::collections::BTreeMap::new();
    for mut record in list_all_job_records(tidex_home)? {
        if record.operation != "probe_runtime" {
            continue;
        }
        hydrate_job_run(&mut record)?;
        if let Some(profile) = runtime_profile_from_probe_job(&record)? {
            if cataloged_models.contains(&profile.model_id) {
                latest.entry(profile.model_id.clone()).or_insert(profile);
            }
        }
    }
    Ok(latest.into_values().collect())
}

pub fn list_runtime_statuses(tidex_home: &Path) -> BrainResult<Vec<OperatorModelRuntimeStatus>> {
    let models = list_catalog_models(tidex_home)?;
    let mut latest_jobs = std::collections::BTreeMap::<Sha256Digest, OperatorJobRecord>::new();
    for mut record in list_all_job_records(tidex_home)? {
        if record.operation != "probe_runtime" {
            continue;
        }
        hydrate_job_run(&mut record)?;
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
            Ok(OperatorModelRuntimeStatus {
                schema: "tidex.operator_model_runtime_status/v1".into(),
                model_id: model.model_id,
                state: job
                    .map(|record| match record.state {
                        OperatorJobState::Queued => "queued",
                        OperatorJobState::Running => "running",
                        OperatorJobState::Completed => "completed",
                        OperatorJobState::Failed => "failed",
                        OperatorJobState::Cancelled => "cancelled",
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
        .join("operator/jobs/by-sha")
        .join(job_id.as_str())
        .join("status.json")
}

fn persist_job_record(tidex_home: &Path, record: &OperatorJobRecord) -> BrainResult<()> {
    let root = tidex_home
        .join("operator/jobs/by-sha")
        .join(record.job_id.as_str());
    ensure_private_dir(&root)?;
    let bytes = serde_json::to_vec(&slim_job_record(record))?;
    let target = root.join("status.json");
    replace_private_file_atomic(tidex_home, &target, &bytes, None)?;
    Ok(())
}

fn slim_job_record(record: &OperatorJobRecord) -> OperatorJobRecord {
    let mut slim = record.clone();
    if let Some(run) = slim.run.as_mut() {
        run.stdout.clear();
        run.stderr.clear();
    }
    slim
}

fn hydrate_job_run(record: &mut OperatorJobRecord) -> BrainResult<()> {
    let Some(run) = record.run.as_ref() else {
        return Ok(());
    };
    if !run.stdout.is_empty() || !run.stderr.is_empty() {
        return Ok(());
    }
    if !run.receipt.stdout.exists() && !run.receipt.stderr.exists() {
        return Ok(());
    }
    record.run = Some(operator_run_view(run.receipt.clone())?);
    Ok(())
}

pub fn load_job_record(tidex_home: &Path, job_id: &Sha256Digest) -> BrainResult<OperatorJobRecord> {
    let bytes = fs::read(job_status_path(tidex_home, job_id))
        .map_err(|_| BrainError::Invalid("operator_job_not_found".into()))?;
    let mut record: OperatorJobRecord = serde_json::from_slice(&bytes)?;
    if record.job_id != *job_id || record.schema != "tidex.operator_job/v1" {
        return Err(BrainError::Integrity("operator_job_identity_invalid".into()));
    }
    if let Some(receipt) = record.evidence_receipt.as_ref() {
        verify_operator_job_evidence_receipt(&record, receipt)?;
    }
    hydrate_job_run(&mut record)?;
    Ok(record)
}

fn collect_job_records(
    tidex_home: &Path,
    limit: Option<usize>,
) -> BrainResult<Vec<OperatorJobRecord>> {
    let root = tidex_home.join("operator/jobs/by-sha");
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
        let mut record: OperatorJobRecord = serde_json::from_slice(&bytes)?;
        if record.schema != "tidex.operator_job/v1" {
            return Err(BrainError::Integrity("operator_job_schema_invalid".into()));
        }
        if entry.file_name().to_string_lossy() != record.job_id.as_str() {
            return Err(BrainError::Integrity("operator_job_path_identity_invalid".into()));
        }
        if let Some(receipt) = record.evidence_receipt.as_ref() {
            verify_operator_job_evidence_receipt(&record, receipt)?;
        }
        record = slim_job_record(&record);
        records.push(record);
    }
    records.sort_by_key(|record| std::cmp::Reverse(record.submitted_unix_ns));
    if let Some(limit) = limit {
        records.truncate(limit);
    }
    Ok(records)
}

pub fn list_job_records(tidex_home: &Path) -> BrainResult<Vec<OperatorJobRecord>> {
    collect_job_records(tidex_home, Some(MAX_LISTED_JOBS))
}

fn list_all_job_records(tidex_home: &Path) -> BrainResult<Vec<OperatorJobRecord>> {
    collect_job_records(tidex_home, None)
}

/// Production enqueue for direct workflows (HTTP control plane + workflow NextAction).
///
/// Same path as `POST /api/workflows/direct` → `start_operator_job(Direct)`.
pub fn start_operator_direct_job(
    tidex_home: &Path,
    request: OperatorDirectWorkflowRequest,
) -> BrainResult<OperatorJobRecord> {
    start_operator_job(tidex_home, OperatorJobRequest::Direct(request))
}

/// Production enqueue for behavioral discovery (HTTP control plane + workflow NextAction).
///
/// Same path as `POST /api/workflows/behavioral-discovery` → `start_operator_job(BehavioralDiscovery)`.
pub fn start_operator_behavioral_discovery_job(
    tidex_home: &Path,
    request: BehavioralDiscoveryWorkflowRequest,
) -> BrainResult<OperatorJobRecord> {
    start_operator_job(tidex_home, OperatorJobRequest::BehavioralDiscovery(request))
}

fn start_operator_job(
    tidex_home: &Path,
    request: OperatorJobRequest,
) -> BrainResult<OperatorJobRecord> {
    let submitted_unix_ns = now_nanos()?;
    let operation = match &request {
        OperatorJobRequest::BehavioralDiscovery(_) => "behavioral_discovery",
        OperatorJobRequest::Direct(value) => match value.operation {
            OperatorDirectOperation::ProbeRuntime => "probe_runtime",
            OperatorDirectOperation::BehavioralEvaluation => "behavioral_evaluation",
            OperatorDirectOperation::ExtractCapability => "extract_capability",
            OperatorDirectOperation::DeepInstrumentation => "deep_instrumentation",
            OperatorDirectOperation::SparseAutoencoderAnalysis => "sparse_autoencoder_analysis",
            OperatorDirectOperation::CounterfactualAnalysis => "counterfactual_analysis",
            OperatorDirectOperation::GenerateBehavioralDataset => "generate_behavioral_dataset",
            OperatorDirectOperation::CalibrateAlignment => "calibrate_alignment",
            OperatorDirectOperation::ActivationTransferExperiment => {
                "activation_transfer_experiment"
            }
        },
    }
    .to_string();
    let request_digest = match &request {
        OperatorJobRequest::BehavioralDiscovery(value) => {
            Sha256Digest::digest_bytes(&serde_json::to_vec(value)?)
        }
        OperatorJobRequest::Direct(value) => {
            Sha256Digest::digest_bytes(&serde_json::to_vec(value)?)
        }
    };
    if let Some(existing) = list_all_job_records(tidex_home)?
        .into_iter()
        .find(|record| {
            record.request_sha256.as_ref() == Some(&request_digest)
                && matches!(record.state, OperatorJobState::Queued | OperatorJobState::Running)
        })
    {
        return Ok(existing);
    }
    let job_id = Sha256Digest::digest_domain(
        b"TIDEX:OPERATOR-JOB:v1\0",
        &serde_json::to_vec(&(submitted_unix_ns, &operation, &request_digest))?,
    );
    let queued = OperatorJobRecord {
        schema: "tidex.operator_job/v1".into(),
        job_id: job_id.clone(),
        request_sha256: Some(request_digest.clone()),
        evidence_receipt: None,
        state: OperatorJobState::Queued,
        operation: operation.clone(),
        submitted_unix_ns,
        run: None,
        error: None,
    };
    persist_job_record(tidex_home, &queued)?;
    let cancellation = Arc::new(AtomicBool::new(false));
    {
        let mut registry = cancellation_registry()
            .lock()
            .map_err(|_| BrainError::Integrity("operator_cancellation_registry_poisoned".into()))?;
        if registry.len() >= MAX_CONCURRENT_OPERATOR_JOBS {
            drop(registry);
            let mut rejected = queued.clone();
            rejected.state = OperatorJobState::Failed;
            rejected.error = Some("operator_job_concurrency_limit".into());
            rejected.evidence_receipt = Some(build_operator_job_evidence_receipt(&rejected)?);
            persist_job_record(tidex_home, &rejected)?;
            return Err(BrainError::Invalid("operator_job_concurrency_limit".into()));
        }
        registry.insert(job_id.as_str().to_string(), cancellation.clone());
    }
    let home = tidex_home.to_path_buf();
    let thread_job_id = job_id.clone();
    let spawn_job = std::thread::Builder::new()
        .name(format!("tidex-operator-{}", &job_id.as_str()[..12]))
        .spawn(move || {
            let mut record = OperatorJobRecord {
                schema: "tidex.operator_job/v1".into(),
                job_id: thread_job_id,
                request_sha256: Some(request_digest),
                evidence_receipt: None,
                state: OperatorJobState::Running,
                operation,
                submitted_unix_ns,
                run: None,
                error: None,
            };
            if persist_job_record(&home, &record).is_err() {
                if let Ok(mut registry) = cancellation_registry().lock() {
                    registry.remove(record.job_id.as_str());
                }
                return;
            }
            let result = match request {
                OperatorJobRequest::BehavioralDiscovery(value) => {
                    execute_behavioral_discovery_workflow_cancelable(
                        &home,
                        &value,
                        Some(&cancellation),
                    )
                    .and_then(operator_run_view)
                }
                OperatorJobRequest::Direct(value) => {
                    execute_direct_workflow_cancelable(&home, &value, Some(&cancellation))
                        .and_then(operator_run_view)
                }
            };
            match result {
                Ok(run) if run.receipt.succeeded => {
                    record.state = OperatorJobState::Completed;
                    record.run = Some(run);
                }
                Ok(run) => {
                    record.state = OperatorJobState::Failed;
                    record.error = Some(if run.stderr.trim().is_empty() {
                        format!("operator_child_exit_failed:{}", run.receipt.exit_code)
                    } else {
                        run.stderr.trim().to_string()
                    });
                    record.run = Some(run);
                }
                Err(error) => {
                    let message = error.to_string();
                    record.state = if message.contains("operator_run_cancelled") {
                        OperatorJobState::Cancelled
                    } else {
                        OperatorJobState::Failed
                    };
                    record.error = Some(message);
                }
            }
            if let Ok(receipt) = build_operator_job_evidence_receipt(&record) {
                record.evidence_receipt = Some(receipt);
            }
            let _ = persist_job_record(&home, &record);
            if let Ok(mut registry) = cancellation_registry().lock() {
                registry.remove(record.job_id.as_str());
            }
        });
    if spawn_job.is_err() {
        if let Ok(mut registry) = cancellation_registry().lock() {
            registry.remove(job_id.as_str());
        }
        return Err(BrainError::Invalid("operator_job_thread_spawn_failed".into()));
    }
    Ok(queued)
}

pub fn cancel_operator_job(
    tidex_home: &Path,
    job_id: &Sha256Digest,
) -> BrainResult<OperatorJobRecord> {
    let mut record = load_job_record(tidex_home, job_id)?;
    match record.state {
        OperatorJobState::Queued | OperatorJobState::Running => {
            let flag = cancellation_registry()
                .lock()
                .map_err(|_| {
                    BrainError::Integrity("operator_cancellation_registry_poisoned".into())
                })?
                .get(job_id.as_str())
                .cloned()
                .ok_or_else(|| {
                    BrainError::Invalid("operator_job_not_active_in_this_process".into())
                })?;
            flag.store(true, Ordering::SeqCst);
            record.error = Some("cancellation_requested".into());
            persist_job_record(tidex_home, &record)?;
            Ok(record)
        }
        _ => Err(BrainError::Invalid("operator_job_not_cancellable".into())),
    }
}

fn recover_incomplete_operator_jobs(tidex_home: &Path) -> BrainResult<usize> {
    let mut recovered = 0usize;
    for mut record in list_all_job_records(tidex_home)? {
        let inconsistent_completed = matches!(record.state, OperatorJobState::Completed)
            && record
                .run
                .as_ref()
                .is_some_and(|run| !run.receipt.succeeded);
        if matches!(record.state, OperatorJobState::Queued | OperatorJobState::Running)
            || inconsistent_completed
        {
            record.state = OperatorJobState::Failed;
            record.error = Some(if inconsistent_completed {
                "recovered_inconsistent_completed_failed_run".into()
            } else {
                "recovered_incomplete_job_after_operator_restart".into()
            });
            record.evidence_receipt = Some(build_operator_job_evidence_receipt(&record)?);
            persist_job_record(tidex_home, &record)?;
            recovered = recovered
                .checked_add(1)
                .ok_or_else(|| BrainError::Invalid("operator_recovery_counter_overflow".into()))?;
        }
    }
    Ok(recovered)
}

fn resolve_assets(
    tidex_home: &Path,
    request: &OperatorRunRequest,
    recipe: &OperatorRecipe,
) -> BrainResult<Vec<PathBuf>> {
    if request.schema != "tidex.operator_run_request/v1"
        || request.assets.len() != recipe.asset_count
        || request.assets.len() > MAX_ASSETS
    {
        return Err(BrainError::Invalid("operator_run_request_invalid".into()));
    }
    let home = tidex_home
        .canonicalize()
        .map_err(|_| BrainError::Invalid("operator_home_unreadable".into()))?;
    let mut out = Vec::with_capacity(request.assets.len());
    for path in &request.assets {
        if !path.is_absolute() {
            return Err(BrainError::Invalid("operator_asset_path_must_be_absolute".into()));
        }
        let meta = fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() || !meta.is_file() || meta.len() == 0 {
            return Err(BrainError::Invalid("operator_asset_invalid".into()));
        }
        let canonical = path.canonicalize()?;
        if !path_is_within(&canonical, &home) {
            return Err(BrainError::Invalid("operator_asset_outside_operator_home".into()));
        }
        out.push(canonical);
    }
    Ok(out)
}

fn render_argv(recipe: &OperatorRecipe, assets: &[PathBuf]) -> BrainResult<Vec<String>> {
    recipe
        .argv_template
        .iter()
        .map(|part| {
            if part.starts_with('{') && part.ends_with('}') {
                let index = part[1..part.len() - 1].parse::<usize>().map_err(|_| {
                    BrainError::Integrity("operator_recipe_placeholder_invalid".into())
                })?;
                let path = assets
                    .get(index)
                    .ok_or_else(|| BrainError::Integrity("operator_recipe_asset_missing".into()))?;
                Ok(path.to_string_lossy().into_owned())
            } else {
                Ok(part.clone())
            }
        })
        .collect()
}

pub fn execute_operator_run(
    tidex_home: &Path,
    request: &OperatorRunRequest,
) -> BrainResult<OperatorRunReceipt> {
    execute_operator_run_cancelable(tidex_home, request, None)
}

fn execute_operator_run_cancelable(
    tidex_home: &Path,
    request: &OperatorRunRequest,
    cancellation: Option<&AtomicBool>,
) -> BrainResult<OperatorRunReceipt> {
    let recipe = find_recipe(&request.recipe_id)?;
    if recipe.production_activation {
        return Err(BrainError::Invalid("operator_recipe_production_activation_forbidden".into()));
    }
    let assets = resolve_assets(tidex_home, request, &recipe)?;
    let argv = render_argv(&recipe, &assets)?;
    let source_tree_sha256 = Sha256Digest::parse(env!("TIDEX_SOURCE_TREE_DIGEST"))?;
    let run_id = Sha256Digest::digest_domain(
        b"TIDEX:OPERATOR-RUN:v1\0",
        &serde_json::to_vec(&(
            &request.recipe_id,
            &argv,
            &request.selected_model_ids,
            &request.dataset_sha256,
            now_nanos()?,
            &source_tree_sha256,
        ))?,
    );
    let run_root = tidex_home
        .join("operator/runs/by-sha")
        .join(run_id.as_str());
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
        OperatorExecutable::Tidex => current_executable.clone(),
        OperatorExecutable::SiblingBinary { name } => {
            if name.is_empty()
                || name.len() > 128
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            {
                return Err(BrainError::Integrity("operator_recipe_binary_invalid".into()));
            }
            let path = current_executable
                .parent()
                .ok_or_else(|| BrainError::Integrity("operator_executable_parent_missing".into()))?
                .join(name);
            let meta = fs::symlink_metadata(&path)
                .map_err(|_| BrainError::Invalid("operator_recipe_binary_not_built".into()))?;
            if meta.file_type().is_symlink() || !meta.is_file() {
                return Err(BrainError::Integrity("operator_recipe_binary_invalid".into()));
            }
            path
        }
    };
    let maximum_runtime_seconds = request.maximum_runtime_seconds.unwrap_or(1_800);
    if !(1..=86_400).contains(&maximum_runtime_seconds) {
        return Err(BrainError::Invalid("operator_run_timeout_invalid".into()));
    }
    let mut child = Command::new(executable)
        .args(&argv)
        .env("TIDEX_OPERATOR_CHILD", "1")
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
            return Err(BrainError::Invalid("operator_run_cancelled".into()));
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(BrainError::Invalid("operator_run_timeout".into()));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let stdout_meta = fs::metadata(&stdout_path)?;
    let stderr_meta = fs::metadata(&stderr_path)?;
    if stdout_meta.len() > MAX_RESULT_BYTES || stderr_meta.len() > MAX_RESULT_BYTES {
        return Err(BrainError::Invalid("operator_run_output_limit".into()));
    }
    let stdout_sha256 = Sha256Digest::digest_bytes(&fs::read(&stdout_path)?);
    let stderr_sha256 = Sha256Digest::digest_bytes(&fs::read(&stderr_path)?);
    let exit_code = status.code().unwrap_or(-1);
    let executor_id =
        executor_for_operator_recipe(&recipe.id)?.map(|descriptor| descriptor.executor_id);
    let receipt = OperatorRunReceipt {
        schema: "tidex.operator_run_receipt/v1".into(),
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
        .map_err(|_| BrainError::Invalid("operator_system_clock_invalid".into()))?
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

fn validate_catalog_model_candidate(model: &LocalModelCandidate) -> BrainResult<()> {
    let (hub, root) = confine_model_scan_root(&model.root)?;
    let expected_weights = match model.layout {
        LocalModelLayout::HuggingFaceSingleSafetensors => root.join("model.safetensors"),
        LocalModelLayout::HuggingFaceShardedSafetensors => {
            root.join("model.safetensors.index.json")
        }
    };
    if root != model.root
        || model.config != root.join("config.json")
        || model.tokenizer != root.join("tokenizer.json")
        || model.weights != expected_weights
    {
        return Err(BrainError::Integrity("operator_model_catalog_path_binding_invalid".into()));
    }
    let identity = model_candidate_identity(
        &hub,
        &root,
        &model.layout,
        &model.config,
        &model.tokenizer,
        &model.weights,
    )?;
    if identity != model.model_id {
        return Err(BrainError::Integrity("operator_model_catalog_content_changed".into()));
    }
    Ok(())
}

fn load_catalog_model(tidex_home: &Path, id: &Sha256Digest) -> BrainResult<LocalModelCandidate> {
    let path = tidex_home
        .join("operator/models/by-sha")
        .join(format!("{}.json", id));
    let bytes =
        fs::read(&path).map_err(|_| BrainError::Invalid("operator_model_not_cataloged".into()))?;
    let model: LocalModelCandidate = serde_json::from_slice(&bytes)?;
    if &model.model_id != id {
        return Err(BrainError::Integrity("operator_model_catalog_identity_mismatch".into()));
    }
    validate_catalog_model_candidate(&model)?;
    Ok(model)
}

fn load_dataset_manifest(
    tidex_home: &Path,
    id: &Sha256Digest,
) -> BrainResult<OperatorDatasetManifest> {
    let path = tidex_home
        .join("operator/datasets/manifests")
        .join(format!("{}.json", id));
    let bytes = fs::read(&path)
        .map_err(|_| BrainError::Invalid("operator_dataset_not_cataloged".into()))?;
    let manifest: OperatorDatasetManifest = serde_json::from_slice(&bytes)?;
    if &manifest.content_sha256 != id {
        return Err(BrainError::Integrity("operator_dataset_manifest_identity_mismatch".into()));
    }
    let artifact = fs::read(&manifest.artifact)?;
    if Sha256Digest::digest_bytes(&artifact) != *id {
        return Err(BrainError::Integrity("operator_dataset_artifact_digest_mismatch".into()));
    }
    Ok(manifest)
}

pub fn list_catalog_models(tidex_home: &Path) -> BrainResult<Vec<LocalModelCandidate>> {
    let root = tidex_home.join("operator/models/by-sha");
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
        validate_catalog_model_candidate(&model)?;
        out.push(model);
    }
    out.sort_by(|a, b| a.root.cmp(&b.root));
    Ok(out)
}

pub fn list_datasets(tidex_home: &Path) -> BrainResult<Vec<OperatorDatasetManifest>> {
    let root = tidex_home.join("operator/datasets/manifests");
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
        let manifest: OperatorDatasetManifest = serde_json::from_slice(&bytes)?;
        out.push(manifest);
    }
    out.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.content_sha256.cmp(&b.content_sha256))
    });
    Ok(out)
}

fn configured_hf_python_candidate() -> BrainResult<PathBuf> {
    if let Some(value) = std::env::var_os("TIDEX_HF_PYTHON") {
        if value.is_empty() {
            return Err(BrainError::Invalid("tidex_hf_python_empty".into()));
        }
        return Ok(PathBuf::from(value));
    }
    default_mechinterp_python()
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
    let candidate = configured_hf_python_candidate()?;
    usable_python_path(candidate)?.ok_or_else(|| BrainError::Invalid("hf_python_not_found".into()))
}

pub fn execute_behavioral_discovery_workflow(
    tidex_home: &Path,
    request: &BehavioralDiscoveryWorkflowRequest,
) -> BrainResult<OperatorRunReceipt> {
    execute_behavioral_discovery_workflow_cancelable(tidex_home, request, None)
}

fn execute_behavioral_discovery_workflow_cancelable(
    tidex_home: &Path,
    request: &BehavioralDiscoveryWorkflowRequest,
    cancellation: Option<&AtomicBool>,
) -> BrainResult<OperatorRunReceipt> {
    if request.schema != "tidex.operator_behavioral_discovery/v1"
        || request.model_ids.len() < 2
        || request.model_ids.len() > 64
        || request.max_new_tokens == 0
        || request.max_new_tokens > 4096
    {
        return Err(BrainError::Invalid("operator_behavioral_discovery_request_invalid".into()));
    }
    let mut unique = std::collections::BTreeSet::new();
    if request
        .model_ids
        .iter()
        .any(|id| !unique.insert(id.clone()))
    {
        return Err(BrainError::Invalid("operator_behavioral_discovery_model_duplicate".into()));
    }
    let dataset = load_dataset_manifest(tidex_home, &request.dataset_sha256)?;
    if dataset.format != OperatorDatasetFormat::Json {
        return Err(BrainError::Invalid("behavioral_benchmark_requires_json_dataset".into()));
    }
    let benchmark_bytes = fs::read(&dataset.artifact)?;
    let benchmark: serde_json::Value = serde_json::from_slice(&benchmark_bytes)?;
    if benchmark.get("schema").and_then(|v| v.as_str())
        != Some("tidex.cross_model.behavioral_benchmark/v1")
    {
        return Err(BrainError::Invalid("dataset_is_not_behavioral_benchmark".into()));
    }
    let python = find_python3()?;
    let mut models = Vec::with_capacity(request.model_ids.len());
    for id in &request.model_ids {
        let model = load_catalog_model(tidex_home, id)?;
        if model.layout != LocalModelLayout::HuggingFaceSingleSafetensors {
            return Err(BrainError::Invalid(
                "operator_hf_workflow_requires_single_safetensors".into(),
            ));
        }
        let name = model
            .root
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| BrainError::Invalid("operator_model_name_unavailable".into()))?;
        models.push(serde_json::json!({
            "backend":"hf_transformers",
            "runtime":{
                "name": format!("operator-{}", &id.as_str()[..12]),
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
        "schema":"tidex.cross_model.runtime/v2",
        "models":models,
        "maximum_stored_capabilities":100000
    });
    let workflow_root = tidex_home.join("operator/workflow-inputs");
    ensure_private_dir(&workflow_root)?;
    let runtime_bytes = serde_json::to_vec(&runtime)?;
    let runtime_sha = Sha256Digest::digest_bytes(&runtime_bytes);
    let runtime_path = workflow_root.join(format!("runtime-{}.json", runtime_sha));
    if !runtime_path.exists() {
        write_private_new(&runtime_path, &runtime_bytes)?;
    }
    execute_operator_run_cancelable(
        tidex_home,
        &OperatorRunRequest {
            schema: "tidex.operator_run_request/v1".into(),
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
pub enum OperatorDirectOperation {
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
pub struct OperatorDirectWorkflowRequest {
    pub schema: String,
    pub operation: OperatorDirectOperation,
    pub model_ids: Vec<Sha256Digest>,
    #[serde(default)]
    pub dataset_sha256: Option<Sha256Digest>,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatorInfo {
    pub schema: String,
    /// Crate/binary that owns the control plane. Not the Hugging Face worker.
    pub engine: String,
    pub operator_home: PathBuf,
    pub default_model_scan_root: Option<PathBuf>,
    /// Interpreter path selected from TIDEX_HF_PYTHON/config/default. This is
    /// the exact path executed so virtualenv symlinks keep their environment.
    /// It is a worker for HF checkpoints, not the TIDE-X runtime identity.
    pub hf_python: Option<PathBuf>,
    pub configured_hf_python: Option<PathBuf>,
    pub resolved_hf_python: Option<PathBuf>,
    pub nnsight_available: bool,
    pub bound_sparse_dictionary: bool,
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

pub fn operator_info(tidex_home: &Path) -> BrainResult<OperatorInfo> {
    let default_model_scan_root = default_hf_hub_root().ok().filter(|path| path.is_dir());
    let configured_hf_python = configured_hf_python_candidate().ok();
    let hf_python = find_python3().ok();
    let resolved_hf_python = hf_python
        .as_deref()
        .and_then(|python| python.canonicalize().ok());
    let nnsight_available = hf_python
        .as_deref()
        .is_some_and(|python| python_module_available(python, "nnsight"));
    let bound_sparse_dictionary = false;
    Ok(OperatorInfo {
        schema: "tidex.operator_info/v1".into(),
        engine: env!("CARGO_PKG_NAME").into(),
        operator_home: tidex_home.to_path_buf(),
        default_model_scan_root,
        hf_python,
        configured_hf_python,
        resolved_hf_python,
        nnsight_available,
        bound_sparse_dictionary,
    })
}

fn hf_runtime_for_model(
    tidex_home: &Path,
    id: &Sha256Digest,
    require_nnsight: bool,
    require_sae_lens: bool,
    max_tokens: usize,
) -> BrainResult<serde_json::Value> {
    let model = load_catalog_model(tidex_home, id)?;
    if model.layout != LocalModelLayout::HuggingFaceSingleSafetensors {
        return Err(BrainError::Invalid("operator_hf_workflow_requires_single_safetensors".into()));
    }
    let python = find_python3()?;
    Ok(serde_json::json!({
        "name": format!("operator-{}", &id.as_str()[..12]),
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
    request: &OperatorDirectWorkflowRequest,
) -> BrainResult<OperatorRunReceipt> {
    execute_direct_workflow_cancelable(tidex_home, request, None)
}

fn execute_direct_workflow_cancelable(
    tidex_home: &Path,
    request: &OperatorDirectWorkflowRequest,
    cancellation: Option<&AtomicBool>,
) -> BrainResult<OperatorRunReceipt> {
    if request.schema != "tidex.operator_direct_workflow/v1" {
        return Err(BrainError::Invalid("operator_direct_workflow_schema_invalid".into()));
    }
    let expected_models = match request.operation {
        OperatorDirectOperation::ProbeRuntime
        | OperatorDirectOperation::BehavioralEvaluation
        | OperatorDirectOperation::ExtractCapability
        | OperatorDirectOperation::DeepInstrumentation
        | OperatorDirectOperation::SparseAutoencoderAnalysis
        | OperatorDirectOperation::CounterfactualAnalysis
        | OperatorDirectOperation::GenerateBehavioralDataset => 1,
        OperatorDirectOperation::CalibrateAlignment
        | OperatorDirectOperation::ActivationTransferExperiment => 2,
    };
    if request.model_ids.len() != expected_models {
        return Err(BrainError::Invalid("operator_direct_workflow_model_count_invalid".into()));
    }
    let mut unique = std::collections::BTreeSet::new();
    if request
        .model_ids
        .iter()
        .any(|id| !unique.insert(id.clone()))
    {
        return Err(BrainError::Invalid("operator_direct_workflow_model_duplicate".into()));
    }
    let parameters = request.parameters.as_object().cloned().unwrap_or_default();
    let mut payload = serde_json::Map::new();
    for (key, value) in parameters {
        if matches!(
            key.as_str(),
            "operation" | "model" | "source" | "target" | "benchmark" | "evaluation"
        ) {
            return Err(BrainError::Invalid("operator_direct_workflow_reserved_parameter".into()));
        }
        if !DIRECT_PARAMETER_ALLOWLIST.contains(&key.as_str()) {
            continue;
        }
        payload.insert(key, value);
    }
    let op_name = match request.operation {
        OperatorDirectOperation::ProbeRuntime => "probe_runtime",
        OperatorDirectOperation::BehavioralEvaluation => "behavioral_evaluation",
        OperatorDirectOperation::ExtractCapability => "extract_capability",
        OperatorDirectOperation::DeepInstrumentation => "deep_instrumentation",
        OperatorDirectOperation::SparseAutoencoderAnalysis => "sparse_autoencoder_analysis",
        OperatorDirectOperation::CounterfactualAnalysis => "counterfactual_analysis",
        OperatorDirectOperation::GenerateBehavioralDataset => "generate_behavioral_dataset",
        OperatorDirectOperation::CalibrateAlignment => "calibrate_alignment",
        OperatorDirectOperation::ActivationTransferExperiment => "activation_transfer_experiment",
    };
    payload.insert("operation".into(), serde_json::Value::String(op_name.into()));
    let requires_nnsight = matches!(
        request.operation,
        OperatorDirectOperation::DeepInstrumentation
            | OperatorDirectOperation::SparseAutoencoderAnalysis
    );
    let requires_sae =
        matches!(request.operation, OperatorDirectOperation::SparseAutoencoderAnalysis);
    if expected_models == 1 {
        let max_tokens =
            if matches!(request.operation, OperatorDirectOperation::GenerateBehavioralDataset) {
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
            )?,
        );
    } else {
        payload.insert(
            "source".into(),
            hf_runtime_for_model(tidex_home, &request.model_ids[0], false, false, 128)?,
        );
        payload.insert(
            "target".into(),
            hf_runtime_for_model(tidex_home, &request.model_ids[1], false, false, 128)?,
        );
    }
    if matches!(
        request.operation,
        OperatorDirectOperation::BehavioralEvaluation
            | OperatorDirectOperation::ActivationTransferExperiment
    ) {
        let dataset_id = request.dataset_sha256.as_ref().ok_or_else(|| {
            BrainError::Invalid("operator_direct_workflow_dataset_required".into())
        })?;
        let dataset = load_dataset_manifest(tidex_home, dataset_id)?;
        if dataset.format != OperatorDatasetFormat::Json {
            return Err(BrainError::Invalid(
                "operator_direct_workflow_benchmark_must_be_json".into(),
            ));
        }
        let bytes = fs::read(&dataset.artifact)?;
        let benchmark: serde_json::Value = serde_json::from_slice(&bytes)?;
        if benchmark.get("schema").and_then(|v| v.as_str())
            != Some("tidex.cross_model.behavioral_benchmark/v1")
        {
            return Err(BrainError::Invalid("dataset_is_not_behavioral_benchmark".into()));
        }
        let key = if matches!(request.operation, OperatorDirectOperation::BehavioralEvaluation) {
            "benchmark"
        } else {
            "evaluation"
        };
        payload.insert(key.into(), benchmark);
    }
    let request_value = serde_json::Value::Object(payload);
    let bytes = serde_json::to_vec(&request_value)?;
    let digest = Sha256Digest::digest_bytes(&bytes);
    let root = tidex_home.join("operator/workflow-inputs");
    ensure_private_dir(&root)?;
    let path = root.join(format!("direct-{}.json", digest));
    if !path.exists() {
        write_private_new(&path, &bytes)?;
    }
    execute_operator_run_cancelable(
        tidex_home,
        &OperatorRunRequest {
            schema: "tidex.operator_run_request/v1".into(),
            recipe_id: "operator.direct_runner".into(),
            assets: vec![path],
            selected_model_ids: request.model_ids.clone(),
            dataset_sha256: request.dataset_sha256.clone(),
            maximum_runtime_seconds: Some(
                if matches!(request.operation, OperatorDirectOperation::GenerateBehavioralDataset) {
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
pub struct OperatorBcmAdvice {
    pub capability: String,
    pub theta_m: f64,
    pub observations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatorEligibilityAdvice {
    pub capability: String,
    pub trace_value: f64,
    pub credit_accumulated: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatorNeuromodulationAdvice {
    pub reward: f64,
    pub attention: f64,
    pub novelty: f64,
    pub stability: f64,
    pub plasticity_modulation: f64,
    pub source_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatorPiAdvice {
    pub setpoint: f64,
    pub measurement: f64,
    pub output: f64,
    pub updates: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatorContentAdvice {
    pub capability: String,
    pub measured_similarity: f64,
    pub adaptation_pressure: f64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatorPlasticityAdvice {
    pub schema: String,
    /// True only when a measured score gap produced ELO comparisons or a unique
    /// routing winner. Controllers may still report BCM/eligibility from a
    /// single measured score; that is not a ranking.
    pub available: bool,
    pub source_jobs: usize,
    pub elo_leaderboard: Vec<(String, f64)>,
    pub elo_entities: Vec<OperatorEloEntityAdvice>,
    pub routing_decisions: Vec<serde_json::Value>,
    pub bcm: Vec<OperatorBcmAdvice>,
    pub eligibility: Vec<OperatorEligibilityAdvice>,
    pub neuromodulation: Option<OperatorNeuromodulationAdvice>,
    pub pi_controller: Option<OperatorPiAdvice>,
    pub content: Vec<OperatorContentAdvice>,
    pub coevolution: Vec<serde_json::Value>,
    /// Next advisory BidirectionalLoop tick (operation proposal + controller bias).
    pub coevolution_directive: Option<serde_json::Value>,
    pub notes: Vec<String>,
}

#[cfg(not(feature = "cross-model-plasticity"))]
fn empty_plasticity_advice(notes: Vec<String>) -> OperatorPlasticityAdvice {
    OperatorPlasticityAdvice {
        schema: "tidex.operator_plasticity_advice/v3".into(),
        available: false,
        source_jobs: 0,
        elo_leaderboard: Vec::new(),
        elo_entities: Vec::new(),
        routing_decisions: Vec::new(),
        bcm: Vec::new(),
        eligibility: Vec::new(),
        neuromodulation: None,
        pi_controller: None,
        content: Vec::new(),
        coevolution: Vec::new(),
        coevolution_directive: None,
        notes,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatorEloEntityAdvice {
    pub entity: String,
    pub rating: f64,
    pub comparisons: usize,
    pub last_evidence_sha256: Option<String>,
    pub last_update: String,
}

#[cfg(feature = "cross-model-plasticity")]
const OPERATOR_PLASTICITY_CONTROLLER_STATE_SCHEMA: &str =
    "tidex.operator_plasticity_controller_state/v1";
#[cfg(feature = "cross-model-plasticity")]
const OPERATOR_PLASTICITY_CONTROLLER_STATE_DOMAIN: &[u8] =
    b"TIDEX:OPERATOR-PLASTICITY-CONTROLLER-STATE:v1\0";

#[cfg(feature = "cross-model-plasticity")]
fn operator_plasticity_controller_state_path(tidex_home: &Path) -> PathBuf {
    tidex_home.join("operator/plasticity/controller_state.json")
}

#[cfg(feature = "cross-model-plasticity")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct OperatorPlasticityControllerState {
    schema: String,
    config_sha256: String,
    evidence_sha256: String,
    elo: std::collections::BTreeMap<String, crate::cross_model::plasticity::ELOState>,
    bcm: std::collections::BTreeMap<String, crate::cross_model::plasticity::BCMState>,
    eligibility:
        std::collections::BTreeMap<String, crate::cross_model::plasticity::EligibilityTrace>,
    neuromodulation_levels:
        std::collections::BTreeMap<crate::cross_model::plasticity::Neuromodulator, f64>,
    pi: crate::cross_model::plasticity::PIControllerState,
    content:
        std::collections::BTreeMap<String, crate::cross_model::plasticity::ContentPlasticityState>,
    content_matrix: crate::cross_model::plasticity::ContentPlasticityMatrix,
    routing_history:
        std::collections::BTreeMap<String, Vec<crate::cross_model::plasticity::RoutingDecision>>,
    routing_matrix: crate::cross_model::plasticity::RoutingPlasticityMatrix,
    applied_observation_keys: std::collections::BTreeSet<String>,
    applied_elo_pair_keys: std::collections::BTreeSet<String>,
    applied_routing_keys: std::collections::BTreeSet<String>,
    coevolution_history: Vec<crate::cross_model::co_evolution::CoEvolutionStep>,
    applied_coevolution_keys: std::collections::BTreeSet<String>,
    #[serde(default)]
    coevolution_directive: Option<crate::cross_model::co_evolution::CoEvolutionDirective>,
    #[serde(default)]
    applied_loop_tick_keys: std::collections::BTreeSet<String>,
}

#[cfg(feature = "cross-model-plasticity")]
fn observation_key(benchmark: &str, model: &str, evidence: &str) -> String {
    format!("{benchmark}\0{model}\0{evidence}")
}

#[cfg(feature = "cross-model-plasticity")]
fn coevolution_report_key(report_value: &serde_json::Value) -> BrainResult<String> {
    Ok(Sha256Digest::digest_domain(
        b"TIDEX:OPERATOR-COEVOLUTION-CYCLE:v1\0",
        &serde_json::to_vec(report_value)?,
    )
    .to_string())
}

#[cfg(feature = "cross-model-plasticity")]
fn seal_operator_plasticity_controller_state(
    mut state: OperatorPlasticityControllerState,
) -> BrainResult<OperatorPlasticityControllerState> {
    state.evidence_sha256 = Sha256Digest::zero().to_string();
    let body = serde_json::to_vec(&state)?;
    state.evidence_sha256 =
        Sha256Digest::digest_domain(OPERATOR_PLASTICITY_CONTROLLER_STATE_DOMAIN, &body).to_string();
    Ok(state)
}

#[cfg(feature = "cross-model-plasticity")]
fn verify_operator_plasticity_controller_state(
    state: &OperatorPlasticityControllerState,
) -> BrainResult<()> {
    if state.schema != OPERATOR_PLASTICITY_CONTROLLER_STATE_SCHEMA
        || !Sha256Digest::is_valid_str(&state.config_sha256)
        || !Sha256Digest::is_valid_str(&state.evidence_sha256)
    {
        return Err(BrainError::Integrity("operator_plasticity_controller_state_invalid".into()));
    }
    let mut candidate = state.clone();
    let claimed = candidate.evidence_sha256.clone();
    candidate.evidence_sha256 = Sha256Digest::zero().to_string();
    let body = serde_json::to_vec(&candidate)?;
    let expected =
        Sha256Digest::digest_domain(OPERATOR_PLASTICITY_CONTROLLER_STATE_DOMAIN, &body).to_string();
    if claimed != expected {
        return Err(BrainError::Integrity(
            "operator_plasticity_controller_state_evidence_mismatch".into(),
        ));
    }
    Ok(())
}

#[cfg(feature = "cross-model-plasticity")]
fn load_operator_plasticity_controller_state(
    tidex_home: &Path,
) -> BrainResult<Option<OperatorPlasticityControllerState>> {
    let path = operator_plasticity_controller_state_path(tidex_home);
    if !path.exists() {
        return Ok(None);
    }
    let meta = regular_file_metadata(&path)?;
    if meta.len() == 0 || meta.len() > 32 * 1024 * 1024 {
        return Err(BrainError::Invalid(
            "operator_plasticity_controller_state_size_invalid".into(),
        ));
    }
    let bytes = fs::read(&path).map_err(|_| {
        BrainError::Invalid("operator_plasticity_controller_state_unreadable".into())
    })?;
    let state: OperatorPlasticityControllerState =
        serde_json::from_slice(&bytes).map_err(|_| {
            BrainError::Integrity("operator_plasticity_controller_state_json_invalid".into())
        })?;
    verify_operator_plasticity_controller_state(&state)?;
    Ok(Some(state))
}

#[cfg(feature = "cross-model-plasticity")]
fn persist_operator_plasticity_controller_state(
    tidex_home: &Path,
    state: &OperatorPlasticityControllerState,
) -> BrainResult<()> {
    verify_operator_plasticity_controller_state(state)?;
    let root = tidex_home.join("operator/plasticity");
    ensure_private_dir(&root)?;
    let bytes = serde_json::to_vec(state)?;
    replace_private_file_atomic(
        tidex_home,
        &operator_plasticity_controller_state_path(tidex_home),
        &bytes,
        None,
    )?;
    Ok(())
}

#[cfg(feature = "cross-model-plasticity")]
pub fn compute_operator_plasticity_advice(
    tidex_home: &Path,
) -> BrainResult<OperatorPlasticityAdvice> {
    use crate::cross_model::plasticity::{
        default_plasticity_toml_path, load_plasticity_control_plane_configs, BCMMetaplasticity,
        ContentPlasticity, ELOSystem, EligibilityTraces, Neuromodulation, NeuromodulationSignal,
        Neuromodulator, PIController, RoutingObservation, RoutingPlasticity,
    };
    use std::collections::{BTreeMap, BTreeSet};

    let (configs, toml_bytes) =
        load_plasticity_control_plane_configs(&default_plasticity_toml_path())
            .map_err(|error| BrainError::Invalid(format!("plasticity_toml_load_failed:{error}")))?;
    let config_sha256 = Sha256Digest::digest_bytes(&toml_bytes).to_string();

    let mut elo =
        ELOSystem::new(configs.elo.clone()).map_err(|error| BrainError::Invalid(error))?;
    let mut routing = RoutingPlasticity::new(configs.routing.clone())
        .map_err(|error| BrainError::Invalid(error))?;
    let mut bcm =
        BCMMetaplasticity::new(configs.bcm.clone()).map_err(|error| BrainError::Invalid(error))?;
    let mut eligibility = EligibilityTraces::new(configs.eligibility.clone())
        .map_err(|error| BrainError::Invalid(error))?;
    let mut neuromodulation = Neuromodulation::new(configs.neuromodulation.clone())
        .map_err(|error| BrainError::Invalid(error))?;
    let mut pi =
        PIController::new(configs.pi.clone()).map_err(|error| BrainError::Invalid(error))?;
    let mut content = ContentPlasticity::new(configs.content.clone())
        .map_err(|error| BrainError::Invalid(error))?;

    let mut applied_observation_keys = BTreeSet::new();
    let mut applied_elo_pair_keys = BTreeSet::new();
    let mut applied_routing_keys = BTreeSet::new();
    let mut applied_coevolution_keys = BTreeSet::new();
    let mut applied_loop_tick_keys = BTreeSet::new();
    let mut pending_coevolution_directive: Option<
        crate::cross_model::co_evolution::CoEvolutionDirective,
    > = None;
    let mut coevolution_loop =
        crate::cross_model::co_evolution::BidirectionalLoop::new(Default::default())
            .map_err(BrainError::Invalid)?;
    let mut notes = Vec::new();

    match load_operator_plasticity_controller_state(tidex_home)? {
        Some(state) if state.config_sha256 == config_sha256 => {
            elo.import_ratings(state.elo.into_iter().collect())
                .map_err(|error| BrainError::Integrity(error))?;
            bcm.import_states(state.bcm.into_iter().collect())
                .map_err(|error| BrainError::Integrity(error))?;
            eligibility
                .import_traces(state.eligibility.into_iter().collect())
                .map_err(|error| BrainError::Integrity(error))?;
            neuromodulation
                .import_levels(state.neuromodulation_levels.into_iter().collect())
                .map_err(|error| BrainError::Integrity(error))?;
            pi.restore_state(state.pi)
                .map_err(|error| BrainError::Integrity(error))?;
            content
                .import_states(state.content.into_iter().collect())
                .map_err(|error| BrainError::Integrity(error))?;
            content.import_matrix(state.content_matrix);
            routing
                .import_history(state.routing_history.into_iter().collect())
                .map_err(|error| BrainError::Integrity(error))?;
            routing
                .import_matrix(state.routing_matrix)
                .map_err(|error| BrainError::Integrity(error))?;
            coevolution_loop
                .import_history(state.coevolution_history)
                .map_err(|error| BrainError::Integrity(error))?;
            applied_observation_keys = state.applied_observation_keys;
            applied_elo_pair_keys = state.applied_elo_pair_keys;
            applied_routing_keys = state.applied_routing_keys;
            applied_coevolution_keys = state.applied_coevolution_keys;
            applied_loop_tick_keys = state.applied_loop_tick_keys;
            pending_coevolution_directive = state.coevolution_directive;
            notes.push(
                "Estado plástico durable recargado desde operator/plasticity/controller_state.json."
                    .into(),
            );
        }
        Some(_) => notes.push(
            "Estado plástico durable ignorado: fingerprint de config/plasticity.toml distinto."
                .into(),
        ),
        None => notes.push(
            "Sin estado plástico durable previo; se inicializa bajo operator/plasticity/.".into(),
        ),
    }

    #[derive(Clone)]
    struct RankedObservation {
        observation: RoutingObservation,
        submitted_unix_ns: u128,
        job_id: String,
    }

    let mut by_domain: BTreeMap<String, BTreeMap<String, RankedObservation>> = BTreeMap::new();
    let mut source_jobs = 0usize;
    #[derive(Clone)]
    struct TimedDiscoveryReport {
        value: serde_json::Value,
        submitted_unix_ns: u128,
        job_id: String,
    }
    #[derive(Clone)]
    struct TimedIntervention {
        evidence: crate::cross_model::co_evolution::AppliedInterventionEvidence,
        submitted_unix_ns: u128,
        #[allow(dead_code)]
        job_id: String,
    }
    let mut discovery_reports: Vec<TimedDiscoveryReport> = Vec::new();
    let mut interventions: Vec<TimedIntervention> = Vec::new();

    // Oldest → newest so newer-valid replaces by submission time.
    let mut jobs = list_all_job_records(tidex_home)?;
    jobs.sort_by(|left, right| {
        left.submitted_unix_ns
            .cmp(&right.submitted_unix_ns)
            .then_with(|| left.job_id.as_str().cmp(right.job_id.as_str()))
    });

    for mut record in jobs {
        if record.state != OperatorJobState::Completed {
            continue;
        }
        hydrate_job_run(&mut record)?;
        let run = match record.run.as_ref() {
            Some(run) => run,
            None => continue,
        };
        let value = match serde_json::from_str::<serde_json::Value>(&run.stdout) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let schema = value.get("schema").and_then(serde_json::Value::as_str);
        if schema == Some("tidex.cross_model.discovery_cycle/v1") {
            discovery_reports.push(TimedDiscoveryReport {
                value: value.clone(),
                submitted_unix_ns: record.submitted_unix_ns,
                job_id: record.job_id.as_str().to_string(),
            });
        }
        if schema == Some("tidex.operator_activation_transfer/v1") {
            if let (Some(capability), Some(target), Some(receipt)) = (
                value
                    .get("capability_name")
                    .and_then(serde_json::Value::as_str),
                value
                    .get("target_model")
                    .and_then(serde_json::Value::as_str),
                value.get("intervention"),
            ) {
                let receipt_sha256 = Sha256Digest::digest_bytes(&serde_json::to_vec(receipt)?);
                interventions.push(TimedIntervention {
                    evidence: crate::cross_model::co_evolution::AppliedInterventionEvidence {
                        capability_name: capability.into(),
                        target_model: target.into(),
                        receipt_sha256: receipt_sha256.to_string(),
                    },
                    submitted_unix_ns: record.submitted_unix_ns,
                    job_id: record.job_id.as_str().to_string(),
                });
            }
        }
        let evaluations = if schema == Some("tidex.cross_model.discovery_cycle/v1") {
            value
                .get("evaluations")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default()
        } else if schema == Some("tidex.cross_model.model_evaluation/v1") {
            vec![value]
        } else {
            Vec::new()
        };
        if evaluations.is_empty() {
            continue;
        }
        source_jobs = source_jobs.checked_add(1).ok_or_else(|| {
            BrainError::Invalid("operator_plasticity_source_counter_overflow".into())
        })?;
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
                notes.push(format!(
                    "Evaluación ignorada en {benchmark}: score o evidencia inválidos."
                ));
                continue;
            }
            let incoming = RankedObservation {
                observation: RoutingObservation {
                    model: model.clone(),
                    score,
                    sample_size,
                    evidence_sha256: evidence.clone(),
                },
                submitted_unix_ns: record.submitted_unix_ns,
                job_id: record.job_id.as_str().to_string(),
            };
            let domain = by_domain.entry(benchmark.clone()).or_default();
            match domain.get(&model) {
                Some(existing)
                    if existing.observation.evidence_sha256 != evidence
                        && (incoming.submitted_unix_ns < existing.submitted_unix_ns
                            || (incoming.submitted_unix_ns == existing.submitted_unix_ns
                                && incoming.job_id <= existing.job_id)) =>
                {
                    notes.push(format!(
                        "Evaluación más antigua ignorada en {benchmark} para {model}; se conserva la evidencia más reciente."
                    ));
                }
                Some(existing) if existing.observation.evidence_sha256 == evidence => {}
                Some(existing) => {
                    notes.push(format!(
                        "Evaluación reemplazada en {benchmark} para {model}; política newer-valid ({} → {}).",
                        &existing.observation.evidence_sha256[..12],
                        &evidence[..12]
                    ));
                    domain.insert(model, incoming);
                }
                None => {
                    domain.insert(model, incoming);
                }
            }
        }
    }

    if source_jobs == 0 {
        notes.push(
            "Sin jobs de evaluación o descubrimiento completados. probe_runtime no alimenta ELO ni routing."
                .into(),
        );
    }

    let mut routing_decisions = Vec::new();
    for (benchmark, observations_by_model) in &by_domain {
        if observations_by_model.is_empty() {
            continue;
        }
        if observations_by_model.len() == 1 {
            let observation = &observations_by_model
                .values()
                .next()
                .expect("len == 1")
                .observation;
            notes.push(format!(
                "Evaluación aislada en {benchmark} de {} (score {:.3}). ELO y rutas exigen al menos dos modelos en el mismo benchmark.",
                observation.model, observation.score
            ));
            continue;
        }

        let models = observations_by_model.keys().cloned().collect::<Vec<_>>();
        for left in 0..models.len() {
            for right in left + 1..models.len() {
                let first = &observations_by_model[&models[left]].observation;
                let second = &observations_by_model[&models[right]].observation;
                if first.score.total_cmp(&second.score) == std::cmp::Ordering::Equal {
                    continue;
                }
                if elo.get_state(&first.model).is_none() {
                    let _ = elo.initialize_rating(&first.model);
                }
                if elo.get_state(&second.model).is_none() {
                    let _ = elo.initialize_rating(&second.model);
                }
                let first_observed = (0.5 + (first.score - second.score) / 2.0).clamp(0.0, 1.0);
                let evidence = Sha256Digest::digest_domain(
                    b"TIDEX:OPERATOR-ELO-PAIR:v1\0",
                    &serde_json::to_vec(&(
                        benchmark,
                        &first.model,
                        &first.evidence_sha256,
                        &second.model,
                        &second.evidence_sha256,
                    ))?,
                );
                let pair_key = evidence.to_string();
                if applied_elo_pair_keys.contains(&pair_key) {
                    continue;
                }
                match elo.update_observed(
                    &first.model,
                    &second.model,
                    first_observed,
                    evidence.as_str(),
                ) {
                    Ok(_) => {
                        applied_elo_pair_keys.insert(pair_key);
                    }
                    Err(error) => notes.push(format!("ELO ignorado en {benchmark}: {error}")),
                }
            }
        }

        let observations = observations_by_model
            .values()
            .map(|row| row.observation.clone())
            .collect::<Vec<_>>();
        let Some(max_score) = observations
            .iter()
            .map(|row| row.score)
            .max_by(|left, right| left.total_cmp(right))
        else {
            continue;
        };
        let winners = observations
            .iter()
            .filter(|row| row.score.total_cmp(&max_score) == std::cmp::Ordering::Equal)
            .count();
        let all_tied = observations
            .iter()
            .all(|row| row.score.total_cmp(&max_score) == std::cmp::Ordering::Equal);
        if all_tied {
            notes.push(format!(
                "Empate medido en {benchmark}: todos los scores son {:.3}. No hay ruta ni ganador ELO; un 1500 por defecto no es un ranking.",
                max_score
            ));
            continue;
        }
        if winners != 1 {
            notes.push(format!(
                "Ruta retenida en {benchmark}: score máximo {:.3} compartido por {winners} modelos. Un desempate no es un ranking.",
                max_score
            ));
            continue;
        }
        let scope = format!("benchmark:{benchmark}");
        let route_key = Sha256Digest::digest_domain(
            b"TIDEX:OPERATOR-ROUTE-OBS:v1 ",
            &serde_json::to_vec(&(
                &scope,
                observations
                    .iter()
                    .map(|row| (&row.model, row.score, row.sample_size, &row.evidence_sha256))
                    .collect::<Vec<_>>(),
            ))?,
        )
        .to_string();
        if applied_routing_keys.contains(&route_key) {
            if let Some(decision) = routing.get_routing_history(&scope).last() {
                routing_decisions.push(serde_json::to_value(decision)?);
            }
            continue;
        }
        match routing.route_capability(&scope, &observations) {
            Ok(decision) => {
                applied_routing_keys.insert(route_key);
                notes.push(format!(
                    "Ruta por score medido en {benchmark}: {} ({:.3}).",
                    decision.target_model, decision.measured_score
                ));
                routing_decisions.push(serde_json::to_value(&decision)?);
            }
            Err(error) => notes.push(format!("Routing ignorado en {scope}: {error}")),
        }
    }

    let mut seen_benchmarks = 0usize;
    let mut pi_snapshot = None;
    let mut neuromodulation_snapshot = None;
    let base_bcm_lr = bcm.config().learning_rate;
    let base_eligibility_rate = eligibility.config().trace_update_rate;
    let base_content_rate = content.config().adaptation_rate;
    let base_routing_matrix_lr = routing.export_matrix().learning_rate;

    for (benchmark, observations_by_model) in &by_domain {
        let mut rows = observations_by_model
            .values()
            .map(|row| row.observation.clone())
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| left.model.cmp(&right.model));
        if bcm.get_state(benchmark).is_none() {
            if bcm.initialize_state(benchmark).is_err() {
                notes.push(format!("BCM ya inicializado para {benchmark}"));
            }
        }
        if eligibility.get_trace(benchmark).is_none() {
            if eligibility.initialize_trace(benchmark).is_err() {
                notes.push(format!("Eligibilidad ya inicializada para {benchmark}"));
            }
        }

        let mut pending_rows = Vec::new();
        for row in &rows {
            let key = observation_key(benchmark, &row.model, &row.evidence_sha256);
            if applied_observation_keys.contains(&key) {
                continue;
            }
            pending_rows.push(row.clone());
        }

        if rows.len() >= 2 {
            let scores = rows.iter().map(|row| row.score).collect::<Vec<_>>();
            let min_score = scores.iter().copied().fold(f64::INFINITY, f64::min);
            let max_score = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let spread = (max_score - min_score).clamp(0.0, 1.0);
            let evidence = Sha256Digest::digest_domain(
                b"TIDEX:OPERATOR-NEUROMODULATION:v1\0",
                &serde_json::to_vec(&(
                    benchmark,
                    rows.iter()
                        .map(|row| row.evidence_sha256.as_str())
                        .collect::<Vec<_>>(),
                ))?,
            );
            let timestamp = chrono::Utc::now().to_rfc3339();
            let novelty = (1.0 / (seen_benchmarks as f64 + 1.0)).clamp(0.0, 1.0);
            let signals = [
                (Neuromodulator::Reward, max_score),
                (Neuromodulator::Attention, spread),
                (Neuromodulator::Novelty, novelty),
                (Neuromodulator::Stability, 1.0 - spread),
            ];
            for (modulator, level) in signals {
                if let Err(error) = neuromodulation.emit_signal(NeuromodulationSignal {
                    modulator,
                    level,
                    timestamp: timestamp.clone(),
                    source: format!("benchmark:{benchmark}"),
                    source_sha256: evidence.to_string(),
                }) {
                    notes.push(format!("Neuromodulación ignorada en {benchmark}: {error}"));
                }
            }
            match neuromodulation.calculate_plasticity_modulation() {
                Ok(plasticity_modulation) => {
                    neuromodulation_snapshot = Some(OperatorNeuromodulationAdvice {
                        reward: max_score,
                        attention: spread,
                        novelty,
                        stability: 1.0 - spread,
                        plasticity_modulation,
                        source_sha256: evidence.to_string(),
                    });
                    match neuromodulation.modulate_learning_rate(base_bcm_lr) {
                        Ok(modulated) => {
                            if let Err(error) = bcm.set_learning_rate(benchmark, modulated) {
                                notes.push(format!(
                                    "BCM lr neuromodulada ignorada en {benchmark}: {error}"
                                ));
                            }
                        }
                        Err(error) => notes
                            .push(format!("Neuromodulación→BCM ignorada en {benchmark}: {error}")),
                    }
                    match neuromodulation.modulate_learning_rate(base_eligibility_rate) {
                        Ok(modulated) => {
                            if let Err(error) = eligibility.set_trace_update_rate(modulated) {
                                notes.push(format!(
                                    "Eligibilidad rate neuromodulada ignorada: {error}"
                                ));
                            }
                        }
                        Err(error) => {
                            notes.push(format!("Neuromodulación→eligibilidad ignorada: {error}"))
                        }
                    }
                    match neuromodulation.modulate_learning_rate(base_content_rate) {
                        Ok(modulated) => {
                            if let Err(error) = content.set_adaptation_rate(modulated) {
                                notes.push(format!("Content rate neuromodulada ignorada: {error}"));
                            }
                        }
                        Err(error) => {
                            notes.push(format!("Neuromodulación→content ignorada: {error}"))
                        }
                    }
                    match neuromodulation.modulate_learning_rate(base_routing_matrix_lr) {
                        Ok(modulated) => {
                            if let Err(error) = routing.set_matrix_learning_rate(modulated) {
                                notes.push(format!(
                                    "Routing matrix lr neuromodulada ignorada: {error}"
                                ));
                            }
                        }
                        Err(error) => {
                            notes.push(format!("Neuromodulación→routing ignorada: {error}"))
                        }
                    }
                }
                Err(error) => notes.push(format!("Neuromodulación incompleta: {error}")),
            }
        }

        for row in &pending_rows {
            match bcm.update_threshold(benchmark, row.score) {
                Ok(_) => {}
                Err(error) => notes.push(format!("BCM ignorado en {benchmark}: {error}")),
            }
            match eligibility.update_trace(benchmark, row.score) {
                Ok(_) => {}
                Err(error) => notes.push(format!("Eligibilidad ignorada en {benchmark}: {error}")),
            }
            applied_observation_keys.insert(observation_key(
                benchmark,
                &row.model,
                &row.evidence_sha256,
            ));
        }

        if !pending_rows.is_empty() && rows.len() >= 2 {
            let mean = rows.iter().map(|row| row.score).sum::<f64>() / rows.len() as f64;
            for row in &pending_rows {
                let credit = row.score - mean;
                if let Err(error) = eligibility.accumulate_credit(benchmark, credit) {
                    notes.push(format!("Crédito de eligibilidad ignorado en {benchmark}: {error}"));
                }
            }
            let scores = rows.iter().map(|row| row.score).collect::<Vec<_>>();
            let min_score = scores.iter().copied().fold(f64::INFINITY, f64::min);
            let max_score = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let spread = (max_score - min_score).clamp(0.0, 1.0);
            let evidence = Sha256Digest::digest_domain(
                b"TIDEX:OPERATOR-NEUROMODULATION:v1\0",
                &serde_json::to_vec(&(
                    benchmark,
                    rows.iter()
                        .map(|row| row.evidence_sha256.as_str())
                        .collect::<Vec<_>>(),
                ))?,
            );
            match pi.update(1.0, mean, 1.0) {
                Ok(output) => {
                    let state = pi.get_state();
                    pi_snapshot = Some(OperatorPiAdvice {
                        setpoint: 1.0,
                        measurement: mean,
                        output,
                        updates: state.updates,
                    });
                }
                Err(error) => notes.push(format!("PI ignorado en {benchmark}: {error}")),
            }
            if content.get_state(benchmark).is_none() {
                if content
                    .initialize_state(
                        benchmark,
                        rows[0].evidence_sha256.clone(),
                        rows[0].evidence_sha256.clone(),
                    )
                    .is_err()
                {
                    notes.push(format!("Contenido ya inicializado para {benchmark}"));
                }
            }
            let similarity = 1.0 - spread;
            match content.update_similarity(
                benchmark,
                rows[rows.len() - 1].evidence_sha256.clone(),
                similarity,
                evidence.to_string(),
            ) {
                Ok(_) => {}
                Err(error) => notes.push(format!("Contenido ignorado en {benchmark}: {error}")),
            }
            seen_benchmarks = seen_benchmarks.saturating_add(1);
        } else if rows.len() >= 2 {
            // Refresh PI/content advisory view from current means without double-counting.
            let mean = rows.iter().map(|row| row.score).sum::<f64>() / rows.len() as f64;
            let state = pi.get_state();
            pi_snapshot = Some(OperatorPiAdvice {
                setpoint: 1.0,
                measurement: mean,
                output: state.last_output,
                updates: state.updates,
            });
            if let Some(state) = content.get_state(benchmark) {
                // keep existing content advice via later collect
                let _ = state;
            }
            seen_benchmarks = seen_benchmarks.saturating_add(1);
        }
    }

    // Restore base rates after modulated updates so persisted config remains the TOML baseline
    // on the next process; per-capability BCM lr stays modulated in state.
    let _ = eligibility.set_trace_update_rate(base_eligibility_rate);
    let _ = content.set_adaptation_rate(base_content_rate);
    let _ = routing.set_matrix_learning_rate(base_routing_matrix_lr);

    let bcm_advice = by_domain
        .keys()
        .filter_map(|capability| {
            let state = bcm.get_state(capability)?;
            Some(OperatorBcmAdvice {
                capability: capability.clone(),
                theta_m: state.theta_m,
                observations: state.sliding_window.len(),
            })
        })
        .collect::<Vec<_>>();
    let eligibility_advice = by_domain
        .keys()
        .filter_map(|capability| {
            Some(OperatorEligibilityAdvice {
                capability: capability.clone(),
                trace_value: eligibility.get_trace(capability)?,
                credit_accumulated: eligibility
                    .get_accumulated_credit(capability)
                    .unwrap_or(0.0),
            })
        })
        .collect::<Vec<_>>();
    let content_advice = by_domain
        .keys()
        .filter_map(|capability| {
            let state = content.get_state(capability)?;
            Some(OperatorContentAdvice {
                capability: capability.clone(),
                measured_similarity: state.measured_similarity,
                adaptation_pressure: state.adaptation_pressure,
                evidence_sha256: state.evidence_sha256.clone(),
            })
        })
        .collect::<Vec<_>>();

    let mut coevolution = Vec::new();
    if discovery_reports.is_empty() {
        if coevolution_loop.get_history().is_empty() {
            notes.push(
                "Coevolución: sin ciclo de discovery persistido. BidirectionalLoop no inventa historia."
                    .into(),
            );
        } else {
            notes.push(
                "Coevolución: sin ciclos nuevos; se reutiliza historia durable de BidirectionalLoop."
                    .into(),
            );
        }
    } else {
        for timed in discovery_reports {
            let report_key = coevolution_report_key(&timed.value)?;
            if applied_coevolution_keys.contains(&report_key) {
                continue;
            }
            match serde_json::from_value::<
                crate::cross_model::plasticity_engine::DiscoveryCycleReport,
            >(timed.value)
            {
                Ok(report) => {
                    let cycle_models = report
                        .evaluations
                        .iter()
                        .map(|evaluation| evaluation.model.clone())
                        .collect::<BTreeSet<_>>();
                    // Fail-closed causal seal: only interventions with
                    // submitted_unix_ns ≤ cycle seal time whose target_model
                    // appears in this cycle. Never reuse the global vector.
                    let mut causal = interventions
                        .iter()
                        .filter(|row| {
                            crate::cross_model::co_evolution::intervention_causally_allowed_for_cycle(
                                timed.submitted_unix_ns,
                                &cycle_models,
                                row.submitted_unix_ns,
                                &row.evidence.target_model,
                            )
                        })
                        .map(|row| row.evidence.clone())
                        .collect::<Vec<_>>();
                    causal.sort_by(|left, right| {
                        left.receipt_sha256
                            .cmp(&right.receipt_sha256)
                            .then_with(|| left.capability_name.cmp(&right.capability_name))
                            .then_with(|| left.target_model.cmp(&right.target_model))
                    });
                    match coevolution_loop.record_cycle(&report, &causal) {
                        Ok(_step) => {
                            applied_coevolution_keys.insert(report_key);
                            notes.push(format!(
                                "Coevolución sellada job {} @{}ns con {} intervención(es) causal(es).",
                                &timed.job_id[..12.min(timed.job_id.len())],
                                timed.submitted_unix_ns,
                                causal.len()
                            ));
                        }
                        Err(error) => notes.push(format!("Coevolución ignorada: {error}")),
                    }
                }
                Err(error) => notes.push(format!("Coevolución: discovery inválido: {error}")),
            }
        }
    }
    for step in coevolution_loop.get_history() {
        coevolution.push(serde_json::to_value(step)?);
    }

    let mut coevolution_directive_value = None;
    match coevolution_loop.plan_next_tick() {
        Ok(Some(directive)) => {
            notes.push(format!(
                "Loop tick: {} — {}",
                directive.recommended_operation, directive.reason
            ));
            // Apply freshly planned tick into durable controllers in this same
            // projection so the next advice/cycle consumes the bias.
            let tick_key = directive.evidence_sha256.clone();
            if !applied_loop_tick_keys.contains(&tick_key) {
                if let (Some(preferred), true) =
                    (directive.source_model.clone(), directive.routing_correlation > 0.0)
                {
                    let scope = format!("benchmark:{}", directive.benchmark_id);
                    let mut matrix = routing.export_matrix();
                    match matrix.update_weight(&scope, &preferred, directive.routing_correlation) {
                        Ok(weight) => {
                            if let Err(error) = routing.import_matrix(matrix) {
                                notes.push(format!("Loop tick routing persist ignorado: {error}"));
                            } else {
                                notes.push(format!(
                                    "Loop tick persistido: routing {preferred}@{scope} → {weight:.4}."
                                ));
                            }
                        }
                        Err(error) => notes.push(format!("Loop tick routing ignorado: {error}")),
                    }
                }
                let measurement = coevolution_loop
                    .get_progress()
                    .average_fitness
                    .unwrap_or(0.5);
                match pi.update(directive.pi_setpoint, measurement, 1.0) {
                    Ok(output) => {
                        pi_snapshot = Some(OperatorPiAdvice {
                            setpoint: directive.pi_setpoint,
                            measurement,
                            output,
                            updates: pi.get_state().updates,
                        });
                        notes.push(format!(
                            "Loop tick persistido: PI → output {output:.4} (setpoint {:.4}).",
                            directive.pi_setpoint
                        ));
                    }
                    Err(error) => notes.push(format!("Loop tick PI ignorado: {error}")),
                }
                applied_loop_tick_keys.insert(tick_key);
            }
            coevolution_directive_value = Some(serde_json::to_value(&directive)?);
            pending_coevolution_directive = Some(directive);
        }
        Ok(None) => {
            pending_coevolution_directive = None;
            notes.push("Loop tick: sin historia sellada; no hay directiva.".into());
        }
        Err(error) => notes.push(format!("Loop tick ignorado: {error}")),
    }

    notes.push(
        "ConsensusBuilder no se ejecuta: no hay votos de política explícitos y no se inventa quórum."
            .into(),
    );
    notes.push(
        "PlasticityEngine/daemon ≠ controladores numéricos: advice acumula BCM/ELO/PI advisory; el engine solo produce evidencia. BidirectionalLoop cierra el bucle advisory sobre routing/PI y propone el siguiente job."
            .into(),
    );

    let elo_entities = elo
        .get_leaderboard()
        .into_iter()
        .filter_map(|(entity, rating)| {
            let state = elo.get_state(&entity)?;
            if state.comparisons == 0 {
                return None;
            }
            Some(OperatorEloEntityAdvice {
                entity,
                rating,
                comparisons: state.comparisons,
                last_evidence_sha256: state.last_evidence_sha256.clone(),
                last_update: state.last_update.clone(),
            })
        })
        .collect::<Vec<_>>();
    let elo_leaderboard = elo_entities
        .iter()
        .map(|item| (item.entity.clone(), item.rating))
        .collect();
    let available = !elo_entities.is_empty() || !routing_decisions.is_empty();

    let durable = seal_operator_plasticity_controller_state(OperatorPlasticityControllerState {
        schema: OPERATOR_PLASTICITY_CONTROLLER_STATE_SCHEMA.into(),
        config_sha256,
        evidence_sha256: Sha256Digest::zero().to_string(),
        elo: elo.export_ratings().into_iter().collect(),
        bcm: bcm.export_states().into_iter().collect(),
        eligibility: eligibility.export_traces().into_iter().collect(),
        neuromodulation_levels: neuromodulation.export_levels().into_iter().collect(),
        pi: pi.get_state().clone(),
        content: content.export_states().into_iter().collect(),
        content_matrix: content.export_matrix(),
        routing_history: routing.export_history().into_iter().collect(),
        routing_matrix: routing.export_matrix(),
        applied_observation_keys,
        applied_elo_pair_keys,
        applied_routing_keys,
        coevolution_history: coevolution_loop.export_history(),
        applied_coevolution_keys,
        coevolution_directive: pending_coevolution_directive,
        applied_loop_tick_keys,
    })?;
    persist_operator_plasticity_controller_state(tidex_home, &durable)?;

    Ok(OperatorPlasticityAdvice {
        schema: "tidex.operator_plasticity_advice/v3".into(),
        available,
        source_jobs,
        elo_leaderboard,
        elo_entities,
        routing_decisions,
        bcm: bcm_advice,
        eligibility: eligibility_advice,
        neuromodulation: neuromodulation_snapshot,
        pi_controller: pi_snapshot,
        content: content_advice,
        coevolution,
        coevolution_directive: coevolution_directive_value,
        notes,
    })
}

#[cfg(not(feature = "cross-model-plasticity"))]
pub fn compute_operator_plasticity_advice(
    _tidex_home: &Path,
) -> BrainResult<OperatorPlasticityAdvice> {
    Ok(empty_plasticity_advice(vec!["cross-model-plasticity feature disabled".into()]))
}

const OPERATOR_LIVING_STAIRCASE_SCHEMA: &str = "tidex.operator_living_staircase/v1";
const OPERATOR_LIVING_STAIRCASE_DOMAIN: &[u8] = b"TIDEX:OPERATOR-LIVING-STAIRCASE:v1\0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatorExecutedAdvancedSystem {
    pub operation: String,
    pub executor_id: Option<String>,
    pub job_id: Sha256Digest,
    pub succeeded: bool,
    pub evidence_sha256: Option<Sha256Digest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatorDiscoveryBinding {
    pub present: bool,
    pub benchmark_id: Option<String>,
    pub evaluation_count: usize,
    pub gap_count: usize,
    pub all_scores_zero: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatorLivingStaircaseReceipt {
    pub schema: String,
    /// KnowledgeEngine::living_staircase is a distinct CLI authority. Operator
    /// jobs never mint knowledge obligations, so this surface stays false
    /// unless a caller already authenticated a knowledge state elsewhere.
    pub knowledge_live: bool,
    pub knowledge_note: String,
    pub plasticity: OperatorPlasticityAdvice,
    pub graph_passed: bool,
    pub graph_finding_count: usize,
    pub discovery: OperatorDiscoveryBinding,
    pub executed_advanced: Vec<OperatorExecutedAdvancedSystem>,
    pub authorizes_production: bool,
    pub staircase_sha256: Sha256Digest,
}

fn empty_discovery_binding() -> OperatorDiscoveryBinding {
    OperatorDiscoveryBinding {
        present: false,
        benchmark_id: None,
        evaluation_count: 0,
        gap_count: 0,
        all_scores_zero: false,
    }
}

fn discovery_binding_from_value(value: &serde_json::Value) -> OperatorDiscoveryBinding {
    if value.get("schema").and_then(serde_json::Value::as_str)
        != Some("tidex.cross_model.discovery_cycle/v1")
    {
        return empty_discovery_binding();
    }
    let evaluations = value
        .get("evaluations")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let gaps = value
        .get("gaps")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let scores = evaluations
        .iter()
        .filter_map(|row| {
            row.get("weighted_score")
                .and_then(serde_json::Value::as_f64)
        })
        .collect::<Vec<_>>();
    let all_scores_zero = !scores.is_empty() && scores.iter().all(|score| *score == 0.0);
    OperatorDiscoveryBinding {
        present: true,
        benchmark_id: value
            .get("benchmark_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        evaluation_count: evaluations.len(),
        gap_count: gaps,
        all_scores_zero,
    }
}

/// Compose the live operator view of the living staircase from authorities
/// that already executed. This does not plan, ingest evidence into
/// KnowledgeEngine, or authorize production.
pub fn compute_operator_living_staircase(
    tidex_home: &Path,
) -> BrainResult<OperatorLivingStaircaseReceipt> {
    let plasticity = compute_operator_plasticity_advice(tidex_home)?;
    let graph = compute_operator_graph()?;
    let mut executed_advanced = Vec::new();
    let mut discovery = empty_discovery_binding();
    for mut record in list_all_job_records(tidex_home)? {
        if record.state != OperatorJobState::Completed {
            continue;
        }
        hydrate_job_run(&mut record)?;
        let executor_id = record
            .evidence_receipt
            .as_ref()
            .and_then(|receipt| receipt.executor_id.clone());
        let evidence_sha256 = record
            .evidence_receipt
            .as_ref()
            .map(|receipt| receipt.evidence_sha256.clone());
        let succeeded = record
            .evidence_receipt
            .as_ref()
            .map(|receipt| receipt.succeeded)
            .unwrap_or(false);
        if record.operation != "probe_runtime" {
            executed_advanced.push(OperatorExecutedAdvancedSystem {
                operation: record.operation.clone(),
                executor_id,
                job_id: record.job_id.clone(),
                succeeded,
                evidence_sha256,
            });
        }
        if !discovery.present {
            if let Some(run) = record.run.as_ref() {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&run.stdout) {
                    let candidate = discovery_binding_from_value(&value);
                    if candidate.present {
                        discovery = candidate;
                    }
                }
            }
        }
    }
    let mut receipt = OperatorLivingStaircaseReceipt {
        schema: OPERATOR_LIVING_STAIRCASE_SCHEMA.into(),
        knowledge_live: false,
        knowledge_note: "KnowledgeEngine::living_staircase requires tidex knowledge staircase over an authenticated knowledge state. Operator jobs do not mint knowledge obligations.".into(),
        plasticity,
        graph_passed: graph.passed,
        graph_finding_count: graph.findings.len(),
        discovery,
        executed_advanced,
        authorizes_production: false,
        staircase_sha256: Sha256Digest::zero(),
    };
    let mut unsigned = receipt.clone();
    unsigned.staircase_sha256 = Sha256Digest::zero();
    receipt.staircase_sha256 = Sha256Digest::digest_domain(
        OPERATOR_LIVING_STAIRCASE_DOMAIN,
        &serde_json::to_vec(&unsigned)?,
    );
    Ok(receipt)
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
    format: OperatorDatasetFormat,
    content: String,
    #[serde(default)]
    generated: bool,
}

fn web_operator_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web-console")
}

fn web_operator_asset(path: &str) -> BrainResult<(&'static str, Vec<u8>)> {
    let name = match path {
        "/" | "/index.html" => "index.html",
        "/app.js" => "app.js",
        "/styles.css" => "styles.css",
        _ => return Err(BrainError::Invalid("web_operator_asset_unknown".into())),
    };
    let content_type = match name {
        "index.html" => "text/html; charset=utf-8",
        "app.js" => "application/javascript; charset=utf-8",
        "styles.css" => "text/css; charset=utf-8",
        _ => "application/octet-stream",
    };
    let file = web_operator_dir().join(name);
    let meta = fs::symlink_metadata(&file)
        .map_err(|_| BrainError::Invalid("web_operator_asset_missing".into()))?;
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > 2 * 1024 * 1024 {
        return Err(BrainError::Invalid("web_operator_asset_invalid".into()));
    }
    let bytes =
        fs::read(&file).map_err(|_| BrainError::Invalid("web_operator_asset_missing".into()))?;
    Ok((content_type, bytes))
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

struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn header_ci<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers.iter().find_map(|(key, value)| {
        if key.eq_ignore_ascii_case(name) {
            Some(value.as_str())
        } else {
            None
        }
    })
}

fn loopback_http_host(value: &str) -> bool {
    let value = value.trim();
    if let Ok(addr) = value.parse::<SocketAddr>() {
        return addr.ip().is_loopback();
    }
    let unbracketed = value
        .strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'))
        .unwrap_or(value);
    if let Ok(ip) = unbracketed.parse::<IpAddr>() {
        return ip.is_loopback();
    }
    let hostname = value
        .rsplit_once(':')
        .filter(|(host, port)| {
            !host.is_empty() && !host.contains(']') && port.bytes().all(|b| b.is_ascii_digit())
        })
        .map(|(host, _)| host)
        .unwrap_or(value);
    hostname.eq_ignore_ascii_case("localhost")
}

fn loopback_origin(origin: &str) -> bool {
    let origin = origin.trim();
    if origin.is_empty() || origin.eq_ignore_ascii_case("null") {
        return false;
    }
    let rest = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"));
    let Some(rest) = rest else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or(rest);
    loopback_http_host(host)
}

fn enforce_http_access(request: &HttpRequest) -> BrainResult<()> {
    let host = header_ci(&request.headers, "Host")
        .ok_or_else(|| BrainError::Invalid("operator_http_host_required".into()))?;
    if !loopback_http_host(host) {
        return Err(BrainError::Invalid("operator_http_host_not_loopback".into()));
    }
    if request.method == "POST" {
        let content_type = header_ci(&request.headers, "Content-Type").unwrap_or("");
        let media = content_type.split(';').next().unwrap_or("").trim();
        if !media.eq_ignore_ascii_case("application/json") {
            return Err(BrainError::Invalid("operator_http_json_content_type_required".into()));
        }
        if let Some(origin) = header_ci(&request.headers, "Origin") {
            if !loopback_origin(origin) {
                return Err(BrainError::Invalid("operator_http_origin_rejected".into()));
            }
        }
    }
    Ok(())
}

fn parse_headers(header_block: &str) -> BrainResult<Vec<(String, String)>> {
    let mut headers = Vec::new();
    for line in header_block.lines().skip(1) {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(BrainError::Invalid("operator_http_header_invalid".into()));
        };
        headers.push((name.trim().to_string(), value.trim().to_string()));
    }
    Ok(headers)
}

fn parse_request(stream: &mut TcpStream) -> BrainResult<HttpRequest> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(HTTP_WRITE_TIMEOUT))?;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() > MAX_HTTP_BODY_BYTES + 64 * 1024 {
            return Err(BrainError::Invalid("operator_http_request_too_large".into()));
        }
        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let header_end = header_end + 4;
            let header = std::str::from_utf8(&bytes[..header_end])
                .map_err(|_| BrainError::Invalid("operator_http_header_invalid".into()))?;
            let headers = parse_headers(header)?;
            if header_ci(&headers, "Transfer-Encoding").is_some() {
                return Err(BrainError::Invalid(
                    "operator_http_transfer_encoding_unsupported".into(),
                ));
            }
            let content_length = header_ci(&headers, "Content-Length")
                .unwrap_or("0")
                .parse::<usize>()
                .map_err(|_| BrainError::Invalid("operator_http_content_length_invalid".into()))?;
            if content_length > MAX_HTTP_BODY_BYTES {
                return Err(BrainError::Invalid("operator_http_body_too_large".into()));
            }
            if bytes.len() >= header_end + content_length {
                break;
            }
        }
    }
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| BrainError::Invalid("operator_http_header_incomplete".into()))?
        + 4;
    let header = std::str::from_utf8(&bytes[..header_end])
        .map_err(|_| BrainError::Invalid("operator_http_header_invalid".into()))?;
    let mut first = header.lines().next().unwrap_or_default().split_whitespace();
    let method = first.next().unwrap_or_default().to_string();
    let path = first.next().unwrap_or_default().to_string();
    let version = first.next();
    if version != Some("HTTP/1.1") || first.next().is_some() {
        return Err(BrainError::Invalid("operator_http_request_line_invalid".into()));
    }
    if path.contains('\0') || path.contains('?') || path.contains('#') {
        return Err(BrainError::Invalid("operator_http_path_invalid".into()));
    }
    let headers = parse_headers(header)?;
    Ok(HttpRequest {
        method,
        path,
        headers,
        body: bytes[header_end..].to_vec(),
    })
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
    let request = parse_request(stream)?;
    enforce_http_access(&request)?;
    let method = request.method;
    let path = request.path;
    let body = request.body;
    let response = match (method.as_str(), path.as_str()) {
        ("GET", "/") | ("GET", "/index.html") | ("GET", "/app.js") | ("GET", "/styles.css") => {
            match web_operator_asset(path.as_str()) {
                Ok((content_type, body)) => http_response("200 OK", content_type, &body),
                Err(_) => http_response(
                    "404 Not Found",
                    "application/json; charset=utf-8",
                    br#"{"error":"web_operator_asset_missing"}"#,
                ),
            }
        }
        ("GET", "/operator") => {
            http_response("200 OK", "text/html; charset=utf-8", OPERATOR_HTML.as_bytes())
        }
        ("GET", "/api/info") => json_response(&operator_info(tidex_home)?)?,
        ("GET", "/api/recipes") => json_response(&recipe_catalog())?,
        ("GET", "/api/graph") => json_response(&compute_operator_graph()?)?,
        ("GET", "/api/staircase") => {
            json_response(&compute_operator_living_staircase(tidex_home)?)?
        }
        ("GET", "/api/executors") => json_response(&executor_catalog()?)?,
        ("GET", "/api/models") => json_response(&list_catalog_models(tidex_home)?)?,
        ("GET", "/api/model-profiles") => json_response(&list_runtime_profiles(tidex_home)?)?,
        ("GET", "/api/model-runtime-statuses") => {
            json_response(&list_runtime_statuses(tidex_home)?)?
        }
        ("GET", "/api/datasets") => json_response(&list_datasets(tidex_home)?)?,
        ("GET", "/api/jobs") => json_response(&list_job_records(tidex_home)?)?,
        ("GET", "/api/plasticity") => {
            json_response(&compute_operator_plasticity_advice(tidex_home)?)?
        }
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
            let request: OperatorRunRequest = serde_json::from_slice(&body)?;
            json_response(&operator_run_view(execute_operator_run(tidex_home, &request)?)?)?
        }
        ("POST", "/api/workflows/behavioral-discovery") => {
            let request: BehavioralDiscoveryWorkflowRequest = serde_json::from_slice(&body)?;
            json_response_status(
                "202 Accepted",
                &start_operator_job(tidex_home, OperatorJobRequest::BehavioralDiscovery(request))?,
            )?
        }
        ("POST", "/api/workflows/direct") => {
            let request: OperatorDirectWorkflowRequest = serde_json::from_slice(&body)?;
            json_response_status(
                "202 Accepted",
                &start_operator_job(tidex_home, OperatorJobRequest::Direct(request))?,
            )?
        }
        ("POST", dynamic_path)
            if dynamic_path.starts_with("/api/jobs/") && dynamic_path.ends_with("/cancel") =>
        {
            let raw = dynamic_path
                .trim_start_matches("/api/jobs/")
                .trim_end_matches("/cancel");
            let job_id = Sha256Digest::parse(raw)
                .map_err(|_| BrainError::Invalid("operator_job_id_invalid".into()))?;
            json_response(&cancel_operator_job(tidex_home, &job_id)?)?
        }
        ("GET", dynamic_path) if dynamic_path.starts_with("/api/jobs/") => {
            let raw = dynamic_path.trim_start_matches("/api/jobs/");
            let job_id = Sha256Digest::parse(raw)
                .map_err(|_| BrainError::Invalid("operator_job_id_invalid".into()))?;
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

/// Serve the TIDE-X interface. Binding to non-loopback addresses is rejected.
pub fn serve(tidex_home: &Path, address: SocketAddr) -> BrainResult<()> {
    if !matches!(address.ip(), IpAddr::V4(ip) if ip.is_loopback())
        && !matches!(address.ip(), IpAddr::V6(ip) if ip.is_loopback())
    {
        return Err(BrainError::Invalid("operator_server_requires_loopback".into()));
    }
    ensure_private_dir(&tidex_home.join("operator"))?;
    let recovered = recover_incomplete_operator_jobs(tidex_home)?;
    if recovered > 0 {
        eprintln!("TIDE-X recovered {recovered} incomplete/inconsistent jobs");
    }
    if let Ok(hub) = default_hf_hub_root() {
        if hub.is_dir() {
            match catalog_local_models(tidex_home, &hub) {
                Ok(models) if !models.is_empty() => {
                    eprintln!("TIDE-X cataloged {} local model(s)", models.len());
                }
                Ok(_) => {}
                Err(error) => eprintln!("TIDE-X model catalog scan skipped: {error}"),
            }
        }
    }
    let listener = TcpListener::bind(address)?;
    eprintln!("TIDE-X: http://{address}");
    for incoming in listener.incoming() {
        let mut stream = incoming?;
        if !stream.peer_addr()?.ip().is_loopback() {
            continue;
        }
        let _ = stream.set_write_timeout(Some(HTTP_WRITE_TIMEOUT));
        if !try_acquire(&OPERATOR_HTTP_INFLIGHT, MAX_HTTP_CONNECTIONS) {
            let _ = stream.write_all(&http_response(
                "503 Service Unavailable",
                "application/json; charset=utf-8",
                br#"{"error":"operator_http_concurrency_limit"}"#,
            ));
            continue;
        }
        let home = tidex_home.to_path_buf();
        if std::thread::Builder::new()
            .name("tidex-operator-http".into())
            .spawn(move || {
                if let Err(error) = handle_http(&home, &mut stream) {
                    let body = serde_json::to_vec(&serde_json::json!({"error": error.to_string()}))
                        .unwrap_or_else(|_| br#"{"error":"operator_http_error"}"#.to_vec());
                    let _ = stream.write_all(&http_response(
                        "400 Bad Request",
                        "application/json; charset=utf-8",
                        &body,
                    ));
                }
                OPERATOR_HTTP_INFLIGHT.fetch_sub(1, Ordering::SeqCst);
            })
            .is_err()
        {
            OPERATOR_HTTP_INFLIGHT.fetch_sub(1, Ordering::SeqCst);
        }
    }
    Ok(())
}

fn try_acquire(counter: &AtomicUsize, max: usize) -> bool {
    loop {
        let current = counter.load(Ordering::SeqCst);
        if current >= max {
            return false;
        }
        if counter
            .compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return true;
        }
    }
}

const OPERATOR_HTML: &str = r#"<!doctype html>
<html lang="es"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>TIDE-X Control de ejecución</title>
<style>
:root{color-scheme:dark;font-family:Inter,ui-sans-serif,system-ui;background:#071019;color:#eaf2f8}*{box-sizing:border-box}body{margin:0;background:linear-gradient(180deg,#071019,#0a121c 45%,#071019)}header{position:sticky;top:0;z-index:5;display:flex;justify-content:space-between;align-items:center;padding:18px 28px;background:#08111beF;border-bottom:1px solid #213141;backdrop-filter:blur(12px)}h1{font-size:20px;margin:0}.sub{color:#91a4b6;font-size:13px;margin-top:4px}.status{font:12px ui-monospace,monospace;color:#7de2a2}.shell{max-width:1500px;margin:auto;padding:22px}.steps{display:grid;grid-template-columns:1fr 1.35fr 1fr;gap:18px}.panel{background:#0c1722;border:1px solid #213141;border-radius:14px;padding:18px;box-shadow:0 10px 28px #0004}.panel h2{font-size:15px;margin:0 0 12px}.panel h3{font-size:13px;margin:18px 0 8px;color:#adc2d3}.muted{color:#8398aa;font-size:12px}.model{display:flex;gap:10px;padding:10px;border:1px solid #23384a;border-radius:10px;margin:7px 0;background:#0a141e}.model input{width:auto}.model small{display:block;color:#8197a9;overflow-wrap:anywhere}.badge{display:inline-block;padding:2px 7px;border:1px solid #38536b;border-radius:999px;font-size:10px;color:#a9c7db}.ok{border-color:#2f7049;color:#7ee19e}.warn{border-color:#765b2c;color:#e7bf6d}button,input,select,textarea{width:100%;border-radius:9px;border:1px solid #2b4052;background:#07111a;color:#eaf2f8;padding:10px;margin-top:7px}button{background:#12304a;font-weight:700;cursor:pointer}button:hover{background:#17405f}button.primary{background:#17613e}button.primary:hover{background:#1b7650}button:disabled{opacity:.45;cursor:not-allowed}textarea{min-height:95px;resize:vertical;font-family:ui-monospace,monospace;font-size:12px}.row{display:grid;grid-template-columns:1fr 1fr;gap:8px}.work{padding:12px;border:1px solid #2c4152;border-radius:10px;background:#0a141e}.result{margin-top:18px}.result pre{white-space:pre-wrap;word-break:break-word;max-height:520px;overflow:auto;background:#050b11;border:1px solid #1d2e3d;border-radius:10px;padding:14px;font-size:12px}.tabs{display:flex;gap:6px;margin-bottom:12px}.tabs button{width:auto;margin:0;padding:7px 11px}.hidden{display:none}.recipe-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(230px,1fr));gap:8px}.recipe{padding:10px;border:1px solid #273c4e;border-radius:9px}.recipe b{display:block;margin-bottom:5px}.metric-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:9px;margin:10px 0}.metric{border:1px solid #2a4051;border-radius:10px;padding:11px;background:#09131c}.metric .v{font-size:20px;font-weight:800;margin-top:4px}.metric .k{font-size:10px;color:#8197a9;text-transform:uppercase;letter-spacing:.06em}.scorebar{height:7px;background:#152533;border-radius:999px;overflow:hidden;margin-top:7px}.scorebar i{display:block;height:100%;background:#36a76a}.obs{border-top:1px solid #1f3344;padding:8px 0}.history{display:grid;gap:7px}.history-item{border:1px solid #293f50;border-radius:9px;padding:10px;cursor:pointer;background:#09131c}.history-item:hover{background:#0d1b27}.history-item .state{float:right}.good{color:#75dfa0}.bad{color:#ff8d8d}.running{color:#f2c46d}.raw-toggle{margin-top:10px}.raw-toggle summary{cursor:pointer;color:#8fa9bd}.section-title{font-size:13px;font-weight:800;margin:16px 0 6px}.model-order{font-size:11px;color:#78bce7;margin:4px 0}.result-card{border:1px solid #294052;border-radius:10px;padding:12px;margin:8px 0;background:#09131c}.modebar{display:flex;gap:8px;margin:0 0 14px}.modebar button{width:auto;margin:0}.modebar button.active{background:#17613e}.goal-grid{display:grid;grid-template-columns:1.4fr .8fr auto;gap:8px;align-items:end}.pipeline{display:grid;gap:8px;margin-top:12px}.pipeline-step{border:1px solid #294052;border-radius:10px;padding:11px;background:#09131c}.pipeline-step .head{display:flex;justify-content:space-between;gap:8px;align-items:center}.pipeline-step .why{color:#8197a9;font-size:11px;margin-top:5px}.pipeline-step.available{border-color:#2f7049}.pipeline-step.manual{border-color:#765b2c}.pipeline-step.blocked{border-color:#63363a}.area-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(230px,1fr));gap:9px}.area-card{border:1px solid #294052;border-radius:10px;padding:12px;background:#09131c;cursor:pointer}.area-card:hover{background:#0d1b27}.area-card b{display:block;margin-bottom:6px}.executor-row{border-top:1px solid #1f3344;padding:8px 0}.executor-row:first-child{border-top:0}.plasticity-tabs{display:flex;gap:6px;flex-wrap:wrap;margin-bottom:10px}.plasticity-tabs button{width:auto;margin:0}.plasticity-pane.hidden{display:none}.chip{display:inline-block;padding:2px 7px;border-radius:999px;border:1px solid #38536b;font-size:10px;margin:2px 3px 2px 0}.chip.op{border-color:#2f7049;color:#7ee19e}.chip.need{border-color:#765b2c;color:#e7bf6d}.chip.exp{border-color:#38536b;color:#78bce7}.chip.arch{border-color:#63363a;color:#ff9a9a}@media(max-width:1050px){.steps{grid-template-columns:1fr}.shell{padding:10px}.goal-grid{grid-template-columns:1fr}}
</style></head>
<body><header><div><h1 data-i18n="operatorTitle">TIDE-X</h1><div class="sub" data-i18n="operatorSubtitle">Modelos → objetivos → flujo → ejecución real → evidencia</div></div><div style="display:flex;align-items:center;gap:10px"><button id="langToggle" style="width:auto;margin:0;padding:6px 10px;font-size:12px" onclick="toggleLanguage()">English</button><div id="status" class="status">inicializando…</div></div></header>
<div class="shell">
<div class="modebar"><button id="systemModeBtn" class="active" onclick="setOperatorMode('system')">Operación real · orquestación</button><button id="manualModeBtn" onclick="setOperatorMode('manual')">Ejecución manual · catálogo</button></div>
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
<div class="panel"><h2>Autoridades y áreas del runtime</h2><div class="muted">Cada bloque agrupa contratos del catálogo. El grafo de producción es la verdad topológica: si hay recetas sin executor o artefactos sin productor, aparece aquí. No se afirma cierre mientras el grafo no pase. La escalera viva es la vista conjunta de KnowledgeEngine, plasticidad y jobs reales; no autoriza producción.</div><div id="graphStatus" class="muted" style="margin-top:8px">El grafo de producción aún no se ha cargado.</div><div id="staircaseStatus" class="muted" style="margin-top:8px">La escalera viva aún no se ha cargado.</div><div id="areaGrid" class="area-grid" style="margin-top:12px"></div><div id="areaDetail" style="margin-top:12px"></div></div>
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
<section class="panel"><h2 data-i18n-key="datasetTitle">3 · Artefactos y datasets</h2><div class="muted" data-i18n-key="datasetHelp">Para benchmarks usa el schema <code>tidex.cross_model.behavioral_benchmark/v1</code>. Los datasets generados quedan marcados como artefactos no independientes.</div><input id="datasetFile" type="file"><div class="row"><input id="dsName" placeholder="nombre del dataset"><select id="dsGenerated"><option value="false">externo / independiente</option><option value="true">generado / no independiente</option></select></div><button onclick="importFileDataset()">Importar artefacto</button><h3>Crear benchmark</h3><input id="benchId" placeholder="benchmark id"><input id="benchDomain" placeholder="dominio / capacidad"><textarea id="benchRows" placeholder="Una prueba por línea: prompt => respuesta esperada"></textarea><button onclick="createBenchmark()">Crear benchmark exacto</button><h3>Datasets disponibles</h3><div id="datasets"></div></section>
</div>
</section>
<section class="panel result"><div class="tabs"><button onclick="showTab('result')">Resultado · evidencia</button><button onclick="showTab('history')">Historial · jobs</button><button onclick="showTab('executors')">Ejecutores · contratos</button><button onclick="showTab('plasticity')">Plasticidad · aprendizaje</button><button onclick="showTab('advanced')">Operaciones avanzadas · autorías</button></div><div id="resultTab"><div id="summary"><p class="muted">Selecciona modelos, workflow y artefactos. La salida representa la evidencia del trabajo real, no una promesa de producción.</p></div><details class="raw-toggle"><summary>JSON / evidencia cruda</summary><pre id="output">Sin ejecución.</pre></details></div><div id="historyTab" class="hidden"><div id="history" class="history"></div></div><div id="executorsTab" class="hidden"><div id="executors" class="history"></div></div><div id="plasticityTab" class="hidden"><div id="plasticity" class="history"></div></div><div id="advancedTab" class="hidden"><div class="muted">Autoridades canónicas del runtime: cada receta mantiene su propósito, su contrato y su ámbito de producción. Para casos normales usa el pipeline principal y su evidencia.</div><textarea id="assets" placeholder="Una ruta absoluta por línea para recetas de runtime"></textarea><div id="recipes" class="recipe-grid"></div></div></section>
</div>
<script>
let state={info:null,models:[],profiles:[],runtimeStatuses:[],datasets:[],executors:[],recipes:[],jobs:[],plasticity:null,graph:null,staircase:null,selectedDataset:null,generatedBenchmark:null,selectedModelOrder:[],activeJob:null,operatorMode:'system',goalPipeline:[]};
const $=id=>document.getElementById(id);
const esc=v=>String(v??'').replace(/[&<>\"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','\"':'&quot;',"'":'&#39;'}[c]));
const pct=v=>`${(Number(v||0)*100).toFixed(1)}%`;
const translations={
  es:{
    operatorTitle:'TIDE-X',operatorSubtitle:'Modelos → objetivos → workflow real → evidencia y gobernanza',langButton:'English',ui:{modelsTitle:'1 · Modelos',modelsHelp:'Selecciona uno para análisis o dos o más para comparación, alineación o transferencia real.',modelRootPlaceholder:'directorio de modelos',scan:'Escanear',workTitle:'2 · Workflow / ejecución',runWork:'Ejecutar workflow',cancelJob:'Cancelar job activo',datasetTitle:'3 · Artefactos y datasets',datasetHelp:'Para benchmarks usa el schema tidex.cross_model.behavioral_benchmark/v1. Los datasets generados quedan marcados como artefactos no independientes.'},
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
      'operator.direct_runner':['Ejecutor directo del runtime TIDE-X','Ejecuta una solicitud tipada del runtime cross-model usando el backend real y los componentes de análisis canónicos.'],
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
  en:{operatorTitle:'TIDE-X Control',operatorSubtitle:'Models → goals → workflow → real execution → evidence and governance',langButton:'Español',ui:{modelsTitle:'1 · Models',modelsHelp:'Select one for analysis or two or more for real comparison, alignment or transfer.',modelRootPlaceholder:'models directory',scan:'Scan',workTitle:'2 · Workflow / execution',runWork:'Run workflow',cancelJob:'Cancel active job',datasetTitle:'3 · Artifacts and datasets',datasetHelp:'For benchmarks use schema tidex.cross_model.behavioral_benchmark/v1. Generated datasets are marked as non-independent artifacts.'},states:{queued:'queued',running:'running',completed:'completed',failed:'failed',cancelled:'cancelled'},categories:{acquisition:'Acquisition',discovery:'Discovery',learning:'Learning',benchmark:'Benchmark',receiver:'Receiver',materialization:'Materialization',evaluation:'Evaluation',governance:'Governance',lifecycle:'Lifecycle'},operations:{behavioral_discovery:'behavioral discovery',probe_runtime:'runtime probe',behavioral_evaluation:'behavioral evaluation',extract_capability:'capability extraction',deep_instrumentation:'deep instrumentation',sparse_autoencoder_analysis:'SAE analysis',counterfactual_analysis:'counterfactual analysis',calibrate_alignment:'alignment calibration',activation_transfer_experiment:'activation transfer experiment',generate_behavioral_dataset:'behavioral dataset generation'},recipes:{}}
};
let currentLanguage=localStorage.getItem('tidex.operator.language')==='en'?'en':'es';
const trState=v=>translations[currentLanguage].states[v]||v;
const trCategory=v=>translations[currentLanguage].categories[String(v).toLowerCase()]||v;
const trOperation=v=>translations[currentLanguage].operations[v]||v;
function trRecipe(x){const es=translations.es.recipes[x.id];return currentLanguage==='es'&&es?{title:es[0],description:es[1]}:{title:x.title,description:x.description}}
function renderStatus(){if(!state.info)return;const es=currentLanguage==='es';const worker=state.info.hf_python||(es?'no encontrado':'not found');const sae=state.info.bound_sparse_dictionary;$('status').textContent=`TIDE-X ${state.info.operator_home} · worker HF ${worker} · NNsight ${state.info.nnsight_available?'OK':'NO'} · SAE dict ${sae?(es?'listo':'ready'):'NO'}`}
function renderGraphStatus(){const box=$('graphStatus');if(!box)return;const g=state.graph;if(!g){box.textContent=currentLanguage==='es'?'El grafo de producción aún no se ha cargado.':'Production graph not loaded yet.';return;}const findings=g.findings||[];if(g.passed){box.textContent=currentLanguage==='es'?'Grafo: cerrado. Recetas, executors y artefactos coinciden.':'Graph: closed. Recipes, executors and artifacts match.';return;}const kinds={};for(const finding of findings)kinds[finding.kind]=(kinds[finding.kind]||0)+1;const summary=Object.entries(kinds).map(([kind,count])=>`${kind} ${count}`).join(' · ');box.innerHTML=`<span class="bad">${currentLanguage==='es'?`Grafo: ${findings.length} hallazgos. No está cerrado.`:`Graph: ${findings.length} findings. Not closed.`}</span> · ${esc(summary)}`}
function renderStaircaseStatus(){const box=$('staircaseStatus');if(!box)return;const s=state.staircase;if(!s){box.textContent=currentLanguage==='es'?'La escalera viva aún no se ha cargado.':'Living staircase not loaded yet.';return;}const d=s.discovery||{};const executed=(s.executed_advanced||[]).length;const p=s.plasticity||{};const knowledge=s.knowledge_live? (currentLanguage==='es'?'KnowledgeEngine vivo':'KnowledgeEngine live'):(currentLanguage==='es'?'KnowledgeEngine espera estado autenticado':'KnowledgeEngine waits authenticated state');const gaps=d.present?(d.all_scores_zero?(currentLanguage==='es'?`discovery real, ${d.evaluation_count} evals, todos 0.0, gaps ${d.gap_count}`:`real discovery, ${d.evaluation_count} evals, all 0.0, gaps ${d.gap_count}`):(currentLanguage==='es'?`discovery real, ${d.evaluation_count} evals, gaps ${d.gap_count}`:`real discovery, ${d.evaluation_count} evals, gaps ${d.gap_count}`)):(currentLanguage==='es'?'sin ciclo de discovery persistido':'no persisted discovery cycle');box.innerHTML=`${esc(knowledge)} · ${currentLanguage==='es'?'plasticidad jobs':'plasticity jobs'} ${Number(p.source_jobs||0)} · ${esc(gaps)} · ${currentLanguage==='es'?'sistemas avanzados ejecutados':'advanced systems executed'} ${executed} · ${s.authorizes_production?'prod':'no prod'}`}
function applyLanguage(){const lang=translations[currentLanguage];document.documentElement.lang=currentLanguage;document.title=lang.operatorTitle;document.querySelector('[data-i18n="operatorTitle"]').textContent=lang.operatorTitle;document.querySelector('[data-i18n="operatorSubtitle"]').textContent=lang.operatorSubtitle;document.querySelectorAll('[data-i18n-key]').forEach(el=>{const value=lang.ui[el.dataset.i18nKey];if(value)el.textContent=value});document.querySelectorAll('[data-i18n-placeholder]').forEach(el=>{const value=lang.ui[el.dataset.i18nPlaceholder];if(value)el.placeholder=value});$('langToggle').textContent=lang.langButton;renderStatus();renderGraphStatus();renderStaircaseStatus();renderModels();renderDatasets();renderExecutors();renderPlasticity();renderAreas();if(state.goalPipeline.length)renderGoalPipeline();renderParams();loadRecipes();loadJobs()}
function toggleLanguage(){currentLanguage=currentLanguage==='es'?'en':'es';localStorage.setItem('tidex.operator.language',currentLanguage);applyLanguage()}
const executorOperationMap={
  'cross_model.probe_runtime':'probe_runtime','cross_model.evaluate':'behavioral_evaluation','cross_model.discovery':'behavioral_discovery_multi','cross_model.extract_steering':'extract_capability','cross_model.nnsight':'deep_instrumentation','cross_model.sae':'sparse_autoencoder_analysis','cross_model.counterfactual':'counterfactual_analysis','cross_model.align':'calibrate_alignment','cross_model.transfer_steering':'activation_transfer_experiment'
};
const operatorAreas=[
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
function workflowLabel(e){return {operator_ready:'Operator',cli_ready:'CLI',engine_ready:'Engine',internal_ready:'Interno',needs_operator_workflow:'workflow pendiente'}[e.workflow_status]||e.workflow_status||'—'}
function executorCanRun(e){return Boolean(executorOperationMap[e.executor_id]||e.operator_recipe_id)}
function executorActionable(e){return e.actionable_now===true}
function executorButtonLabel(e){if(executorCanRun(e))return 'Abrir operación';if(e.workflow_status==='internal_ready')return 'Ver contrato interno';if(e.runtime_status==='advisory_from_evidence')return 'Ver señal advisory';return 'Ver contrato'}
function setOperatorMode(mode){state.operatorMode=mode;$('systemMode').classList.toggle('hidden',mode!=='system');$('manualMode').classList.toggle('hidden',mode!=='manual');$('systemModeBtn').classList.toggle('active',mode==='system');$('manualModeBtn').classList.toggle('active',mode==='manual');if(mode==='manual')renderAreas()}
function renderAreas(){const box=$('areaGrid');if(!box)return;box.innerHTML=operatorAreas.map(area=>{const xs=state.executors.filter(area.match),registered=xs.filter(executorActionable).length,runnable=xs.filter(executorCanRun).length,ui=xs.filter(e=>(e.surfaces||[]).includes('tidex_operator')).length,prod=area.id==='production'&&xs.some(e=>e.production_authority&&e.executor_id==='adapter.bank');return `<div class="area-card" onclick="openArea('${area.id}')"><b>${esc(area.title)}</b><div class="muted">${esc(area.description)}</div><div style="margin-top:8px"><span class="chip op">${registered} accionables</span><span class="chip">${runnable} receta/operación</span><span class="chip">${ui} superficie Operator</span><span class="chip">${xs.length} contratos</span>${prod?'<span class="chip op">autoridad prod adapter.bank</span>':''}</div></div>`}).join('')}
function executorDetail(e){return `<div class="executor-row"><span class="chip ${executorStateClass(e)}">${esc(executorStateLabel(e))}</span><span class="chip">${esc(runtimeLabel(e))}</span><span class="chip">${esc(workflowLabel(e))}</span><span class="chip">${esc(e.evidence_status||'—')}</span>${e.production_authority?'<span class="chip op">autoridad producción</span>':'<span class="chip">sin autoridad prod</span>'}<b>${esc(e.title)}</b><div class="muted">${esc(e.executor_id)} · ${esc(e.module_path)}</div><div class="muted">requiere: ${esc((e.requires||[]).join(', ')||'—')} · produce: ${esc((e.produces||[]).join(', ')||'—')}</div><div class="why">${esc(e.notes||'')}</div>${executorActionable(e)?`<button style="width:auto" onclick="activateExecutor('${e.executor_id}')">${esc(executorButtonLabel(e))}</button>`:''}<details><summary>contrato</summary><pre>${esc(JSON.stringify(e,null,2))}</pre></details></div>`}
function openArea(id){const area=operatorAreas.find(x=>x.id===id);if(!area)return;const xs=state.executors.filter(area.match);$('areaDetail').innerHTML=`<div class="result-card"><b>${esc(area.title)}</b><div class="muted">${esc(area.description)}</div>${xs.map(executorDetail).join('')}</div>`}
function activateExecutor(id){const e=state.executors.find(x=>x.executor_id===id);if(!e)return;setOperatorMode('manual');const op=executorOperationMap[id];if(op){$('operation').value=op;renderParams();$('operation').scrollIntoView({behavior:'smooth',block:'center'});return}if(e.operator_recipe_id){showTab('advanced');const card=document.querySelector(`[data-recipe-id="${CSS.escape(e.operator_recipe_id)}"]`);if(card)card.scrollIntoView({behavior:'smooth',block:'center'});return}const area=operatorAreas.find(a=>a.match(e));openArea(area?.id||'research')}
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
function renderDiscovery(x){const ev=x.evaluations||[],g=x.gaps||[],p=x.proposals||[];const allZero=ev.length>0&&ev.every(e=>Number(e.weighted_score||0)===0);const gapsHtml=g.length?g.map(v=>`<div class="result-card"><pre>${esc(JSON.stringify(v,null,2))}</pre></div>`).join(''):(allZero?'<div class="muted">Todos los modelos puntuaron 0.0. No hay gap diferencial: nadie demostró la capacidad en este benchmark.</div>':'<div class="muted">No hay gap diferencial con este benchmark.</div>');const propsHtml=p.length?p.map(v=>`<div class="result-card"><pre>${esc(JSON.stringify(v,null,2))}</pre></div>`).join(''):(allZero?'<div class="muted">Sin propuestas: no hay evidencia de capacidad que transferir.</div>':'<div class="muted">No se generaron propuestas.</div>');return `<div class="metric-grid">${metric('modelos',ev.length)}${metric('gaps',g.length,g.length?'good':'')}${metric('propuestas',p.length,p.length?'good':'')}${metric('benchmark',x.benchmark_id||'-')}</div><div class="section-title">Modelos</div>${ev.map(e=>`<div class="result-card"><b>${esc(e.model)}</b> · ${pct(e.weighted_score)}<div class="scorebar"><i style="width:${Math.max(0,Math.min(100,Number(e.weighted_score||0)*100))}%"></i></div></div>`).join('')}<div class="section-title">Gaps</div>${gapsHtml}<div class="section-title">Propuestas</div>${propsHtml}`}
function renderTransfer(x){const b=x.baseline?.weighted_score||0,i=x.intervened?.weighted_score||0,r=x.restored?.weighted_score||0;return `<div class="metric-grid">${metric('baseline',pct(b))}${metric('intervenido',pct(i),i>b?'good':i<b?'bad':'')}${metric('restaurado',pct(r))}${metric('delta',Number(x.score_delta||0).toFixed(4),x.score_delta>0?'good':x.score_delta<0?'bad':'')}${metric('restore Δ',Number(x.restore_delta||0).toFixed(4),Math.abs(x.restore_delta||0)<1e-9?'good':'warn')}${metric('mejora observada',x.behavioral_improvement_observed?'SÍ':'NO',x.behavioral_improvement_observed?'good':'')}</div><div class="result-card"><b>${esc(x.source_model)} → ${esc(x.target_model)}</b><div class="muted">capacidad: ${esc(x.capability_name)}</div></div><div class="section-title">Alineamiento</div><div class="metric-grid">${metric('alignment score',pct(x.alignment?.alignment_score))}${metric('residual',Number(x.alignment?.normalized_residual||0).toFixed(6))}${metric('strength',x.intervention?.strength??'-')}</div>`}
function renderRuntimeProbe(x){const a=x.access||{},m=x.model||{};return `<div class="metric-grid">${metric('arquitectura',m.runtime_architecture||'-')}${metric('parámetros',Number(m.parameter_count||0).toLocaleString())}${metric('capas',m.num_layers??'-')}${metric('contexto',m.max_sequence_length??'-')}</div><div class="section-title">Accesos comprobados</div><div class="metric-grid">${Object.entries(a).map(([k,v])=>metric(k,v?'SÍ':'NO',v?'good':'bad')).join('')}</div><div class="result-card"><b>${esc(m.name||'-')}</b><div class="muted">runtime metadata: ${esc(m.runtime_metadata_sha256||'')}</div></div>`}
function renderGeneratedBenchmark(x){state.generatedBenchmark=x.benchmark||null;const probes=x.benchmark?.probes||[];return `<div class="metric-grid">${metric('generator',x.generator_model||'-')}${metric('probes',probes.length)}${metric('dominio',x.benchmark?.domain||'-')}${metric('independiente',x.independent_evidence?'SÍ':'NO',x.independent_evidence?'good':'warn')}</div><div class="result-card"><b>${esc(x.benchmark?.benchmark_id||'benchmark generado')}</b><div class="muted">${esc(x.generation_response_sha256||'')}</div></div><div class="section-title">Probes generados</div>${probes.slice(0,50).map(p=>`<div class="obs"><b>${esc(p.probe_id)}</b><div>${esc(p.prompt)}</div><div class="muted">verifier: ${esc(JSON.stringify(p.verifier))}</div></div>`).join('')}<button class="primary" onclick="saveGeneratedBenchmark()">Guardar benchmark generado</button>`}
async function saveGeneratedBenchmark(){try{if(!state.generatedBenchmark)throw Error('No hay benchmark generado');const name=state.generatedBenchmark.benchmark_id||'generated-benchmark';const v=await jpost('/api/datasets/import',{name,format:'json',content:JSON.stringify(state.generatedBenchmark),generated:true});state.selectedDataset=v.content_sha256;await reload();const radio=document.querySelector(`input[name=ds][value="${v.content_sha256}"]`);if(radio)radio.checked=true;out(v)}catch(e){out(e.message)}}
function renderGeneric(x){if(Array.isArray(x))return `<div class="metric-grid">${metric('resultados',x.length)}</div>${x.slice(0,50).map(v=>`<div class="result-card"><pre>${esc(JSON.stringify(v,null,2))}</pre></div>`).join('')}`;if(x?.steering_vector)return `<div class="metric-grid">${metric('capacidad',x.steering_vector?.metadata?.name||'-')}${metric('confianza',pct(x.steering_vector?.metadata?.confidence))}${metric('capa',x.steering_vector?.metadata?.source_layer??'-')}${metric('componentes',(x.components||[]).length)}</div>`;if(x?.calibration_sha256)return `<div class="metric-grid">${metric('source',x.source_model||'-')}${metric('target',x.target_model||'-')}${metric('source layer',x.source_layer)}${metric('target layer',x.target_layer)}${metric('calibration',String(x.calibration_sha256).slice(0,12)+'…')}</div>`;return `<div class="result-card"><pre>${esc(JSON.stringify(x,null,2))}</pre></div>`}
function renderJob(job){raw(job);const box=$('summary');const stateLabel=currentLanguage==='es'?'estado':'state',operationLabel=currentLanguage==='es'?'operación':'operation';if(job.state==='queued'||job.state==='running'){box.innerHTML=`<div class="metric-grid">${metric(stateLabel,trState(job.state),'running')}${metric(operationLabel,trOperation(job.operation))}${metric('job',job.job_id.slice(0,12)+'…')}</div><div class="muted">${currentLanguage==='es'?'El trabajo sigue ejecutándose. El resultado persistirá aunque cierres esta pestaña.':'The job is still running. Its result will persist even if you close this tab.'}</div>`;return}if(job.state==='failed'){box.innerHTML=`<div class="metric-grid">${metric(stateLabel,trState(job.state),'bad')}${metric(operationLabel,trOperation(job.operation))}</div><div class="result-card bad">${esc(job.error||(currentLanguage==='es'?'error desconocido':'unknown error'))}</div>`;return}const x=parseRunStdout(job);if(!x){box.innerHTML=`<div class="metric-grid">${metric(stateLabel,trState(job.state),'good')}${metric(operationLabel,trOperation(job.operation))}</div><div class="muted">${currentLanguage==='es'?'La ejecución terminó, pero la salida no es JSON estructurado.':'Execution finished, but the output is not structured JSON.'}</div>`;return}if(x.schema==='tidex.cross_model.discovery_cycle/v1')box.innerHTML=renderDiscovery(x);else if(x.schema==='tidex.cross_model.model_evaluation/v1')box.innerHTML=renderEvaluation(x);else if(x.schema==='tidex.operator_activation_transfer/v1')box.innerHTML=renderTransfer(x);else if(x.schema==='tidex.operator_runtime_probe/v1')box.innerHTML=renderRuntimeProbe(x);else if(x.schema==='tidex.operator_generated_benchmark/v1')box.innerHTML=renderGeneratedBenchmark(x);else box.innerHTML=renderGeneric(x)}
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
function renderModels(){const box=$('models');box.innerHTML=state.models.map(m=>{const p=profileFor(m.model_id);const st=runtimeStatusFor(m.model_id);let caps;if(p){caps=`<br>${accessBadge('INF',p.access.behavioral_inference)} ${accessBadge('ACT',p.access.internal_activations)} ${accessBadge('STEER',p.access.activation_intervention)} ${accessBadge('NN',p.access.deep_instrumentation)} ${accessBadge('SAE',p.access.sparse_autoencoder_analysis)}`}else if(st&&st.state!=='missing'){caps=`<br><span class="badge ${st.state==='failed'||st.state==='cancelled'?'warn':'ok'}">runtime ${esc(st.state)}</span>${st.error?` <span class="badge warn" title="${esc(st.error)}">error</span>`:''}`}else{caps='<br><span class="badge warn">runtime sin comprobar</span>'}return `<label class="model"><input class="modelSel" type="checkbox" value="${esc(m.model_id)}" ${state.selectedModelOrder.includes(m.model_id)?'checked':''} onchange="toggleModel('${esc(m.model_id)}',this.checked)"><span><b>${esc(m.architecture||'arquitectura desconocida')}</b> <span class="badge ${m.layout==='hugging_face_single_safetensors'?'ok':'warn'}">${esc(m.layout)}</span>${caps}<small>${esc(m.root)}</small></span></label>`}).join('')||'<p class="muted">No hay modelos catalogados.</p>';state.selectedModelOrder=state.selectedModelOrder.filter(id=>state.models.some(m=>m.model_id===id));updateModelSelection()}
function renderDatasets(){const box=$('datasets');box.innerHTML=state.datasets.map(d=>`<label class="model"><input type="radio" name="ds" value="${esc(d.content_sha256)}" onchange="state.selectedDataset=this.value"><span><b>${esc(d.name)}</b> <span class="badge ${d.independent_evidence?'ok':'warn'}">${d.independent_evidence?'independiente':'generado'}</span><small>${esc(d.format)} · ${esc(d.bytes)} bytes</small></span></label>`).join('')||'<p class="muted">Sin datasets.</p>'}
async function reload(){state.models=await jget('/api/models');state.profiles=await jget('/api/model-profiles');state.runtimeStatuses=await jget('/api/model-runtime-statuses');state.datasets=await jget('/api/datasets');state.executors=await jget('/api/executors');state.plasticity=await jget('/api/plasticity');state.graph=await jget('/api/graph');state.staircase=await jget('/api/staircase');renderModels();renderDatasets();renderExecutors();renderPlasticity();renderGraphStatus();renderStaircaseStatus();renderAreas();if(state.goalPipeline.length)buildGoalPipeline()}
async function scanModels(){try{state.models=await jpost('/api/models/scan',{root:$('modelRoot').value});renderModels();out({modelos_encontrados:state.models.length})}catch(e){out(e.message)}}
async function importFileDataset(){try{const f=$('datasetFile').files[0];if(!f)throw Error('Selecciona un archivo');const content=await f.text();const name=$('dsName').value.trim()||f.name.replace(/[^A-Za-z0-9_.-]/g,'_');const ext=f.name.split('.').pop().toLowerCase();const format=['json','jsonl','csv'].includes(ext)?ext:'text';const v=await jpost('/api/datasets/import',{name,format,content,generated:$('dsGenerated').value==='true'});state.selectedDataset=v.content_sha256;await reload();const radio=document.querySelector(`input[name=ds][value="${v.content_sha256}"]`);if(radio)radio.checked=true;out(v)}catch(e){out(e.message)}}
async function createBenchmark(){try{const id=$('benchId').value.trim();const domain=$('benchDomain').value.trim();const rows=$('benchRows').value.split('\n').map(x=>x.trim()).filter(Boolean);if(!id||!domain||rows.length<2)throw Error('Indica benchmark id, dominio y al menos 2 pruebas');const probes=rows.map((row,i)=>{const parts=row.split('=>');if(parts.length<2)throw Error(`Línea ${i+1}: usa prompt => respuesta`);const prompt=parts.shift().trim(),expected=parts.join('=>').trim();if(!prompt||!expected)throw Error(`Línea ${i+1}: prompt/respuesta vacíos`);return{probe_id:`p${i+1}`,prompt,verifier:{kind:'exact_text',expected,trim:true,case_sensitive:true},weight:1.0}});const content=JSON.stringify({schema:'tidex.cross_model.behavioral_benchmark/v1',benchmark_id:id,domain,probes,minimum_mean_gap:0.0,significance_alpha:0.05});const v=await jpost('/api/datasets/import',{name:id,format:'json',content,generated:true});state.selectedDataset=v.content_sha256;await reload();const radio=document.querySelector(`input[name=ds][value="${v.content_sha256}"]`);if(radio)radio.checked=true;out(v)}catch(e){out(e.message)}}
function input(id,label,value=''){return `<label class="muted">${label}<input id="${id}" value="${value}"></label>`}
function area(id,label,ph=''){return `<label class="muted">${label}<textarea id="${id}" placeholder="${ph}"></textarea></label>`}
function renderParams(){const op=$('operation').value;let h='<p class="muted">';
if(op==='behavioral_discovery_multi')h+='Compara 2–64 modelos con un benchmark y descubre gaps/propuestas reales.</p>';
if(op==='probe_runtime')h+='Abre realmente el checkpoint con el backend HF y devuelve qué accesos soporta el runtime actual.</p>';
if(op==='behavioral_evaluation')h+='Ejecuta un benchmark real sobre un único LLM.</p>';
if(op==='generate_behavioral_dataset')h+='El LLM propone un benchmark verificable. Rust exige JSON exacto y lo valida; siempre queda marcado como generado/no independiente.</p>'+input('genBenchId','benchmark id','generated.benchmark')+input('genDomain','dominio','general')+input('genProbeCount','número de probes','8')+area('genObjective','objetivo del benchmark','Describe con precisión qué capacidad quieres medir y qué tipo de casos debe cubrir.');
if(op==='extract_capability')h+='Necesita un modelo HF con activaciones internas.</p>'+input('capName','capability name','capability.test')+input('domain','dominio','general')+input('layer','capa','0')+area('positive','ejemplos positivos, uno por línea')+area('negative','ejemplos negativos, uno por línea');
if(op==='deep_instrumentation')h+='Requiere NNsight disponible en el Python seleccionado por TIDEX_HF_PYTHON.</p>'+input('modulePath','module path','model.layers.0')+input('tokenFromEnd','token desde el final','0')+area('prompt','prompt');
if(op==='sparse_autoencoder_analysis')h+='Aplica un diccionario SAE local ligado (sae.safetensors + config.json) sobre activaciones medidas. No entrena y no descarga releases del Hub.</p>'+input('modulePath','module path','model.layers.0')+input('tokenFromEnd','token desde el final','0')+input('saeDir','directorio SAE')+input('saeWeights','ruta absoluta sae.safetensors')+input('saeConfig','ruta absoluta config.json')+input('topK','top K','32')+area('prompt','prompt');
if(op==='counterfactual_analysis')h+='Ejecuta original/perturbado y mide efecto; pega escenarios JSON.</p>'+area('scenarios','escenarios JSON','[{"scenario_id":"x","original_input":"...","perturbed_input":"...","perturbation_type":"caller_defined","verifier":{"kind":"exact_text","expected":"...","trim":true,"case_sensitive":true}}]')+input('activationLayers','capas de activación separadas por coma','');
if(op==='calibrate_alignment')h+='Primer modelo seleccionado = source; segundo = target.</p>'+input('sourceLayer','source layer','0')+input('targetLayer','target layer','0')+area('trainingPrompts','prompts de calibración, uno por línea')+area('validationPrompts','prompts held-out, uno por línea');
if(op==='activation_transfer_experiment')h+='Pipeline real: baseline B → extracción A → alineamiento → steering B → evaluación → clear → restauración.</p>'+input('capName','capability name','capability.test')+input('domain','dominio','general')+input('sourceLayer','source layer','0')+input('targetLayer','target layer','0')+input('strength','strength','1.0')+area('positive','ejemplos positivos, uno por línea')+area('negative','ejemplos negativos, uno por línea')+area('trainingPrompts','prompts calibración, uno por línea')+area('validationPrompts','prompts validación, uno por línea');
$('params').innerHTML=h}
function parameters(op){if(op==='generate_behavioral_dataset')return{benchmark_id:$('genBenchId').value,domain:$('genDomain').value,objective:$('genObjective').value,probe_count:num('genProbeCount',8)};if(op==='extract_capability')return{capability_name:$('capName').value,domain:$('domain').value,level:{kind:'single_layer',layer:num('layer',0)},positive_examples:lines('positive'),negative_examples:lines('negative')};if(op==='deep_instrumentation')return{request:{module_path:$('modulePath').value,prompt:$('prompt').value,token_from_end:num('tokenFromEnd',0)}};if(op==='sparse_autoencoder_analysis')return{request:{module_path:$('modulePath').value,prompt:$('prompt').value,token_from_end:num('tokenFromEnd',0),sae_dir:$('saeDir').value,weights_path:$('saeWeights').value,config_path:$('saeConfig').value,top_k:num('topK',32)}};if(op==='counterfactual_analysis')return{scenarios:JSON.parse($('scenarios').value),activation_layers:$('activationLayers').value.split(',').map(x=>x.trim()).filter(Boolean).map(Number)};if(op==='calibrate_alignment')return{source_layer:num('sourceLayer',0),target_layer:num('targetLayer',0),training_prompts:lines('trainingPrompts'),validation_prompts:lines('validationPrompts')};if(op==='activation_transfer_experiment')return{capability_name:$('capName').value,domain:$('domain').value,extraction_level:{kind:'single_layer',layer:num('sourceLayer',0)},positive_examples:lines('positive'),negative_examples:lines('negative'),source_layer:num('sourceLayer',0),target_layer:num('targetLayer',0),calibration_prompts:lines('trainingPrompts'),validation_prompts:lines('validationPrompts'),strength:num('strength',1)};return{}}
function ensureCompatible(op,models){const needs={behavioral_evaluation:['behavioral_inference'],behavioral_discovery_multi:['behavioral_inference'],extract_capability:['internal_activations'],deep_instrumentation:['deep_instrumentation'],sparse_autoencoder_analysis:['sparse_autoencoder_analysis'],counterfactual_analysis:['behavioral_inference'],calibrate_alignment:['internal_activations'],activation_transfer_experiment:['behavioral_inference','internal_activations','activation_intervention'],generate_behavioral_dataset:['behavioral_inference']};if(op==='probe_runtime')return;for(const id of models){const p=profileFor(id);if(!p)throw Error('Primero ejecuta Comprobar runtime para '+id.slice(0,12));for(const k of (needs[op]||[])){if(!p.access[k])throw Error(`Modelo ${id.slice(0,12)} no soporta ${k}`)}}}
async function cancelActiveJob(){try{if(!state.activeJob)throw Error('No hay job activo');const v=await jpost('/api/jobs/'+state.activeJob+'/cancel',{});renderJob(v);await loadJobs()}catch(e){out(e.message)}}
async function pollJob(job){state.activeJob=job.job_id;$('cancelBtn').disabled=false;renderJob(job);for(;;){await new Promise(r=>setTimeout(r,1000));const v=await jget('/api/jobs/'+job.job_id);renderJob(v);if(v.state==='completed'||v.state==='failed'||v.state==='cancelled'){if(v.operation==='probe_runtime'&&v.state==='completed')await reload();await loadJobs();state.activeJob=null;$('cancelBtn').disabled=true;return v}}}
async function runWorkflow(){try{$('runBtn').disabled=true;$('runBtn').textContent='Ejecutando…';const op=$('operation').value,models=selectedModels();ensureCompatible(op,models);let job;if(op==='behavioral_discovery_multi'){if(models.length<2)throw Error('Selecciona al menos 2 modelos');if(!state.selectedDataset)throw Error('Selecciona un benchmark');job=await jpost('/api/workflows/behavioral-discovery',{schema:'tidex.operator_behavioral_discovery/v1',model_ids:models,dataset_sha256:state.selectedDataset,max_new_tokens:128,seed:0})}else{job=await jpost('/api/workflows/direct',{schema:'tidex.operator_direct_workflow/v1',operation:op,model_ids:models,dataset_sha256:state.selectedDataset,parameters:parameters(op)})}await pollJob(job)}catch(e){out(e.message)}finally{$('runBtn').disabled=false;$('runBtn').textContent='Ejecutar trabajo'}}
function renderPlasticity(){const p=state.plasticity;if(!p){$('plasticity').innerHTML='<p class="muted">Sin señales plásticas calculadas.</p>';return}const jobs=(state.jobs||[]);const related=jobs.filter(j=>['behavioral_discovery','behavioral_evaluation','extract_capability','calibrate_alignment','activation_transfer_experiment'].includes(j.operation));const transfers=related.map(j=>({job:j,data:parseRunStdout(j)})).filter(x=>x.data?.schema==='tidex.operator_activation_transfer/v1');const last=transfers[0]?.data;const leaders=((p.elo_entities||[]).length?p.elo_entities:(p.elo_leaderboard||[]).map(([entity,rating])=>({entity,rating,comparisons:0,last_evidence_sha256:null,last_update:null}))).map(x=>`<div class="result-card"><b>${esc(x.entity)}</b><div class="scorebar"><i style="width:${Math.max(0,Math.min(100,(Number(x.rating)-100)/29))}%"></i></div><div class="muted">rating ${Number(x.rating).toFixed(2)} · comparaciones ${Number(x.comparisons||0)}${x.last_evidence_sha256?' · evidencia '+esc(String(x.last_evidence_sha256).slice(0,12))+'…':''}</div></div>`).join('')||'<p class="muted">Sin pares suficientes para ELO.</p>';const routes=(p.routing_decisions||[]).map(r=>`<div class="result-card"><b>${esc(String(r.capability||'').startsWith('benchmark:')?'benchmark '+String(r.capability).slice(10):String(r.capability||''))} → ${esc(r.target_model)}</b><div class="muted">routing ${Number(r.routing_score||0).toFixed(4)} · medido ${Number(r.measured_score||0).toFixed(4)} · incertidumbre ${Number(r.uncertainty_bonus||0).toFixed(4)} · evidencia ${esc(String(r.evidence_sha256||'').slice(0,12))}…</div></div>`).join('')||'<p class="muted">Sin observaciones comparativas suficientes para routing.</p>';const experiments=related.map(j=>`<div class="history-item" onclick="openJob('${j.job_id}')"><span class="state ${j.state==='completed'?'good':(j.state==='failed'||j.state==='cancelled')?'bad':'running'}">${esc(trState(j.state))}</span><b>${esc(trOperation(j.operation))}</b><div class="muted">${esc(j.job_id.slice(0,16))}…${j.evidence_receipt?.evidence_sha256?' · evidence '+esc(j.evidence_receipt.evidence_sha256.slice(0,12))+'…':''}</div></div>`).join('')||'<p class="muted">Sin experimentos plásticos registrados.</p>';const learning=state.executors.filter(e=>e.executor_id.startsWith('plasticity.')||e.executor_id.startsWith('learning.')||e.executor_id==='numerical.evolve'||e.executor_id==='procedural.memory').map(e=>`<div class="executor-row"><span class="chip ${executorStateClass(e)}">${esc(executorStateLabel(e))}</span><b>${esc(e.title)}</b><div class="muted">${esc(e.executor_id)} · ${esc(e.notes||'')}</div>${executorCanRun(e)?`<button style="width:auto" onclick="activateExecutor('${e.executor_id}')">Abrir operación</button>`:`<details><summary>Contrato interno</summary><pre>${esc(JSON.stringify(e,null,2))}</pre></details>`}</div>`).join('');$('plasticity').innerHTML=`<div class="plasticity-tabs"><button onclick="switchPlasticityPane('state')">Estado</button><button onclick="switchPlasticityPane('experiments')">Experimentos</button><button onclick="switchPlasticityPane('elo')">Entidades / ELO</button><button onclick="switchPlasticityPane('routes')">Rutas</button><button onclick="switchPlasticityPane('learning')">Aprendizaje</button></div><div id="plasticity-state" class="plasticity-pane"><div class="metric-grid">${metric('disponible',p.available?'SÍ':'NO',p.available?'good':'bad')}${metric('jobs fuente',p.source_jobs||0)}${metric('ELO entities',(p.elo_leaderboard||[]).length)}${metric('rutas',(p.routing_decisions||[]).length)}${last?metric('último baseline',pct(last.baseline?.weighted_score)):''}${last?metric('último candidato',pct(last.intervened?.weighted_score),last.score_delta>0?'good':last.score_delta<0?'bad':''):''}${last?metric('delta',Number(last.score_delta||0).toFixed(4),last.score_delta>0?'good':last.score_delta<0?'bad':''):''}</div><div class="section-title">Notas de evidencia</div>${(p.notes||[]).map(n=>`<div class="muted">${esc(n)}</div>`).join('')||'<div class="muted">Sin notas.</div>'}</div><div id="plasticity-experiments" class="plasticity-pane hidden">${experiments}</div><div id="plasticity-elo" class="plasticity-pane hidden">${leaders}</div><div id="plasticity-routes" class="plasticity-pane hidden">${routes}</div><div id="plasticity-learning" class="plasticity-pane hidden">${learning||'<p class="muted">No hay ejecutores de aprendizaje registrados.</p>'}</div>`}
function switchPlasticityPane(name){document.querySelectorAll('.plasticity-pane').forEach(x=>x.classList.add('hidden'));const pane=$('plasticity-'+name);if(pane)pane.classList.remove('hidden')}
function renderExecutors(){const total=state.executors.length,operational=state.executors.filter(e=>e.state==='operational').length,implemented=state.executors.filter(e=>e.implementation_status==='implemented').length,actionable=state.executors.filter(executorActionable).length,runnable=state.executors.filter(executorCanRun).length,prod=state.executors.filter(e=>e.production_authority).length;const grouped={};for(const e of state.executors){(grouped[e.maturity||'unknown'] ||= []).push(e)}const order=['production_lifecycle','operational','operational_candidate','operational_advisory','needs_workflow','experimental','unknown'];$('executors').innerHTML=`<div class="metric-grid">${metric('registrados',total)}${metric('operativos contrato',operational,'good')}${metric('implementados',implemented,'good')}${metric('accionables',actionable,'good')}${metric('ejecutables UI/CLI',runnable)}${metric('autoridad producción',prod,prod===1?'good':'warn')}</div><div class="muted">Operativo aquí significa contrato implementado y rastreable. Madurez, evidencia y autoridad se muestran por separado para no mezclar candidatos/advisory con producción.</div>`+order.filter(k=>grouped[k]?.length).map(k=>`<div class="section-title">${esc(({production_lifecycle:'ciclo de vida producción',operational:'operativo',operational_candidate:'operativo candidato',operational_advisory:'operativo advisory',needs_workflow:'necesita workflow',experimental:'experimental',unknown:'desconocido'}[k]||k))} · ${grouped[k].length}</div>`+grouped[k].map(executorDetail).join('')).join('')||'<p class="muted">Sin ejecutores.</p>'}
function showTab(t){$('resultTab').classList.toggle('hidden',t!=='result');$('historyTab').classList.toggle('hidden',t!=='history');$('executorsTab').classList.toggle('hidden',t!=='executors');$('plasticityTab').classList.toggle('hidden',t!=='plasticity');$('advancedTab').classList.toggle('hidden',t!=='advanced');if(t==='history')loadJobs();if(t==='executors')renderExecutors();if(t==='plasticity')renderPlasticity()}
async function loadJobs(){try{const xs=await jget('/api/jobs');state.jobs=Array.isArray(xs)?xs:[];$('history').innerHTML=state.jobs.map(j=>`<div class="history-item" onclick="openJob('${j.job_id}')"><span class="state ${j.state==='completed'?'good':(j.state==='failed'||j.state==='cancelled')?'bad':'running'}">${esc(trState(j.state))}</span><b>${esc(trOperation(j.operation))}</b><div class="muted">${esc(j.job_id.slice(0,16))}… · ${new Date(Number(j.submitted_unix_ns||0)/1e6).toLocaleString()}${j.evidence_receipt?.evidence_sha256?' · evidence '+esc(j.evidence_receipt.evidence_sha256.slice(0,12))+'…':''}</div></div>`).join('')||`<p class="muted">${currentLanguage==='es'?'Sin ejecuciones todavía.':'No executions yet.'}</p>`;renderPlasticity()}catch(e){$('history').textContent=e.message}}
async function openJob(id){try{const j=await jget('/api/jobs/'+id);showTab('result');renderJob(j)}catch(e){out(e.message)}}
async function loadRecipes(){const xs=await jget('/api/recipes');state.recipes=Array.isArray(xs)?xs:[];$('recipes').innerHTML=state.recipes.map(x=>{const r=trRecipe(x);return `<div class="recipe" data-recipe-id="${esc(x.id)}"><b>${esc(r.title)}</b><span class="badge">${esc(trCategory(x.category))}</span><p class="muted">${esc(r.description)}</p><button onclick='runRaw(${JSON.stringify(JSON.stringify(x.id))})'>${currentLanguage==='es'?'Ejecutar':'Run'}</button></div>`}).join('');renderAreas();if(state.goalPipeline.length)renderGoalPipeline()}
async function runRaw(encoded){try{const id=JSON.parse(encoded),assets=$('assets').value.split('\n').map(s=>s.trim()).filter(Boolean);out(await jpost('/api/runs',{schema:'tidex.operator_run_request/v1',recipe_id:id,assets,selected_model_ids:[],dataset_sha256:null}))}catch(e){out(e.message)}}
(async()=>{try{state.info=await jget('/api/info');renderStatus();if(state.info.default_model_scan_root)$('modelRoot').value=state.info.default_model_scan_root;await reload();if(!state.models.length&&state.info.default_model_scan_root)await scanModels();await loadRecipes();await loadJobs();renderParams();setOperatorMode('system');buildGoalPipeline();applyLanguage()}catch(e){out(e.message);$('status').textContent=currentLanguage==='es'?'error de inicialización':'initialization error'}})();
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipes_are_unique_and_operator_run_rejects_production() {
        let recipes = recipe_catalog();
        let ids = recipes
            .iter()
            .map(|item| &item.id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), recipes.len());
        assert!(recipes.iter().all(|item| item.asset_count <= MAX_ASSETS));
        assert!(recipes.iter().any(|item| item.production_activation));
        let home = isolated_operator_home("production");
        let request = OperatorRunRequest {
            schema: "tidex.operator_run_request/v1".into(),
            recipe_id: "adapter.activate".into(),
            assets: Vec::new(),
            selected_model_ids: Vec::new(),
            dataset_sha256: None,
            maximum_runtime_seconds: None,
        };
        let error = execute_operator_run(&home, &request)
            .unwrap_err()
            .to_string();
        assert!(error.contains("operator_recipe_production_activation_forbidden"));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn web_operator_assets_are_present_and_restricted() {
        let (html_type, html) = web_operator_asset("/").expect("web-console index");
        assert_eq!(html_type, "text/html; charset=utf-8");
        let html_text = String::from_utf8(html).unwrap();
        assert!(html_text.contains("TIDE-X"));
        assert!(html_text.contains("id=\"eventLog\""));
        let (js_type, js) = web_operator_asset("/app.js").expect("web-console app.js");
        assert_eq!(js_type, "application/javascript; charset=utf-8");
        assert!(String::from_utf8(js).unwrap().contains("/api/info"));
        assert!(web_operator_asset("/secret").is_err());
        assert!(web_operator_asset("/../Cargo.toml").is_err());
        assert!(OPERATOR_HTML.contains("esc(m.architecture||'arquitectura desconocida')"));
        assert!(OPERATOR_HTML.contains("generated:true"));
        assert!(OPERATOR_HTML.contains("/api/graph"));
        assert!(OPERATOR_HTML.contains("/api/staircase"));
        assert!(OPERATOR_HTML.contains("bound_sparse_dictionary"));
        assert!(!OPERATOR_HTML.contains("sae_lens_available"));
        assert!(!OPERATOR_HTML.contains("SAE Lens"));
        assert!(!OPERATOR_HTML.contains("No existen módulos huérfanos"));
    }

    #[test]
    fn operator_living_staircase_composes_plasticity_and_graph_without_production() {
        let home = isolated_operator_home("staircase");
        let receipt = compute_operator_living_staircase(&home).expect("staircase");
        assert_eq!(receipt.schema, "tidex.operator_living_staircase/v1");
        assert!(!receipt.knowledge_live);
        assert!(!receipt.authorizes_production);
        assert!(!receipt.discovery.present);
        assert!(receipt.executed_advanced.is_empty());
        assert_eq!(receipt.plasticity.source_jobs, 0);
        assert_ne!(receipt.staircase_sha256, Sha256Digest::zero());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn non_loopback_server_is_rejected_before_bind() {
        let root = isolated_operator_home("bind");
        let address: SocketAddr = "0.0.0.0:0".parse().unwrap();
        assert!(serve(&root, address).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn configured_hf_python_is_fail_closed_and_ignores_path() {
        let previous = std::env::var_os("TIDEX_HF_PYTHON");
        std::env::set_var("TIDEX_HF_PYTHON", "/tmp/override-python");
        let chosen = configured_hf_python_candidate().unwrap();
        assert_eq!(chosen, PathBuf::from("/tmp/override-python"));
        std::env::set_var("TIDEX_HF_PYTHON", "/tmp/tidex-missing-python");
        let error = find_python3().unwrap_err().to_string();
        assert!(error.contains("hf_python_not_found"));
        match previous {
            Some(value) => std::env::set_var("TIDEX_HF_PYTHON", value),
            None => std::env::remove_var("TIDEX_HF_PYTHON"),
        }
    }

    #[test]
    fn model_scan_rejects_roots_outside_hub() {
        let error = discover_local_models(Path::new("/tmp"))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("operator_model_scan_root_outside_hub")
                || error.contains("operator_model_hub_missing")
                || error.contains("operator_model_hub_invalid")
        );
    }

    #[test]
    fn model_scan_accepts_hub_blob_symlinks_and_rejects_escapes() {
        let hub = default_hf_hub_root().expect("hub root");
        fs::create_dir_all(&hub).unwrap();
        let tag = format!("{}-{}", std::process::id(), now_nanos().unwrap_or(0));
        let repo = hub.join(format!("models--tidex-test--{tag}"));
        let blobs = repo.join("blobs");
        let snap = repo.join("snapshots").join("deadbeef");
        fs::create_dir_all(&blobs).unwrap();
        fs::create_dir_all(&snap).unwrap();
        fs::write(
            blobs.join("config"),
            br#"{"model_type":"llama","architectures":["LlamaForCausalLM"]}"#,
        )
        .unwrap();
        fs::write(blobs.join("tok"), br#"{}"#).unwrap();
        fs::write(blobs.join("weights"), b"not-a-real-checkpoint").unwrap();
        std::os::unix::fs::symlink("../../blobs/config", snap.join("config.json")).unwrap();
        std::os::unix::fs::symlink("../../blobs/tok", snap.join("tokenizer.json")).unwrap();
        std::os::unix::fs::symlink("../../blobs/weights", snap.join("model.safetensors")).unwrap();

        let evil_repo = hub.join(format!("models--tidex-escape--{tag}"));
        let evil_snap = evil_repo.join("snapshots").join("evil");
        fs::create_dir_all(&evil_snap).unwrap();
        std::os::unix::fs::symlink("/etc/hosts", evil_snap.join("config.json")).unwrap();
        fs::write(evil_snap.join("tokenizer.json"), br#"{}"#).unwrap();
        fs::write(evil_snap.join("model.safetensors"), b"x").unwrap();

        let found = discover_local_models(&hub).expect("scan");
        let hit = found.iter().any(|model| model.root == snap);
        let evil_hit = found.iter().any(|model| model.root == evil_snap);
        let _ = fs::remove_dir_all(repo);
        let _ = fs::remove_dir_all(evil_repo);
        assert!(hit, "HF blob-symlink snapshots inside the hub must catalog");
        assert!(!evil_hit, "symlinks that escape the hub must not catalog");
    }

    #[test]
    fn model_id_is_content_bound_and_stale_catalog_identity_is_rejected() {
        let hub = default_hf_hub_root().expect("hub root");
        fs::create_dir_all(&hub).unwrap();
        let tag = format!("{}-{}", std::process::id(), now_nanos().unwrap_or(0));
        let repo = hub.join(format!("models--tidex-content--{tag}"));
        let blobs = repo.join("blobs");
        let snap = repo.join("snapshots").join("abc123def456");
        fs::create_dir_all(&blobs).unwrap();
        fs::create_dir_all(&snap).unwrap();
        fs::write(
            blobs.join("config"),
            br#"{"model_type":"llama","architectures":["LlamaForCausalLM"]}"#,
        )
        .unwrap();
        fs::write(blobs.join("tok"), br#"{}"#).unwrap();
        fs::write(blobs.join("weights"), b"checkpoint-bytes-v1").unwrap();
        std::os::unix::fs::symlink("../../blobs/config", snap.join("config.json")).unwrap();
        std::os::unix::fs::symlink("../../blobs/tok", snap.join("tokenizer.json")).unwrap();
        std::os::unix::fs::symlink("../../blobs/weights", snap.join("model.safetensors")).unwrap();

        let home = isolated_operator_home("content-model-id");
        // Scan only this fixture repo so leftover hub snapshots cannot collide
        // on content-identical model_ids.
        let first = catalog_local_models(&home, &repo).expect("first catalog");
        let original = first
            .iter()
            .find(|model| model.root == snap)
            .expect("original model")
            .clone();
        let repeated = discover_local_models(&repo).expect("repeat scan");
        let repeated = repeated
            .iter()
            .find(|model| model.root == snap)
            .expect("repeated model");
        assert_eq!(original.model_id, repeated.model_id);

        fs::write(blobs.join("weights"), b"checkpoint-bytes-MUTATED").unwrap();
        let second = catalog_local_models(&home, &repo).expect("recatalog after mutation");
        let mutated = second
            .iter()
            .find(|model| model.root == snap)
            .expect("mutated model");
        assert_ne!(original.model_id, mutated.model_id);
        assert!(load_catalog_model(&home, &original.model_id).is_err());
        let loaded = load_catalog_model(&home, &mutated.model_id).expect("load current model");
        assert_eq!(loaded.model_id, mutated.model_id);
        assert_eq!(loaded.root, snap);

        let _ = fs::remove_dir_all(repo);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn catalog_dedupes_content_identical_roots_without_collision() {
        let hub = default_hf_hub_root().expect("hub root");
        fs::create_dir_all(&hub).unwrap();
        let tag = format!("{}-{}", std::process::id(), now_nanos().unwrap_or(0));
        let repo = hub.join(format!("models--tidex-dup--{tag}"));
        let blobs = repo.join("blobs");
        fs::create_dir_all(&blobs).unwrap();
        fs::write(
            blobs.join("config"),
            br#"{"model_type":"llama","architectures":["LlamaForCausalLM"]}"#,
        )
        .unwrap();
        fs::write(blobs.join("tok"), br#"{}"#).unwrap();
        fs::write(blobs.join("weights"), b"identical-checkpoint-bytes").unwrap();

        let mut snaps = Vec::new();
        for name in ["aaa111", "zzz999"] {
            let snap = repo.join("snapshots").join(name);
            fs::create_dir_all(&snap).unwrap();
            std::os::unix::fs::symlink("../../blobs/config", snap.join("config.json")).unwrap();
            std::os::unix::fs::symlink("../../blobs/tok", snap.join("tokenizer.json")).unwrap();
            std::os::unix::fs::symlink("../../blobs/weights", snap.join("model.safetensors")).unwrap();
            snaps.push(snap);
        }

        let home = isolated_operator_home("dup-content");
        let cataloged = catalog_local_models(&home, &repo).expect("deduped catalog");
        let ids: std::collections::BTreeSet<_> =
            cataloged.iter().map(|m| m.model_id.to_string()).collect();
        assert_eq!(ids.len(), 1, "identical content must share one content-bound id");
        assert_eq!(cataloged.len(), 1);
        assert_eq!(cataloged[0].root, snaps[0], "prefer lexicographically smallest root");
        // Rescan must refresh without collision even if architecture metadata changes.
        let again = catalog_local_models(&home, &repo).expect("rescan refresh");
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].model_id, cataloged[0].model_id);

        let _ = fs::remove_dir_all(repo);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn operator_assets_must_stay_inside_operator_home() {
        let home = isolated_operator_home("assets");
        let outside = std::env::temp_dir().join(format!(
            "tidex-operator-outside-{}-{}",
            std::process::id(),
            "assets"
        ));
        fs::write(&outside, b"not-in-vault").unwrap();
        let request = OperatorRunRequest {
            schema: "tidex.operator_run_request/v1".into(),
            recipe_id: "knowledge.plan".into(),
            assets: vec![outside.clone()],
            selected_model_ids: Vec::new(),
            dataset_sha256: None,
            maximum_runtime_seconds: None,
        };
        let error = execute_operator_run(&home, &request)
            .unwrap_err()
            .to_string();
        assert!(error.contains("operator_asset_outside_operator_home"));
        let _ = fs::remove_file(outside);
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(feature = "cross-model-plasticity")]
    fn seed_completed_evaluation(
        home: &Path,
        tag: &str,
        model: &str,
        score: f64,
        benchmark: &str,
        submitted_unix_ns: u128,
    ) {
        let job_id = Sha256Digest::digest_bytes(tag.as_bytes());
        let run_id = Sha256Digest::digest_bytes(format!("{tag}-run").as_bytes());
        let run_root = home.join("operator/runs/by-sha").join(run_id.as_str());
        fs::create_dir_all(&run_root).unwrap();
        let stdout = serde_json::json!({
            "schema": "tidex.cross_model.model_evaluation/v1",
            "benchmark_id": benchmark,
            "benchmark_sha256": Sha256Digest::digest_bytes(benchmark.as_bytes()).as_str(),
            "evidence_sha256": Sha256Digest::digest_bytes(format!("{tag}-evidence").as_bytes()).as_str(),
            "model": model,
            "weighted_score": score,
            "observations": [{"probe_id": "p1"}]
        });
        let stdout_bytes = serde_json::to_vec(&stdout).unwrap();
        let stdout_path = run_root.join("stdout.json");
        let stderr_path = run_root.join("stderr.txt");
        fs::write(&stdout_path, &stdout_bytes).unwrap();
        fs::write(&stderr_path, b"").unwrap();
        let record = OperatorJobRecord {
            schema: "tidex.operator_job/v1".into(),
            job_id,
            request_sha256: None,
            evidence_receipt: None,
            state: OperatorJobState::Completed,
            operation: "behavioral_evaluation".into(),
            submitted_unix_ns,
            run: Some(OperatorRunView {
                receipt: OperatorRunReceipt {
                    schema: "tidex.operator_run_receipt/v1".into(),
                    run_id,
                    recipe_id: "operator.direct_runner".into(),
                    executor_id: Some("cross_model.evaluate".into()),
                    argv: Vec::new(),
                    selected_model_ids: Vec::new(),
                    dataset_sha256: None,
                    source_tree_sha256: Sha256Digest::zero(),
                    exit_code: 0,
                    stdout_sha256: Sha256Digest::digest_bytes(&stdout_bytes),
                    stderr_sha256: Sha256Digest::digest_bytes(b""),
                    stdout: stdout_path,
                    stderr: stderr_path,
                    succeeded: true,
                    production_activation_recipe: false,
                    authorizes_production: false,
                },
                stdout: String::new(),
                stderr: String::new(),
            }),
            error: None,
        };
        persist_job_record(home, &record).unwrap();
    }

    #[test]
    fn plasticity_advice_is_empty_without_evaluation_jobs() {
        let home = isolated_operator_home("plasticity-empty");
        let advice = compute_operator_plasticity_advice(&home).unwrap();
        assert!(!advice.available);
        assert_eq!(advice.source_jobs, 0);
        assert!(advice.elo_leaderboard.is_empty());
        assert!(advice.routing_decisions.is_empty());
        #[cfg(feature = "cross-model-plasticity")]
        assert!(advice.notes.iter().any(|note| note.contains("Sin jobs")));
        #[cfg(not(feature = "cross-model-plasticity"))]
        assert!(advice
            .notes
            .iter()
            .any(|note| note.contains("feature disabled")));
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(feature = "cross-model-plasticity")]
    #[test]
    fn plasticity_advice_does_not_invent_elo_from_an_isolated_evaluation() {
        let home = isolated_operator_home("plasticity-isolated");
        seed_completed_evaluation(&home, "eval-a", "model-a", 0.0, "integer_arithmetic_v1", 1);
        let advice = compute_operator_plasticity_advice(&home).unwrap();
        assert!(!advice.available);
        assert_eq!(advice.source_jobs, 1);
        assert!(advice.elo_entities.is_empty());
        assert!(advice.routing_decisions.is_empty());
        assert!(advice.notes.iter().any(|note| note.contains("aislada")));
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(feature = "cross-model-plasticity")]
    #[test]
    fn plasticity_advice_withholds_ranking_on_measured_tie() {
        let home = isolated_operator_home("plasticity-tie");
        seed_completed_evaluation(&home, "eval-a", "model-a", 0.0, "integer_arithmetic_v1", 1);
        seed_completed_evaluation(&home, "eval-b", "model-b", 0.0, "integer_arithmetic_v1", 2);
        let advice = compute_operator_plasticity_advice(&home).unwrap();
        assert!(!advice.available);
        assert_eq!(advice.source_jobs, 2);
        assert!(advice.elo_entities.is_empty());
        assert!(advice.routing_decisions.is_empty());
        assert!(advice
            .notes
            .iter()
            .any(|note| note.contains("Empate medido")));
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(feature = "cross-model-plasticity")]
    #[test]
    fn plasticity_advice_ranks_and_routes_on_measured_score_gap() {
        let home = isolated_operator_home("plasticity-gap");
        seed_completed_evaluation(&home, "eval-a", "model-a", 0.0, "integer_arithmetic_v1", 1);
        seed_completed_evaluation(&home, "eval-b", "model-b", 1.0, "integer_arithmetic_v1", 2);
        let advice = compute_operator_plasticity_advice(&home).unwrap();
        assert!(advice.available);
        assert_eq!(advice.source_jobs, 2);
        assert_eq!(advice.elo_entities.len(), 2);
        assert!(advice.elo_entities.iter().all(|item| item.comparisons == 1));
        assert_eq!(advice.elo_leaderboard[0].0, "model-b");
        assert_eq!(advice.routing_decisions.len(), 1);
        assert_eq!(advice.routing_decisions[0]["target_model"].as_str(), Some("model-b"));
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(feature = "cross-model-plasticity")]
    #[test]
    fn plasticity_advice_persists_controller_state_across_calls() {
        let home = isolated_operator_home("plasticity-durable");
        seed_completed_evaluation(&home, "eval-a", "model-a", 0.0, "integer_arithmetic_v1", 1);
        seed_completed_evaluation(&home, "eval-b", "model-b", 1.0, "integer_arithmetic_v1", 2);
        let first = compute_operator_plasticity_advice(&home).unwrap();
        assert!(first.available);
        assert!(advice_path_exists(&home));
        let rating_b = first.elo_leaderboard[0].1;
        let second = compute_operator_plasticity_advice(&home).unwrap();
        assert_eq!(second.elo_entities.len(), 2);
        assert!(second.elo_entities.iter().all(|item| item.comparisons == 1));
        assert!((second.elo_leaderboard[0].1 - rating_b).abs() < 1e-12);
        assert!(second
            .notes
            .iter()
            .any(|note| note.contains("durable recargado")));
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(feature = "cross-model-plasticity")]
    fn sealed_model_evaluation_value(
        model: &str,
        score: f64,
        benchmark: &str,
        tag: &str,
    ) -> serde_json::Value {
        use crate::cross_model::discovery::{ModelEvaluation, ProbeObservation};
        use crate::cross_model::models::sha256_hex;
        let mut evaluation = ModelEvaluation {
            schema: "tidex.cross_model.model_evaluation/v1".into(),
            benchmark_id: benchmark.into(),
            benchmark_sha256: sha256_hex(benchmark.as_bytes()),
            model: model.into(),
            runtime_metadata_sha256: sha256_hex(format!("{tag}-runtime").as_bytes()),
            observations: vec![ProbeObservation {
                probe_id: format!("{tag}-probe"),
                prompt_sha256: sha256_hex(format!("{tag}-prompt").as_bytes()),
                response_sha256: sha256_hex(format!("{tag}-response").as_bytes()),
                response_text: format!("{tag}-text"),
                score,
                weight: 1.0,
                total_duration_ns: None,
                prompt_eval_count: None,
                eval_count: None,
                execution_sha256: sha256_hex(format!("{tag}-exec").as_bytes()),
                active_interventions_sha256: sha256_hex(b"[]"),
                active_intervention_count: 0,
            }],
            weighted_score: score,
            evidence_sha256: String::new(),
        };
        let mut unsigned = evaluation.clone();
        unsigned.evidence_sha256.clear();
        evaluation.evidence_sha256 = sha256_hex(&serde_json::to_vec(&unsigned).unwrap());
        evaluation.validate().unwrap();
        serde_json::to_value(evaluation).unwrap()
    }

    #[cfg(feature = "cross-model-plasticity")]
    fn seed_completed_discovery_cycle(
        home: &Path,
        tag: &str,
        benchmark: &str,
        scores: &[(&str, f64)],
        submitted_unix_ns: u128,
    ) {
        let job_id = Sha256Digest::digest_bytes(tag.as_bytes());
        let run_id = Sha256Digest::digest_bytes(format!("{tag}-run").as_bytes());
        let run_root = home.join("operator/runs/by-sha").join(run_id.as_str());
        fs::create_dir_all(&run_root).unwrap();
        let evaluations = scores
            .iter()
            .enumerate()
            .map(|(idx, (model, score))| {
                sealed_model_evaluation_value(model, *score, benchmark, &format!("{tag}-{idx}"))
            })
            .collect::<Vec<_>>();
        let stdout = serde_json::json!({
            "schema": "tidex.cross_model.discovery_cycle/v1",
            "benchmark_id": benchmark,
            "evaluations": evaluations,
            "gaps": [],
            "priorities": [],
            "proposals": []
        });
        let stdout_bytes = serde_json::to_vec(&stdout).unwrap();
        let stdout_path = run_root.join("stdout.json");
        let stderr_path = run_root.join("stderr.txt");
        fs::write(&stdout_path, &stdout_bytes).unwrap();
        fs::write(&stderr_path, b"").unwrap();
        let record = OperatorJobRecord {
            schema: "tidex.operator_job/v1".into(),
            job_id,
            request_sha256: None,
            evidence_receipt: None,
            state: OperatorJobState::Completed,
            operation: "behavioral_discovery".into(),
            submitted_unix_ns,
            run: Some(OperatorRunView {
                receipt: OperatorRunReceipt {
                    schema: "tidex.operator_run_receipt/v1".into(),
                    run_id,
                    recipe_id: "cross_model.discovery_cycle".into(),
                    executor_id: Some("cross_model.discovery_cycle".into()),
                    argv: Vec::new(),
                    selected_model_ids: Vec::new(),
                    dataset_sha256: None,
                    source_tree_sha256: Sha256Digest::zero(),
                    exit_code: 0,
                    stdout_sha256: Sha256Digest::digest_bytes(&stdout_bytes),
                    stderr_sha256: Sha256Digest::digest_bytes(b""),
                    stdout: stdout_path,
                    stderr: stderr_path,
                    succeeded: true,
                    production_activation_recipe: false,
                    authorizes_production: false,
                },
                stdout: String::new(),
                stderr: String::new(),
            }),
            error: None,
        };
        persist_job_record(home, &record).unwrap();
    }

    #[cfg(feature = "cross-model-plasticity")]
    fn seed_completed_activation_transfer(
        home: &Path,
        tag: &str,
        capability: &str,
        target_model: &str,
        submitted_unix_ns: u128,
    ) {
        let job_id = Sha256Digest::digest_bytes(tag.as_bytes());
        let run_id = Sha256Digest::digest_bytes(format!("{tag}-run").as_bytes());
        let run_root = home.join("operator/runs/by-sha").join(run_id.as_str());
        fs::create_dir_all(&run_root).unwrap();
        let intervention = serde_json::json!({
            "schema": "tidex.cross_model.activation_intervention_receipt/v1",
            "capability": capability,
            "target_model": target_model,
            "strength": 1.0,
            "evidence_sha256": Sha256Digest::digest_bytes(format!("{tag}-iv").as_bytes()).as_str(),
        });
        let stdout = serde_json::json!({
            "schema": "tidex.operator_activation_transfer/v1",
            "capability_name": capability,
            "source_model": "model-b",
            "target_model": target_model,
            "intervention": intervention,
            "score_delta": 0.1,
            "behavioral_improvement_observed": true
        });
        let stdout_bytes = serde_json::to_vec(&stdout).unwrap();
        let stdout_path = run_root.join("stdout.json");
        let stderr_path = run_root.join("stderr.txt");
        fs::write(&stdout_path, &stdout_bytes).unwrap();
        fs::write(&stderr_path, b"").unwrap();
        let record = OperatorJobRecord {
            schema: "tidex.operator_job/v1".into(),
            job_id,
            request_sha256: None,
            evidence_receipt: None,
            state: OperatorJobState::Completed,
            operation: "activation_transfer_experiment".into(),
            submitted_unix_ns,
            run: Some(OperatorRunView {
                receipt: OperatorRunReceipt {
                    schema: "tidex.operator_run_receipt/v1".into(),
                    run_id,
                    recipe_id: "cross_model.transfer_steering".into(),
                    executor_id: Some("cross_model.transfer_steering".into()),
                    argv: Vec::new(),
                    selected_model_ids: Vec::new(),
                    dataset_sha256: None,
                    source_tree_sha256: Sha256Digest::zero(),
                    exit_code: 0,
                    stdout_sha256: Sha256Digest::digest_bytes(&stdout_bytes),
                    stderr_sha256: Sha256Digest::digest_bytes(b""),
                    stdout: stdout_path,
                    stderr: stderr_path,
                    succeeded: true,
                    production_activation_recipe: false,
                    authorizes_production: false,
                },
                stdout: String::new(),
                stderr: String::new(),
            }),
            error: None,
        };
        persist_job_record(home, &record).unwrap();
    }

    #[cfg(feature = "cross-model-plasticity")]
    #[test]
    fn plasticity_advice_persists_bidirectional_loop_across_calls() {
        let home = isolated_operator_home("plasticity-coevo-durable");
        seed_completed_discovery_cycle(
            &home,
            "disc-1",
            "integer_arithmetic_v1",
            &[("model-a", 0.2), ("model-b", 0.8)],
            1,
        );
        let first = compute_operator_plasticity_advice(&home).unwrap();
        assert_eq!(first.coevolution.len(), 1);
        assert_eq!(first.coevolution[0]["iteration"].as_u64(), Some(0));
        let first_evidence = first.coevolution[0]["evidence_sha256"]
            .as_str()
            .unwrap()
            .to_string();

        let second = compute_operator_plasticity_advice(&home).unwrap();
        assert_eq!(second.coevolution.len(), 1, "same discovery must not double-record");
        assert_eq!(
            second.coevolution[0]["evidence_sha256"].as_str(),
            Some(first_evidence.as_str())
        );
        assert!(second
            .notes
            .iter()
            .any(|note| note.contains("durable recargado")));

        seed_completed_discovery_cycle(
            &home,
            "disc-2",
            "integer_arithmetic_v1",
            &[("model-a", 0.3), ("model-b", 0.7)],
            2,
        );
        let third = compute_operator_plasticity_advice(&home).unwrap();
        assert_eq!(third.coevolution.len(), 2, "new discovery must append to durable history");
        assert_eq!(third.coevolution[0]["iteration"].as_u64(), Some(0));
        assert_eq!(third.coevolution[1]["iteration"].as_u64(), Some(1));
        assert_eq!(third.coevolution[0]["evidence_sha256"].as_str(), Some(first_evidence.as_str()));
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(feature = "cross-model-plasticity")]
    #[test]
    fn plasticity_advice_coevolution_respects_causal_intervention_order() {
        let home = isolated_operator_home("plasticity-coevo-causal");
        seed_completed_discovery_cycle(
            &home,
            "disc-early",
            "integer_arithmetic_v1",
            &[("model-a", 0.2), ("model-b", 0.8)],
            10,
        );
        seed_completed_activation_transfer(&home, "xfer-mid", "cap.arith", "model-a", 20);
        seed_completed_discovery_cycle(
            &home,
            "disc-late",
            "integer_arithmetic_v1",
            &[("model-a", 0.35), ("model-b", 0.75)],
            30,
        );
        let advice = compute_operator_plasticity_advice(&home).unwrap();
        assert_eq!(advice.coevolution.len(), 2);
        let early = &advice.coevolution[0];
        let late = &advice.coevolution[1];
        let early_iv = early["applied_interventions"].as_array().unwrap();
        let late_iv = late["applied_interventions"].as_array().unwrap();
        assert!(
            early_iv.is_empty(),
            "later intervention must NOT seal into earlier cycle: {early_iv:?}"
        );
        assert_eq!(
            late_iv.len(),
            1,
            "earlier valid intervention may seal into later cycle: {late_iv:?}"
        );
        assert_eq!(late_iv[0]["target_model"].as_str(), Some("model-a"));
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(feature = "cross-model-plasticity")]
    #[test]
    fn plasticity_advice_loop_tick_changes_durable_controllers_for_next_advice() {
        let home = isolated_operator_home("plasticity-coevo-loop");
        seed_completed_discovery_cycle(
            &home,
            "disc-gap",
            "integer_arithmetic_v1",
            &[("model-a", 0.1), ("model-b", 0.9)],
            1,
        );
        let first = compute_operator_plasticity_advice(&home).unwrap();
        let directive = first
            .coevolution_directive
            .as_ref()
            .expect("operational loop must emit directive");
        assert_eq!(
            directive["recommended_operation"].as_str(),
            Some("activation_transfer_experiment")
        );
        assert_eq!(directive["source_model"].as_str(), Some("model-b"));
        assert_eq!(directive["target_model"].as_str(), Some("model-a"));
        let first_pi_updates = first
            .pi_controller
            .as_ref()
            .map(|row| row.updates)
            .unwrap_or(0);
        assert!(first_pi_updates >= 1, "loop tick must nudge PI");

        let state_path = home.join("operator/plasticity/controller_state.json");
        let state: serde_json::Value =
            serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
        let weight = state["routing_matrix"]["weights"]["benchmark:integer_arithmetic_v1"]
            ["model-b"]
            .as_f64()
            .expect("loop tick must persist routing weight for preferred model");
        assert!(
            (weight - 0.5).abs() > 1e-9,
            "routing matrix must leave default 0.5 after loop tick, got {weight}"
        );

        let second = compute_operator_plasticity_advice(&home).unwrap();
        assert!(second.coevolution_directive.is_some());
        let state2: serde_json::Value =
            serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
        let weight2 = state2["routing_matrix"]["weights"]["benchmark:integer_arithmetic_v1"]
            ["model-b"]
            .as_f64()
            .unwrap();
        assert!(
            (weight2 - weight).abs() < 1e-12,
            "next advice must consume durable loop-steered routing weight ({weight} vs {weight2})"
        );
        assert!(second
            .notes
            .iter()
            .any(|note| note.contains("durable recargado")));
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(feature = "cross-model-plasticity")]
    #[test]
    fn plasticity_advice_prefers_newer_valid_evidence_for_same_model() {
        let home = isolated_operator_home("plasticity-newer");
        seed_completed_evaluation(&home, "eval-old", "model-a", 0.0, "integer_arithmetic_v1", 1);
        seed_completed_evaluation(&home, "eval-new", "model-a", 1.0, "integer_arithmetic_v1", 5);
        seed_completed_evaluation(&home, "eval-b", "model-b", 0.0, "integer_arithmetic_v1", 2);
        let advice = compute_operator_plasticity_advice(&home).unwrap();
        assert!(advice.available);
        assert_eq!(advice.elo_leaderboard[0].0, "model-a");
        assert!(advice
            .notes
            .iter()
            .any(|note| note.contains("newer-valid") || note.contains("reemplazada")));
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(feature = "cross-model-plasticity")]
    fn advice_path_exists(home: &Path) -> bool {
        home.join("operator/plasticity/controller_state.json")
            .is_file()
    }

    #[test]
    fn persist_job_record_strips_stdout() {
        let home = isolated_operator_home("jobs");
        let job_id = Sha256Digest::digest_bytes(b"operator-job-slim-test");
        let record = OperatorJobRecord {
            schema: "tidex.operator_job/v1".into(),
            job_id: job_id.clone(),
            request_sha256: None,
            evidence_receipt: None,
            state: OperatorJobState::Completed,
            operation: "probe_runtime".into(),
            submitted_unix_ns: 1,
            run: Some(OperatorRunView {
                receipt: OperatorRunReceipt {
                    schema: "tidex.operator_run_receipt/v1".into(),
                    run_id: Sha256Digest::digest_bytes(b"operator-run-slim-test"),
                    recipe_id: "operator.direct_runner".into(),
                    executor_id: None,
                    argv: Vec::new(),
                    selected_model_ids: Vec::new(),
                    dataset_sha256: None,
                    source_tree_sha256: Sha256Digest::zero(),
                    exit_code: 0,
                    stdout_sha256: Sha256Digest::digest_bytes(b"{}"),
                    stderr_sha256: Sha256Digest::digest_bytes(b""),
                    stdout: home.join("missing-stdout.json"),
                    stderr: home.join("missing-stderr.txt"),
                    succeeded: true,
                    production_activation_recipe: false,
                    authorizes_production: false,
                },
                stdout: "SHOULD_NOT_PERSIST".into(),
                stderr: "SHOULD_NOT_PERSIST".into(),
            }),
            error: None,
        };
        persist_job_record(&home, &record).unwrap();
        let listed = list_job_records(&home).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].run.as_ref().unwrap().stdout, "");
        let loaded = load_job_record(&home, &job_id).unwrap();
        assert_eq!(loaded.run.as_ref().unwrap().stdout, "");
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn persisted_job_rejects_tampered_evidence_receipt_from_real_supervisor_path() {
        let home = isolated_operator_home("job-evidence-tamper");
        let missing_model = Sha256Digest::digest_bytes(b"operator-missing-model");
        let queued = start_operator_job(
            &home,
            OperatorJobRequest::Direct(OperatorDirectWorkflowRequest {
                schema: "tidex.operator_direct_workflow/v1".into(),
                operation: OperatorDirectOperation::ProbeRuntime,
                model_ids: vec![missing_model],
                dataset_sha256: None,
                parameters: serde_json::json!({}),
            }),
        )
        .unwrap();

        let terminal = (0..200)
            .find_map(|_| {
                let record = load_job_record(&home, &queued.job_id).ok()?;
                if matches!(
                    record.state,
                    OperatorJobState::Completed
                        | OperatorJobState::Failed
                        | OperatorJobState::Cancelled
                ) {
                    Some(record)
                } else {
                    std::thread::sleep(Duration::from_millis(5));
                    None
                }
            })
            .expect("real operator supervisor did not reach a terminal state");
        assert_eq!(terminal.state, OperatorJobState::Failed);
        assert!(terminal.evidence_receipt.is_some());

        let status = job_status_path(&home, &queued.job_id);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&status).unwrap()).unwrap();
        value["evidence_receipt"]["evidence_sha256"] = serde_json::Value::String(
            Sha256Digest::digest_bytes(b"tampered-evidence-receipt")
                .as_str()
                .to_string(),
        );
        fs::write(&status, serde_json::to_vec(&value).unwrap()).unwrap();

        assert!(load_job_record(&home, &queued.job_id).is_err());
        assert!(list_job_records(&home).is_err());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn http_access_rejects_cross_origin_and_non_loopback_host() {
        assert!(loopback_http_host("127.0.0.1:8793"));
        assert!(loopback_http_host("localhost"));
        assert!(loopback_http_host("[::1]:8793"));
        assert!(!loopback_http_host("example.com"));
        assert!(loopback_origin("http://127.0.0.1:8793"));
        assert!(!loopback_origin("https://evil.example"));
        assert!(!loopback_origin("null"));
        let mut request = HttpRequest {
            method: "POST".into(),
            path: "/api/info".into(),
            headers: vec![
                ("Host".into(), "127.0.0.1:8793".into()),
                ("Content-Type".into(), "application/json".into()),
                ("Origin".into(), "https://evil.example".into()),
            ],
            body: Vec::new(),
        };
        assert!(enforce_http_access(&request).is_err());
        request.headers[2].1 = "http://127.0.0.1:8793".into();
        assert!(enforce_http_access(&request).is_ok());
        request.headers[0].1 = "evil.example".into();
        assert!(enforce_http_access(&request).is_err());
    }

    #[test]
    fn path_prefix_does_not_confuse_sibling_directories() {
        assert!(path_is_within(
            Path::new("/home/yo/Future/runtime/tidex/operator"),
            Path::new("/home/yo/Future/runtime/tidex")
        ));
        assert!(!path_is_within(
            Path::new("/home/yo/Future/runtime/tidex-evil/x"),
            Path::new("/home/yo/Future/runtime/tidex")
        ));
    }

    fn isolated_operator_home(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "tidex-operator-{}-{}-{}",
            tag,
            std::process::id(),
            now_nanos().unwrap_or(0)
        ));
        fs::create_dir_all(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        root
    }
}
