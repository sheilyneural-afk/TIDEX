//! Bounded temporal tomography over the *measured* TIDE-X weight-update corpus.
//!
//! This module deliberately does not invent fast/slow weights, RPE, eligibility,
//! or other channels that are not present in [`DeltaObservation`].  It derives a
//! generation-ordered, reliability-weighted view of the real parameter deltas,
//! samples a bounded deterministic set of coordinates, and combines that with
//! full-vector RMS evidence.  The result is diagnostic evidence; it can veto a
//! mature pathological trajectory but never promotes a capability by itself.

use crate::foundation::contracts::DeltaObservation;
use crate::foundation::error::{BrainError, BrainResult};
use crate::foundation::linalg::{compensated_sum, stable_rms};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const WEIGHT_TOMOGRAPHY_SCHEMA: &str = "tidex.weight_tomography_observation/v1";
pub const TOMOGRAPHY_HISTORY_LIMIT: usize = 128;
pub const TOMOGRAPHY_MIN_GENERATIONS: usize = 16;
pub const TOMOGRAPHY_MAX_COORDINATES: usize = 64;

const MIN_GATE_CONFIDENCE: f64 = 0.55;
const PATHOLOGICAL_INSTABILITY: f64 = 0.72;
const HIGH_FREQUENCY_NOISE: f64 = 0.72;
const HIGH_ENTROPY_NOISE: f64 = 0.55;
const MAX_NOISY_TEMPORAL_DEPTH: f64 = 0.30;
const CHATTER_HIGH_FREQUENCY: f64 = 0.85;
const CHATTER_MIN_DOMINANT_FREQUENCY: f64 = 0.40;
const CHATTER_MAX_DIRECTIONAL_CONSISTENCY: f64 = 0.20;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WeightDynamicsMode {
    InsufficientHistory,
    Degenerate,
    DeepPersistent,
    ShallowOscillatory,
    CoherentTransitional,
    Diffuse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WeightTomographyObservation {
    pub schema: String,
    pub generation_count: usize,
    pub observation_count: usize,
    pub generation_span: u64,
    /// Constant interval between retained generations when the time grid is uniform.
    pub generation_interval: Option<u64>,
    pub uniform_generation_spacing: bool,
    pub parameter_dimension: usize,
    pub sampled_coordinate_count: usize,
    pub coordinate_coverage_ratio: f64,
    pub active_spectral_channels: usize,
    pub mean_reliability: f64,
    pub mean_update_rms: f64,
    pub max_update_rms: f64,
    pub dominant_frequency: f64,
    pub dominant_period: Option<f64>,
    /// Mean normalized linear trend across active sampled channels.
    pub dc_drift: f64,
    /// Scale-normalized spectral energy.  It is comparable within this schema,
    /// not a claim about physical energy in the model.
    pub total_spectral_energy: f64,
    pub low_frequency_ratio: f64,
    pub high_frequency_ratio: f64,
    pub spectral_entropy: f64,
    pub subaperture_coherence: f64,
    pub phase_lag_radians: f64,
    pub phase_stability: f64,
    pub persistence: f64,
    pub directional_consistency: f64,
    pub temporal_depth: f64,
    pub instability: f64,
    pub confidence: f64,
    pub consolidation_support: f64,
    pub consolidation_opposition: f64,
    /// Evidence quality inherited from observation reliability and SBAS closure.
    pub trajectory_quality: f64,
    pub evaluable: bool,
    pub classification: WeightDynamicsMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WeightTomographyGateReason {
    HistoryNotMature,
    IrregularGenerationSpacingAdvisory,
    SignalDegenerate,
    LowConfidenceAdvisory,
    BlocksPathologicalOscillation,
    BlocksHighFrequencyChatter,
    BlocksHighFrequencyNoise,
    SupportsOrNeutral,
    OppositionBelowBlockThreshold,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WeightTomographyGateDecision {
    pub evaluable: bool,
    pub allow: bool,
    pub reason: WeightTomographyGateReason,
    pub support: f64,
    pub opposition: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy)]
struct ComplexPair {
    re: f64,
    im: f64,
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn stable_mean(values: &[f64]) -> BrainResult<f64> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Invalid("weight_tomography_mean_input_invalid".into()));
    }
    Ok(compensated_sum(values.iter().copied())? / values.len() as f64)
}

