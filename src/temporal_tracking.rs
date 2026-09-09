//! SAR-inspired temporal tracking for weight drift detection.
//!
//! This module adapts three techniques from Synthetic Aperture Radar (SAR)
//! interferometry to track how model parameters evolve across learning epochs:
//!
//! 1. **Phase correlation** — detects minimal distributed changes between weight
//!    snapshots with sub-element precision, analogous to SAR phase registration.
//!
//! 2. **SBAS (Small BAseline Subset)** — reconstructs continuous deformation
//!    time-series from pairwise differential measurements, adapted from InSAR
//!    multi-temporal analysis.
//!
//! 3. **PS identification (Persistent Scatterer)** — identifies structurally
//!    invariant parameters that remain stable across all learning epochs,
//!    analogous to PS-InSAR point selection.
//!
//! All algorithms are implemented in safe Rust with no external dependencies
//! beyond `crate::linalg` and `crate::error`.

use crate::error::{BrainError, BrainResult};
use crate::linalg::stable_rms;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

// ─── Phase Correlation ───────────────────────────────────────────────────────

/// Result of a phase correlation between two weight snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhaseCorrelationResult {
    pub schema: String,
    /// Normalised cross-power spectrum magnitude — 1.0 means identical.
    pub peak_magnitude: f64,
    /// Fractional shift estimate in each dimension (sub-element precision).
    pub estimated_shift: Vec<f64>,
    /// Per-element phase difference (wrapped to [−π, π]).
    pub phase_residuals: Vec<f64>,
    /// Root-mean-square of phase residuals — small means coherent change.
    pub phase_rms: f64,
    /// Fraction of elements whose absolute phase residual exceeds π/2.
    pub incoherent_fraction: f64,
}

/// Compute normalised phase correlation between two weight vectors.
///
/// Analogous to InSAR interferogram formation:
///
/// ```text
///   Z_k = a_k · conj(b_k) / |a_k · conj(b_k)|
///   phase_k = atan2(Im(Z_k), Re(Z_k))
/// ```
///
/// Since neural weights are real-valued, we treat each element pair as
/// a unit-amplitude phasor whose "phase" is the arctangent ratio of the
/// difference to the mean, giving sub-element sensitivity to distributed
/// perturbations that L2 norm would miss.
pub fn phase_correlation(
    snapshot_a: &[f64],
    snapshot_b: &[f64],
) -> BrainResult<PhaseCorrelationResult> {
    if snapshot_a.len() != snapshot_b.len() || snapshot_a.is_empty() {
        return Err(BrainError::Invalid(
            "phase_correlation_dimension_mismatch".into(),
        ));
    }
    let n = snapshot_a.len();

    let mut phase_residuals = Vec::with_capacity(n);
    let mut shifts = Vec::with_capacity(n);
    let mut cross_real_sum = 0.0_f64;
    let mut cross_imag_sum = 0.0_f64;
    let mut incoherent_count = 0_usize;

    for k in 0..n {
        let a = snapshot_a[k];
        let b = snapshot_b[k];

        if !a.is_finite() || !b.is_finite() {
            return Err(BrainError::Numerical(
                "phase_correlation_non_finite_input".into(),
            ));
        }

        let diff = b - a;
        let mean = (a + b) * 0.5;
        // Phase: atan2(diff, |mean|+eps) — measures the angular shift
        // relative to the signal magnitude, giving sub-element sensitivity.
        let phase = diff.atan2(mean.abs() + 1e-15);
        phase_residuals.push(phase);
        shifts.push(diff);

        // Accumulate normalised cross-power: cos(phase) + j·sin(phase)
        cross_real_sum += phase.cos();
        cross_imag_sum += phase.sin();

        if phase.abs() > PI * 0.25 {
            incoherent_count += 1;
        }
    }

    let nf = n as f64;
    let peak_real = cross_real_sum / nf;
    let peak_imag = cross_imag_sum / nf;
    let peak_magnitude = (peak_real * peak_real + peak_imag * peak_imag).sqrt();

    let phase_rms = stable_rms(phase_residuals.iter().copied())?;
    let incoherent_fraction = incoherent_count as f64 / nf;

    Ok(PhaseCorrelationResult {
        schema: "phase_correlation:v1".into(),
        peak_magnitude,
        estimated_shift: shifts,
        phase_residuals,
        phase_rms,
        incoherent_fraction,
    })
}

