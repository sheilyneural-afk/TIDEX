use crate::foundation::error::{BrainError, BrainResult};
use crate::foundation::linalg::dot;
use std::collections::BTreeSet;

pub fn validate_values(values: &[f64], dimension: usize) -> BrainResult<()> {
    if values.len() != dimension || values.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Invalid("receiver_weight_response_shape_or_nonfinite".into()));
    }
    Ok(())
}

pub fn readout_dot_roundoff_bound(weights: &[f64], input: &[f64]) -> BrainResult<f64> {
    let dimension = weights.len();
    if dimension == 0 || dimension != input.len() || dimension > 4_096 {
        return Err(BrainError::Invalid("readout_dot_bound_shape".into()));
    }
    let absolute_weights = weights.iter().map(|value| value.abs()).collect::<Vec<_>>();
    let absolute_input = input.iter().map(|value| value.abs()).collect::<Vec<_>>();
    let absolute_sum = dot(&absolute_weights, &absolute_input)?;
    let unit32 = 2.0_f64.powi(-24);
    let unit64 = 2.0_f64.powi(-53);
    let count32 = dimension as f64;
    let count64 = (8 * dimension + 8) as f64;
    let gamma32 = (count32 * unit32) / (1.0 - count32 * unit32);
    let gamma64 = (count64 * unit64) / (1.0 - count64 * unit64);
    let upper_sum = absolute_sum / (1.0 - gamma64);
    let underflow = (2 * dimension + 1) as f64 * 2.0_f64.powi(-150);
    let bound = (gamma32 + gamma64) * upper_sum + underflow;
    if !bound.is_finite() || bound < 0.0 {
        return Err(BrainError::Invalid("readout_dot_bound_nonfinite".into()));
    }
    Ok(bound)
}

pub fn families_include(required: &[&str], families: &BTreeSet<String>) -> bool {
    required.iter().all(|r| families.contains(*r))
}

pub fn is_full_layer_coverage(total_layers: usize, covered_layers: &BTreeSet<usize>) -> bool {
    if total_layers == 0 {
        return false;
    }
    covered_layers.len() == total_layers && covered_layers.iter().all(|&i| i < total_layers)
}

pub fn compute_writable_fraction(writable: u64, total: u64) -> BrainResult<f64> {
    if total == 0 {
        return Err(BrainError::Invalid("writable_fraction_total_zero".into()));
    }
    let fraction = writable as f64 / total as f64;
    if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
        return Err(BrainError::Invalid("writable_fraction_invalid".into()));
    }
    Ok(fraction)
}

pub fn distributed_permitted_from_primitives(
    axis_construction_is_calibration: bool,
    learned_parameter_count_per_axis: u64,
    attention_and_mlp_coverage: bool,
    full_layer_coverage: bool,
) -> bool {
    axis_construction_is_calibration
        && learned_parameter_count_per_axis > 0
        && attention_and_mlp_coverage
        && full_layer_coverage
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receiver_validation_contracts_fail_closed() {
        assert!(validate_values(&[1.0, 2.0], 2).is_ok());
        assert!(validate_values(&[1.0, f64::NAN], 2).is_err());
        assert!(readout_dot_roundoff_bound(&[1.0, 2.0], &[3.0, 4.0]).unwrap() >= 0.0);
        assert!(readout_dot_roundoff_bound(&[], &[]).is_err());
        let families = BTreeSet::from(["attention".to_string(), "mlp".to_string()]);
        assert!(families_include(&["attention", "mlp"], &families));
        assert!(!families_include(&["missing"], &families));
        let layers = BTreeSet::from([0usize, 1usize, 2usize]);
        assert!(is_full_layer_coverage(3, &layers));
        assert!(!is_full_layer_coverage(2, &layers));
        assert_eq!(compute_writable_fraction(2, 4).unwrap(), 0.5);
        assert!(compute_writable_fraction(5, 4).is_err());
        assert!(distributed_permitted_from_primitives(true, 1, true, true));
        assert!(!distributed_permitted_from_primitives(false, 1, true, true));
    }
}