fn weighted_nonnegative_mean(values: &[f64], weights: &[f64]) -> BrainResult<f64> {
    if values.is_empty()
        || values.len() != weights.len()
        || values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        || weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight <= 0.0)
    {
        return Err(BrainError::Invalid("weight_tomography_weighted_mean_input_invalid".into()));
    }
    let scale = values.iter().copied().fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return Ok(0.0);
    }
    let weight_sum = compensated_sum(weights.iter().copied())?;
    let normalized = compensated_sum(
        values
            .iter()
            .zip(weights)
            .map(|(value, weight)| weight * (value / scale)),
    )?;
    let result = scale * normalized / weight_sum;
    if !result.is_finite() || result < 0.0 {
        return Err(BrainError::Numerical("weight_tomography_weighted_mean_nonfinite".into()));
    }
    Ok(result)
}

fn deterministic_coordinate_indices(dimension: usize) -> Vec<usize> {
    let count = dimension.min(TOMOGRAPHY_MAX_COORDINATES);
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![0];
    }
    (0..count)
        .map(|slot| slot * (dimension - 1) / (count - 1))
        .collect()
}

fn detrend_scale_normalized(values: &[f64]) -> BrainResult<(Vec<f64>, f64)> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Invalid("weight_tomography_signal_invalid".into()));
    }
    let scale = values
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return Ok((vec![0.0; values.len()], 0.0));
    }
    let normalized = values.iter().map(|value| value / scale).collect::<Vec<_>>();
    if normalized.len() < 2 {
        return Ok((normalized, 0.0));
    }
    let n = normalized.len() as f64;
    let x_mean = (n - 1.0) * 0.5;
    let y_mean = stable_mean(&normalized)?;
    let numerator = compensated_sum(normalized.iter().enumerate().map(|(index, value)| {
        let x = index as f64 - x_mean;
        x * (*value - y_mean)
    }))?;
    let denominator = compensated_sum((0..normalized.len()).map(|index| {
        let x = index as f64 - x_mean;
        x * x
    }))?;
    let slope = if denominator > 0.0 {
        numerator / denominator
    } else {
        0.0
    };
    let detrended = normalized
        .iter()
        .enumerate()
        .map(|(index, value)| value - (y_mean + slope * (index as f64 - x_mean)))
        .collect::<Vec<_>>();
    if !slope.is_finite() || detrended.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Numerical("weight_tomography_detrend_nonfinite".into()));
    }
    Ok((detrended, slope))
}

fn hann_window(values: &[f64]) -> Vec<f64> {
    if values.len() <= 1 {
        return values.to_vec();
    }
    let denominator = values.len() as f64 - 1.0;
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let phase = 2.0 * std::f64::consts::PI * index as f64 / denominator;
            value * (0.5 - 0.5 * phase.cos())
        })
        .collect()
}

fn neumaier_add(sum: &mut f64, correction: &mut f64, value: f64) -> BrainResult<()> {
    if !value.is_finite() {
        return Err(BrainError::Numerical("weight_tomography_reduction_input_nonfinite".into()));
    }
    let updated = *sum + value;
    if sum.abs() >= value.abs() {
        *correction += (*sum - updated) + value;
    } else {
        *correction += (value - updated) + *sum;
    }
    *sum = updated;
    if !sum.is_finite() || !correction.is_finite() {
        return Err(BrainError::Numerical("weight_tomography_reduction_nonfinite".into()));
    }
    Ok(())
}

/// Bounded direct DFT.  History is capped at 128 samples, so avoiding another
/// numerical dependency keeps the production surface small while bounding work.
fn positive_dft(values: &[f64]) -> BrainResult<Vec<ComplexPair>> {
    if values.len() < 2 || values.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Invalid("weight_tomography_dft_input_invalid".into()));
    }
    let n = values.len();
    let mut output = Vec::with_capacity(n / 2);
    for frequency in 1..=n / 2 {
        let mut re = 0.0;
        let mut re_correction = 0.0;
        let mut im = 0.0;
        let mut im_correction = 0.0;
        for (time, value) in values.iter().enumerate() {
            let angle = -2.0 * std::f64::consts::PI * frequency as f64 * time as f64 / n as f64;
            let (sin, cos) = angle.sin_cos();
            neumaier_add(&mut re, &mut re_correction, value * cos)?;
            neumaier_add(&mut im, &mut im_correction, value * sin)?;
        }
        re += re_correction;
        im += im_correction;
        if !re.is_finite() || !im.is_finite() {
            return Err(BrainError::Numerical("weight_tomography_dft_nonfinite".into()));
        }
        output.push(ComplexPair { re, im });
    }
    Ok(output)
}

