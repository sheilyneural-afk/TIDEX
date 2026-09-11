use cerebro_tidex::universality_evidence::UniversalityEvidenceInput;
use std::path::Path;

#[test]
fn universality_reducer_matches_trial_predicate() {
    let path = Path::new("quality/experiments/universality_heldout_real.json");
    let input: UniversalityEvidenceInput = serde_json::from_slice(&std::fs::read(path).expect("read input"))
        .expect("parse input");
    let receipt = input.execute().expect("execute reducer");

    // recompute successes directly
    let mut recomputed_global_successes: usize = 0;
    let mut recomputed_per_capability = std::collections::BTreeMap::new();
    for trial in &input.trials {
        if trial.passes(&input.protocol) {
            recomputed_global_successes += 1;
            *recomputed_per_capability.entry(trial.capability_id.clone()).or_insert(0) += 1;
        }
    }
    let receipt_global_successes: usize = receipt.capabilities.iter().map(|c| c.successes).sum();

    assert_eq!(recomputed_global_successes, receipt_global_successes, "Reducer global successes must match per-trial recomputation");
    // Also ensure no silent zeroing: if recomputed shows passes, receipt must reflect them
    if recomputed_global_successes > 0 {
        assert!(receipt.global_success_rate > 0.0, "receipt global_success_rate should be > 0 when trials pass");
    }
}
