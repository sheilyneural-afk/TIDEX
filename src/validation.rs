use crate::block_tomography::ParameterBlockLayout;
use crate::contracts::DeltaObservation;
use crate::error::{BrainError, BrainResult};
use crate::identity::ObservationId;
use crate::linalg::{dot, symmetric_eigen_jacobi_signed, Matrix};
use std::collections::BTreeSet;

/// Compatibility predicate for callers that only need a boolean.
///
/// `ObservationId` is the single authority for the persisted observation-ID
/// contract. Keeping this wrapper preserves existing call sites without
/// recreating a second validator that could drift from the typed boundary.
pub fn valid_observation_id(value: &str) -> bool {
    ObservationId::parse(value).is_ok()
}

/// Canonical identifier validator shared by all persistent manifests.
/// This keeps stable naming rules in one place, preventing divergent checks in
/// different subsystems of the same runtime.
pub fn validate_identifier(value: &str, label: &str, max_len: usize) -> BrainResult<()> {
    if value.is_empty()
        || value.len() > max_len
        || value == "."
        || value == ".."
        || value
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
    {
        return Err(BrainError::Invalid(format!("{label}_invalid")));
    }
    Ok(())
}

/// Validate a network endpoint used by external model providers.
pub fn validate_http_endpoint(value: &str, max_len: usize) -> BrainResult<()> {
    if value.is_empty() || value.len() > max_len || value.contains(char::is_whitespace) {
        return Err(BrainError::Invalid("model_endpoint_invalid".into()));
    }
    let normalized = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
        .ok_or_else(|| BrainError::Invalid("model_endpoint_scheme_invalid".into()))?;
    if normalized.is_empty() || normalized.contains(char::is_whitespace) {
        return Err(BrainError::Invalid("model_endpoint_invalid".into()));
    }
    Ok(())
}

/// Accept only real evidence weights. A missing or unusable observation must
/// be rejected by the owning protocol, never promoted to a small positive
/// weight by a numerical convenience clamp.
pub fn validate_reliability(value: f64, label: &str) -> BrainResult<f64> {
    if !value.is_finite() || value <= 0.0 || value > 1.0 {
        return Err(BrainError::Invalid(format!("{label}_reliability_invalid")));
    }
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupFold {
    pub holdout_group: String,
    pub train: Vec<usize>,
    pub test: Vec<usize>,
}

/// Indices whose coefficient is numerically material in a source mixture.
/// The tolerance is scale-relative and shared by memory, dense materialization,
/// dual-space reconstruction and structured geometry so those authorities do
/// not disagree about which observations support a field.
pub fn source_support_indices(mixture: &[f64]) -> BrainResult<Vec<usize>> {
    if mixture.is_empty() || mixture.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Invalid("source_mixture_invalid".into()));
    }
    let max_abs = mixture
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if max_abs <= 0.0 {
        return Ok(Vec::new());
    }
    let tolerance = max_abs * f64::EPSILON.sqrt();
    Ok(mixture
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (value.abs() > tolerance).then_some(index))
        .collect())
}

pub fn independence_group_folds(
    observations: &[DeltaObservation],
    minimum_groups: usize,
) -> BrainResult<Vec<GroupFold>> {
    if observations.is_empty()
        || minimum_groups < 2
        || observations
            .iter()
            .any(|observation| observation.independence_group.trim().is_empty())
    {
        return Err(BrainError::Invalid("grouped_cv_input_invalid".into()));
    }
    let groups = observations
        .iter()
        .map(|observation| observation.independence_group.clone())
        .collect::<BTreeSet<_>>();
    if groups.len() < minimum_groups {
        return Err(BrainError::Invalid(format!(
            "grouped_cv_requires_{minimum_groups}_independent_groups"
        )));
    }
    let mut folds = Vec::with_capacity(groups.len());
    for holdout_group in groups {
        let train = observations
            .iter()
            .enumerate()
            .filter_map(|(index, observation)| {
                (observation.independence_group != holdout_group).then_some(index)
            })
            .collect::<Vec<_>>();
        let test = observations
            .iter()
            .enumerate()
            .filter_map(|(index, observation)| {
                (observation.independence_group == holdout_group).then_some(index)
            })
            .collect::<Vec<_>>();
        if train.is_empty() || test.is_empty() {
            return Err(BrainError::Integrity("grouped_cv_empty_fold".into()));
        }
        folds.push(GroupFold {
            holdout_group,
            train,
            test,
        });
    }
    Ok(folds)
}