fn normalized_power(values: &[f64]) -> BrainResult<(Vec<f64>, f64)> {
    let (detrended, slope) = detrend_scale_normalized(values)?;
    if detrended.iter().all(|value| *value == 0.0) {
        return Ok((vec![0.0; values.len() / 2], slope));
    }
    let windowed = hann_window(&detrended);
    let power = positive_dft(&windowed)?
        .into_iter()
        .map(|value| value.re * value.re + value.im * value.im)
        .collect::<Vec<_>>();
    if power.iter().any(|value| !value.is_finite() || *value < 0.0) {
        return Err(BrainError::Numerical("weight_tomography_power_nonfinite".into()));
    }
    Ok((power, slope))
}

fn normalized_spectral_entropy(power: &[f64]) -> BrainResult<f64> {
    if power.is_empty() {
        return Ok(0.0);
    }
    let total = compensated_sum(power.iter().copied())?;
    if total <= f64::EPSILON || power.len() <= 1 {
        return Ok(0.0);
    }
    let entropy = compensated_sum(power.iter().copied().filter(|value| *value > 0.0).map(
        |value| {
            let probability = value / total;
            -probability * probability.ln()
        },
    ))?;
    Ok(clamp01(entropy / (power.len() as f64).ln()))
}

fn positive_autocorrelation(values: &[f64], lag: usize) -> BrainResult<Option<f64>> {
    if lag == 0 || values.len() <= lag + 2 {
        return Ok(None);
    }
    let scale = values
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return Ok(None);
    }
    let normalized = values.iter().map(|value| value / scale).collect::<Vec<_>>();
    let left = &normalized[..normalized.len() - lag];
    let right = &normalized[lag..];
    let left_mean = stable_mean(left)?;
    let right_mean = stable_mean(right)?;
    let numerator = compensated_sum(
        left.iter()
            .zip(right)
            .map(|(left, right)| (left - left_mean) * (right - right_mean)),
    )?;
    let left_energy = compensated_sum(left.iter().map(|value| (value - left_mean).powi(2)))?;
    let right_energy = compensated_sum(right.iter().map(|value| (value - right_mean).powi(2)))?;
    if left_energy <= f64::EPSILON || right_energy <= f64::EPSILON {
        return Ok(None);
    }
    let correlation = numerator / (left_energy.sqrt() * right_energy.sqrt());
    if !correlation.is_finite() {
        return Err(BrainError::Numerical("weight_tomography_autocorrelation_nonfinite".into()));
    }
    Ok(Some(correlation.clamp(0.0, 1.0)))
}

fn persistence(channels: &[Vec<f64>]) -> BrainResult<f64> {
    let mut scores = Vec::new();
    for channel in channels {
        for lag in [1_usize, 2, 4, 8] {
            if let Some(score) = positive_autocorrelation(channel, lag)? {
                scores.push(score);
            }
        }
    }
    if scores.is_empty() {
        Ok(0.0)
    } else {
        stable_mean(&scores)
    }
}

fn scaled_cosine(left: &[f64], right: &[f64]) -> BrainResult<Option<f64>> {
    if left.len() != right.len()
        || left.is_empty()
        || left.iter().chain(right).any(|value| !value.is_finite())
    {
        return Err(BrainError::Invalid("weight_tomography_direction_shape_invalid".into()));
    }
    let left_scale = left.iter().map(|value| value.abs()).fold(0.0, f64::max);
    let right_scale = right.iter().map(|value| value.abs()).fold(0.0, f64::max);
    if left_scale == 0.0 || right_scale == 0.0 {
        return Ok(None);
    }
    let numerator = compensated_sum(
        left.iter()
            .zip(right)
            .map(|(left, right)| (left / left_scale) * (right / right_scale)),
    )?;
    let left_energy = compensated_sum(left.iter().map(|value| (value / left_scale).powi(2)))?;
    let right_energy = compensated_sum(right.iter().map(|value| (value / right_scale).powi(2)))?;
    if left_energy <= 0.0 || right_energy <= 0.0 {
        return Ok(None);
    }
    let value = numerator / (left_energy.sqrt() * right_energy.sqrt());
    if !value.is_finite() {
        return Err(BrainError::Numerical("weight_tomography_direction_nonfinite".into()));
    }
    Ok(Some(value.clamp(-1.0, 1.0)))
}

fn directional_consistency(rows: &[Vec<f64>]) -> BrainResult<f64> {
    let mut scores = Vec::new();
    for pair in rows.windows(2) {
        if let Some(cosine) = scaled_cosine(&pair[0], &pair[1])? {
            scores.push(cosine.max(0.0));
        }
    }
    if scores.is_empty() {
        Ok(0.0)
    } else {
        stable_mean(&scores)
    }
}

fn normalized_window(values: &[f64]) -> BrainResult<Vec<f64>> {
    let (detrended, _) = detrend_scale_normalized(values)?;
    Ok(hann_window(&detrended))
}

