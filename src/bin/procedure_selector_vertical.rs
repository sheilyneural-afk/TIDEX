//! CLI / composition root for Paso 6 procedure-selector vertical.
//!
//! Library pipeline lives in
//! `tidex::governance::procedure_selector_vertical`. This bin module prints the
//! JSON receipt for `tidex demo procedure-selector`.
//!
//! Productive chain (frozen acceptance):
//! 1. Seed governed GPEM store (real SHEI APIs)
//! 2. Live observe → seal AuthenticatedCapacity
//! 3. ResidencyDecision (Software is valid success for this vertical)
//! 4. Non-synthetic B-loop second tick via [`crate::workflow_b_loop::prove_b_loop`]
//!
//! **Fail-closed:** when GPEM/donor is unavailable the demo ends hard.
//! No fixture substitute. No fabricated ProceduralWorkflowHint.

use serde::Serialize;
use std::path::PathBuf;
use tidex::foundation::error::BrainResult;
use tidex::governance::procedure_selector_vertical::{
    seed_and_run_procedure_selector_vertical, ProcedureSelectorVerticalReceipt,
};
use tidex::governance::residency_decision::ResidencyDecision;

use crate::workflow_b_loop::{prove_b_loop, BLoopProofReceipt};

const DEMO_RECEIPT_SCHEMA: &str = "tidex.procedure_selector_demo_receipt/v1";

#[derive(Debug, Clone, Serialize)]
pub struct ProcedureSelectorDemoSecondTick {
    /// Real B-loop proof (evidence → replay → NextAction → Start → receipt → redecide).
    pub b_loop: BLoopProofReceipt,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcedureSelectorDemoReceipt {
    pub schema: String,
    pub seeded_trace_ids: Vec<String>,
    pub vertical: ProcedureSelectorVerticalReceipt,
    /// Present only when the non-synthetic B-loop proof succeeds.
    pub second_tick: Option<ProcedureSelectorDemoSecondTick>,
    pub residency_outcome: String,
    pub capability_ir_emitted: bool,
    pub receptor_entered: bool,
    pub authorizes_production: bool,
    pub note: String,
}

/// End-to-end Paso 6 operator demo against a seeded live GPEM store.
///
/// Uses `tidex_home` for the B-loop Operator job surface and `gpem_store_root`
/// for the governed SHEI/GPEM ledger. Fail-closed if either leg cannot run.
pub fn run_demo(
    tidex_home: PathBuf,
    gpem_store_root: PathBuf,
) -> BrainResult<ProcedureSelectorDemoReceipt> {
    let (seeded_trace_ids, vertical) = seed_and_run_procedure_selector_vertical(gpem_store_root)?;
    vertical.verify()?;

    let residency_outcome = match vertical.residency_decision() {
        ResidencyDecision::Software {} => "software".to_string(),
        ResidencyDecision::Weights {} => "weights".to_string(),
        ResidencyDecision::Hybrid {} => "hybrid".to_string(),
        ResidencyDecision::Blocked { .. } => "blocked".to_string(),
        ResidencyDecision::BoundedUnknown { .. } => "bounded_unknown".to_string(),
    };

    // Non-synthetic second tick: real numerical.evolve evidence → PM → NextAction
    // → Start → receipt → different NextAction. Composition-root only (bin).
    let b_loop = prove_b_loop(&tidex_home)?;
    let second_tick = ProcedureSelectorDemoSecondTick {
        b_loop,
        note: "second tick from workflow_b_loop::prove_b_loop (real evidence, no fabricated ProceduralWorkflowHint)".into(),
    };

    let note = if matches!(vertical.residency_decision(), ResidencyDecision::Software {}) {
        "paso6_demo: live SHEI/GPEM seed→seal→Software residency (honest stop, no IR/receptor) + real B-loop second tick; no fixture substitute".into()
    } else {
        "paso6_demo: live SHEI/GPEM seed→seal→residency + real B-loop second tick; IR/receptor only with measured Weights/Hybrid warrant".into()
    };

    Ok(ProcedureSelectorDemoReceipt {
        schema: DEMO_RECEIPT_SCHEMA.into(),
        seeded_trace_ids,
        capability_ir_emitted: vertical.capability_ir_emitted(),
        receptor_entered: vertical.receptor_entered(),
        residency_outcome,
        vertical,
        second_tick: Some(second_tick),
        authorizes_production: false,
        note,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tidex::foundation::digest::Sha256Digest;
    use tidex::governance::procedure_selector_vertical::seed_live_gpem_demo_store;

    fn isolated_home(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "tidex-paso6-demo-{}-{}-{}",
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
    fn demo_fail_closed_without_fixture_or_synthetic_tick() {
        let home = isolated_home("fail-closed");
        let store = home.join("gpem");
        fs::create_dir_all(&store).unwrap();
        fs::write(store.join(".tidex_gpem_force_unavailable"), b"1").unwrap();
        let err = run_demo(home.clone(), store).unwrap_err().to_string();
        assert!(
            err.contains("gpem_v2_recommend_donor_unavailable")
                || err.contains("gpem_v2_recommend_donor_misconfigured")
                || err.contains("gpem_v2_recommend_invoke_failed")
                || err.contains("gpem_v2_recommend_insufficient_live_evidence"),
            "demo must not continue with fixture: {err}"
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn demo_seeded_live_gpem_plus_real_b_loop_when_shei_available() {
        if !std::path::Path::new("/home/yo/Projects/SHEI/research_python").is_dir() {
            // CI without SHEI: fail-closed vocabulary still covered above.
            return;
        }
        let home = isolated_home("e2e");
        let store = home.join("state/demo/procedure_selector/gpem-store");
        let receipt = run_demo(home.clone(), store).expect("paso6 e2e demo");
        assert_eq!(receipt.schema, DEMO_RECEIPT_SCHEMA);
        assert!(!receipt.authorizes_production);
        assert!(!receipt.seeded_trace_ids.is_empty());
        assert_eq!(receipt.residency_outcome, "software");
        assert!(!receipt.capability_ir_emitted);
        assert!(!receipt.receptor_entered);
        let tick = receipt.second_tick.as_ref().expect("real second tick");
        assert!(tick.b_loop.next_action_changed);
        assert!(tick.b_loop.start_evidence_receipt_present);
        assert_eq!(tick.b_loop.tick1.next_action_operation, "calibrate_alignment");
        assert_eq!(tick.b_loop.tick2.next_action_operation, "activation_transfer_experiment");
        receipt.vertical.verify().unwrap();
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn seed_live_gpem_demo_store_writes_real_traces_when_shei_available() {
        if !std::path::Path::new("/home/yo/Projects/SHEI/research_python").is_dir() {
            return;
        }
        let home = isolated_home("seed-only");
        let store = home.join("gpem-store");
        let ids = seed_live_gpem_demo_store(store).expect("seed");
        assert!(ids.len() >= 3);
        let _ = fs::remove_dir_all(home);
    }
}