// ─── SBAS Time-Series Inversion ──────────────────────────────────────────────

/// A differential measurement between two epochs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DifferentialPair {
    /// Index of the earlier epoch (0-based).
    pub epoch_a: usize,
    /// Index of the later epoch (0-based).
    pub epoch_b: usize,
    /// Measured differential displacement at this parameter index.
    /// Analogous to InSAR: δφ = (4π/λ)[u(t_{b}) - u(t_{a})]
    pub differential: f64,
}

/// Time-series result for one parameter across all epochs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SbasTimeSeries {
    pub schema: String,
    /// Cumulative displacement at each epoch (epoch 0 = 0.0).
    pub cumulative: Vec<f64>,
    /// Velocity (rate of change per epoch).
    pub velocity: f64,
    /// Residual RMS after linear detrending.
    pub residual_rms: f64,
    /// Condition number of the design matrix.
    pub condition: f64,
}

/// Reconstruct a time-series of parameter drift from pairwise differentials.
///
/// This is the SBAS (Small BAseline Subset) algorithm adapted from InSAR:
///
/// ```text
///   For each pair (a, b):   δ_i = u(t_b) - u(t_a)
///   Design matrix A:        A[i, epoch_b] = +1, A[i, epoch_a] = -1
///   Solve via SVD:           Ax = δ  →  x = A† δ
/// ```
///
/// The result `x` is the cumulative displacement at each epoch.
/// Regularisation via Tikhonov ensures stability when the graph is not
/// fully connected.
pub fn sbas_inversion(
    num_epochs: usize,
    pairs: &[DifferentialPair],
    regularisation: f64,
) -> BrainResult<SbasTimeSeries> {
    if num_epochs < 2 {
        return Err(BrainError::Invalid("sbas_insufficient_epochs".into()));
    }
    if pairs.is_empty() {
        return Err(BrainError::Invalid("sbas_no_pairs".into()));
    }
    if !regularisation.is_finite() || regularisation < 0.0 {
        return Err(BrainError::Invalid("sbas_regularisation_invalid".into()));
    }
    for pair in pairs {
        if pair.epoch_a >= num_epochs || pair.epoch_b >= num_epochs {
            return Err(BrainError::Invalid("sbas_epoch_out_of_range".into()));
        }
        if pair.epoch_a == pair.epoch_b {
            return Err(BrainError::Invalid("sbas_self_pair".into()));
        }
        if !pair.differential.is_finite() {
            return Err(BrainError::Numerical("sbas_non_finite_differential".into()));
        }
    }

    let unknowns = num_epochs - 1;
    let _m = pairs.len();

    // Build A^T A + μI  and  A^T δ  (normal equations).
    let mut ata = vec![0.0_f64; unknowns * unknowns];
    let mut atb = vec![0.0_f64; unknowns];

    for pair in pairs {
        // Row of A has +1 at epoch_b-1 and -1 at epoch_a-1 (if not epoch 0).
        let col_b = if pair.epoch_b > 0 {
            Some(pair.epoch_b - 1)
        } else {
            None
        };
        let col_a = if pair.epoch_a > 0 {
            Some(pair.epoch_a - 1)
        } else {
            None
        };

        // A^T A contributions:
        if let Some(cb) = col_b {
            ata[cb * unknowns + cb] += 1.0;
            atb[cb] += pair.differential;
        }
        if let Some(ca) = col_a {
            ata[ca * unknowns + ca] += 1.0;
            atb[ca] -= pair.differential;
        }
        if let (Some(ca), Some(cb)) = (col_a, col_b) {
            ata[ca * unknowns + cb] -= 1.0;
            ata[cb * unknowns + ca] -= 1.0;
        }
    }

    // Tikhonov regularisation.
    for i in 0..unknowns {
        ata[i * unknowns + i] += regularisation;
    }

    // Compute condition number estimate (ratio of diagonal extremes).
    let mut diag_min = f64::MAX;
    let mut diag_max = 0.0_f64;
    for i in 0..unknowns {
        let d = ata[i * unknowns + i].abs();
        if d < diag_min {
            diag_min = d;
        }
        if d > diag_max {
            diag_max = d;
        }
    }
    let condition = if diag_min > 1e-15 {
        diag_max / diag_min
    } else {
        f64::MAX
    };

    // Solve via Cholesky (the normal equations matrix is SPD).
    let solution = cholesky_solve(unknowns, &ata, &atb)?;

    // Build cumulative: epoch 0 = 0.0, rest from solution.
    let mut cumulative = Vec::with_capacity(num_epochs);
    cumulative.push(0.0);
    cumulative.extend_from_slice(&solution);

    // Velocity: linear fit  u(t) = v·t + c  →  v = (n·Σ(t·u) - Σt·Σu) / (n·Σt² - (Σt)²)
    let nf = num_epochs as f64;
    let sum_t: f64 = (0..num_epochs).map(|i| i as f64).sum();
    let sum_t2: f64 = (0..num_epochs).map(|i| (i as f64).powi(2)).sum();
    let sum_tu: f64 = cumulative
        .iter()
        .enumerate()
        .map(|(i, u)| i as f64 * u)
        .sum();
    let sum_u: f64 = cumulative.iter().sum();
    let denom = nf * sum_t2 - sum_t * sum_t;
    let velocity = if denom.abs() > 1e-15 {
        (nf * sum_tu - sum_t * sum_u) / denom
    } else {
        0.0
    };
    let intercept = if denom.abs() > 1e-15 {
        (sum_u * sum_t2 - sum_t * sum_tu) / denom
    } else {
        0.0
    };

    // Residual RMS after detrending.
    let residuals: Vec<f64> = cumulative
        .iter()
        .enumerate()
        .map(|(i, u)| u - (velocity * i as f64 + intercept))
        .collect();
    let residual_rms = stable_rms(residuals.into_iter())?;

    Ok(SbasTimeSeries {
        schema: "sbas_time_series:v1".into(),
        cumulative,
        velocity,
        residual_rms,
        condition,
    })
}

