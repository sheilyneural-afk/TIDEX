#![allow(clippy::needless_range_loop)]
use crate::foundation::error::{BrainError, BrainResult};
use faer::{Mat, Side};

#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    pub(crate) rows: usize,
    pub(crate) cols: usize,
    pub(crate) data: Vec<f64>,
}
impl Matrix {
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }
    pub fn from_rows(rows: &[Vec<f64>]) -> BrainResult<Self> {
        if rows.is_empty() {
            return Ok(Self::zeros(0, 0));
        }
        let cols = rows[0].len();
        if cols == 0
            || rows
                .iter()
                .any(|r| r.len() != cols || r.iter().any(|v| !v.is_finite()))
        {
            return Err(BrainError::Invalid("matrix_rows_invalid".into()));
        }
        Ok(Self {
            rows: rows.len(),
            cols,
            data: rows.iter().flatten().copied().collect(),
        })
    }
    pub fn row_count(&self) -> usize {
        self.rows
    }
    pub fn column_count(&self) -> usize {
        self.cols
    }
    pub fn as_slice(&self) -> &[f64] {
        &self.data
    }
    pub fn validate(&self, label: &str) -> BrainResult<()> {
        let expected = self
            .rows
            .checked_mul(self.cols)
            .ok_or_else(|| BrainError::Invalid(format!("{label}_shape_overflow")))?;
        if self.data.len() != expected || self.data.iter().any(|value| !value.is_finite()) {
            return Err(BrainError::Invalid(format!("{label}_shape_or_value")));
        }
        Ok(())
    }
    pub fn get(&self, r: usize, c: usize) -> f64 {
        self.data[r * self.cols + c]
    }
    pub(crate) fn set(&mut self, r: usize, c: usize, v: f64) {
        self.data[r * self.cols + c] = v;
    }
    pub fn row(&self, r: usize) -> &[f64] {
        &self.data[r * self.cols..(r + 1) * self.cols]
    }
    pub fn row_vec(&self, r: usize) -> Vec<f64> {
        self.row(r).to_vec()
    }
    pub fn transpose(&self) -> Self {
        let mut out = Self::zeros(self.cols, self.rows);
        for r in 0..self.rows {
            for c in 0..self.cols {
                out.set(c, r, self.get(r, c));
            }
        }
        out
    }
    pub fn matmul(&self, other: &Self) -> BrainResult<Self> {
        self.validate("matmul_left")?;
        other.validate("matmul_right")?;
        if self.cols != other.rows {
            return Err(BrainError::Invalid("matmul_shape".into()));
        }
        let mut out = Self::zeros(self.rows, other.cols);
        for i in 0..self.rows {
            for k in 0..self.cols {
                let a = self.get(i, k);
                if a == 0.0 {
                    continue;
                }
                for j in 0..other.cols {
                    out.data[i * out.cols + j] += a * other.get(k, j);
                }
            }
        }
        out.validate("matmul_output")?;
        Ok(out)
    }
    pub fn matvec(&self, v: &[f64]) -> BrainResult<Vec<f64>> {
        self.validate("matvec_matrix")?;
        if self.cols != v.len() || v.iter().any(|value| !value.is_finite()) {
            return Err(BrainError::Invalid("matvec_shape".into()));
        }
        (0..self.rows)
            .map(|row| dot(self.row(row), v))
            .collect::<BrainResult<Vec<_>>>()
    }
    pub fn identity(n: usize) -> Self {
        let mut m = Self::zeros(n, n);
        for i in 0..n {
            m.set(i, i, 1.0);
        }
        m
    }
}

pub fn dot(a: &[f64], b: &[f64]) -> BrainResult<f64> {
    if a.len() != b.len() || a.iter().chain(b).any(|value| !value.is_finite()) {
        return Err(BrainError::Invalid("dot_shape_or_value".into()));
    }
    let left_scale = a.iter().map(|value| value.abs()).fold(0.0_f64, f64::max);
    let right_scale = b.iter().map(|value| value.abs()).fold(0.0_f64, f64::max);
    if left_scale == 0.0 || right_scale == 0.0 {
        return Ok(0.0);
    }
    let normalized = compensated_sum(
        a.iter()
            .zip(b)
            .map(|(left, right)| (left / left_scale) * (right / right_scale)),
    )?;
    let value = (normalized * left_scale) * right_scale;
    if !value.is_finite() {
        return Err(BrainError::Numerical("dot_non_finite_result".into()));
    }
    Ok(value)
}

