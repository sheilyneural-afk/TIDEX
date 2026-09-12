//! Live Start proof: cataloged HF SmolLM → operator Start → Completed+succeeded+run.
//!
//! No mocks/stubs/shims. Fail-closed if the runner cannot complete.

use crate::workflow_b_loop::chain_start_accepted;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use tidex::foundation::digest::Sha256Digest;
use tidex::foundation::error::{BrainError, BrainResult};
use tidex::operator::control_plane::{
    catalog_local_models, load_job_record, start_operator_direct_job, OperatorDirectOperation,
    OperatorDirectWorkflowRequest, OperatorJobState,
};

const PROOF_SCHEMA: &str = "tidex.start_live_hf_proof/v1";

fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartLiveStageReceipt {
    pub operation: String,
    pub model_ids: Vec<String>,
    pub job_id: String,
    pub state: String,
    pub chain_accepted: bool,
    pub evidence_succeeded: Option<bool>,
    pub run_present: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartLiveHfProofReceipt {
    pub schema: String,
    pub tidex_home: String,
    pub hub_root: String,
    pub cataloged_model_count: usize,
    pub probe: StartLiveStageReceipt,
    pub align: Option<StartLiveStageReceipt>,
    pub pass: bool,
    pub authorizes_production: bool,
    pub notes: Vec<String>,
}

fn wait_job_terminal_long(
    tidex_home: &Path,
    job_id: &Sha256Digest,
    max_secs: u64,
) -> BrainResult<tidex::operator::control_plane::OperatorJobRecord> {
    let polls = (max_secs * 10).max(1);
    for _ in 0..polls {
        let record = load_job_record(tidex_home, job_id)?;
        if matches!(
            record.state,
            OperatorJobState::Completed | OperatorJobState::Failed | OperatorJobState::Cancelled
        ) {
            return Ok(record);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(invalid("start_live_job_did_not_reach_terminal_state"))
}

fn stage_from_terminal(
    operation: &str,
    model_ids: &[String],
    terminal: &tidex::operator::control_plane::OperatorJobRecord,
) -> StartLiveStageReceipt {
    StartLiveStageReceipt {
        operation: operation.into(),
        model_ids: model_ids.to_vec(),
        job_id: terminal.job_id.to_string(),
        state: format!("{:?}", terminal.state),
        chain_accepted: chain_start_accepted(terminal),
        evidence_succeeded: terminal
            .evidence_receipt
            .as_ref()
            .map(|r| r.succeeded),
        run_present: terminal.run.is_some(),
        error: terminal.error.clone(),
    }
}

fn pick_smollm_pair(
    models: &[tidex::operator::control_plane::LocalModelCandidate],
) -> BrainResult<(String, String)> {
    let mut base = None;
    let mut instruct = None;
    for m in models {
        let root = m.root.to_string_lossy();
        if root.contains("SmolLM2-135M-Instruct") {
            instruct = Some(m.model_id.to_string());
        } else if root.contains("SmolLM2-135M") && !root.contains("Instruct") {
            base = Some(m.model_id.to_string());
        }
    }
    match (base, instruct) {
        (Some(a), Some(b)) => Ok((a, b)),
        _ => Err(invalid("start_live_smollm_pair_missing")),
    }
}

fn run_direct(
    tidex_home: &Path,
    operation: OperatorDirectOperation,
    model_ids: Vec<Sha256Digest>,
    parameters: serde_json::Value,
    max_wait_secs: u64,
) -> BrainResult<StartLiveStageReceipt> {
    let op_name = match operation {
        OperatorDirectOperation::ProbeRuntime => "probe_runtime",
        OperatorDirectOperation::CalibrateAlignment => "calibrate_alignment",
        _ => return Err(invalid("start_live_operation_unsupported")),
    };
    let request = OperatorDirectWorkflowRequest {
        schema: "tidex.operator_direct_workflow/v1".into(),
        operation,
        model_ids: model_ids.clone(),
        dataset_sha256: None,
        parameters,
    };
    let job = start_operator_direct_job(tidex_home, request)?;
    let terminal = wait_job_terminal_long(tidex_home, &job.job_id, max_wait_secs)?;
    let ids: Vec<String> = model_ids.iter().map(|id| id.to_string()).collect();
    Ok(stage_from_terminal(op_name, &ids, &terminal))
}

/// Catalog HF hub models into `tidex_home`, then Start probe + align live.
pub fn prove_start_live_hf(
    tidex_home: &Path,
    hub_root: &Path,
) -> BrainResult<StartLiveHfProofReceipt> {
    if !tidex_home.is_absolute() || !hub_root.is_absolute() {
        return Err(invalid("start_live_paths_must_be_absolute"));
    }
    fs::create_dir_all(tidex_home)?;
    let cataloged = catalog_local_models(tidex_home, hub_root)?;
    if cataloged.is_empty() {
        return Err(invalid("start_live_no_models_cataloged"));
    }
    let (base_id, instruct_id) = pick_smollm_pair(&cataloged)?;
    let base = Sha256Digest::parse(&base_id)?;
    let instruct = Sha256Digest::parse(&instruct_id)?;

    let mut notes = vec![
        format!("cataloged_models={}", cataloged.len()),
        format!("base_model={base_id}"),
        format!("instruct_model={instruct_id}"),
        "probe_runtime then calibrate_alignment; chain success = Completed+succeeded+run".into(),
    ];

    let probe = run_direct(
        tidex_home,
        OperatorDirectOperation::ProbeRuntime,
        vec![base.clone()],
        json!({}),
        900,
    )?;
    if !probe.chain_accepted {
        notes.push(format!(
            "probe_runtime failed closed: state={} error={:?}",
            probe.state, probe.error
        ));
        return Ok(StartLiveHfProofReceipt {
            schema: PROOF_SCHEMA.into(),
            tidex_home: tidex_home.display().to_string(),
            hub_root: hub_root.display().to_string(),
            cataloged_model_count: cataloged.len(),
            probe,
            align: None,
            pass: false,
            authorizes_production: false,
            notes,
        });
    }
    notes.push("probe_runtime Completed+succeeded+run".into());

    let align = run_direct(
        tidex_home,
        OperatorDirectOperation::CalibrateAlignment,
        vec![base, instruct],
        json!({
            "source_layer": 0,
            "target_layer": 0,
            "training_prompts": ["The capital of France is", "2 + 2 ="],
            "validation_prompts": ["The largest planet is", "10 - 3 ="]
        }),
        1_800,
    )?;
    let pass = align.chain_accepted;
    if pass {
        notes.push("calibrate_alignment Completed+succeeded+run".into());
    } else {
        notes.push(format!(
            "calibrate_alignment failed closed: state={} error={:?}",
            align.state, align.error
        ));
    }

    Ok(StartLiveHfProofReceipt {
        schema: PROOF_SCHEMA.into(),
        tidex_home: tidex_home.display().to_string(),
        hub_root: hub_root.display().to_string(),
        cataloged_model_count: cataloged.len(),
        probe,
        align: Some(align),
        pass,
        authorizes_production: false,
        notes,
    })
}

pub fn default_hub_root() -> PathBuf {
    PathBuf::from("/home/yo/Future/runtime/llms/huggingface/hub")
}