fn subaperture_metrics(channels: &[Vec<f64>]) -> BrainResult<(f64, f64, f64)> {
    let Some(length) = channels.first().map(Vec::len) else {
        return Ok((0.0, 0.0, 0.0));
    };
    if length < 8 {
        return Ok((0.0, 0.0, 0.0));
    }
    let window = (length / 2).max(8).min(length);
    let mut cross_re = 0.0;
    let mut cross_im = 0.0;
    let mut master_energy = 0.0;
    let mut slave_energy = 0.0;
    let mut phase_x = 0.0;
    let mut phase_y = 0.0;
    let mut phase_weight_sum = 0.0;
    for channel in channels {
        if channel.len() != length {
            return Err(BrainError::Invalid("weight_tomography_channel_length_mismatch".into()));
        }
        let master = normalized_window(&channel[..window])?;
        let slave = normalized_window(&channel[length - window..])?;
        let master_fft = positive_dft(&master)?;
        let slave_fft = positive_dft(&slave)?;
        for (master, slave) in master_fft.iter().zip(&slave_fft) {
            let cross_r = master.re * slave.re + master.im * slave.im;
            let cross_i = master.im * slave.re - master.re * slave.im;
            let master_power = master.re * master.re + master.im * master.im;
            let slave_power = slave.re * slave.re + slave.im * slave.im;
            let weight = (master_power * slave_power).sqrt();
            cross_re += cross_r;
            cross_im += cross_i;
            master_energy += master_power;
            slave_energy += slave_power;
            if weight > f64::EPSILON {
                let phase = cross_i.atan2(cross_r);
                phase_x += weight * phase.cos();
                phase_y += weight * phase.sin();
                phase_weight_sum += weight;
            }
        }
    }
    for value in [
        cross_re,
        cross_im,
        master_energy,
        slave_energy,
        phase_x,
        phase_y,
        phase_weight_sum,
    ] {
        if !value.is_finite() {
            return Err(BrainError::Numerical("weight_tomography_subaperture_nonfinite".into()));
        }
    }
    let cross_magnitude = cross_re.hypot(cross_im);
    let coherence = if master_energy > f64::EPSILON && slave_energy > f64::EPSILON {
        clamp01(cross_magnitude / (master_energy.sqrt() * slave_energy.sqrt()))
    } else {
        0.0
    };
    let phase_lag = if cross_magnitude > f64::EPSILON {
        cross_im.atan2(cross_re)
    } else {
        0.0
    };
    let phase_stability = if phase_weight_sum > f64::EPSILON {
        clamp01(phase_x.hypot(phase_y) / phase_weight_sum)
    } else {
        0.0
    };
    Ok((coherence, phase_lag, phase_stability))
}