/// Neumaier compensated summation for finite values. This is the shared
/// numerical reduction primitive for persisted metrics and decisions.
pub fn compensated_sum(values: impl IntoIterator<Item = f64>) -> BrainResult<f64> {
    let mut sum = 0.0_f64;
    let mut correction = 0.0_f64;
    for value in values {
        if !value.is_finite() {
            return Err(BrainError::Numerical("compensated_sum_input_nonfinite".into()));
        }
        let updated = sum + value;
        if sum.abs() >= value.abs() {
            correction += (sum - updated) + value;
        } else {
            correction += (value - updated) + sum;
        }
        sum = updated;
    }
    let result = sum + correction;
    if !result.is_finite() {
        return Err(BrainError::Numerical("compensated_sum_nonfinite".into()));
    }
    Ok(result)
}

/// Overflow-resistant RMS using a scaled sum-of-squares recurrence.
pub fn stable_rms(values: impl IntoIterator<Item = f64>) -> BrainResult<f64> {
    let mut count = 0_u64;
    let mut scale = 0.0_f64;
    let mut sum_squares = 1.0_f64;
    for value in values {
        let absolute = value.abs();
        if !absolute.is_finite() {
            return Err(BrainError::Numerical("stable_rms_input_nonfinite".into()));
        }
        count = count
            .checked_add(1)
            .ok_or_else(|| BrainError::Invalid("stable_rms_count_overflow".into()))?;
        if absolute == 0.0 {
            continue;
        }
        if scale < absolute {
            let ratio = if scale == 0.0 { 0.0 } else { scale / absolute };
            sum_squares = 1.0 + sum_squares * ratio * ratio;
            scale = absolute;
        } else {
            let ratio = absolute / scale;
            sum_squares += ratio * ratio;
        }
    }
    if count == 0 {
        return Err(BrainError::Invalid("stable_rms_empty".into()));
    }
    let result = if scale == 0.0 {
        0.0
    } else {
        scale * (sum_squares / count as f64).sqrt()
    };
    if !result.is_finite() {
        return Err(BrainError::Numerical("stable_rms_nonfinite".into()));
    }
    Ok(result)
}
pub fn norm(a: &[f64]) -> BrainResult<f64> {
    if a.is_empty() || a.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Invalid("norm_input_invalid".into()));
    }
    let scale = a.iter().map(|value| value.abs()).fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return Ok(0.0);
    }
    let scaled_square_sum = a.iter().map(|value| (value / scale).powi(2)).sum::<f64>();
    let value = scale * scaled_square_sum.sqrt();
    if !value.is_finite() {
        return Err(BrainError::Numerical("norm_non_finite_result".into()));
    }
    Ok(value)
}
pub fn normalize(a: &[f64]) -> BrainResult<Vec<f64>> {
    let n = norm(a)?;
    if n == 0.0 {
        return Err(BrainError::Numerical("normalize_zero_norm".into()));
    }
    let normalized = a.iter().map(|value| value / n).collect::<Vec<_>>();
    if normalized.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Numerical("normalize_non_finite_result".into()));
    }
    Ok(normalized)
}
pub fn cosine(a: &[f64], b: &[f64]) -> BrainResult<f64> {
    if a.len() != b.len() {
        return Err(BrainError::Invalid("cosine_dimension_mismatch".into()));
    }
    let left_norm = norm(a)?;
    let right_norm = norm(b)?;
    if left_norm == 0.0 || right_norm == 0.0 {
        return Err(BrainError::Numerical("cosine_zero_norm".into()));
    }
    let value = a
        .iter()
        .zip(b)
        .map(|(left, right)| (left / left_norm) * (right / right_norm))
        .sum::<f64>();
    if !value.is_finite() {
        return Err(BrainError::Numerical("cosine_non_finite_result".into()));
    }
    Ok(value.clamp(-1.0, 1.0))
}
pub fn sub(a: &[f64], b: &[f64]) -> BrainResult<Vec<f64>> {
    if a.len() != b.len() || a.iter().chain(b).any(|value| !value.is_finite()) {
        return Err(BrainError::Invalid("sub_shape_or_value".into()));
    }
    let result = a.iter().zip(b).map(|(x, y)| x - y).collect::<Vec<_>>();
    if result.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Numerical("sub_non_finite_result".into()));
    }
    Ok(result)
}
pub fn add_scaled(a: &mut [f64], b: &[f64], s: f64) -> BrainResult<()> {
    if a.len() != b.len() || !s.is_finite() || a.iter().chain(b).any(|value| !value.is_finite()) {
        return Err(BrainError::Invalid("add_scaled_shape_or_value".into()));
    }
    let updated = a.iter().zip(b).map(|(x, y)| x + s * y).collect::<Vec<_>>();
    if updated.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Numerical("add_scaled_non_finite_result".into()));
    }
    a.copy_from_slice(&updated);
    Ok(())
}

