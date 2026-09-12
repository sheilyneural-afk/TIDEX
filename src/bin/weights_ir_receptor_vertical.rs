//! CLI composition root for Weights/Hybrid → CapabilityIR → receptor vertical.
//!
//! Separate from `procedure_selector_vertical` (Software). Fail-closed when the
//! measured warrant does not admit Weights/Hybrid.

use serde::Serialize;
use std::path::PathBuf;
use tidex::foundation::error::BrainResult;
use tidex::foundation::security::secure_dir;
use tidex::governance::residency_decision::ResidencyDecision;
use tidex::governance::weights_ir_receptor_vertical::{
    run_weights_ir_receptor_vertical, WeightsIrReceptorVerticalReceipt,
};

const DEMO_SCHEMA: &str = "tidex.weights_ir_receptor_demo_receipt/v1";

#[derive(Debug, Clone, Serialize)]
pub struct WeightsIrReceptorDemoReceipt {
    pub schema: String,
    pub vertical: WeightsIrReceptorVerticalReceipt,
    pub residency_outcome: String,
    pub capability_ir_emitted: bool,
    pub receptor_entered: bool,
    pub authorizes_production: bool,
    pub note: String,
}

pub fn run_demo(tidex_home: PathBuf) -> BrainResult<WeightsIrReceptorDemoReceipt> {
    let private = tidex_home.join("private");
    std::fs::create_dir_all(&private)?;
    secure_dir(&private)?;
    let private = private.canonicalize().map_err(|e| {
        tidex::foundation::error::BrainError::Integrity(format!("private_root_unreadable:{e}"))
    })?;
    // CapabilityIR persist / capture require the configured private root.
    std::env::set_var("TIDEX_PRIVATE_ROOT", &private);

    let vertical = run_weights_ir_receptor_vertical(&private)?;
    vertical.verify()?;
    let residency_outcome = match vertical.residency_decision() {
        ResidencyDecision::Software {} => "software",
        ResidencyDecision::Weights {} => "weights",
        ResidencyDecision::Hybrid {} => "hybrid",
        ResidencyDecision::Blocked { .. } => "blocked",
        ResidencyDecision::BoundedUnknown { .. } => "bounded_unknown",
    }
    .to_string();
    Ok(WeightsIrReceptorDemoReceipt {
        schema: DEMO_SCHEMA.into(),
        capability_ir_emitted: vertical.capability_ir_emitted(),
        receptor_entered: vertical.receptor_entered(),
        residency_outcome,
        note: "weights/hybrid→IR→receptor vertical from measured closed linear map; GPEM procedure-selector remains Software-only".into(),
        vertical,
        authorizes_production: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn demo_weights_emits_ir_and_enters_receptor() {
        let home = std::env::temp_dir().join(format!(
            "tidex-weights-demo-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let receipt = run_demo(home.clone()).unwrap();
        assert_eq!(receipt.residency_outcome, "weights");
        assert!(receipt.capability_ir_emitted);
        assert!(receipt.receptor_entered);
        let _ = std::fs::remove_dir_all(&home);
    }
}