/// Cholesky LLT solve for SPD system.
fn cholesky_solve(n: usize, ata: &[f64], atb: &[f64]) -> BrainResult<Vec<f64>> {
    // Factor: A = L L^T
    let mut l = vec![0.0_f64; n * n];
    for j in 0..n {
        let mut sum = 0.0;
        for k in 0..j {
            sum += l[j * n + k] * l[j * n + k];
        }
        let diag = ata[j * n + j] - sum;
        if diag <= 0.0 {
            return Err(BrainError::Numerical("sbas_cholesky_not_positive".into()));
        }
        l[j * n + j] = diag.sqrt();

        for i in (j + 1)..n {
            let mut s = 0.0;
            for k in 0..j {
                s += l[i * n + k] * l[j * n + k];
            }
            l[i * n + j] = (ata[i * n + j] - s) / l[j * n + j];
        }
    }

    // Forward solve: L y = b
    let mut y = vec![0.0; n];
    for i in 0..n {
        let mut s = 0.0;
        for k in 0..i {
            s += l[i * n + k] * y[k];
        }
        y[i] = (atb[i] - s) / l[i * n + i];
    }

    // Back solve: L^T x = y
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut s = 0.0;
        for k in (i + 1)..n {
            s += l[k * n + i] * x[k];
        }
        x[i] = (y[i] - s) / l[i * n + i];
    }

    for val in &x {
        if !val.is_finite() {
            return Err(BrainError::Numerical("sbas_solution_non_finite".into()));
        }
    }

    Ok(x)
}

// ─── Persistent Scatterer (PS) Identification ────────────────────────────────

/// A parameter identified as a persistent scatterer — structurally invariant
/// across all learning epochs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistentScatterer {
    /// Index of this parameter in the weight vector.
    pub parameter_index: usize,
    /// Mean value across all epochs.
    pub mean_value: f64,
    /// Amplitude dispersion index (σ/μ). Values < 0.25 are PS candidates.
    pub amplitude_dispersion: f64,
    /// Temporal coherence: mean of cos(phase_residual) across all pairs.
    /// Values > 0.85 indicate structural invariance.
    pub temporal_coherence: f64,
    /// Classification: `invariant`, `quasi_stable`, or `unstable`.
    pub classification: String,
}