pub fn regression_r2(actual: &[Vec<f64>], predicted: &[Vec<f64>]) -> BrainResult<f64> {
    if actual.is_empty()
        || actual.len() != predicted.len()
        || actual[0].is_empty()
        || actual
            .iter()
            .any(|row| row.len() != actual[0].len() || row.iter().any(|value| !value.is_finite()))
        || predicted
            .iter()
            .any(|row| row.len() != actual[0].len() || row.iter().any(|value| !value.is_finite()))
    {
        return Err(BrainError::Invalid("regression_r2_shape".into()));
    }
    let dim = actual[0].len();
    let means = (0..dim)
        .map(|column| actual.iter().map(|row| row[column]).sum::<f64>() / actual.len() as f64)
        .collect::<Vec<_>>();
    let mut sse = 0.0;
    let mut sst = 0.0;
    for (actual_row, predicted_row) in actual.iter().zip(predicted) {
        for column in 0..dim {
            sse += (actual_row[column] - predicted_row[column]).powi(2);
            sst += (actual_row[column] - means[column]).powi(2);
        }
    }
    Ok(if sst <= 1e-18 { 0.0 } else { 1.0 - sse / sst })
}

pub fn validate_symmetric_psd(matrix: &Matrix, label: &str) -> BrainResult<Vec<(f64, Vec<f64>)>> {
    if matrix.rows != matrix.cols || matrix.rows == 0 || matrix.data.iter().any(|v| !v.is_finite())
    {
        return Err(BrainError::Invalid(format!("{label}_shape")));
    }
    let scale = matrix
        .data
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let tolerance = f64::EPSILON.sqrt() * scale * matrix.rows as f64;
    for i in 0..matrix.rows {
        for j in 0..i {
            if (matrix.get(i, j) - matrix.get(j, i)).abs() > tolerance {
                return Err(BrainError::Invalid(format!("{label}_not_symmetric")));
            }
        }
    }
    let eigs = symmetric_eigen_jacobi_signed(matrix, 1e-12, matrix.rows * matrix.rows * 100)?;
    if eigs.iter().any(|(value, _)| *value < -tolerance) {
        return Err(BrainError::Invalid(format!("{label}_not_psd")));
    }
    Ok(eigs)
}

pub fn symmetric_psd_condition(matrix: &Matrix, label: &str) -> BrainResult<f64> {
    let eigs = validate_symmetric_psd(matrix, label)?;
    let largest = eigs
        .iter()
        .map(|(value, _)| value.max(0.0))
        .fold(0.0_f64, f64::max);
    if largest <= 1e-18 {
        return Ok(f64::INFINITY);
    }
    let tolerance = largest * f64::EPSILON.sqrt() * matrix.rows.max(1) as f64;
    let smallest = eigs
        .iter()
        .map(|(value, _)| value.max(0.0))
        .filter(|value| *value > tolerance)
        .fold(f64::INFINITY, f64::min);
    if !smallest.is_finite() {
        Ok(f64::INFINITY)
    } else {
        Ok(largest / smallest)
    }
}

pub fn effective_rank_from_spectrum(eigenvalues: &[f64]) -> BrainResult<f64> {
    if eigenvalues.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Invalid(
            "effective_rank_spectrum_nonfinite".into(),
        ));
    }
    let total = eigenvalues.iter().map(|value| value.max(0.0)).sum::<f64>();
    if total <= 1e-18 {
        return Ok(0.0);
    }
    let entropy = eigenvalues
        .iter()
        .filter_map(|value| {
            let probability = value.max(0.0) / total;
            (probability > 1e-15).then_some(-probability * probability.ln())
        })
        .sum::<f64>();
    Ok(entropy.exp())
}