pub fn solve(mut a: Matrix, mut b: Vec<f64>) -> BrainResult<Vec<f64>> {
    a.validate("linear_solve_matrix")?;
    if a.rows == 0
        || a.rows != a.cols
        || a.rows != b.len()
        || b.iter().any(|value| !value.is_finite())
    {
        return Err(BrainError::Invalid("linear_solve_shape".into()));
    }
    let n = a.rows;
    let matrix_scale = a
        .data
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let singularity_tolerance = f64::EPSILON * matrix_scale * n as f64;
    for k in 0..n {
        let mut pivot = k;
        let mut best = a.get(k, k).abs();
        for r in k + 1..n {
            let v = a.get(r, k).abs();
            if v > best {
                best = v;
                pivot = r;
            }
        }
        if best == 0.0 || best <= singularity_tolerance {
            return Err(BrainError::Numerical("singular_system".into()));
        }
        if pivot != k {
            for c in k..n {
                let x = a.get(k, c);
                a.set(k, c, a.get(pivot, c));
                a.set(pivot, c, x);
            }
            b.swap(k, pivot);
        }
        let diag = a.get(k, k);
        for c in k..n {
            a.set(k, c, a.get(k, c) / diag);
        }
        b[k] /= diag;
        for r in 0..n {
            if r == k {
                continue;
            }
            let f = a.get(r, k);
            if f.abs() < 1e-18 {
                continue;
            }
            for c in k..n {
                a.set(r, c, a.get(r, c) - f * a.get(k, c));
            }
            b[r] -= f * b[k];
        }
        if a.data.iter().any(|value| !value.is_finite()) || b.iter().any(|value| !value.is_finite())
        {
            return Err(BrainError::Numerical("linear_solve_non_finite_result".into()));
        }
    }
    if b.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Numerical("linear_solve_non_finite_result".into()));
    }
    Ok(b)
}

pub fn inverse_with_ridge(a: &Matrix, ridge: f64) -> BrainResult<Matrix> {
    a.validate("inverse_matrix")?;
    if a.rows != a.cols || a.rows == 0 || !ridge.is_finite() || ridge < 0.0 {
        return Err(BrainError::Invalid("inverse_shape".into()));
    }
    let n = a.rows;
    let mut base = a.clone();
    for i in 0..n {
        base.data[i * n + i] += ridge;
    }
    let mut out = Matrix::zeros(n, n);
    for c in 0..n {
        let mut e = vec![0.0; n];
        e[c] = 1.0;
        let x = solve(base.clone(), e)?;
        for r in 0..n {
            out.set(r, c, x[r]);
        }
    }
    Ok(out)
}

pub fn weighted_normal_solve(
    x: &Matrix,
    y: &[f64],
    weights: &[f64],
    ridge: f64,
) -> BrainResult<Vec<f64>> {
    x.validate("weighted_ls_design")?;
    if x.rows == 0
        || x.cols == 0
        || x.rows != y.len()
        || x.rows != weights.len()
        || y.iter().any(|value| !value.is_finite())
        || weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight < 0.0)
        || !ridge.is_finite()
        || ridge < 0.0
    {
        return Err(BrainError::Invalid("weighted_ls_shape".into()));
    }
    let mut a = Matrix::zeros(x.cols, x.cols);
    let mut b = vec![0.0; x.cols];
    for r in 0..x.rows {
        let w = weights[r];
        for i in 0..x.cols {
            let xi = x.get(r, i);
            b[i] += w * xi * y[r];
            for j in 0..x.cols {
                a.data[i * x.cols + j] += w * xi * x.get(r, j);
            }
        }
    }
    for i in 0..x.cols {
        a.data[i * x.cols + i] += ridge;
    }
    solve(a, b)
}

/// Top-k eigenpairs of a real symmetric matrix.
///
/// Implemented via a full self-adjoint EVD (`faer`). The `iterations`
/// parameter is retained for call-site compatibility with the old power
/// iteration helper and is ignored.
pub fn symmetric_top_eigen(
    a: &Matrix,
    k: usize,
    _iterations: usize,
) -> BrainResult<Vec<(f64, Vec<f64>)>> {
    if a.rows != a.cols {
        return Err(BrainError::Invalid("eigen_shape".into()));
    }
    let signed = symmetric_eigen_faer_raw(a, 1e-12)?;
    let mut out = signed
        .into_iter()
        .map(|(value, vector)| (value.max(0.0), vector))
        .filter(|(value, _)| *value > 1e-12)
        .take(k.min(a.rows))
        .collect::<Vec<_>>();
    out.sort_by(|left, right| right.0.total_cmp(&left.0));
    Ok(out)
}