pub fn analyze_weight_dynamics(
    observations: &[DeltaObservation],
    cycle_rms: f64,
    max_edge_residual: f64,
) -> BrainResult<WeightTomographyObservation> {
    if observations.is_empty()
        || !cycle_rms.is_finite()
        || cycle_rms < 0.0
        || !max_edge_residual.is_finite()
        || max_edge_residual < 0.0
    {
        return Err(BrainError::Invalid("weight_tomography_input_invalid".into()));
    }
    let parameter_dimension = observations[0].delta.len();
    if parameter_dimension == 0
        || observations.iter().any(|observation| {
            observation.delta.len() != parameter_dimension
                || observation.delta.iter().any(|value| !value.is_finite())
                || !observation.reliability.is_finite()
                || observation.reliability <= 0.0
                || observation.reliability > 1.0
        })
    {
        return Err(BrainError::Invalid("weight_tomography_observation_contract_invalid".into()));
    }

    let mut by_generation = BTreeMap::<u64, Vec<&DeltaObservation>>::new();
    for observation in observations {
        by_generation
            .entry(observation.generation)
            .or_default()
            .push(observation);
    }
    for members in by_generation.values_mut() {
        members.sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
    }
    let all_generations = by_generation.keys().copied().collect::<Vec<_>>();
    let start = all_generations
        .len()
        .saturating_sub(TOMOGRAPHY_HISTORY_LIMIT);
    let generations = &all_generations[start..];
    let generation_count = generations.len();
    let generation_span = generations
        .last()
        .zip(generations.first())
        .map(|(last, first)| last.saturating_sub(*first))
        .unwrap_or(0);
    let generation_intervals = generations
        .windows(2)
        .map(|pair| pair[1].saturating_sub(pair[0]))
        .collect::<Vec<_>>();
    let generation_interval = generation_intervals
        .first()
        .copied()
        .filter(|value| *value > 0);
    let uniform_generation_spacing = generation_interval.is_some_and(|expected| {
        generation_intervals
            .iter()
            .all(|interval| *interval == expected)
    });
    let coordinate_indices = deterministic_coordinate_indices(parameter_dimension);
    let sampled_coordinate_count = coordinate_indices.len();
    let coordinate_coverage_ratio = sampled_coordinate_count as f64 / parameter_dimension as f64;

    let mut sampled_rows = Vec::with_capacity(generation_count);
    let mut update_rms_series = Vec::with_capacity(generation_count);
    let mut recent_reliabilities = Vec::new();
    let mut recent_observation_count = 0usize;

    for generation in generations {
        let members = by_generation
            .get(generation)
            .ok_or_else(|| BrainError::Integrity("weight_tomography_generation_missing".into()))?;
        let weights = members
            .iter()
            .map(|member| member.reliability)
            .collect::<Vec<_>>();
        let total_weight = compensated_sum(weights.iter().copied())?;
        if total_weight <= 0.0 || !total_weight.is_finite() {
            return Err(BrainError::Numerical(
                "weight_tomography_generation_weight_invalid".into(),
            ));
        }
        let mut row = Vec::with_capacity(sampled_coordinate_count);
        for coordinate in &coordinate_indices {
            let value = compensated_sum(
                members
                    .iter()
                    .map(|member| member.reliability * member.delta[*coordinate]),
            )? / total_weight;
            if !value.is_finite() {
                return Err(BrainError::Numerical(
                    "weight_tomography_generation_coordinate_nonfinite".into(),
                ));
            }
            row.push(value);
        }
        let member_rms = members
            .iter()
            .map(|member| stable_rms(member.delta.iter().copied()))
            .collect::<BrainResult<Vec<_>>>()?;
        update_rms_series.push(weighted_nonnegative_mean(&member_rms, &weights)?);
        recent_reliabilities.extend(weights);
        recent_observation_count = recent_observation_count
            .checked_add(members.len())
            .ok_or_else(|| BrainError::Invalid("weight_tomography_count_overflow".into()))?;
        sampled_rows.push(row);
    }

    let mean_reliability = if recent_reliabilities.is_empty() {
        0.0
    } else {
        stable_mean(&recent_reliabilities)?
    };
    let mean_update_rms = if update_rms_series.is_empty() {
        0.0
    } else {
        weighted_nonnegative_mean(&update_rms_series, &vec![1.0; update_rms_series.len()])?
    };
    let max_update_rms = update_rms_series.iter().copied().fold(0.0_f64, f64::max);

    let mut channels = Vec::with_capacity(sampled_coordinate_count + 1);
    for coordinate in 0..sampled_coordinate_count {
        channels.push(
            sampled_rows
                .iter()
                .map(|row| row[coordinate])
                .collect::<Vec<_>>(),
        );
    }
    // Full-vector RMS is a real all-parameter channel and prevents the bounded
    // coordinate sample from being the only view of update magnitude.
    channels.push(update_rms_series.clone());

    let spectrum_len = generation_count / 2;
    let mut aggregate_power = vec![0.0_f64; spectrum_len];
    let mut active_spectral_channels = 0usize;
    let mut normalized_spectral_energy_sum = 0.0_f64;
    let mut slopes = Vec::new();
    for channel in &channels {
        if channel.len() < 2 || channel.iter().all(|value| *value == 0.0) {
            continue;
        }
        let (power, slope) = normalized_power(channel)?;
        let total = compensated_sum(power.iter().copied())?;
        slopes.push(slope);
        if total <= f64::EPSILON {
            continue;
        }
        let normalization = (channel.len() as f64).powi(2).max(1.0);
        normalized_spectral_energy_sum += total / normalization;
        if !normalized_spectral_energy_sum.is_finite() {
            return Err(BrainError::Numerical(
                "weight_tomography_spectral_energy_nonfinite".into(),
            ));
        }
        for (aggregate, value) in aggregate_power.iter_mut().zip(power) {
            *aggregate += value / total;
        }
        active_spectral_channels += 1;
    }
    if active_spectral_channels > 0 {
        for value in &mut aggregate_power {
            *value /= active_spectral_channels as f64;
        }
    }
    let total_spectral_energy = if active_spectral_channels == 0 {
        0.0
    } else {
        normalized_spectral_energy_sum / active_spectral_channels as f64
    };
    let aggregate_total = compensated_sum(aggregate_power.iter().copied())?;
    let mut dominant_bin = 0usize;
    let mut dominant_power = 0.0;
    let mut low = 0.0;
    let mut high = 0.0;
    for (index, value) in aggregate_power.iter().copied().enumerate() {
        let bin = index + 1;
        let frequency = if generation_count > 0 {
            bin as f64 / generation_count as f64
        } else {
            0.0
        };
        if value > dominant_power {
            dominant_power = value;
            dominant_bin = bin;
        }
        if frequency <= 0.125 {
            low += value;
        }
        if frequency >= 0.25 {
            high += value;
        }
    }
    let dominant_frequency = if dominant_bin > 0 && generation_count > 0 {
        dominant_bin as f64 / generation_count as f64
    } else {
        0.0
    };
    let dominant_period = (dominant_frequency > 0.0).then_some(1.0 / dominant_frequency);
    let low_frequency_ratio = if aggregate_total > f64::EPSILON {
        clamp01(low / aggregate_total)
    } else {
        0.0
    };
    let high_frequency_ratio = if aggregate_total > f64::EPSILON {
        clamp01(high / aggregate_total)
    } else {
        0.0
    };
    let spectral_entropy = normalized_spectral_entropy(&aggregate_power)?;
    let persistence = persistence(&channels)?;
    let directional_consistency = directional_consistency(&sampled_rows)?;
    let (subaperture_coherence, phase_lag_radians, phase_stability) =
        subaperture_metrics(&channels)?;
    let dc_drift = if slopes.is_empty() {
        0.0
    } else {
        stable_mean(&slopes)?.clamp(-1.0, 1.0)
    };

    let residual_sum = cycle_rms + max_edge_residual;
    let residual_quality = if residual_sum.is_finite() {
        1.0 / (1.0 + residual_sum)
    } else {
        0.0
    };
    let trajectory_quality = clamp01(mean_reliability * residual_quality);
    let maturity = if generation_count <= 4 {
        0.0
    } else {
        clamp01((generation_count as f64 - 4.0) / (TOMOGRAPHY_MIN_GENERATIONS as f64 - 4.0))
    };
    let temporal_depth = clamp01(
        maturity
            * (0.30 * low_frequency_ratio
                + 0.22 * persistence
                + 0.22 * subaperture_coherence
                + 0.16 * directional_consistency
                + 0.10 * (1.0 - spectral_entropy)),
    );
    let instability = clamp01(
        maturity
            * (0.42 * high_frequency_ratio
                + 0.28 * spectral_entropy
                + 0.18 * (1.0 - phase_stability)
                + 0.12 * (1.0 - directional_consistency)),
    );
    let confidence = clamp01(
        maturity
            * trajectory_quality
            * (0.34 * subaperture_coherence
                + 0.24 * phase_stability
                + 0.20 * persistence
                + 0.12 * directional_consistency
                + 0.10 * (1.0 - spectral_entropy)),
    );
    let consolidation_support = clamp01(
        confidence
            * temporal_depth
            * (0.45 + 0.55 * directional_consistency)
            * (1.0 - 0.70 * instability),
    );
    let consolidation_opposition =
        clamp01(confidence * instability * (0.55 * high_frequency_ratio + 0.45 * spectral_entropy));
    let evaluable = generation_count >= TOMOGRAPHY_MIN_GENERATIONS
        && active_spectral_channels > 0
        && uniform_generation_spacing;
    let classification = if generation_count < TOMOGRAPHY_MIN_GENERATIONS {
        WeightDynamicsMode::InsufficientHistory
    } else if active_spectral_channels == 0 {
        WeightDynamicsMode::Degenerate
    } else if temporal_depth >= 0.55 && instability <= 0.45 {
        WeightDynamicsMode::DeepPersistent
    } else if instability >= 0.60 || high_frequency_ratio >= 0.65 {
        WeightDynamicsMode::ShallowOscillatory
    } else if subaperture_coherence >= 0.60 {
        WeightDynamicsMode::CoherentTransitional
    } else {
        WeightDynamicsMode::Diffuse
    };

    let report = WeightTomographyObservation {
        schema: WEIGHT_TOMOGRAPHY_SCHEMA.into(),
        generation_count,
        observation_count: recent_observation_count,
        generation_span,
        generation_interval,
        uniform_generation_spacing,
        parameter_dimension,
        sampled_coordinate_count,
        coordinate_coverage_ratio,
        active_spectral_channels,
        mean_reliability,
        mean_update_rms,
        max_update_rms,
        dominant_frequency,
        dominant_period,
        dc_drift,
        total_spectral_energy,
        low_frequency_ratio,
        high_frequency_ratio,
        spectral_entropy,
        subaperture_coherence,
        phase_lag_radians,
        phase_stability,
        persistence,
        directional_consistency,
        temporal_depth,
        instability,
        confidence,
        consolidation_support,
        consolidation_opposition,
        trajectory_quality,
        evaluable,
        classification,
    };
    let scalar_values = [
        report.coordinate_coverage_ratio,
        report.mean_reliability,
        report.mean_update_rms,
        report.max_update_rms,
        report.dominant_frequency,
        report.dc_drift,
        report.total_spectral_energy,
        report.low_frequency_ratio,
        report.high_frequency_ratio,
        report.spectral_entropy,
        report.subaperture_coherence,
        report.phase_lag_radians,
        report.phase_stability,
        report.persistence,
        report.directional_consistency,
        report.temporal_depth,
        report.instability,
        report.confidence,
        report.consolidation_support,
        report.consolidation_opposition,
        report.trajectory_quality,
    ];
    if scalar_values.iter().any(|value| !value.is_finite())
        || report
            .dominant_period
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        return Err(BrainError::Numerical("weight_tomography_report_nonfinite".into()));
    }
    Ok(report)
}

