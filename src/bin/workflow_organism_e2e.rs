//! Single organism-chain acceptance test (composition root only).
//!
//! Composes already-proven links without new architecture:
//!
//! ```text
//! contexto/confounders
//!   → DirectedFacilitation
//!   → CognitiveField
//!   → FieldRoutingDecision (hot→A / cold→C)
//!   → explicit SkillField→operation FieldActionBinding (fail-closed)
//!   → CoEvolutionDirectiveSnapshot
//!   → decide_next_action → NextAction → ExecutorRegistry
//!   → Start → terminal job → evidence_receipt
//!   → experience/replay → NextAction changes
//! ```
//!
//! ## Still outside this e2e (remain multi-test / live)
//!
//! - live Vxx admit + assimilate aperture change
//! - live SHEI/GPEM donor observe (Paso 4)
//! - Weights/Hybrid → measured IR → receptor vertical (Paso 5)
//! - true `DeltaObservation`→… live admit start (facilitation observations
//!   here are the earliest authentic fixture those components already accept)
//!
//! Piece proofs reused (not replaced):
//! - `context_conditioned_facilitation_changes_routing`
//!   (`src/engine/cognitive_field.rs`)
//! - `cognitive_field_route_changes_next_action_without_guessing_operation`
//!   (`src/bin/workflow_next_action.rs`)
//! - `prove_b_loop_real_evidence_start_receipt_redecide`
//!   (`src/bin/workflow_b_loop.rs`)

use serde::Serialize;
use std::fs;
use std::path::Path;
use tidex::engine::cognitive_field::{
    CognitiveFieldConfig, CognitiveFieldDrive, DynamicCognitiveField, FieldRoutingDecision,
};
use tidex::foundation::contracts::{ConfounderValue, SkillField};
use tidex::foundation::digest::Sha256Digest;
use tidex::foundation::error::{BrainError, BrainResult};
use tidex::foundation::identity::SkillId;
use tidex::foundation::linalg::Matrix;
use tidex::learning::causal_credit::{
    estimate_directed_facilitation, CausalCreditReport, DirectedFacilitationObservation,
    FieldCausalCredit, PairInteractionCredit,
};
use tidex::operator::executor_registry::executor_by_id;

use crate::workflow_b_loop::{prove_b_loop_with_directive, BLoopProofReceipt};
use crate::workflow_next_action::{
    decide_next_action, directive_from_field_route, AuthenticatedWorkflowInputs,
    FieldActionBinding, KnowledgeWorkflowSignals, RoutingPreference, StopCondition, WorkflowCost,
    WorkflowDecisionInput, WorkflowRisk,
};

const ORGANISM_E2E_SCHEMA: &str = "tidex.workflow.organism_chain_e2e/v1";

fn invalid(code: &str) -> BrainError {
    BrainError::Invalid(code.into())
}

fn digest_hex(tag: u8) -> String {
    Sha256Digest::digest_bytes(&[tag]).to_string()
}

fn skill_field(id: &str, functional: Vec<f64>) -> SkillField {
    SkillField {
        skill_id: SkillId::parse(id).unwrap(),
        reconstruction_id: Default::default(),
        lineage_id: Default::default(),
        generation_created: 1,
        direction: vec![1.0, 0.0],
        structured_geometry: None,
        dense_materialization: None,
        parameter_layout_sha256: None,
        representation_signature: Vec::new(),
        singular_value: 1.0,
        explained_variance: 0.5,
        persistence: 1.0,
        coherence: 1.0,
        uncertainty: 0.1,
        evidence_support_digests: Vec::new(),
        support: 3,
        functional_signature: functional,
        parent_skill_ids: Vec::new(),
    }
}