pub fn weighted_row_gram(d: &Matrix, weights: &[f64]) -> BrainResult<Matrix> {
    d.validate("gram_matrix")?;
    if d.rows != weights.len()
        || weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight < 0.0)
    {
        return Err(BrainError::Invalid("gram_weight_shape".into()));
    }
    let mut g = Matrix::zeros(d.rows, d.rows);
    for i in 0..d.rows {
        for j in i..d.rows {
            let v = weights[i].sqrt() * weights[j].sqrt() * dot(d.row(i), d.row(j))?;
            g.set(i, j, v);
            g.set(j, i, v);
        }
    }
    Ok(g)
}

pub fn median(mut values: Vec<f64>) -> BrainResult<f64> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Invalid("median_input_invalid".into()));
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let n = values.len();
    if n % 2 == 1 {
        Ok(values[n / 2])
    } else {
        Ok(values[n / 2 - 1] * 0.5 + values[n / 2] * 0.5)
    }
}

fn matrix_to_faer(a: &Matrix) -> Mat<f64> {
    Mat::from_fn(a.rows, a.cols, |row, column| a.get(row, column))
}

/// Self-adjoint EVD via `faer` (replaces the hand-rolled Jacobi iteration).
///
/// `tolerance` still gates the explicit symmetry check. `max_rotations` is
/// retained for call-site compatibility and ignored by the faer backend.
fn symmetric_eigen_faer_raw(a: &Matrix, tolerance: f64) -> BrainResult<Vec<(f64, Vec<f64>)>> {
    a.validate("jacobi_eigen_matrix")?;
    if a.rows != a.cols || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(BrainError::Invalid("jacobi_eigen_shape".into()));
    }
    let n = a.rows;
    if n == 0 {
        return Ok(Vec::new());
    }
    let scale = a
        .data
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let symmetry_tolerance = f64::EPSILON.sqrt() * scale * n as f64;
    for row in 0..n {
        for column in 0..row {
            if (a.get(row, column) - a.get(column, row)).abs() > symmetry_tolerance {
                return Err(BrainError::Invalid("jacobi_eigen_matrix_not_symmetric".into()));
            }
        }
    }
    let mat = matrix_to_faer(a);
    let evd = mat
        .self_adjoint_eigen(Side::Lower)
        .map_err(|_| BrainError::Numerical("faer_self_adjoint_evd_failed".into()))?;
    let s = evd.S();
    let u = evd.U();
    let mut out = Vec::with_capacity(n);
    for index in 0..n {
        let eigenvalue = s[index];
        if !eigenvalue.is_finite() {
            return Err(BrainError::Numerical("faer_eigenvalue_non_finite".into()));
        }
        let mut vector = Vec::with_capacity(n);
        for row in 0..n {
            let value = u[(row, index)];
            if !value.is_finite() {
                return Err(BrainError::Numerical("faer_eigenvector_non_finite".into()));
            }
            vector.push(value);
        }
        out.push((eigenvalue, normalize(&vector)?));
    }
    // faer returns nondecreasing eigenvalues; TIDEX callers expect descending.
    out.sort_by(|left, right| right.0.total_cmp(&left.0));
    Ok(out)
}

fn symmetric_eigen_jacobi_raw(
    a: &Matrix,
    tolerance: f64,
    _max_rotations: usize,
) -> BrainResult<Vec<(f64, Vec<f64>)>> {
    symmetric_eigen_faer_raw(a, tolerance)
}

/// Signed eigen-decomposition for symmetric matrices (`faer` self-adjoint EVD).
///
/// Unlike the energy helper, this preserves negative and near-zero eigenvalues so
/// callers can validate positive semidefiniteness instead of silently truncating
/// dangerous negative curvature. The historical Jacobi name is kept for API
/// stability; `max_rotations` is ignored.
pub fn symmetric_eigen_jacobi_signed(
    a: &Matrix,
    tolerance: f64,
    max_rotations: usize,
) -> BrainResult<Vec<(f64, Vec<f64>)>> {
    symmetric_eigen_jacobi_raw(a, tolerance, max_rotations)
}

