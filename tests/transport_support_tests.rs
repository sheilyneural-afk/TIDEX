use cerebro_tidex::transport::{functional_leverage, functional_support_envelope};
use cerebro_tidex::error::BrainResult;

#[test]
fn support_envelope_small_example() -> BrainResult<()> {
    let calibration = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0], vec![2.0, 0.5], vec![0.5, 2.0]];
    let query = vec![0.9, 0.1];
    let ridge = 1e-3;
    let (query_score, max_loo) = functional_support_envelope(&calibration, &query, ridge)?;
    assert!(query_score.is_finite());
    assert!(max_loo.is_finite());
    assert!(query_score <= max_loo + 1e-12);
    Ok(())
}