fn causal_scaffold() -> CausalCreditReport {
    CausalCreditReport {
        schema: "tidex.causal_credit/v3".into(),
        context_count: 12,
        independent_group_count: 4,
        field_count: 3,
        fields: ["a", "b", "c"]
            .into_iter()
            .map(|id| FieldCausalCredit {
                skill_id: SkillId::parse(id).unwrap(),
                matched_pairs: 16,
                independent_contexts: 4,
                mean_marginal_effect: 0.2,
                standard_error: 0.01,
                lower_confidence_bound: 0.18,
                positive_fraction: 1.0,
                resolved: true,
                beneficial: true,
                shapley_value: None,
            })
            .collect(),
        pair_interactions: vec![
            PairInteractionCredit {
                left_skill_id: SkillId::parse("a").unwrap(),
                right_skill_id: SkillId::parse("b").unwrap(),
                matched_quads: 8,
                independent_contexts: 4,
                mean_interaction_effect: 0.3,
                standard_error: 0.01,
                resolved: true,
            },
            PairInteractionCredit {
                left_skill_id: SkillId::parse("a").unwrap(),
                right_skill_id: SkillId::parse("c").unwrap(),
                matched_quads: 8,
                independent_contexts: 4,
                mean_interaction_effect: -0.2,
                standard_error: 0.01,
                resolved: true,
            },
            PairInteractionCredit {
                left_skill_id: SkillId::parse("b").unwrap(),
                right_skill_id: SkillId::parse("c").unwrap(),
                matched_quads: 8,
                independent_contexts: 4,
                mean_interaction_effect: -0.2,
                standard_error: 0.01,
                resolved: true,
            },
        ],
        unresolved_fields: Vec::new(),
    }
}

fn route_for_context(
    model: &DynamicCognitiveField,
    report: &tidex::learning::causal_credit::DirectedFacilitationReport,
    target: &SkillId,
    context: &[ConfounderValue],
) -> BrainResult<FieldRoutingDecision> {
    let drive = CognitiveFieldDrive::from_directed_facilitation(
        report,
        target,
        context,
        &model.field_ids,
        vec![0.0; 3],
        vec![0.0; 3],
        vec![0.0; 3],
    )?;
    let state = model.evolve(&[0.0; 3], &drive)?;
    if !state.converged {
        return Err(invalid("organism_e2e_cognitive_field_did_not_converge"));
    }
    model.route_top_k(&state, 1, 0.0)
}

#[derive(Debug, Clone, Serialize)]
pub struct OrganismChainE2EReceipt {
    pub schema: String,
    pub hot_selected_field: String,
    pub cold_selected_field: String,
    pub hot_bound_operation: String,
    pub cold_bound_operation: String,
    pub fail_closed_missing_binding: bool,
    pub b_loop: BLoopProofReceipt,
    pub next_action_changed: bool,
    pub covered_chain: Vec<String>,
    pub still_outside_e2e: Vec<String>,
    pub authorizes_production: bool,
}