/// Energy-oriented symmetric eigendecomposition (`faer` self-adjoint EVD).
///
/// Negative eigenvalues are intentionally discarded because Gram/energy callers
/// require a PSD spectrum. The historical Jacobi name is kept for API stability;
/// `max_rotations` is ignored.
pub fn symmetric_eigen_jacobi(
    a: &Matrix,
    tolerance: f64,
    max_rotations: usize,
) -> BrainResult<Vec<(f64, Vec<f64>)>> {
    let signed = symmetric_eigen_jacobi_raw(a, tolerance, max_rotations)?;
    let scale = signed
        .iter()
        .map(|(value, _)| value.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let negative_tolerance = f64::EPSILON.sqrt() * scale * a.rows.max(1) as f64;
    if signed.iter().any(|(value, _)| *value < -negative_tolerance) {
        return Err(BrainError::Invalid("energy_matrix_not_psd".into()));
    }
    let mut out = signed
        .into_iter()
        .map(|(value, vector)| (value.max(0.0), vector))
        .filter(|(value, _)| *value > 1e-14)
        .collect::<Vec<_>>();
    out.sort_by(|a, b| b.0.total_cmp(&a.0));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faer_top_eigen_matches_full_descending_spectrum() {
        let mut matrix = Matrix::zeros(3, 3);
        // diag(4, 1, 0.25) in the standard basis
        matrix.set(0, 0, 4.0);
        matrix.set(1, 1, 1.0);
        matrix.set(2, 2, 0.25);
        let full = symmetric_eigen_jacobi(&matrix, 1e-12, 100).unwrap();
        let top = symmetric_top_eigen(&matrix, 2, 0).unwrap();
        assert_eq!(top.len(), 2);
        assert!((top[0].0 - full[0].0).abs() < 1e-10);
        assert!((top[1].0 - full[1].0).abs() < 1e-10);
        assert!((top[0].0 - 4.0).abs() < 1e-10);
        assert!((top[1].0 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn signed_eigensolver_preserves_negative_eigenvalues() {
        let mut matrix = Matrix::zeros(2, 2);
        matrix.set(0, 0, 2.0);
        matrix.set(1, 1, -0.5);
        let signed = symmetric_eigen_jacobi_signed(&matrix, 1e-12, 100).unwrap();
        assert!(signed.iter().any(|(value, _)| *value < -0.49));
        assert!(matches!(
            symmetric_eigen_jacobi(&matrix, 1e-12, 100),
            Err(BrainError::Invalid(message)) if message == "energy_matrix_not_psd"
        ));
    }

    #[test]
    fn direction_operations_reject_zero_norm_instead_of_fabricating_geometry() {
        assert!(matches!(
            normalize(&[0.0, 0.0]),
            Err(BrainError::Numerical(message)) if message == "normalize_zero_norm"
        ));
        assert!(matches!(
            cosine(&[1.0, 0.0], &[0.0, 0.0]),
            Err(BrainError::Numerical(message)) if message == "cosine_zero_norm"
        ));
        let tiny = normalize(&[1e-300, 0.0]).unwrap();
        assert!((tiny[0] - 1.0).abs() <= 4.0 * f64::EPSILON);
        assert_eq!(tiny[1], 0.0);
        assert!((cosine(&[1e300, 1e300], &[1e300, 1e300]).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn failed_vector_update_is_atomic() {
        let mut destination = vec![f64::MAX, 1.0];
        let original = destination.clone();
        assert!(add_scaled(&mut destination, &[f64::MAX, 2.0], 1.0).is_err());
        assert_eq!(destination, original);
    }

    #[test]
    fn stable_reductions_survive_large_finite_inputs_and_cancellation() {
        let scale = f64::MAX.sqrt();
        let rms = stable_rms([scale, scale]).unwrap();
        assert!(rms.is_finite());
        assert!((rms / scale - 1.0).abs() < 1e-12);

        let cancellation = compensated_sum([1.0e16, 1.0, -1.0e16]).unwrap();
        assert_eq!(cancellation, 1.0);

        let stable_dot = dot(&[1.0e16, 1.0, -1.0e16], &[1.0, 1.0, 1.0]).unwrap();
        assert!((stable_dot - 1.0).abs() < 1e-9);
    }

    #[test]
    fn weighted_algebra_rejects_invalid_weights_and_regularization() {
        let design = Matrix::from_rows(&[vec![1.0], vec![2.0]]).unwrap();
        assert!(weighted_normal_solve(&design, &[1.0, 2.0], &[1.0, -0.1], 1e-6).is_err());
        assert!(weighted_normal_solve(&design, &[1.0, 2.0], &[1.0, f64::NAN], 1e-6).is_err());
        assert!(weighted_normal_solve(&design, &[1.0, 2.0], &[1.0, 1.0], -1.0).is_err());
        assert!(weighted_row_gram(&design, &[1.0, -0.1]).is_err());
        assert!(inverse_with_ridge(&Matrix::identity(1), f64::NAN).is_err());
    }
}