/// Result of persistent scatterer analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistentScattererReport {
    pub schema: String,
    pub total_parameters: usize,
    pub total_epochs: usize,
    pub invariant_count: usize,
    pub quasi_stable_count: usize,
    pub unstable_count: usize,
    /// Fraction of parameters classified as invariant.
    pub invariant_ratio: f64,
    /// The identified persistent scatterers (only invariant and quasi-stable).
    pub scatterers: Vec<PersistentScatterer>,
}

/// Identify persistent scatterers — parameters that remain structurally
/// invariant across multiple learning epochs.
///
/// Adapted from PS-InSAR: for each parameter, we compute:
///
/// 1. **Amplitude dispersion** $D_A = \sigma_A / \bar{A}$ — ratio of temporal
///    standard deviation to mean amplitude. PS candidates have $D_A < 0.25$.
///
/// 2. **Temporal coherence** $\gamma_t = |\frac{1}{N}\sum_k e^{j\phi_k}|$ —
///    the mean phasor over all differential phase measurements. High coherence
///    ($\gamma_t > 0.85$) means the parameter barely changes.
///
/// Parameters are classified as:
/// - **invariant**: $D_A < 0.15$ and $\gamma_t > 0.90$
/// - **quasi_stable**: $D_A < 0.25$ and $\gamma_t > 0.70$
/// - **unstable**: everything else
pub fn identify_persistent_scatterers(
    epochs: &[Vec<f64>],
    dispersion_threshold: f64,
    coherence_threshold: f64,
) -> BrainResult<PersistentScattererReport> {
    if epochs.len() < 3 {
        return Err(BrainError::Invalid("ps_insufficient_epochs".into()));
    }
    let param_count = epochs[0].len();
    if param_count == 0 {
        return Err(BrainError::Invalid("ps_empty_parameters".into()));
    }
    for epoch in epochs {
        if epoch.len() != param_count {
            return Err(BrainError::Invalid("ps_epoch_dimension_mismatch".into()));
        }
    }

    let num_epochs = epochs.len();
    let nf = num_epochs as f64;
    let mut scatterers = Vec::new();
    let mut invariant_count = 0_usize;
    let mut quasi_stable_count = 0_usize;
    let mut unstable_count = 0_usize;

    for p in 0..param_count {
        // Gather temporal series for this parameter.
        let mut values = Vec::with_capacity(num_epochs);
        for epoch in epochs {
            let v = epoch[p];
            if !v.is_finite() {
                return Err(BrainError::Numerical(
                    "ps_non_finite_parameter_value".into(),
                ));
            }
            values.push(v);
        }

        // Mean and standard deviation.
        let mean = values.iter().sum::<f64>() / nf;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / nf;
        let std_dev = variance.sqrt();

        // Amplitude dispersion: σ/|μ|
        let amplitude_dispersion = if mean.abs() > 1e-15 {
            std_dev / mean.abs()
        } else {
            f64::MAX
        };

        // Temporal coherence: for consecutive pairs, compute phase and average.
        let mut cos_sum = 0.0_f64;
        let mut sin_sum = 0.0_f64;
        let num_pairs = num_epochs - 1;
        for i in 0..num_pairs {
            let diff = values[i + 1] - values[i];
            let phase = diff.atan2(mean.abs() + 1e-15);
            cos_sum += phase.cos();
            sin_sum += phase.sin();
        }
        let npf = num_pairs as f64;
        let temporal_coherence = ((cos_sum / npf).powi(2) + (sin_sum / npf).powi(2)).sqrt();

        // Classification.
        let classification = if amplitude_dispersion < dispersion_threshold * 0.6
            && temporal_coherence > coherence_threshold * 1.05
        {
            invariant_count += 1;
            "invariant"
        } else if amplitude_dispersion < dispersion_threshold
            && temporal_coherence > coherence_threshold * 0.78
        {
            quasi_stable_count += 1;
            "quasi_stable"
        } else {
            unstable_count += 1;
            "unstable"
        };

        if classification != "unstable" {
            scatterers.push(PersistentScatterer {
                parameter_index: p,
                mean_value: mean,
                amplitude_dispersion,
                temporal_coherence,
                classification: classification.to_string(),
            });
        }
    }

    let invariant_ratio = invariant_count as f64 / param_count as f64;

    Ok(PersistentScattererReport {
        schema: "persistent_scatterer_report:v1".into(),
        total_parameters: param_count,
        total_epochs: num_epochs,
        invariant_count,
        quasi_stable_count,
        unstable_count,
        invariant_ratio,
        scatterers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_correlation_detects_identical_snapshots() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = phase_correlation(&a, &a).unwrap();
        assert!(
            (result.peak_magnitude - 1.0).abs() < 1e-10,
            "identical snapshots should have peak magnitude ~1.0, got {}",
            result.peak_magnitude
        );
        assert!(
            result.phase_rms < 1e-12,
            "identical snapshots should have zero phase RMS"
        );
    }

    #[test]
    fn phase_correlation_detects_small_perturbation() {
        let a = vec![10.0, 20.0, 30.0, 40.0];
        let b = vec![10.001, 20.001, 30.001, 40.001];
        let result = phase_correlation(&a, &b).unwrap();
        assert!(
            result.peak_magnitude > 0.99,
            "small perturbation should keep high coherence, got {}",
            result.peak_magnitude
        );
        assert!(result.incoherent_fraction < 0.01);
    }

    #[test]
    fn phase_correlation_detects_large_change() {
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![10.0, -5.0, 30.0, -40.0];
        let result = phase_correlation(&a, &b).unwrap();
        assert!(
            result.incoherent_fraction > 0.2,
            "large change should produce high incoherent fraction"
        );
    }

    #[test]
    fn sbas_recovers_linear_drift() {
        // 5 epochs with linear drift: u = [0, 2, 4, 6, 8]
        let pairs = vec![
            DifferentialPair {
                epoch_a: 0,
                epoch_b: 1,
                differential: 2.0,
            },
            DifferentialPair {
                epoch_a: 1,
                epoch_b: 2,
                differential: 2.0,
            },
            DifferentialPair {
                epoch_a: 2,
                epoch_b: 3,
                differential: 2.0,
            },
            DifferentialPair {
                epoch_a: 3,
                epoch_b: 4,
                differential: 2.0,
            },
            // Cross-link for redundancy:
            DifferentialPair {
                epoch_a: 0,
                epoch_b: 2,
                differential: 4.0,
            },
            DifferentialPair {
                epoch_a: 1,
                epoch_b: 3,
                differential: 4.0,
            },
        ];
        let result = sbas_inversion(5, &pairs, 1e-6).unwrap();
        assert_eq!(result.cumulative.len(), 5);
        assert_eq!(result.cumulative[0], 0.0);
        for i in 1..5 {
            assert!(
                (result.cumulative[i] - (i as f64 * 2.0)).abs() < 0.01,
                "epoch {}: expected {}, got {}",
                i,
                i as f64 * 2.0,
                result.cumulative[i]
            );
        }
        assert!((result.velocity - 2.0).abs() < 0.01);
        assert!(result.residual_rms < 0.01);
    }

    #[test]
    fn sbas_rejects_insufficient_epochs() {
        assert!(sbas_inversion(1, &[], 0.1).is_err());
    }

    #[test]
    fn persistent_scatterers_identify_stable_parameters() {
        // 5 epochs: params 0,1 are stable, param 2 drifts, param 3 oscillates
        let epochs = vec![
            vec![100.0, 50.0, 1.0, 10.0],
            vec![100.001, 50.002, 3.0, -10.0],
            vec![100.0, 49.999, 5.0, 10.0],
            vec![100.002, 50.001, 7.0, -10.0],
            vec![100.001, 50.0, 9.0, 10.0],
        ];
        let result = identify_persistent_scatterers(&epochs, 0.25, 0.70).unwrap();
        assert_eq!(result.total_parameters, 4);
        assert!(
            result.invariant_count >= 2,
            "expected at least 2 invariant, got {}",
            result.invariant_count
        );
        // The drifting and oscillating params should NOT be invariant
        let drifting = result.scatterers.iter().find(|s| s.parameter_index == 2);
        assert!(
            drifting.is_none() || drifting.unwrap().classification != "invariant",
            "drifting parameter should not be invariant"
        );
    }

    #[test]
    fn persistent_scatterers_reject_insufficient_epochs() {
        let epochs = vec![vec![1.0], vec![2.0]];
        assert!(identify_persistent_scatterers(&epochs, 0.25, 0.70).is_err());
    }
}
