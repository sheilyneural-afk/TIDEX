//! CLI / composition root for Paso 6 procedure-selector vertical.
//!
//! Library pipeline lives in
//! `tidex::governance::procedure_selector_vertical`. This bin module adds:
//! - JSON receipt printing for `tidex demo procedure-selector`
//! - optional second-tick: after Software stop, a synthetic procedural hint
//!   changes `decide_next_action` (Paso 3 metaplasticity story)

use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;
use tidex::foundation::digest::Sha256Digest;
use tidex::foundation::error::BrainResult;
use tidex::governance::procedure_selector_vertical::{
    run_procedure_selector_vertical, ProcedureSelectorVerticalReceipt,
};
use tidex::governance::residency_decision::ResidencyDecision;

use crate::workflow_next_action::{
    decide_next_action, AuthenticatedWorkflowInputs, CoEvolutionDirectiveSnapshot,
    KnowledgeWorkflowSignals, NextAction, ProceduralWorkflowHint, RoutingPreference, StopCondition,
    WorkflowCost, WorkflowDecisionInput, WorkflowRisk,
};

const DEMO_RECEIPT_SCHEMA: &str = "tidex.procedure_selector_demo_receipt/v1";

#[derive(Debug, Clone, Serialize)]
pub struct SecondTickComparison {
    pub first_executor_id: String,
    pub first_operation: String,
    pub second_executor_id: String,
    pub second_operation: String,
    pub changed: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcedureSelectorDemoReceipt {
    pub schema: String,
    pub vertical: ProcedureSelectorVerticalReceipt,
    pub second_tick: Option<SecondTickComparison>,
    pub authorizes_production: bool,
}

fn digest_hex(tag: u8) -> String {
    Sha256Digest::digest_bytes(&[tag]).to_string()
}

fn base_workflow_input(hint: ProceduralWorkflowHint) -> WorkflowDecisionInput {
    WorkflowDecisionInput {
        knowledge: KnowledgeWorkflowSignals {
            calibration_sufficient: true,
            causal_evidence_sufficient: true,
            notes: vec!["paso6_software_residency_experience".into()],
        },
        coevolution_directive: Some(CoEvolutionDirectiveSnapshot {
            recommended_operation: "activation_transfer_experiment".into(),
            reason: "paso6 synthetic directive after software residency".into(),
            converged: false,
            source_model: Some(digest_hex(1)),
            target_model: Some(digest_hex(2)),
            capability_hint: Some("procedure_selector_or_explore".into()),
            evidence_sha256: Some(digest_hex(9)),
        }),
        routing: RoutingPreference::default(),
        procedural_hint: Some(hint),
        authorized_inputs: AuthenticatedWorkflowInputs {
            model_ids: vec![digest_hex(1), digest_hex(2)],
            dataset_sha256: Some(digest_hex(3)),
            parameters: json!({}),
        },
        cost_ceiling: WorkflowCost { relative_units: 2 },
        risk_ceiling: WorkflowRisk { level: 2 },
        stop_condition: StopCondition::default(),
        require_directive: true,
    }
}

/// After Software residency, show that a learned procedural hint can change NextAction.
pub fn second_tick_after_software(
    vertical: &ProcedureSelectorVerticalReceipt,
) -> BrainResult<Option<SecondTickComparison>> {
    if !matches!(vertical.residency_decision(), ResidencyDecision::Software {}) {
        return Ok(None);
    }

    let first_hint = ProceduralWorkflowHint {
        steering_successes: 0,
        steering_failures: 5,
        low_rank_successes: 7,
        low_rank_failures: 0,
        top_family: Some("CholeskyRidgeLowRank".into()),
        top_disposition: Some("PrioritizeExploration".into()),
        notes: vec![
            "pre_software_stop:steering_unreliable".into(),
            format!("capacity:{}", vertical.capacity_key()),
        ],
    };
    let second_hint = ProceduralWorkflowHint {
        steering_successes: 5,
        steering_failures: 0,
        low_rank_successes: 7,
        low_rank_failures: 0,
        top_family: Some("HybridRuntime".into()),
        top_disposition: Some("PrioritizeExploration".into()),
        notes: vec![
            "post_software_residency:prefer_authenticated_software_binding".into(),
            "synthetic_learned_procedural_hint".into(),
        ],
    };

    let first: NextAction = decide_next_action(&base_workflow_input(first_hint))?;
    let second: NextAction = decide_next_action(&base_workflow_input(second_hint))?;
    Ok(Some(SecondTickComparison {
        first_executor_id: first.executor_id.clone(),
        first_operation: first.operation.clone(),
        second_executor_id: second.executor_id.clone(),
        second_operation: second.operation.clone(),
        changed: first.executor_id != second.executor_id,
        note: "Paso 3 metaplasticity story: synthetic procedural hint after Software residency changes NextAction (dry composition only).".into(),
    }))
}

pub fn run_demo(
    gpem_store_root: PathBuf,
    with_second_tick: bool,
) -> BrainResult<ProcedureSelectorDemoReceipt> {
    let vertical = run_procedure_selector_vertical(gpem_store_root)?;
    let second_tick = if with_second_tick {
        second_tick_after_software(&vertical)?
    } else {
        None
    };
    Ok(ProcedureSelectorDemoReceipt {
        schema: DEMO_RECEIPT_SCHEMA.into(),
        vertical,
        second_tick,
        authorizes_production: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tidex::governance::authenticated_capacity_residency::CapabilityIrStopReason;
    use tidex::governance::procedure_selector_vertical::VerticalTerminal;

    #[test]
    fn demo_software_path_and_second_tick_change() {
        let root =
            std::env::temp_dir().join(format!("tidex-paso6-demo-{}-{}", std::process::id(), "cli"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let receipt = run_demo(root.join("gpem"), true).unwrap();
        assert_eq!(receipt.schema, DEMO_RECEIPT_SCHEMA);
        assert!(!receipt.authorizes_production);
        assert_eq!(receipt.vertical.residency_decision(), &ResidencyDecision::Software {});
        assert!(matches!(
            receipt.vertical.terminal(),
            VerticalTerminal::StoppedHonestly {
                stop_reason: CapabilityIrStopReason::SoftwareResidency,
                receptor_entered: false,
            }
        ));
        let tick = receipt.second_tick.expect("second tick");
        assert!(tick.changed);
        assert_ne!(tick.first_executor_id, tick.second_executor_id);
        let _ = fs::remove_dir_all(&root);
    }
}