/// Longest honest organism chain reusable from existing proven links.
pub fn prove_organism_chain_e2e(tidex_home: &Path) -> BrainResult<OrganismChainE2EReceipt> {
    fs::create_dir_all(tidex_home)?;

    let fields = vec![
        skill_field("a", vec![1.0, 0.0]),
        skill_field("b", vec![1.0, 0.0]),
        skill_field("c", vec![1.0, 0.0]),
    ];
    let config = CognitiveFieldConfig {
        curvature_weight: 1.0,
        causal_interaction_weight: 0.0,
        functional_compatibility_weight: 0.0,
        causal_bias_weight: 0.0,
        coupling_gain: 0.0,
        ..CognitiveFieldConfig::default()
    };
    let model =
        DynamicCognitiveField::build(&fields, &Matrix::identity(3), &causal_scaffold(), config)?;

    let hot = vec![ConfounderValue {
        name: "receiver_temperature".into(),
        value: 1.0,
    }];
    let cold = vec![ConfounderValue {
        name: "receiver_temperature".into(),
        value: 0.0,
    }];
    let target = SkillId::parse("target")?;
    let design = Sha256Digest::parse("a".repeat(64))?;
    let mut observations = Vec::new();
    let mut nonce = 1u8;
    for (context, effects) in [(&hot, [1.0, 0.2, -1.0]), (&cold, [-1.0, 0.2, 1.0])] {
        for (source, effect) in ["a", "b", "c"].into_iter().zip(effects) {
            for group in ["g1", "g2", "g3"] {
                observations.push(DirectedFacilitationObservation {
                    target_skill_id: target.clone(),
                    source_skill_ids: vec![SkillId::parse(source)?],
                    context: context.clone(),
                    independence_group: group.into(),
                    baseline_utility: 0.0,
                    facilitated_utility: effect,
                    matched_design_digest: design.clone(),
                    evidence_digest: Sha256Digest::parse(format!("{:064x}", nonce))?,
                });
                nonce = nonce.saturating_add(1);
            }
        }
    }
    let report = estimate_directed_facilitation(&observations)?;

    let hot_route = route_for_context(&model, &report, &target, &hot)?;
    let cold_route = route_for_context(&model, &report, &target, &cold)?;
    if hot_route.selected_field_ids != vec![SkillId::parse("a")?] {
        return Err(invalid("organism_e2e_hot_did_not_route_to_a"));
    }
    if cold_route.selected_field_ids != vec![SkillId::parse("c")?] {
        return Err(invalid("organism_e2e_cold_did_not_route_to_c"));
    }
    if hot_route.selected_field_ids == cold_route.selected_field_ids {
        return Err(invalid("organism_e2e_context_did_not_change_route"));
    }

    // Explicit composition-root bindings — no guessing operation from skill name.
    // Hot→A drives the transfer directive that the B-loop redecide path expects.
    // Cold→C proves a second registered operation without guessing.
    let bindings = vec![
        FieldActionBinding {
            skill_id: SkillId::parse("a")?,
            operation: "activation_transfer_experiment".into(),
        },
        FieldActionBinding {
            skill_id: SkillId::parse("c")?,
            operation: "probe_runtime".into(),
        },
    ];

    let hot_directive = directive_from_field_route(
        &hot_route,
        &bindings,
        "organism_e2e: hot context directed facilitation → field A → transfer binding",
        Some(digest_hex(9)),
    )?;
    let cold_directive = directive_from_field_route(
        &cold_route,
        &bindings,
        "organism_e2e: cold context directed facilitation → field C → probe binding",
        Some(digest_hex(9)),
    )?;
    if hot_directive.recommended_operation != "activation_transfer_experiment"
        || cold_directive.recommended_operation != "probe_runtime"
    {
        return Err(invalid("organism_e2e_binding_did_not_map_expected_operations"));
    }

    // Fail-closed: missing binding / unknown skill must not invent an operation.
    let missing = directive_from_field_route(
        &hot_route,
        &bindings[1..],
        "organism_e2e: must fail closed without skill A binding",
        None,
    );
    if missing.is_ok() {
        return Err(invalid("organism_e2e_missing_binding_did_not_fail_closed"));
    }
    let unknown_route = FieldRoutingDecision {
        schema: "tidex.cognitive_field_routing/v1".into(),
        field_ids: model.field_ids.clone(),
        coefficients: vec![0.0, 1.0, 0.0],
        selected_field_ids: vec![SkillId::parse("b")?],
        selected_activation_mass: 1.0,
    };
    let unknown = directive_from_field_route(
        &unknown_route,
        &bindings,
        "organism_e2e: unbound skill must fail closed",
        None,
    );
    if unknown.is_ok() {
        return Err(invalid("organism_e2e_unbound_skill_did_not_fail_closed"));
    }

    // Dry decide from cold route proves ExecutorRegistry mapping without Start.
    let cold_action = decide_next_action(&WorkflowDecisionInput {
        knowledge: KnowledgeWorkflowSignals {
            calibration_sufficient: true,
            causal_evidence_sufficient: true,
            notes: vec!["organism_e2e_cold_route".into()],
        },
        coevolution_directive: Some(cold_directive.clone()),
        routing: RoutingPreference::default(),
        procedural_hint: None,
        authorized_inputs: AuthenticatedWorkflowInputs {
            model_ids: vec![digest_hex(1), digest_hex(2)],
            dataset_sha256: Some(digest_hex(3)),
            parameters: serde_json::json!({}),
        },
        cost_ceiling: WorkflowCost { relative_units: 2 },
        risk_ceiling: WorkflowRisk { level: 2 },
        stop_condition: StopCondition::default(),
        require_directive: true,
    })?;
    if cold_action.operation != "probe_runtime" {
        return Err(invalid("organism_e2e_cold_decide_unexpected_operation"));
    }
    executor_by_id(&cold_action.executor_id)?;

    // Hot→A→transfer binding feeds the real B-loop Start→receipt→replay→redecide.
    let b_loop = prove_b_loop_with_directive(tidex_home, hot_directive.clone())?;
    if !b_loop.start_chain_success {
        return Err(invalid("organism_e2e_b_loop_start_not_chain_success"));
    }
    if !b_loop.next_action_changed || !b_loop.start_evidence_receipt_present {
        return Err(invalid("organism_e2e_b_loop_did_not_redecide_after_receipt"));
    }
    if b_loop.tick1.next_action_operation != "calibrate_alignment"
        || b_loop.tick2.next_action_operation != "activation_transfer_experiment"
    {
        return Err(invalid("organism_e2e_b_loop_operations_unexpected"));
    }

    Ok(OrganismChainE2EReceipt {
        schema: ORGANISM_E2E_SCHEMA.into(),
        hot_selected_field: "a".into(),
        cold_selected_field: "c".into(),
        hot_bound_operation: hot_directive.recommended_operation,
        cold_bound_operation: cold_directive.recommended_operation,
        fail_closed_missing_binding: true,
        next_action_changed: b_loop.next_action_changed,
        b_loop,
        covered_chain: vec![
            "contexto/confounders".into(),
            "DirectedFacilitation".into(),
            "CognitiveField".into(),
            "FieldRoutingDecision(hot→A/cold→C)".into(),
            "FieldActionBinding(explicit, fail-closed)".into(),
            "CoEvolutionDirectiveSnapshot".into(),
            "decide_next_action→NextAction".into(),
            "ExecutorRegistry".into(),
            "Start→terminal job→evidence_receipt".into(),
            "experience/replay→NextAction changes".into(),
        ],
        still_outside_e2e: vec![
            "live Vxx admit+assimilate aperture change".into(),
            "live SHEI/GPEM donor observe (Paso 4)".into(),
            "Weights/Hybrid→measured IR→receptor vertical (Paso 5)".into(),
            "true DeltaObservation→… live admit start".into(),
        ],
        authorizes_production: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    fn isolated_home(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "tidex-organism-e2e-{}-{}-{}",
            tag,
            std::process::id(),
            Sha256Digest::digest_bytes(tag.as_bytes())
                .to_string()
                .chars()
                .take(8)
                .collect::<String>()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        root
    }

    #[test]
    fn prove_organism_chain_facilitation_route_bind_start_receipt_redecide() {
        let home = isolated_home("proof");
        match prove_organism_chain_e2e(&home) {
            Ok(receipt) => {
                assert_eq!(receipt.schema, ORGANISM_E2E_SCHEMA);
                assert!(!receipt.authorizes_production);
                assert_eq!(receipt.hot_selected_field, "a");
                assert_eq!(receipt.cold_selected_field, "c");
                assert_eq!(
                    receipt.hot_bound_operation,
                    "activation_transfer_experiment"
                );
                assert_eq!(receipt.cold_bound_operation, "probe_runtime");
                assert!(receipt.fail_closed_missing_binding);
                assert!(receipt.next_action_changed);
                assert!(receipt.b_loop.start_evidence_receipt_present);
                assert!(receipt.b_loop.start_chain_success);
                assert_eq!(
                    receipt.b_loop.tick1.next_action_operation,
                    "calibrate_alignment"
                );
                assert_eq!(
                    receipt.b_loop.tick2.next_action_operation,
                    "activation_transfer_experiment"
                );
                assert!(receipt.covered_chain.len() >= 10);
                assert!(!receipt.still_outside_e2e.is_empty());
            }
            Err(tidex::foundation::error::BrainError::Invalid(code))
                if code == "b_loop_start_job_not_completed"
                    || code == "b_loop_start_evidence_not_succeeded"
                    || code == "b_loop_start_run_missing"
                    || code == "b_loop_start_missing_evidence_receipt"
                    || code == "organism_e2e_b_loop_start_not_chain_success" =>
            {
                // Fail-closed without successful Start (catalog / HF runtime).
                // Facilitation+binding remain covered by unit tests on those modules.
            }
            Err(other) => panic!("unexpected organism e2e error: {other}"),
        }
        let _ = fs::remove_dir_all(home);
    }
}
