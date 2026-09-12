//! CLI / composition root for Paso 6 procedure-selector vertical.
//!
//! Library pipeline lives in
//! `tidex::governance::procedure_selector_vertical`. This bin module prints the
//! JSON receipt for `tidex demo procedure-selector`.
//!
//! **Fail-closed:** when GPEM/donor is unavailable or cannot seal live
//! evidence the demo ends hard (`gpem_v2_recommend_donor_unavailable` /
//! `…_insufficient_live_evidence` / …). No fixture substitute. No synthetic
//! second-tick ProceduralWorkflowHint (that violated frozen acceptance).

use serde::Serialize;
use std::path::PathBuf;
use tidex::foundation::error::BrainResult;
use tidex::governance::procedure_selector_vertical::{
    run_procedure_selector_vertical, ProcedureSelectorVerticalReceipt,
};

const DEMO_RECEIPT_SCHEMA: &str = "tidex.procedure_selector_demo_receipt/v1";

#[derive(Debug, Clone, Serialize)]
pub struct ProcedureSelectorDemoReceipt {
    pub schema: String,
    pub vertical: ProcedureSelectorVerticalReceipt,
    /// Always none on the productive path — real second ticks belong to the
    /// B-loop proof (`workflow_b_loop`), not fabricated procedural hints.
    pub second_tick: Option<()>,
    pub authorizes_production: bool,
    pub note: String,
}

pub fn run_demo(gpem_store_root: PathBuf) -> BrainResult<ProcedureSelectorDemoReceipt> {
    let vertical = run_procedure_selector_vertical(gpem_store_root)?;
    Ok(ProcedureSelectorDemoReceipt {
        schema: DEMO_RECEIPT_SCHEMA.into(),
        vertical,
        second_tick: None,
        authorizes_production: false,
        note: "productive demo: live SHEI/GPEM only; fail-closed if unavailable/insufficient; no fixture substitute; no synthetic second-tick".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn demo_fail_closed_without_fixture_or_synthetic_tick() {
        let root =
            std::env::temp_dir().join(format!("tidex-paso6-demo-{}-{}", std::process::id(), "cli"));
        let _ = fs::remove_dir_all(&root);
        let store = root.join("gpem");
        fs::create_dir_all(&store).unwrap();
        fs::write(store.join(".tidex_gpem_force_unavailable"), b"1").unwrap();
        let err = run_demo(store).unwrap_err().to_string();
        assert!(
            err.contains("gpem_v2_recommend_donor_unavailable")
                || err.contains("gpem_v2_recommend_donor_misconfigured")
                || err.contains("gpem_v2_recommend_insufficient_live_evidence"),
            "demo must not continue with fixture: {err}"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