pub fn choose_energy_rank(
    eigenvalues: &[f64],
    target_explained_variance: f64,
    max_rank: usize,
    minimum_rank: usize,
) -> BrainResult<usize> {
    if eigenvalues.is_empty()
        || eigenvalues.iter().any(|value| !value.is_finite())
        || !target_explained_variance.is_finite()
        || !(0.0..=1.0).contains(&target_explained_variance)
        || max_rank == 0
        || minimum_rank > max_rank
    {
        return Err(BrainError::Invalid("energy_rank_input_invalid".into()));
    }
    let total = eigenvalues.iter().map(|value| value.max(0.0)).sum::<f64>();
    if total <= 1e-18 {
        return Ok(minimum_rank.min(eigenvalues.len()));
    }
    let cap = max_rank.min(eigenvalues.len());
    let mut accumulated = 0.0;
    for (index, value) in eigenvalues.iter().enumerate().take(cap) {
        accumulated += value.max(0.0);
        if accumulated / total >= target_explained_variance {
            return Ok((index + 1).max(minimum_rank).min(cap));
        }
    }
    Ok(cap.max(minimum_rank.min(eigenvalues.len())))
}

/// Validate rows with optional expected dimension. Returns the dimension.
pub fn validate_rows_expected(
    rows: &[Vec<f64>],
    expected_dim: Option<usize>,
    label: &str,
) -> BrainResult<usize> {
    if rows.is_empty() {
        return Err(BrainError::Invalid(format!("{label}_empty")));
    }
    let dimension = expected_dim.unwrap_or(rows[0].len());
    if dimension == 0
        || rows
            .iter()
            .any(|row| row.len() != dimension || row.iter().any(|value| !value.is_finite()))
    {
        return Err(BrainError::Invalid(format!("{label}_shape")));
    }
    Ok(dimension)
}

/// Validate that `values` has length `dimension` and all entries are finite.
pub fn validate_values(values: &[f64], dimension: usize) -> BrainResult<()> {
    if values.len() != dimension || values.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Invalid(
            "receiver_weight_response_shape_or_nonfinite".into(),
        ));
    }
    Ok(())
}

/// Standard forward bound for a dot product over exact F32 operands.
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

/// Return (families, layers) present in a `ParameterBlockLayout`.
pub fn layout_realization_coverage(
    layout: &ParameterBlockLayout,
) -> (BTreeSet<String>, BTreeSet<usize>) {
    let mut families = BTreeSet::new();
    let mut layers = BTreeSet::new();
    for block in &layout.blocks {
        if let Some(stem) = block.name.strip_suffix(".weight") {
            if let Some(family) = stem.rsplit('.').next() {
                families.insert(family.to_string());
            }
        }
        let parts = block.name.split('.').collect::<Vec<_>>();
        if let Some(layer) = parts
            .windows(2)
            .find(|window| window[0] == "layers")
            .and_then(|window| window[1].parse::<usize>().ok())
        {
            layers.insert(layer);
        }
    }
    (families, layers)
}

/// Helper: check that `families` contains every `required` family name.
pub fn families_include(required: &[&str], families: &BTreeSet<String>) -> bool {
    required.iter().all(|r| families.contains(*r))
}

/// Helper: whether covered layers equal the full range [0..total_layers).
pub fn is_full_layer_coverage(total_layers: usize, covered_layers: &BTreeSet<usize>) -> bool {
    if total_layers == 0 {
        return false;
    }
    covered_layers.len() == total_layers && covered_layers.iter().all(|&i| i < total_layers)
}

/// Compute writable fraction with validation and safety checks.
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

/// Given primitive flags, decide whether distributed compilation is permitted.
/// This keeps decision logic pure so callers can reuse it without importing
/// domain enums into `validation` and avoids circular deps.
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

/// Validate a short free-text reason field (no control chars, trimmed, bounded).
pub fn validate_reason(reason: &str, max_bytes: usize) -> BrainResult<()> {
    if reason.is_empty()
        || reason.len() > max_bytes
        || reason != reason.trim()
        || reason.chars().any(char::is_control)
    {
        return Err(BrainError::Invalid(
            "adapter_lifecycle_reason_invalid".into(),
        ));
    }
    Ok(())
}