pub fn tomography_gate(observation: &WeightTomographyObservation) -> WeightTomographyGateDecision {
    if observation.generation_count < TOMOGRAPHY_MIN_GENERATIONS {
        return WeightTomographyGateDecision {
            evaluable: false,
            allow: true,
            reason: WeightTomographyGateReason::HistoryNotMature,
            support: observation.consolidation_support,
            opposition: observation.consolidation_opposition,
            confidence: observation.confidence,
        };
    }
    if !observation.uniform_generation_spacing {
        return WeightTomographyGateDecision {
            evaluable: false,
            allow: true,
            reason: WeightTomographyGateReason::IrregularGenerationSpacingAdvisory,
            support: observation.consolidation_support,
            opposition: observation.consolidation_opposition,
            confidence: observation.confidence,
        };
    }
    if !observation.evaluable {
        return WeightTomographyGateDecision {
            evaluable: false,
            allow: true,
            reason: WeightTomographyGateReason::SignalDegenerate,
            support: observation.consolidation_support,
            opposition: observation.consolidation_opposition,
            confidence: observation.confidence,
        };
    }
    if observation.confidence < MIN_GATE_CONFIDENCE {
        return WeightTomographyGateDecision {
            evaluable: true,
            allow: true,
            reason: WeightTomographyGateReason::LowConfidenceAdvisory,
            support: observation.consolidation_support,
            opposition: observation.consolidation_opposition,
            confidence: observation.confidence,
        };
    }
    let pathological_oscillation = observation.instability >= PATHOLOGICAL_INSTABILITY
        && observation.consolidation_opposition > observation.consolidation_support;
    let coherent_chatter = observation.high_frequency_ratio >= CHATTER_HIGH_FREQUENCY
        && observation.dominant_frequency >= CHATTER_MIN_DOMINANT_FREQUENCY
        && observation.directional_consistency <= CHATTER_MAX_DIRECTIONAL_CONSISTENCY;
    let high_frequency_noise = observation.high_frequency_ratio >= HIGH_FREQUENCY_NOISE
        && observation.spectral_entropy >= HIGH_ENTROPY_NOISE
        && observation.temporal_depth < MAX_NOISY_TEMPORAL_DEPTH;
    let (allow, reason) = if pathological_oscillation {
        (false, WeightTomographyGateReason::BlocksPathologicalOscillation)
    } else if coherent_chatter {
        (false, WeightTomographyGateReason::BlocksHighFrequencyChatter)
    } else if high_frequency_noise {
        (false, WeightTomographyGateReason::BlocksHighFrequencyNoise)
    } else if observation.consolidation_support >= observation.consolidation_opposition {
        (true, WeightTomographyGateReason::SupportsOrNeutral)
    } else {
        (true, WeightTomographyGateReason::OppositionBelowBlockThreshold)
    };
    WeightTomographyGateDecision {
        evaluable: true,
        allow,
        reason,
        support: observation.consolidation_support,
        opposition: observation.consolidation_opposition,
        confidence: observation.confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::digest::{ProvenanceDigest, Sha256Digest};
    use crate::foundation::identity::ObservationId;

    fn observation(generation: u64, delta: Vec<f64>, reliability: f64) -> DeltaObservation {
        DeltaObservation {
            observation_id: ObservationId::parse(format!("wt-{generation:04}")).unwrap(),
            from_checkpoint: format!("c-{generation:04}"),
            to_checkpoint: format!("c-{:04}", generation + 1),
            generation,
            delta,
            functional_response: Vec::new(),
            confounders: Vec::new(),
            reliability,
            independence_group: format!("g-{generation:04}"),
            experiment_lineage: Default::default(),
            dense_artifact: None,
            parameter_layout_sha256: None,
            representation_artifact: None,
            representation_protocol_sha256: None,
            provenance_digest: ProvenanceDigest::from(Sha256Digest::digest_bytes(
                format!("wt-{generation}").as_bytes(),
            )),
        }
    }

    #[test]
    fn slow_coherent_dynamics_are_deeper_than_alternating_chatter() {
        let coherent = (0..64)
            .map(|generation| {
                let t = generation as f64;
                let value = 0.10 + 0.025 * (2.0 * std::f64::consts::PI * t / 32.0).sin();
                observation(generation, vec![value, 0.8 * value, 0.6 * value, 0.4 * value], 0.95)
            })
            .collect::<Vec<_>>();
        let chatter = (0..64)
            .map(|generation| {
                let sign = if generation % 2 == 0 { 1.0 } else { -1.0 };
                observation(
                    generation,
                    vec![0.12 * sign, 0.10 * sign, 0.08 * sign, 0.06 * sign],
                    0.95,
                )
            })
            .collect::<Vec<_>>();
        let stable = analyze_weight_dynamics(&coherent, 0.01, 0.01).unwrap();
        let noisy = analyze_weight_dynamics(&chatter, 0.01, 0.01).unwrap();
        assert!(
            stable.temporal_depth > noisy.temporal_depth,
            "stable={stable:#?} noisy={noisy:#?}"
        );
        assert!(noisy.high_frequency_ratio > stable.high_frequency_ratio);
        assert!(noisy.instability > stable.instability);
        assert!(stable.directional_consistency > noisy.directional_consistency);
        let noisy_gate = tomography_gate(&noisy);
        assert!(!noisy_gate.allow, "gate={noisy_gate:#?} noisy={noisy:#?}");
    }

    #[test]
    fn immature_history_is_advisory_and_never_blocks() {
        let observations = (0..10)
            .map(|generation| observation(generation, vec![0.1, 0.2], 1.0))
            .collect::<Vec<_>>();
        let report = analyze_weight_dynamics(&observations, 0.0, 0.0).unwrap();
        let gate = tomography_gate(&report);
        assert!(!report.evaluable);
        assert!(!gate.evaluable);
        assert!(gate.allow);
        assert_eq!(gate.reason, WeightTomographyGateReason::HistoryNotMature);
    }

    #[test]
    fn poor_trajectory_closure_reduces_confidence_without_fabricating_a_veto() {
        let observations = (0..32)
            .map(|generation| {
                let value = 0.1 + 0.01 * (generation as f64 * 0.2).sin();
                observation(generation, vec![value, 0.5 * value], 0.9)
            })
            .collect::<Vec<_>>();
        let clean = analyze_weight_dynamics(&observations, 0.0, 0.0).unwrap();
        let poor = analyze_weight_dynamics(&observations, 10.0, 10.0).unwrap();
        assert!(clean.confidence > poor.confidence);
        assert!(clean.trajectory_quality > poor.trajectory_quality);
        let gate = tomography_gate(&poor);
        assert!(gate.allow);
        assert_eq!(gate.reason, WeightTomographyGateReason::LowConfidenceAdvisory);
    }

    #[test]
    fn coordinate_sampling_is_bounded_and_uses_the_full_span() {
        let indices = deterministic_coordinate_indices(10_000);
        assert_eq!(indices.len(), TOMOGRAPHY_MAX_COORDINATES);
        assert_eq!(indices.first().copied(), Some(0));
        assert_eq!(indices.last().copied(), Some(9_999));
        assert!(indices.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn irregular_generation_spacing_is_diagnostic_but_cannot_veto() {
        let observations = (0..20)
            .map(|index| {
                let generation = if index < 10 { index } else { index + 3 } as u64;
                let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
                observation(generation, vec![0.1 * sign, 0.08 * sign], 1.0)
            })
            .collect::<Vec<_>>();
        let report = analyze_weight_dynamics(&observations, 0.0, 0.0).unwrap();
        let gate = tomography_gate(&report);
        assert!(!report.uniform_generation_spacing);
        assert!(!report.evaluable);
        assert!(gate.allow);
        assert_eq!(gate.reason, WeightTomographyGateReason::IrregularGenerationSpacingAdvisory);
    }

    #[test]
    fn invalid_numeric_evidence_is_rejected_not_zero_filled() {
        let mut observations = (0..16)
            .map(|generation| observation(generation, vec![0.1, 0.2], 1.0))
            .collect::<Vec<_>>();
        observations[3].delta[0] = f64::NAN;
        assert!(analyze_weight_dynamics(&observations, 0.0, 0.0).is_err());
    }
}