/// Validate composition coefficient (nonzero finite f64).
pub fn validate_coefficient(value: f64) -> BrainResult<()> {
    if !value.is_finite() || value == 0.0 {
        return Err(BrainError::Invalid(
            "adapter_composition_coefficient_invalid".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_tomography::ParameterBlockSpec;
    use crate::identity::ObservationId;
    use std::collections::BTreeSet;

    #[test]
    fn observation_id_predicate_delegates_to_the_canonical_type() {
        for value in [
            "observation-01",
            "observation.v2",
            ".hidden",
            "../escape",
            "observation/slash",
            "",
        ] {
            assert_eq!(
                valid_observation_id(value),
                ObservationId::parse(value).is_ok(),
                "validator drift for {value:?}",
            );
        }
    }

    #[test]
    fn identifier_validator_matches_managed_name_policy() {
        assert!(validate_identifier("workspace-01", "workspace_name", 128).is_ok());
        assert!(validate_identifier(".", "workspace_name", 128).is_err());
        assert!(validate_identifier("invalid name", "workspace_name", 128).is_err());
        assert!(validate_identifier(&"a".repeat(129), "workspace_name", 128).is_err());
    }

    #[test]
    fn http_endpoint_validator_accepts_supported_protocols() {
        assert!(validate_http_endpoint("https://api.example.com/v1", 4096).is_ok());
        assert!(validate_http_endpoint("http://127.0.0.1:8080/v1", 4096).is_ok());
        assert!(validate_http_endpoint("ftp://example.com", 4096).is_err());
        assert!(validate_http_endpoint("http://bad space", 4096).is_err());
    }

    #[test]
    fn families_include_behaviour() {
        let mut set = BTreeSet::new();
        set.insert("attention".to_string());
        set.insert("mlp".to_string());
        assert!(families_include(&["attention", "mlp"], &set));
        assert!(!families_include(&["attention", "missing"], &set));
    }

    #[test]
    fn is_full_layer_coverage_examples() {
        let mut covered = BTreeSet::new();
        // total_layers == 0 => false
        assert!(!is_full_layer_coverage(0, &covered));
        // full coverage for 3 layers
        covered.insert(0);
        covered.insert(1);
        covered.insert(2);
        assert!(is_full_layer_coverage(3, &covered));
        // out-of-range index should fail full-coverage predicate
        covered.insert(3);
        assert!(!is_full_layer_coverage(3, &covered));
    }

    #[test]
    fn compute_writable_fraction_valid_and_invalid() {
        assert_eq!(compute_writable_fraction(2, 4).unwrap(), 0.5);
        assert!(compute_writable_fraction(5, 4).is_err());
        assert!(compute_writable_fraction(0, 0).is_err());
    }

    #[test]
    fn distributed_permitted_predicates() {
        assert!(distributed_permitted_from_primitives(true, 1, true, true));
        assert!(!distributed_permitted_from_primitives(false, 1, true, true));
        assert!(!distributed_permitted_from_primitives(true, 0, true, true));
        assert!(!distributed_permitted_from_primitives(true, 1, false, true));
        assert!(!distributed_permitted_from_primitives(true, 1, true, false));
    }

    #[test]
    fn validate_reason_accepts_and_rejects_expected() {
        assert!(validate_reason("update", 32).is_ok());
        assert!(validate_reason("", 32).is_err());
        assert!(validate_reason(" leading", 32).is_err());
        assert!(validate_reason("ok\u{0001}", 32).is_err());
        assert!(validate_reason("longtext", 3).is_err());
    }

    #[test]
    fn validate_coefficient_accepts_nonzero_and_rejects_bad() {
        assert!(validate_coefficient(0.1).is_ok());
        assert!(validate_coefficient(0.0).is_err());
        assert!(validate_coefficient(f64::NAN).is_err());
        assert!(validate_coefficient(f64::INFINITY).is_err());
    }

    #[test]
    fn layout_realization_coverage_parses_families_and_layers() {
        let a = ParameterBlockSpec {
            name: "layers.0.attention.weight".into(),
            shape: vec![2, 2],
            offset: 0,
            count: 4,
        };
        let b = ParameterBlockSpec {
            name: "layers.1.mlp.weight".into(),
            shape: vec![3, 2],
            offset: 4,
            count: 6,
        };
        let layout = crate::block_tomography::ParameterBlockLayout {
            schema: "test".into(),
            blocks: vec![a, b],
            total_parameter_count: 10,
        };
        let (families, layers) = layout_realization_coverage(&layout);
        assert!(families.contains("attention"));
        assert!(families.contains("mlp"));
        assert!(layers.contains(&0));
        assert!(layers.contains(&1));
    }
}
