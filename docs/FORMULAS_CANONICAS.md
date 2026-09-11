# Fórmulas canónicas TIDE-X (forma cerrada + ancla en código)

Este documento lista **todas** las identidades matemáticas explícitas y reglas de actualización del sistema, con referencia a archivo/función. El inventario línea-a-línea de 849 funciones está en `FORMULAS_ALGORITMICAS.md` + `FORMULAS_INDEX.md`.

**Total fórmulas/anclas en esta hoja:** 151

### 1. F01 Dot product
- **Ancla:** `src/foundation/linalg.rs::dot`
- **Forma / líneas:** \(a\cdot b=\sum_i a_i b_i\)
- **Fragmento:**
```rust
pub fn dot(a: &[f64], b: &[f64]) -> BrainResult<f64> {
        return Err(BrainError::Invalid("dot_shape_or_value".into()));
    let normalized = compensated_sum(
    let value = (normalized * left_scale) * right_scale;
        return Err(BrainError::Numerical("dot_non_finite_result".into()));
```

### 2. F02 L2 norm
- **Ancla:** `src/foundation/linalg.rs::norm`
- **Forma / líneas:** \(\|x\|_2=\sqrt{\sum_i x_i^2}\)
- **Fragmento:**
```rust
pub fn norm(a: &[f64]) -> BrainResult<f64> {
        return Err(BrainError::Invalid("norm_input_invalid".into()));
    let value = scale * scaled_square_sum.sqrt();
        return Err(BrainError::Numerical("norm_non_finite_result".into()));
```

### 3. F03 Stable RMS
- **Ancla:** `src/foundation/linalg.rs::stable_rms`
- **Forma / líneas:** RMS escalado anti-overflow (recurrencia)
- **Fragmento:**
```rust
            return Err(BrainError::Numerical("stable_rms_input_nonfinite".into()));
            .ok_or_else(|| BrainError::Invalid("stable_rms_count_overflow".into()))?;
            sum_squares += ratio * ratio;
        return Err(BrainError::Invalid("stable_rms_empty".into()));
        scale * (sum_squares / count as f64).sqrt()
        return Err(BrainError::Numerical("stable_rms_nonfinite".into()));
```

### 4. F04 Neumaier sum
- **Ancla:** `src/foundation/linalg.rs::compensated_sum`
- **Forma / líneas:** Suma compensada Neumaier
- **Fragmento:**
```rust
            return Err(BrainError::Numerical("compensated_sum_input_nonfinite".into()));
            correction += (sum - updated) + value;
            correction += (value - updated) + sum;
        return Err(BrainError::Numerical("compensated_sum_nonfinite".into()));
```

### 5. F05 Normalize
- **Ancla:** `src/foundation/linalg.rs::normalize`
- **Forma / líneas:** \(\hat x=x/\|x\|_2\)
- **Fragmento:**
```rust
pub fn normalize(a: &[f64]) -> BrainResult<Vec<f64>> {
    let n = norm(a)?;
        return Err(BrainError::Numerical("normalize_zero_norm".into()));
    let normalized = a.iter().map(|value| value / n).collect::<Vec<_>>();
    if normalized.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Numerical("normalize_non_finite_result".into()));
    Ok(normalized)
```

### 6. F06 Cosine similarity
- **Ancla:** `src/foundation/linalg.rs::cosine`
- **Forma / líneas:** \(\cos(a,b)=(a\cdot b)/(\|a\|\|b\|)\)
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("cosine_dimension_mismatch".into()));
    let left_norm = norm(a)?;
    let right_norm = norm(b)?;
    if left_norm == 0.0 || right_norm == 0.0 {
        return Err(BrainError::Numerical("cosine_zero_norm".into()));
        .map(|(left, right)| (left / left_norm) * (right / right_norm))
        return Err(BrainError::Numerical("cosine_non_finite_result".into()));
```

### 7. F07 Ridge inverse
- **Ancla:** `src/foundation/linalg.rs::inverse_with_ridge`
- **Forma / líneas:** \((A^\top A+\lambda I)^{-1}\)
- **Fragmento:**
```rust
pub fn inverse_with_ridge(a: &Matrix, ridge: f64) -> BrainResult<Matrix> {
    if a.rows != a.cols || a.rows == 0 || !ridge.is_finite() || ridge < 0.0 {
        return Err(BrainError::Invalid("inverse_shape".into()));
        base.data[i * n + i] += ridge;
```

### 8. F08 Jacobi eigen
- **Ancla:** `src/foundation/linalg.rs::symmetric_eigen_jacobi_raw`
- **Forma / líneas:** Eigenvalores simétricos Jacobi
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("jacobi_eigen_shape".into()));
    let symmetry_tolerance = f64::EPSILON.sqrt() * scale * n as f64;
                return Err(BrainError::Invalid("jacobi_eigen_matrix_not_symmetric".into()));
    let default_rotations = n.saturating_mul(n).saturating_mul(8);
        let c = phi.cos();
            let dkp = d.get(k, p);
            let np = c * dkp - s * dkq;
            let nq = s * dkp + c * dkq;
```

### 9. F09 Rank-1 damped min-norm
- **Ancla:** `src/foundation/low_rank_math.rs::solve_minimum_norm_rank_one`
- **Forma / líneas:** \(\Delta W = y x^\top / (x^\top x + d)\)
- **Fragmento:**
```rust
pub fn solve_minimum_norm_rank_one(
) -> BrainResult<MinimumNormRankOneSolution> {
        return Err(BrainError::Invalid("minimum_norm_rank_one_input_invalid".into()));
    let input_squared_norm = input_activation
    if !input_squared_norm.is_finite() || input_squared_norm <= f64::EPSILON {
        return Err(BrainError::Numerical("minimum_norm_rank_one_activation_degenerate".into()));
    let denominator = input_squared_norm + damping;
        return Err(BrainError::Numerical("minimum_norm_rank_one_denominator_invalid".into()));
        return Err(BrainError::Numerical("minimum_norm_rank_one_factor_non_finite".into()));
    let residual_norm = predicted_shift
        .sqrt();
    let left_squared_norm = desired_output_shift
    let right_squared_norm = right
    let frobenius_norm = (left_squared_norm * right_squared_norm).sqrt();
    if !residual_norm.is_finite() || !frobenius_norm.is_finite() || frobenius_norm == 0.0 {
        return Err(BrainError::Numerical("minimum_norm_rank_one_solution_invalid".into()));
    Ok(MinimumNormRankOneSolution {
        residual_norm,
        frobenius_norm,
```

### 10. F10 Multi-case ridge
- **Ancla:** `src/foundation/low_rank_math.rs::solve_regularized_multi_case_low_rank`
- **Forma / líneas:** \(Y(XX^\top+\lambda I)^{-1}X\)
- **Fragmento:**
```rust
pub fn solve_regularized_multi_case_low_rank(
) -> BrainResult<MultiCaseLowRankSolution> {
        || cases as u64 > MAX_LOW_RANK
        return Err(BrainError::Invalid("multi_case_low_rank_input_invalid".into()));
        return Err(BrainError::Invalid("multi_case_low_rank_examples_invalid".into()));
        gram[row * cases + row] += damping;
```

### 11. F11 Relative residual
- **Ancla:** `src/foundation/low_rank_math.rs::dense_multi_case_relative_residual`
- **Forma / líneas:** \(\|\hat Y-Y\|/\|Y\|\)
- **Fragmento:**
```rust
pub fn dense_multi_case_relative_residual(
        || dense.len() != rows.saturating_mul(columns)
        return Err(BrainError::Integrity("multi_case_low_rank_residual_inputs_invalid".into()));
    let mut residual_squared = 0.0_f64;
            residual_squared += (predicted - shift[row]).powi(2);
            target_squared += shift[row].powi(2);
        return Err(BrainError::Numerical("multi_case_low_rank_relative_residual_invalid".into()));
    let relative = residual_squared.sqrt() / target_squared.sqrt();
        return Err(BrainError::Numerical("multi_case_low_rank_relative_residual_invalid".into()));
```

### 12. F12 Regression R²
- **Ancla:** `src/foundation/validation.rs::regression_r2`
- **Forma / líneas:** \(R^2=1-SS_{res}/SS_{tot}\)
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("regression_r2_shape".into()));
    let means = (0..dim)
            sse += (actual_row[column] - predicted_row[column]).powi(2);
            sst += (actual_row[column] - means[column]).powi(2);
    Ok(if sst <= 1e-18 { 0.0 } else { 1.0 - sse / sst })
```

### 13. F13 Effective rank
- **Ancla:** `src/foundation/validation.rs::effective_rank_from_spectrum`
- **Forma / líneas:** rango efectivo del espectro
- **Fragmento:**
```rust
pub fn effective_rank_from_spectrum(eigenvalues: &[f64]) -> BrainResult<f64> {
        return Err(BrainError::Invalid("effective_rank_spectrum_nonfinite".into()));
```

### 14. F14 Energy rank
- **Ancla:** `src/foundation/validation.rs::choose_energy_rank`
- **Forma / líneas:** selección de rango por energía acumulada
- **Fragmento:**
```rust
pub fn choose_energy_rank(
    target_explained_variance: f64,
    max_rank: usize,
    minimum_rank: usize,
        || !target_explained_variance.is_finite()
        || !(0.0..=1.0).contains(&target_explained_variance)
        || max_rank == 0
        || minimum_rank > max_rank
        return Err(BrainError::Invalid("energy_rank_input_invalid".into()));
        return Ok(minimum_rank.min(eigenvalues.len()));
    let cap = max_rank.min(eigenvalues.len());
        accumulated += value.max(0.0);
        if accumulated / total >= target_explained_variance {
            return Ok((index + 1).max(minimum_rank).min(cap));
    Ok(cap.max(minimum_rank.min(eigenvalues.len())))
```

### 15. BCM: `apply_decay`
- **Ancla:** `src/cross_model/plasticity/bcm_metaplasticity.rs::apply_decay`
- **Forma / líneas:** state.theta_m = (state.theta_m * (1.0 - self.config.theta_decay)).clamp(0.0, 1.0);
- **Fragmento:**
```rust
    pub fn apply_decay(&mut self) {
            state.theta_m = (state.theta_m * (1.0 - self.config.theta_decay)).clamp(0.0, 1.0);
```

### 16. Eligibility: `update_trace`
- **Ancla:** `src/cross_model/plasticity/eligibility_traces.rs::update_trace`
- **Forma / líneas:** trace.trace_value = (trace.trace_value * self.config.decay_factor
- **Fragmento:**
```rust
    pub fn update_trace(&mut self, capability_name: &str, activation: f64) -> Result<f64, String> {
        let trace = self
            .traces
            .ok_or("eligibility_trace_missing")?;
        trace.trace_value = (trace.trace_value * self.config.decay_factor
            + self.config.trace_update_rate * activation)
            .clamp(0.0, self.config.max_trace_value);
        trace.last_update = chrono::Utc::now().to_rfc3339();
        Ok(trace.trace_value)
```

### 17. Eligibility: `accumulate_credit`
- **Ancla:** `src/cross_model/plasticity/eligibility_traces.rs::accumulate_credit`
- **Forma / líneas:** trace.credit_accumulated += credit * trace.trace_value;
- **Fragmento:**
```rust
        let trace = self
            .traces
            .ok_or("eligibility_trace_missing")?;
        trace.credit_accumulated += credit * trace.trace_value;
        if !trace.credit_accumulated.is_finite() {
        Ok(trace.credit_accumulated)
```

### 18. Neuromodulation: `calculate_plasticity_modulation`
- **Ancla:** `src/cross_model/plasticity/neuromodulation.rs::calculate_plasticity_modulation`
- **Forma / líneas:** modulation += *weight * self.get_level(*modulator)?;
- **Fragmento:**
```rust
            modulation += *weight * self.get_level(*modulator)?;
```

### 19. Elo: `update_observed`
- **Ancla:** `src/cross_model/plasticity/elo_system.rs::update_observed`
- **Forma / líneas:** let observed_second = 1.0 - first_observed_score;
- **Fragmento:**
```rust
        first_observed_score: f64,
            || !first_observed_score.is_finite()
            || !(0.0..=1.0).contains(&first_observed_score)
        let first_rating = self
            .ratings
            .rating;
        let second_rating = self
            .ratings
            .rating;
            / (1.0 + 10.0_f64.powf((second_rating - first_rating) / self.config.logistic_scale));
        let expected_second = 1.0 - expected_first;
        let observed_second = 1.0 - first_observed_score;
        let first_new = (first_rating
            + self.config.k_factor * (first_observed_score - expected_first))
            .clamp(self.config.rating_floor, self.config.rating_ceiling);
        let second_new = (second_rating
            .clamp(self.config.rating_floor, self.config.rating_ceiling);
                .ratings
            state.rating = first_new;
                .ratings
```

### 20. PI: `update`
- **Ancla:** `src/cross_model/plasticity/pi_controller.rs::update`
- **Forma / líneas:** let candidate_integral = (self.state.integral + error * dt)
- **Fragmento:**
```rust
        let error = setpoint - measurement;
        let candidate_integral = (self.state.integral + error * dt)
            self.config.proportional_gain * error + self.config.integral_gain * candidate_integral;
        let saturated_high = raw > self.config.output_max && error > 0.0;
        let saturated_low = raw < self.config.output_min && error < 0.0;
        self.state.last_error = error;
```

### 21. Routing plasticity: `update_weight`
- **Ancla:** `src/cross_model/plasticity/routing_plasticity.rs::update_weight`
- **Forma / líneas:** let delta = self.learning_rate * correlation - self.decay_rate;; *weight = (*weight + delta).clamp(0.0, 1.0);
- **Fragmento:**
```rust
        let delta = self.learning_rate * correlation - self.decay_rate;
```

### 22. Routing plasticity: `decay_tick`
- **Ancla:** `src/cross_model/plasticity/routing_plasticity.rs::decay_tick`
- **Forma / líneas:** let factor = 1.0 - self.decay_rate;
- **Fragmento:**
```rust
    pub fn decay_tick(&mut self) -> Result<(), String> {
        if !self.decay_rate.is_finite() || !(0.0..=1.0).contains(&self.decay_rate) {
            return Err("routing_plasticity_decay_rate_must_be_within_0_1".into());
        let factor = 1.0 - self.decay_rate;
                *weight *= factor;
```

### 23. Routing plasticity: `route_capability`
- **Ancla:** `src/cross_model/plasticity/routing_plasticity.rs::route_capability`
- **Forma / líneas:** ver código
- **Fragmento:**
```rust
        let mut scored = observations
            .filter(|row| row.score >= self.config.minimum_measured_score)
                    * ((total_samples.max(1.0).ln() / row.sample_size as f64).max(0.0)).sqrt();
                    let mean_gap = prior_history
                        .map(|decision| (decision.routing_score - row.score).abs())
                    (1.0 - mean_gap.clamp(0.0, 1.0)).clamp(0.0, 1.0)
                let adjusted_score = self.matrix.stability_adjusted_score(
                    row.score,
                Ok((row, uncertainty, adjusted_score))
        let selected = scored
                .map_err(|error| format!("routing_serialize:{error}"))?,
            measured_score: selected.0.score,
            routing_score: selected.2,
```

### 24. Content plasticity: `consolidate_fact`
- **Ancla:** `src/cross_model/plasticity/content_plasticity.rs::consolidate_fact`
- **Forma / líneas:** confidence = (confidence * (1.0 - decay) + evidence_strength * self.consolidation_rate)
- **Fragmento:**
```rust
        let decay = self.temporal_decay.get(fact_id).copied().unwrap_or(0.0);
        confidence = (confidence * (1.0 - decay) + evidence_strength * self.consolidation_rate)
        self.temporal_decay
            .insert(fact_id.to_string(), (decay + self.eligibility_decay_content).min(0.99));
```

### 25. Pearson / protected map: `pearson`
- **Ancla:** `src/analysis/protected_map.rs::pearson`
- **Forma / líneas:** let lm = left.iter().sum::<f64>() / left.len() as f64;; let rm = right.iter().sum::<f64>() / right.len() as f64;; let denom = (ld * rd).sqrt();
- **Fragmento:**
```rust
        numerator += (l - lm) * (r - rm);
        ld += (l - lm).powi(2);
        rd += (r - rm).powi(2);
    let denom = (ld * rd).sqrt();
```

### 26. Pearson / protected map: `metric_dual_direction`
- **Ancla:** `src/analysis/protected_map.rs::metric_dual_direction`
- **Forma / líneas:** Represent a sensitivity covector g in the metric used by protected.rs.
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("protected_map_metric_dual_shape".into()));
                return Err(BrainError::Numerical(
            whitened.push(gradient / importance.sqrt());
    let dual_norm = norm(&whitened)?;
    if dual_norm == 0.0 {
        return Err(BrainError::Numerical("protected_map_metric_dual_degenerate".into()));
            (value / dual_norm) / importance.sqrt()
            return Err(BrainError::Numerical(
        metric_energy += energy;
        || (metric_energy - 1.0).abs() > f64::EPSILON.sqrt() * covector.len() as f64
        return Err(BrainError::Numerical("protected_map_metric_dual_normalization".into()));
```

### 27. Pearson / protected map: `build_protected_cortex_map`
- **Ancla:** `src/analysis/protected_map.rs::build_protected_cortex_map`
- **Forma / líneas:** parameter_importance[parameter] += weight * probe.sensitivity[parameter].powi(2);; let denom = lambda.sqrt();; let coefficient = weights[row].sqrt() * eigenvector[row] / denom;
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("protected_map_config_invalid".into()));
    let selected_rank =
        choose_energy_rank(&eigenvalues, target_explained_sensitivity, eigenvalues.len(), 0)?;
        return Err(BrainError::Numerical("protected_map_sensitivity_energy_degenerate".into()));
        eigenvalues.iter().take(selected_rank).sum::<f64>() / total_energy;
            parameter_importance[parameter] += weight * probe.sensitivity[parameter].powi(2);
    let fisher_trace = parameter_importance.iter().sum::<f64>();
    if !fisher_trace.is_finite() || fisher_trace <= 0.0 {
        return Err(BrainError::Numerical("protected_map_fisher_trace_degenerate".into()));
            return Err(BrainError::Numerical(
    let mut directions = Vec::with_capacity(selected_rank);
    for (component, (lambda, eigenvector)) in eigs.iter().take(selected_rank).enumerate() {
        let denom = lambda.sqrt();
            return Err(BrainError::Numerical("protected_map_zero_singular_value".into()));
```

### 28. Pearson / protected map: `build_protected_cortex_from_persistent_scatterers`
- **Ancla:** `src/analysis/protected_map.rs::build_protected_cortex_from_persistent_scatterers`
- **Forma / líneas:** let score = (ps.temporal_coherence / (1.0 + ps.amplitude_dispersion)).clamp(0.05, 1.0);; parameter_importance[ps.parameter_index] = (weight * score).clamp(0.05, 1.0);
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("protected_cortex_damage_ratio_invalid".into()));
        crate::analysis::temporal_tracking::identify_persistent_scatterers(epochs, 0.25, 0.70)?;
            "invariant" => 1.0,
        let score = (ps.temporal_coherence / (1.0 + ps.amplitude_dispersion)).clamp(0.05, 1.0);
        parameter_importance[ps.parameter_index] = (weight * score).clamp(0.05, 1.0);
        // For invariant parameters, construct coordinate-aligned canonical protected direction
        if ps.classification == "invariant" && directions.len() < 32 {
                importance: score,
```

### 29. Relational transport: `validate`
- **Ancla:** `src/analysis/transport.rs::validate`
- **Forma / líneas:** Self::FixedRidge { ridge } => *ridge,; Self::CenteredTraceRidge { relative_ridge } => *relative_ridge,
- **Fragmento:**
```rust
            Self::FixedRidge { ridge } => *ridge,
            Self::CenteredTraceRidge { relative_ridge } => *relative_ridge,
                    return Err(BrainError::Invalid("transport_sinkhorn_max_iter_zero".into()));
            return Err(BrainError::Invalid("transport_regularization_policy_invalid".into()));
```

### 30. Relational transport: `fit_affine`
- **Ancla:** `src/analysis/transport.rs::fit_affine`
- **Forma / líneas:** ver código
- **Fragmento:**
```rust
fn fit_affine(source: &[Vec<f64>], target: &[Vec<f64>], ridge: f64) -> BrainResult<TransportMap> {
        return Err(BrainError::Invalid("transport_anchor_count".into()));
    if !ridge.is_finite() || ridge <= 0.0 {
        return Err(BrainError::Invalid("transport_ridge_invalid".into()));
    let mut squared_error = 0.0;
        let beta = weighted_normal_solve(&design, &y, &row_weights, ridge)?;
            squared_error += (prediction - target[row][output]).powi(2);
        training_rms: (squared_error / (source.len() * target_dim) as f64).sqrt(),
```

### 31. Relational transport: `fit_affine_with_policy`
- **Ancla:** `src/analysis/transport.rs::fit_affine_with_policy`
- **Forma / líneas:** let map = fit_affine(source, target, *ridge)?;; AffineTransportPolicy::CenteredTraceRidge { relative_ridge } => *relative_ridge,; let unit_scale = stable_rms(centered.iter().flatten().copied())? * count.sqrt();; let centered_design_trace = regularization_scale * source_dim as f64;; let effective_ridge = relative_ridge * regularization_scale;
- **Fragmento:**
```rust
    let relative_ridge = match policy {
        AffineTransportPolicy::FixedRidge { ridge } => {
            let map = fit_affine(source, target, *ridge)?;
                centered_design_trace: None,
                effective_ridge: *ridge,
        AffineTransportPolicy::CenteredTraceRidge { relative_ridge } => *relative_ridge,
                centered_design_trace: None,
                effective_ridge: *reg,
        return Err(BrainError::Invalid("transport_anchor_count".into()));
    let means = |rows: &[Vec<f64>], dimension: usize| -> BrainResult<Vec<f64>> {
    let source_mean = means(source, source_dim)?;
    let target_mean = means(target, target_dim)?;
                .zip(&source_mean)
                .map(|(x, mean)| x - mean)
    // This is algebraically the declared trace-scaled ridge, not a new solver.
    let unit_scale = stable_rms(centered.iter().flatten().copied())? * count.sqrt();
    let centered_design_trace = regularization_scale * source_dim as f64;
    let effective_ridge = relative_ridge * regularization_scale;
        || !centered_design_trace.is_finite()
        || !effective_ridge.is_finite()
```

### 32. Relational transport: `compute_sinkhorn_optimal_transport`
- **Ancla:** `src/analysis/transport.rs::compute_sinkhorn_optimal_transport`
- **Forma / líneas:** kernel[i][j] = (-cost_matrix[i][j] / reg).exp();; wasserstein_distance += p_ij * cost_matrix[i][j];
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("sinkhorn_transport_invalid_inputs".into()));
            return Err(BrainError::Invalid("sinkhorn_source_dimension_mismatch".into()));
                return Err(BrainError::Invalid("sinkhorn_target_dimension_mismatch".into()));
                sq_dist += (source[i][d] - target[j][d]).powi(2);
                kb += kernel[i][j] * b[j];
                kt_a += kernel[i][j] * next_a[i];
```

### 33. Relational transport: `validated_from_predictions`
- **Ancla:** `src/analysis/transport.rs::validated_from_predictions`
- **Forma / líneas:** let loo_cv_rms = (squared_error / (target.len() * target[0].len()) as f64).sqrt();; let mean_loo_cosine = cosines.iter().sum::<f64>() / cosines.len() as f64;
- **Fragmento:**
```rust
    let squared_error = target
    let loo_cv_rms = (squared_error / (target.len() * target[0].len()) as f64).sqrt();
    let mean_loo_cosine = cosines.iter().sum::<f64>() / cosines.len() as f64;
        mean_loo_cosine,
```

### 34. Relational transport: `learn_relational_transport`
- **Ancla:** `src/analysis/transport.rs::learn_relational_transport`
- **Forma / líneas:** let numerical_tolerance = f64::EPSILON.sqrt();
- **Fragmento:**
```rust
    ridge: f64,
        return Err(BrainError::Invalid("relational_transport_anchor_count".into()));
    if !ridge.is_finite() || ridge <= 0.0 {
        return Err(BrainError::Invalid("relational_transport_ridge".into()));
    let source = normalize_anchor_rows(source_anchors, "relational_source")?;
    let target = normalize_anchor_rows(target_anchors, "relational_target")?;
    let mut loo_coefficient_norms = Vec::with_capacity(source.len());
        let coefficients = relational_coefficients(&train_source, &source[holdout], ridge)?;
        let coefficient_norm = norm(&coefficients)?;
        if !source_cosine.is_finite() || !target_cosine.is_finite() || !coefficient_norm.is_finite()
            return Err(BrainError::Numerical("relational_transport_non_finite_loo".into()));
        loo_coefficient_norms.push(coefficient_norm);
    let mean_loo_source_cosine =
    let mean_loo_target_cosine =
    let max_loo_coefficient_norm = loo_coefficient_norms
    let numerical_tolerance = f64::EPSILON.sqrt();
        && max_loo_coefficient_norm.is_finite();
        ridge,
```

### 35. Relational transport: `centered_trace_ridge_preserves_source_units_and_affine_origins`
- **Ancla:** `src/analysis/transport.rs::centered_trace_ridge_preserves_source_units_and_affine_origins`
- **Forma / líneas:** let expected_ridge = reference_diagnostics.full_fit.effective_ridge * scale * scale;
- **Fragmento:**
```rust
    fn centered_trace_ridge_preserves_source_units_and_affine_origins() {
        let policy = AffineTransportPolicy::CenteredTraceRidge {
            relative_ridge: 0.2,
            let expected_ridge = reference_diagnostics.full_fit.effective_ridge * scale * scale;
            assert!((diagnostics.full_fit.effective_ridge / expected_ridge - 1.0).abs() < 1e-12);
```

### 36. Pythagoras geodesic: `evaluate_and_correct`
- **Ancla:** `src/analysis/pythagoras_topology.rs::evaluate_and_correct`
- **Forma / líneas:** if l2_norm <= f64::EPSILON * (dim as f64).sqrt() || l1_sum <= f64::EPSILON * dim as f64 {; let inflation_ratio = l1_sum / l2_norm;; let correction_factor = l2_norm / l1_sum;; let corrected_norm = l2_norm; // Corrected norm IS the L2 norm
- **Fragmento:**
```rust
            return Err(BrainError::Invalid("pythagoras_empty_delta".into()));
        // Use robust norm from linalg instead of manual calculation
        let l2_norm = norm(delta_w)?;
        // Check if both norms are effectively zero (relative to precision)
        if l2_norm <= f64::EPSILON * (dim as f64).sqrt() || l1_sum <= f64::EPSILON * dim as f64 {
                geodesic_l2_length: l2_norm,
                corrected_geodesic_norm: 0.0,
        if !l1_sum.is_finite() || !l2_norm.is_finite() {
            return Err(BrainError::Numerical("pythagoras_norm_overflow".into()));
        let inflation_ratio = l1_sum / l2_norm;
        let correction_factor = l2_norm / l1_sum;
        let corrected_norm = l2_norm; // Corrected norm IS the L2 norm
            return Err(BrainError::Numerical("pythagoras_ratio_overflow".into()));
            geodesic_l2_length: l2_norm,
            corrected_geodesic_norm: corrected_norm,
```

### 37. Pythagoras geodesic: `analyze_topology`
- **Ancla:** `src/analysis/pythagoras_topology.rs::analyze_topology`
- **Forma / líneas:** let dist = sum_sq.sqrt();
- **Fragmento:**
```rust
            return Err(BrainError::Invalid("topology_empty_points".into()));
                return Err(BrainError::Invalid("topology_dimension_mismatch".into()));
                schema: "topological_skill_manifold:v2".into(),
                mean_persistence_lifetime: 1.0,
                topological_homotopy_score: 1.0,
                let dist = sum_sq.sqrt();
                j += 1;
            i += 1;
```

### 38. Pythagoras geodesic: `process_range_doppler`
- **Ancla:** `src/analysis/pythagoras_topology.rs::process_range_doppler`
- **Forma / líneas:** let row_b = weight_matrix.row(r + 1);; let shift = corr.estimated_shift.iter().sum::<f64>() / (cols as f64);
- **Fragmento:**
```rust
            return Err(BrainError::Invalid("sar_subapertures_zero".into()));
            return Err(BrainError::Invalid("sar_empty_matrix".into()));
                sub_energies[sub_idx] += val * val;
        // Normalize subaperture energies
                let corr = crate::analysis::temporal_tracking::phase_correlation(row_a, row_b)
                    .map_err(|error| {
                        BrainError::Integrity(format!("sar_phase_correlation_failed:{r}:{error}"))
                cumulative_shift += shift;
                coherence_sum += corr.peak_magnitude;
                coherence_count += 1;
        // Focused coherence score from true phase correlation peaks
```

### 39. Pythagoras geodesic: `pythagoras_staircase_corrects_high_dim_step_inflation`
- **Ancla:** `src/analysis/pythagoras_topology.rs::pythagoras_staircase_corrects_high_dim_step_inflation`
- **Forma / líneas:** ver código
- **Fragmento:**
```rust
        // L1 length = 100.0, L2 length = sqrt(100) = 10.0
        // Inflation ratio = 10.0 (sqrt(100))
        let corrected_l2 = norm(&corrected).unwrap();
```

### 40. Temporal / SBAS inversion: `phase_correlation`
- **Ancla:** `src/analysis/temporal_tracking.rs::phase_correlation`
- **Forma / líneas:** let mean = (a + b) * 0.5;; let phase = diff.atan2(mean.abs() + 1e-15);; let peak_real = cross_real_sum / nf;; let peak_imag = cross_imag_sum / nf;; let peak_magnitude = (peak_real * peak_real + peak_imag * peak_imag).sqrt();
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("phase_correlation_dimension_mismatch".into()));
    let mut phase_residuals = Vec::with_capacity(n);
            return Err(BrainError::Numerical("phase_correlation_non_finite_input".into()));
        let mean = (a + b) * 0.5;
        // Phase: atan2(diff, |mean|+eps) — measures the angular shift
        let phase = diff.atan2(mean.abs() + 1e-15);
        phase_residuals.push(phase);
        // Accumulate normalised cross-power: cos(phase) + j·sin(phase)
        cross_real_sum += phase.cos();
        cross_imag_sum += phase.sin();
            incoherent_count += 1;
    let peak_magnitude = (peak_real * peak_real + peak_imag * peak_imag).sqrt();
    let phase_rms = stable_rms(phase_residuals.iter().copied())?;
        phase_residuals,
```

### 41. Temporal / SBAS inversion: `cholesky_solve`
- **Ancla:** `src/analysis/temporal_tracking.rs::cholesky_solve`
- **Forma / líneas:** sum += l[j * n + k] * l[j * n + k];; let diag = ata[j * n + j] - sum;; l[j * n + j] = diag.sqrt();
- **Fragmento:**
```rust
            sum += l[j * n + k] * l[j * n + k];
            return Err(BrainError::Numerical("sbas_cholesky_not_positive".into()));
        l[j * n + j] = diag.sqrt();
                s += l[i * n + k] * l[j * n + k];
            s += l[i * n + k] * y[k];
            s += l[k * n + i] * x[k];
            return Err(BrainError::Numerical("sbas_solution_non_finite".into()));
```

### 42. Temporal / SBAS inversion: `identify_persistent_scatterers`
- **Ancla:** `src/analysis/temporal_tracking.rs::identify_persistent_scatterers`
- **Forma / líneas:** let mean = values.iter().sum::<f64>() / nf;; let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / nf;; let std_dev = variance.sqrt();; let phase = diff.atan2(mean.abs() + 1e-15);; let temporal_coherence = ((cos_sum / npf).powi(2) + (sin_sum / npf).powi(2)).sqrt();
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("ps_insufficient_epochs".into()));
        return Err(BrainError::Invalid("ps_empty_parameters".into()));
            return Err(BrainError::Invalid("ps_epoch_dimension_mismatch".into()));
    let mut invariant_count = 0_usize;
                return Err(BrainError::Numerical("ps_non_finite_parameter_value".into()));
        // Mean and standard deviation.
        let mean = values.iter().sum::<f64>() / nf;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / nf;
        let std_dev = variance.sqrt();
        let amplitude_dispersion = if mean.abs() > 1e-15 {
            std_dev / mean.abs()
            let phase = diff.atan2(mean.abs() + 1e-15);
            cos_sum += phase.cos();
            sin_sum += phase.sin();
        let temporal_coherence = ((cos_sum / npf).powi(2) + (sin_sum / npf).powi(2)).sqrt();
            invariant_count += 1;
            "invariant"
```

### 43. Trust region: `quadratic_cost`
- **Ancla:** `src/analysis/trust_region.rs::quadratic_cost`
- **Forma / líneas:** ver código
- **Fragmento:**
```rust
pub fn quadratic_cost(covariance: &Matrix, coefficients: &[f64]) -> BrainResult<f64> {
    validate_quadratic_metric(covariance)?;
    if covariance.rows != coefficients.len()
        return Err(BrainError::Invalid("trust_region_shape".into()));
    let product = covariance.matvec(coefficients)?;
    let cost = dot(coefficients, &product)?;
    if cost < -f64::EPSILON.sqrt() {
        return Err(BrainError::Numerical("trust_region_negative_psd_cost".into()));
```

### 44. Trust region: `apply_quadratic_trust_region`
- **Ancla:** `src/analysis/trust_region.rs::apply_quadratic_trust_region`
- **Forma / líneas:** ver código
- **Fragmento:**
```rust
    covariance: &Matrix,
        return Err(BrainError::Invalid("trust_region_budget_invalid".into()));
    let proposed = quadratic_cost(covariance, coefficients)?;
        (max_quadratic_cost / proposed).sqrt().clamp(0.0, 1.0)
    let accepted = quadratic_cost(covariance, &accepted_coefficients)?;
```

### 45. Trust region: `apply_pythagoras_geodesic_trust_region`
- **Ancla:** `src/analysis/trust_region.rs::apply_pythagoras_geodesic_trust_region`
- **Forma / líneas:** Applies a trust region constraint that eliminates the Pythagoras Staircase
- **Fragmento:**
```rust
    covariance: &Matrix,
        return Err(BrainError::Invalid("trust_region_budget_invalid".into()));
    let proposed = quadratic_cost(covariance, &corrected_coefficients)?;
        (max_quadratic_cost / proposed).sqrt().clamp(0.0, 1.0)
    let accepted_cost = quadratic_cost(covariance, &accepted_coefficients)?;
        return Err(BrainError::Numerical(format!(
```

### 46. Trust region: `causal_priority_candidate`
- **Ancla:** `src/analysis/trust_region.rs::causal_priority_candidate`
- **Forma / líneas:** let tolerance = f64::EPSILON.sqrt()
- **Fragmento:**
```rust
    let tolerance = f64::EPSILON.sqrt()
            return Err(BrainError::Integrity("causal_trust_region_priority_ratio_invalid".into()));
```

### 47. Trust region: `apply_causal_priority_trust_region`
- **Ancla:** `src/analysis/trust_region.rs::apply_causal_priority_trust_region`
- **Forma / líneas:** let tolerance = f64::EPSILON.sqrt() * (1.0 + proposed.abs() + max_quadratic_cost.abs());; if accepted <= max_quadratic_cost + tolerance {; if best_cost <= max_quadratic_cost + tolerance {
- **Fragmento:**
```rust
    covariance: &Matrix,
        return Err(BrainError::Invalid("causal_trust_region_contract_invalid".into()));
    validate_quadratic_metric(covariance)?;
    let proposed = quadratic_cost(covariance, coefficients)?;
    let tolerance = f64::EPSILON.sqrt() * (1.0 + proposed.abs() + max_quadratic_cost.abs());
    let signed_metric = signed_magnitude_metric(covariance, coefficients)?;
        let max_iterations = coefficients.len().saturating_mul(128).max(128);
                        BrainError::Integrity("causal_trust_region_no_lawful_contraction".into())
                return Err(BrainError::Integrity("causal_trust_region_curvature_invalid".into()));
                return Err(BrainError::Integrity("causal_trust_region_zero_relief".into()));
                return Err(BrainError::Numerical(
```

### 48. Identifiability spectrum: `gram_of_fields`
- **Ancla:** `src/analysis/identifiability.rs::gram_of_fields`
- **Forma / líneas:** let norm_tolerance = f64::EPSILON.sqrt() * (dim.max(1) as f64).sqrt() * 16.0;
- **Fragmento:**
```rust
fn gram_of_fields(fields: &[SkillField]) -> BrainResult<Matrix> {
        return Err(BrainError::Invalid("identifiability_fields_required".into()));
    let mut skill_ids = BTreeSet::new();
    let norm_tolerance = f64::EPSILON.sqrt() * (dim.max(1) as f64).sqrt() * 16.0;
        return Err(BrainError::Invalid("identifiability_field_shape".into()));
        if field.skill_id.trim().is_empty()
            || !skill_ids.insert(field.skill_id.as_str())
            || (norm(&field.direction)? - 1.0).abs() > norm_tolerance
            return Err(BrainError::Invalid("identifiability_field_shape".into()));
            let value = crate::foundation::linalg::dot(&fields[i].direction, &fields[j].direction)?;
```

### 49. Identifiability spectrum: `excitation_information`
- **Ancla:** `src/analysis/identifiability.rs::excitation_information`
- **Forma / líneas:** info.data[i * info.cols + j] += weight * left * coefficients.get(row, j);
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("identifiability_excitation_shape".into()));
                info.data[i * info.cols + j] += weight * left * coefficients.get(row, j);
```

### 50. Identifiability spectrum: `spectrum_and_rank`
- **Ancla:** `src/analysis/identifiability.rs::spectrum_and_rank`
- **Forma / líneas:** let tolerance = largest * f64::EPSILON.sqrt() * matrix.rows.max(1) as f64;; let rank = spectrum.iter().filter(|value| **value > tolerance).count();
- **Fragmento:**
```rust
fn spectrum_and_rank(matrix: &Matrix) -> BrainResult<(Vec<f64>, usize, f64)> {
        .map(|(eigenvalue, _)| eigenvalue.max(0.0).sqrt())
    // Numerical rank is derived from floating-point resolution and matrix size,
    let tolerance = largest * f64::EPSILON.sqrt() * matrix.rows.max(1) as f64;
    let rank = spectrum.iter().filter(|value| **value > tolerance).count();
    let condition = if rank == 0 || !smallest_resolved.is_finite() {
    Ok((spectrum, rank, condition))
```

### 51. Identifiability spectrum: `resolution_map`
- **Ancla:** `src/analysis/identifiability.rs::resolution_map`
- **Forma / líneas:** let coefficient_rms = (weighted_squared_coefficient / total_observation_weight).sqrt();; let covariance_tolerance = f64::EPSILON.sqrt(); let posterior_std = (covariance_diagonal.max(0.0) * noise_variance).sqrt();
- **Fragmento:**
```rust
    fields: &[SkillField],
    ridge: f64,
        || !ridge.is_finite()
        || ridge <= 0.0
        return Err(BrainError::Invalid("identifiability_input_contract".into()));
    let (geometry_spectrum, geometry_rank, geometry_condition) = spectrum_and_rank(&geometry)?;
    let (excitation_spectrum, excitation_rank, excitation_condition) =
        spectrum_and_rank(&excitation)?;
    let resolved_rank = geometry_rank.min(excitation_rank);
    let covariance = inverse_with_ridge(&excitation, ridge)?;
    let noise_variance = reconstruction_rms.powi(2).max(f64::EPSILON);
            return Err(BrainError::Numerical("identifiability_coefficient_energy_invalid".into()));
        let coefficient_rms = (weighted_squared_coefficient / total_observation_weight).sqrt();
        let covariance_diagonal = covariance.get(field_index, field_index);
        let covariance_tolerance = f64::EPSILON.sqrt()
            * covariance
        if covariance_diagonal < -covariance_tolerance {
            return Err(BrainError::Numerical(
                "identifiability_negative_posterior_variance".into(),
        let posterior_std = (covariance_diagonal.max(0.0) * noise_variance).sqrt();
```

### 52. Aperture independence: `standardized_profiles`
- **Ancla:** `src/analysis/aperture_independence.rs::standardized_profiles`
- **Forma / líneas:** means[col] = profiles.iter().map(|row| row[col]).sum::<f64>() / profiles.len() as f64;; stds[col] = variance.sqrt();
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("aperture_independence_profiles_required".into()));
        return Err(BrainError::Invalid("aperture_independence_profile_shape".into()));
    let mut means = vec![0.0; cols];
        means[col] = profiles.iter().map(|row| row[col]).sum::<f64>() / profiles.len() as f64;
        let variance = profiles
            .map(|row| (row[col] - means[col]).powi(2))
        stds[col] = variance.sqrt();
        if stds[col] > f64::EPSILON.sqrt() * scale {
                .map(|&col| (row[col] - means[col]) / stds[col].max(1e-15))
```

### 53. Aperture independence: `design_numerical_rank`
- **Ancla:** `src/analysis/aperture_independence.rs::design_numerical_rank`
- **Forma / líneas:** let tolerance = largest * f64::EPSILON.sqrt() * gram.rows.max(1) as f64;
- **Fragmento:**
```rust
fn design_numerical_rank(profiles: &[Vec<f64>]) -> BrainResult<usize> {
    let tolerance = largest * f64::EPSILON.sqrt() * gram.rows.max(1) as f64;
```

### 54. Aperture independence: `estimate_aperture_independence`
- **Ancla:** `src/analysis/aperture_independence.rs::estimate_aperture_independence`
- **Forma / líneas:** let square_sum = eigenvalues.iter().map(|value| value * value).sum::<f64>();
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("aperture_minimum_required_groups_invalid".into()));
    let numerical_design_rank = design_numerical_rank(&profiles)?;
        return Err(BrainError::Integrity("aperture_lineage_profile_group_order_mismatch".into()));
    let mean_cross_group_similarity = if cross.is_empty() {
```

### 55. Dual space recurrence: `representation_centroid`
- **Ancla:** `src/analysis/dual_space.rs::representation_centroid`
- **Forma / líneas:** centroid[dimension] += weight * sign * rows[index][dimension];
- **Fragmento:**
```rust
        .skill_source_mixtures
        .ok_or_else(|| BrainError::Integrity("dual_space_mixture_missing".into()))?;
        return Err(BrainError::Invalid("dual_space_empty_field_support".into()));
        total += weight;
            centroid[dimension] += weight * sign * rows[index][dimension];
        return Err(BrainError::Invalid("dual_space_holdout_removes_all_field_support".into()));
    normalize(&centroid).map_err(|error| match error {
        BrainError::Numerical(_) => {
            BrainError::Numerical("dual_space_representation_centroid_degenerate".into())
```

### 56. Dual space recurrence: `expected_field_for_observation`
- **Ancla:** `src/analysis/dual_space.rs::expected_field_for_observation`
- **Forma / líneas:** let ambiguity_tolerance = magnitude * f64::EPSILON.sqrt();
- **Fragmento:**
```rust
    let mut ranked = Vec::with_capacity(model.fields.len());
        let coefficient = model.skill_source_mixtures[field_index][observation_index];
        ranked.push((field_index, coefficient.abs(), coefficient.signum()));
    ranked.sort_by(|left, right| right.1.total_cmp(&left.1).then(left.0.cmp(&right.0)));
    let (field_index, magnitude, sign) = ranked
            BrainError::Invalid("dual_space_observation_without_field_support".into())
    let ambiguity_tolerance = magnitude * f64::EPSILON.sqrt();
    if ranked
        return Err(BrainError::Invalid(
```

### 57. Dual space recurrence: `analyze_dual_space`
- **Ancla:** `src/analysis/dual_space.rs::analyze_dual_space`
- **Forma / líneas:** let tolerance = max_abs * f64::EPSILON.sqrt();
- **Fragmento:**
```rust
        || !config.ridge.is_finite()
        || config.ridge < 0.0
        return Err(BrainError::Invalid("dual_space_input_contract".into()));
        config.ridge,
    let (match_accuracy, mean_matched_cosine, min_match_margin_observed, recurrence_centroids) =
        return Err(BrainError::Integrity("dual_space_signature_count_mismatch".into()));
        let mixture = &model.skill_source_mixtures[field_index];
        let tolerance = max_abs * f64::EPSILON.sqrt();
            skill_id: field.skill_id.clone(),
```

### 58. Functional map: `fit_functional_map`
- **Ancla:** `src/analysis/functional.rs::fit_functional_map`
- **Forma / líneas:** let mean = y.iter().sum::<f64>() / y.len().max(1) as f64;
- **Fragmento:**
```rust
    ridge: f64,
        return Err(BrainError::Invalid("functional_shape".into()));
        return Err(BrainError::Invalid("functional_coefficients_non_finite".into()));
    if !ridge.is_finite() || ridge <= 0.0 {
        return Err(BrainError::Invalid("functional_ridge_invalid".into()));
        return Err(BrainError::Invalid("functional_reliability_invalid".into()));
        return Err(BrainError::Invalid("functional_response_non_finite".into()));
        .ok_or_else(|| BrainError::Invalid("functional_evidence_required".into()))?;
        return Err(BrainError::Invalid("functional_response_dimension_mismatch".into()));
            let beta = weighted_normal_solve(&x, &y, &weights, ridge)?;
            let mean = y.iter().sum::<f64>() / y.len().max(1) as f64;
                sse += (actual - pred).powi(2);
                sst += (actual - mean).powi(2);
```

### 59. Gauge align: `align_bases`
- **Ancla:** `src/analysis/gauge.rs::align_bases`
- **Forma / líneas:** cost[i][j] = 1.0 - cosine(&reference[i], &candidate[j])?.abs().clamp(0.0, 1.0);
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("gauge_empty_basis".into()));
        return Err(BrainError::Invalid("gauge_dimension_mismatch".into()));
            cost[i][j] = 1.0 - cosine(&reference[i], &candidate[j])?.abs().clamp(0.0, 1.0);
                    u[p[j]] += delta;
```

### 60. Confounder removal: `raw_design`
- **Ancla:** `src/analysis/confounders.rs::raw_design`
- **Forma / líneas:** let mean = (0..x.rows).map(|row| x.get(row, column)).sum::<f64>() / x.rows.max(1) as f64;; let std = variance.sqrt();
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("confounder_observations_required".into()));
        return Err(BrainError::Invalid("confounder_duplicate_name".into()));
            return Err(BrainError::Invalid(format!(
            return Err(BrainError::Invalid(format!(
                BrainError::Invalid(format!("confounder_missing:{row_index}:{name}"))
    // No intercept is fitted: a common persistent update may itself be skill.
        let mean = (0..x.rows).map(|row| x.get(row, column)).sum::<f64>() / x.rows.max(1) as f64;
        let variance = (0..x.rows)
            .map(|row| (x.get(row, column) - mean).powi(2))
        let std = variance.sqrt();
                x.set(row, column, (x.get(row, column) - mean) / std);
```

### 61. Confounder removal: `remove_confounders`
- **Ancla:** `src/analysis/confounders.rs::remove_confounders`
- **Forma / líneas:** normal.data[i * q + j] += weights[r] * x.get(r, i) * x.get(r, j);; projection += x.get(r, i) * inv.get(i, j) * x.get(sidx, j) * weights[sidx];; total_energy += weights[r] * d.get(r, p).powi(2);; residual_energy += weights[r] * residuals.get(r, p).powi(2);
- **Fragmento:**
```rust
    ridge: f64,
        return Err(BrainError::Invalid("confounder_min_observations".into()));
    if !ridge.is_finite() || ridge <= 0.0 {
        return Err(BrainError::Invalid("confounder_ridge_invalid".into()));
        return Err(BrainError::Invalid("delta_dimension_mismatch".into()));
        return Err(BrainError::Invalid("confounder_reliability_invalid".into()));
            residuals: d,
    let mut normal = Matrix::zeros(q, q);
                normal.data[i * q + j] += weights[r] * x.get(r, i) * x.get(r, j);
    let inv = inverse_with_ridge(&normal, ridge)?;
                    projection += x.get(r, i) * inv.get(i, j) * x.get(sidx, j) * weights[sidx];
    let residuals = transform.matmul(&d)?;
```

### 62. Weight tomography: `weighted_nonnegative_mean`
- **Ancla:** `src/analysis/weight_tomography.rs::weighted_nonnegative_mean`
- **Forma / líneas:** let result = scale * normalized / weight_sum;
- **Fragmento:**
```rust
fn weighted_nonnegative_mean(values: &[f64], weights: &[f64]) -> BrainResult<f64> {
        return Err(BrainError::Invalid("weight_tomography_weighted_mean_input_invalid".into()));
    let normalized = compensated_sum(
    let result = scale * normalized / weight_sum;
        return Err(BrainError::Numerical("weight_tomography_weighted_mean_nonfinite".into()));
```

### 63. Weight tomography: `detrend_scale_normalized`
- **Ancla:** `src/analysis/weight_tomography.rs::detrend_scale_normalized`
- **Forma / líneas:** let normalized = values.iter().map(|value| value / scale).collect::<Vec<_>>();; let x_mean = (n - 1.0) * 0.5;
- **Fragmento:**
```rust
fn detrend_scale_normalized(values: &[f64]) -> BrainResult<(Vec<f64>, f64)> {
        return Err(BrainError::Invalid("weight_tomography_signal_invalid".into()));
    let normalized = values.iter().map(|value| value / scale).collect::<Vec<_>>();
    if normalized.len() < 2 {
        return Ok((normalized, 0.0));
    let n = normalized.len() as f64;
    let x_mean = (n - 1.0) * 0.5;
    let y_mean = stable_mean(&normalized)?;
    let numerator = compensated_sum(normalized.iter().enumerate().map(|(index, value)| {
        let x = index as f64 - x_mean;
        x * (*value - y_mean)
    let denominator = compensated_sum((0..normalized.len()).map(|index| {
        let x = index as f64 - x_mean;
    let detrended = normalized
        .map(|(index, value)| value - (y_mean + slope * (index as f64 - x_mean)))
        return Err(BrainError::Numerical("weight_tomography_detrend_nonfinite".into()));
```

### 64. Weight tomography: `neumaier_add`
- **Ancla:** `src/analysis/weight_tomography.rs::neumaier_add`
- **Forma / líneas:** let updated = *sum + value;; *correction += (*sum - updated) + value;; *correction += (value - updated) + *sum;
- **Fragmento:**
```rust
        return Err(BrainError::Numerical("weight_tomography_reduction_input_nonfinite".into()));
        *correction += (*sum - updated) + value;
        *correction += (value - updated) + *sum;
        return Err(BrainError::Numerical("weight_tomography_reduction_nonfinite".into()));
```

### 65. Weight tomography: `normalized_spectral_entropy`
- **Ancla:** `src/analysis/weight_tomography.rs::normalized_spectral_entropy`
- **Forma / líneas:** let entropy = compensated_sum(power.iter().copied().filter(|value| *value > 0.0).map(
- **Fragmento:**
```rust
fn normalized_spectral_entropy(power: &[f64]) -> BrainResult<f64> {
```

### 66. Weight tomography: `positive_autocorrelation`
- **Ancla:** `src/analysis/weight_tomography.rs::positive_autocorrelation`
- **Forma / líneas:** let normalized = values.iter().map(|value| value / scale).collect::<Vec<_>>();; let correlation = numerator / (left_energy.sqrt() * right_energy.sqrt());
- **Fragmento:**
```rust
    let normalized = values.iter().map(|value| value / scale).collect::<Vec<_>>();
    let left = &normalized[..normalized.len() - lag];
    let right = &normalized[lag..];
    let left_mean = stable_mean(left)?;
    let right_mean = stable_mean(right)?;
            .map(|(left, right)| (left - left_mean) * (right - right_mean)),
    let left_energy = compensated_sum(left.iter().map(|value| (value - left_mean).powi(2)))?;
    let right_energy = compensated_sum(right.iter().map(|value| (value - right_mean).powi(2)))?;
    let correlation = numerator / (left_energy.sqrt() * right_energy.sqrt());
        return Err(BrainError::Numerical("weight_tomography_autocorrelation_nonfinite".into()));
```

### 67. Weight tomography: `scaled_cosine`
- **Ancla:** `src/analysis/weight_tomography.rs::scaled_cosine`
- **Forma / líneas:** let left_energy = compensated_sum(left.iter().map(|value| (value / left_scale).powi(2)))?;; let right_energy = compensated_sum(right.iter().map(|value| (value / right_scale).powi(2)))?;; let value = numerator / (left_energy.sqrt() * right_energy.sqrt());
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("weight_tomography_direction_shape_invalid".into()));
    let value = numerator / (left_energy.sqrt() * right_energy.sqrt());
        return Err(BrainError::Numerical("weight_tomography_direction_nonfinite".into()));
```

### 68. Weight tomography: `subaperture_metrics`
- **Ancla:** `src/analysis/weight_tomography.rs::subaperture_metrics`
- **Forma / líneas:** let weight = (master_power * slave_power).sqrt();; phase_x += weight * phase.cos();; phase_y += weight * phase.sin();
- **Fragmento:**
```rust
            return Err(BrainError::Invalid("weight_tomography_channel_length_mismatch".into()));
        let master = normalized_window(&channel[..window])?;
        let slave = normalized_window(&channel[length - window..])?;
            let weight = (master_power * slave_power).sqrt();
            cross_re += cross_r;
            cross_im += cross_i;
            master_energy += master_power;
            slave_energy += slave_power;
                phase_x += weight * phase.cos();
                phase_y += weight * phase.sin();
                phase_weight_sum += weight;
            return Err(BrainError::Numerical("weight_tomography_subaperture_nonfinite".into()));
        clamp01(cross_magnitude / (master_energy.sqrt() * slave_energy.sqrt()))
```

### 69. Skill field tomography: `reconstruct`
- **Ancla:** `src/analysis/tomography.rs::reconstruct`
- **Forma / líneas:** ver código
- **Fragmento:**
```rust
            let a = dot(d.row(r), h)?;
                rec.data[r * d.cols + p] += a * h[p];
            err += e * e;
    Ok((coeff, rec, (err / (d.rows * d.cols).max(1) as f64).sqrt()))
```

### 70. Skill field tomography: `reconstruct_skill_fields`
- **Ancla:** `src/analysis/tomography.rs::reconstruct_skill_fields`
- **Forma / líneas:** let denom = lambda.sqrt();; let z = (residual_norms[r] - med).abs() / scale;; weights[r] = base_weights[r] * huber;
- **Fragmento:**
```rust
pub fn reconstruct_skill_fields(
        return Err(BrainError::Invalid("tomography_input_shape".into()));
            return Err(BrainError::Numerical("tomography_zero_spectrum".into()));
        let rank =
            choose_energy_rank(&eigenvalues, cfg.target_explained_variance, cfg.max_rank, 1)?;
        for (lambda, u) in eigs.iter().take(rank) {
            let denom = lambda.sqrt();
                return Err(BrainError::Numerical("tomography_zero_singular_value".into()));
                .map(|r| weights[r].sqrt() * u[r] / denom)
                    h[p] += mix[r] * d.get(r, p);
            let h_norm = norm(&h)?;
            if h_norm == 0.0 {
                return Err(BrainError::Numerical(
                *coefficient /= h_norm;
            dirs.push(normalize(&h)?);
        let residual_norms = (0..d.rows)
```

### 71. Persistent skill fields: `centroid_for_members`
- **Ancla:** `src/analysis/persistent.rs::centroid_for_members`
- **Forma / líneas:** pre[p] += weight * sign * rows[index][p];
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("persistent_empty_cluster".into()));
        total_weight += weight;
            pre[p] += weight * sign * rows[index][p];
        return Err(BrainError::Numerical("persistent_cluster_weight_zero".into()));
    let pre_norm = norm(&pre)?;
    if !pre_norm.is_finite() || pre_norm <= 1e-15 {
        return Err(BrainError::Numerical("persistent_cluster_centroid_degenerate".into()));
    Ok(pre.iter().map(|value| value / pre_norm).collect())
```

### 72. Persistent skill fields: `cross_aperture_functional_cv`
- **Ancla:** `src/analysis/persistent.rs::cross_aperture_functional_cv`
- **Forma / líneas:** sse += error * error;
- **Fragmento:**
```rust
            return Err(BrainError::Invalid("persistent_cv_fold_cardinality_invalid".into()));
                BrainError::Numerical("persistent_cv_no_cluster_assignment".into())
        return Err(BrainError::Integrity("persistent_cv_incomplete_prediction_coverage".into()));
    let means = (0..output_dim)
```

### 73. Persistent skill fields: `reconstruct_persistent_skill_fields`
- **Ancla:** `src/analysis/persistent.rs::reconstruct_persistent_skill_fields`
- **Forma / líneas:** original_mixture[member] = sign * reliability[member] / total_materialization_weight;; let residual = 1.0 - cosine(&rows[row], &direction)?.abs().clamp(0.0, 1.0);
- **Fragmento:**
```rust
pub fn reconstruct_persistent_skill_fields(
        return Err(BrainError::Invalid("persistent_tomography_input_shape".into()));
    let rows = normalized_rows(d)?;
        Ok::<f64, BrainError>(total + reliability[row] * dot(d.row(row), d.row(row))?)
        return Err(BrainError::Numerical("persistent_total_energy_degenerate".into()));
        // `direction` is deliberately unit-normalized for geometry, but the
        // materialized SkillField must retain a real update amplitude.  A
        // dense LoRA by roughly its source norm.  Materialization therefore
        // uses the reliability-weighted aligned mean of the original deltas;
```

### 74. Persistent skill fields: `structural_threshold_selection_avoids_largest_gap_fragmentation`
- **Ancla:** `src/analysis/persistent.rs::structural_threshold_selection_avoids_largest_gap_fragmentation`
- **Forma / líneas:** let g = 0.3_f64.sqrt();; let skill = 0.2_f64.sqrt();; let pair = 0.4_f64.sqrt();; let unique_pair = 0.1_f64.sqrt();; let unique_single = 0.5_f64.sqrt();
- **Fragmento:**
```rust
        let g = 0.3_f64.sqrt();
        let skill = 0.2_f64.sqrt();
        let pair = 0.4_f64.sqrt();
        let unique_pair = 0.1_f64.sqrt();
        let unique_single = 0.5_f64.sqrt();
            // Skill A: first two runs are extremely close (.9), third is only .5.
            vec![g, skill, 0.0, pair, 0.0, unique_pair, 0.0, 0.0, 0.0],
            vec![g, skill, 0.0, pair, 0.0, 0.0, unique_pair, 0.0, 0.0],
            vec![g, skill, 0.0, 0.0, 0.0, 0.0, 0.0, unique_single, 0.0],
            // Skill B has the same internal geometry in an orthogonal skill/pair axis.
            vec![g, 0.0, skill, 0.0, pair, unique_pair, 0.0, 0.0, 0.0],
            vec![g, 0.0, skill, 0.0, pair, 0.0, unique_pair, 0.0, 0.0],
            vec![g, 0.0, skill, 0.0, 0.0, 0.0, 0.0, 0.0, unique_single],
```

### 75. Persistent skill fields: `persistent_inverse_solves_nonorthogonal_field_coefficients_jointly`
- **Ancla:** `src/analysis/persistent.rs::persistent_inverse_solves_nonorthogonal_field_coefficients_jointly`
- **Forma / líneas:** let second = vec![0.5, 0.75_f64.sqrt()];
- **Fragmento:**
```rust
        let second = vec![0.5, 0.75_f64.sqrt()];
            index += 1;
            index += 1;
        let result = reconstruct_persistent_skill_fields(&matrix, &observations, 1, 6, 3).unwrap();
        assert_eq!(result.selected_rank, 2);
```

### 76. Block tomography: `reconstruct_block`
- **Ancla:** `src/analysis/block_tomography.rs::reconstruct_block`
- **Forma / líneas:** let retained_energy = eigenvalues.iter().take(rank).sum::<f64>() / total_energy;; let denom = lambda.sqrt().max(1e-15);; let local_coefficient = weights[local_row].sqrt() * eigenvector[local_row] / denom;
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("structured_geometry_empty_skill_support".into()));
        Ok::<f64, BrainError>(total + weights[row] * dot(data.row(row), data.row(row))?)
            selected_rank: 0,
            effective_rank: 0.0,
            normalized_block_energy: 0.0,
    let rank = choose_energy_rank(
        cfg.target_explained_variance,
        cfg.max_rank.min(support.len()),
    let retained_energy = eigenvalues.iter().take(rank).sum::<f64>() / total_energy;
    let mut basis = Vec::<Vec<f64>>::with_capacity(rank);
    let mut axes = Vec::with_capacity(rank);
    for (lambda, eigenvector) in eigs.iter().take(rank) {
        let denom = lambda.sqrt().max(1e-15);
            let local_coefficient = weights[local_row].sqrt() * eigenvector[local_row] / denom;
```

### 77. Block tomography: `reconstruct_structured_geometry`
- **Ancla:** `src/analysis/block_tomography.rs::reconstruct_structured_geometry`
- **Forma / líneas:** block.normalized_block_energy = block.block_energy / total_block_energy;
- **Fragmento:**
```rust
    fields: &[SkillField],
    let mut skills = Vec::with_capacity(fields.len());
    for (skill_index, field) in fields.iter().enumerate() {
            .get(skill_index)
            .ok_or_else(|| BrainError::Integrity("structured_geometry_mixture_missing".into()))?;
            return Err(BrainError::Invalid(format!(
                "structured_geometry_skill_support_empty:{skill_index}"
                block.normalized_block_energy = block.block_energy / total_block_energy;
        let max_local_rank = blocks
            .map(|block| block.selected_rank)
            .filter(|block| block.selected_rank > 0)
        let mean_effective_rank = if active_blocks == 0 {
                .filter(|block| block.selected_rank > 0)
                .map(|block| block.effective_rank)
        skills.push(SkillSubspaceGeometry {
            skill_id: field.skill_id.clone(),
            max_local_rank,
            mean_effective_rank,
        skills,
```

### 78. Block tomography: `layout_complexity_limits_fail_closed_before_geometry_work`
- **Ancla:** `src/analysis/block_tomography.rs::layout_complexity_limits_fail_closed_before_geometry_work`
- **Forma / líneas:** excessive_rank.blocks[0].shape = vec![1; MAX_PARAMETER_BLOCK_RANK + 1];
- **Fragmento:**
```rust
        let mut excessive_rank = minimal_layout();
        excessive_rank.blocks[0].shape = vec![1; MAX_PARAMETER_BLOCK_RANK + 1];
        excessive_rank.blocks[0].count = 1;
        excessive_rank.blocks[1].offset = 1;
        excessive_rank.total_parameter_count = 3;
        assert!(excessive_rank.validate().is_err());
```

### 79. Active aperture: `choose_active_aperture`
- **Ancla:** `src/analysis/active.rs::choose_active_aperture`
- **Forma / líneas:** let snr = dot(&candidate.sensing_vector, &q)?.max(0.0) / candidate.noise_variance;; let information_gain = 0.5 * (1.0 + snr).ln();
- **Fragmento:**
```rust
) -> BrainResult<ApertureScore> {
    validate_covariance(posterior_cov)?;
        return Err(BrainError::Invalid("active_objective_weights_invalid".into()));
    let mut best: Option<ApertureScore> = None;
            return Err(BrainError::Invalid("active_duplicate_aperture_id".into()));
        let snr = dot(&candidate.sensing_vector, &q)?.max(0.0) / candidate.noise_variance;
        let information_gain = 0.5 * (1.0 + snr).ln();
            information_gain - cost_weight * candidate.cost - risk_weight * candidate.risk;
        let score = ApertureScore {
            information_gain,
            score.objective > current.objective
                || (score.objective == current.objective && score.aperture_id < current.aperture_id)
            best = Some(score);
    best.ok_or_else(|| BrainError::Invalid("active_no_candidates".into()))
```

### 80. Active aperture: `update_posterior_covariance`
- **Ancla:** `src/analysis/active.rs::update_posterior_covariance`
- **Forma / líneas:** let mean = 0.5 * (updated.get(row, col) + updated.get(col, row));
- **Fragmento:**
```rust
pub fn update_posterior_covariance(
    validate_covariance(posterior_cov)?;
    let predictive_variance = dot(&candidate.sensing_vector, &projected)?.max(0.0);
    let denominator = candidate.noise_variance + predictive_variance;
        return Err(BrainError::Numerical("active_posterior_update_degenerate".into()));
    // Restore exact symmetry against accumulated floating-point error.
            let mean = 0.5 * (updated.get(row, col) + updated.get(col, row));
            updated.set(row, col, mean);
            updated.set(col, row, mean);
    validate_covariance(&updated)?;
```

### 81. Safe subspace projection: `project_to_safe_subspace`
- **Ancla:** `src/analysis/protected.rs::project_to_safe_subspace`
- **Forma / líneas:** let tolerance = f64::EPSILON.sqrt() * scale * m.max(1) as f64;
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("protected_shape_or_values".into()));
    let mut protected_rank = 0usize;
    let mut max_weighted_residual = 0.0f64;
            if wdot(&direction.direction, &direction.direction, &cortex.parameter_importance)?
                return Err(BrainError::Numerical(
                    wdot(&active[i].direction, &active[j].direction, &cortex.parameter_importance)?;
        let tolerance = f64::EPSILON.sqrt() * scale * m.max(1) as f64;
            return Err(BrainError::Numerical("protected_gram_not_psd".into()));
```

### 82. Cross-model aligner: `validation_residual`
- **Ancla:** `src/cross_model/extraction/cross_model_aligner.rs::validation_residual`
- **Forma / líneas:** ver código
- **Fragmento:**
```rust
fn validation_residual(map: &FittedRidgeMap, pairs: &[ActivationPair]) -> Result<f64, String> {
    let mut error_sq = 0.0;
            error_sq += (prediction - target).powi(2);
            target_sq += target.powi(2);
    Ok((error_sq / target_sq).sqrt())
```

### 83. Hierarchical steering: `extract_layer`
- **Ancla:** `src/cross_model/extraction/hierarchical_steering_extractor.rs::extract_layer`
- **Forma / líneas:** let mean_direction_cosine = cosine_sum / pair_differences.len() as f64;; let rmse = (squared_error / (pair_differences.len() * dimension) as f64).sqrt();; let relative_dispersion = rmse / (norm / (dimension as f64).sqrt());
- **Fragmento:**
```rust
    ) -> Result<HierarchicalComponent, Box<dyn Error + Send + Sync>> {
        let mut mean = vec![0.0; dimension];
            for (target, value) in mean.iter_mut().zip(row) {
                *target += *value / pair_differences.len() as f64;
        let vector = Tensor::new(mean, vec![dimension], Device::Cpu, DType::F64, layer);
        let norm = vector.l2_norm();
        if norm <= f64::EPSILON {
        let mut squared_error = 0.0;
                .ok_or("activation_pair_zero_norm")?;
            cosine_sum += cosine;
                persistent += 1;
            squared_error += row
                .map(|(value, mean)| (value - mean).powi(2))
```

### 84. LoRA synthesizer: `materialize_dense`
- **Ancla:** `src/cross_model/extraction/lora_synthesizer.rs::materialize_dense`
- **Forma / líneas:** let scale = self.alpha / self.rank as f64;; value += self.b.data[row * self.rank + component]
- **Fragmento:**
```rust
        let scale = self.alpha / self.rank as f64;
                for component in 0..self.rank {
                    value += self.b.data[row * self.rank + component]
```

### 85. Counterfactual: `analyze_one`
- **Ancla:** `src/cross_model/extraction/counterfactual_analyzer.rs::analyze_one`
- **Forma / líneas:** ver código
- **Fragmento:**
```rust
    ) -> Result<CounterfactualResult, Box<dyn Error + Send + Sync>> {
        let original_score = scenario.verifier.score(&original.text)?;
        let perturbed_score = scenario.verifier.score(&perturbed.text)?;
            let difference_norm = before
                .sqrt();
            let denominator = before.l2_norm().max(f64::EPSILON);
                .ok_or("counterfactual_activation_zero_norm")?;
                relative_activation_change: difference_norm / denominator,
            original_score,
            perturbed_score,
            verified_effect: original_score - perturbed_score,
```

### 86. Gap detector: `detect_from_evaluations`
- **Ancla:** `src/cross_model/discovery/gap_detector.rs::detect_from_evaluations`
- **Forma / líneas:** weighted_difference += probe.weight * (source_row.score - target_row.score);; sum_weight_sq += probe.weight * probe.weight;; let mean_gap = weighted_difference / sum_weight;; let radius = ((1.0 / benchmark.significance_alpha).ln() * sum_weight_sq
- **Fragmento:**
```rust
        let (source, target) = if first.weighted_score > second.weighted_score {
        } else if second.weighted_score > first.weighted_score {
            weighted_difference += probe.weight * (source_row.score - target_row.score);
            sum_weight += probe.weight;
            sum_weight_sq += probe.weight * probe.weight;
        let mean_gap = weighted_difference / sum_weight;
        if mean_gap <= 0.0 {
            .sqrt();
        let conservative_gap_lcb = mean_gap - radius;
        if mean_gap < benchmark.minimum_mean_gap || conservative_gap_lcb <= 0.0 {
            source_score: source.weighted_score,
```

### 87. Gap detector: `evaluation`
- **Ancla:** `src/cross_model/discovery/gap_detector.rs::evaluation`
- **Forma / líneas:** let successes = (score * count as f64).round() as usize;
- **Fragmento:**
```rust
    fn evaluation(name: &str, score: f64, benchmark: &BehavioralBenchmark) -> ModelEvaluation {
        let successes = (score * count as f64).round() as usize;
                score: if index < successes { 1.0 } else { 0.0 },
            weighted_score: successes as f64 / count as f64,
```

### 88. Prioritizer: `score_gap`
- **Ancla:** `src/cross_model/discovery/prioritizer.rs::score_gap`
- **Forma / líneas:** let receiver_deficit = (1.0 - gap.target_score).clamp(0.0, 1.0);; let overall_score = self.config.effect_weight * measured_effect; } else if overall_score >= (self.config.minimum_score + self.config.high_threshold) / 2.0 {
- **Fragmento:**
```rust
    fn score_gap(&self, gap: &CapabilityGap) -> Result<PriorityScore, String> {
        let measured_effect = gap.mean_gap.clamp(0.0, 1.0);
        let receiver_deficit = (1.0 - gap.target_score).clamp(0.0, 1.0);
        let overall_score = self.config.effect_weight * measured_effect
        let priority = if overall_score >= self.config.critical_threshold {
        } else if overall_score >= self.config.high_threshold {
        } else if overall_score >= (self.config.minimum_score + self.config.high_threshold) / 2.0 {
        let mut result = PriorityScore {
            overall_score,
```

### 89. Jacobi SVD / portfolio: `rank`
- **Ancla:** `src/learning/solver_portfolio.rs::rank`
- **Forma / líneas:** Self::LowRank { rank, .. } => Some(*rank),
- **Fragmento:**
```rust
    pub fn rank(&self) -> Option<usize> {
            Self::LowRank { rank, .. } => Some(*rank),
```

### 90. Jacobi SVD / portfolio: `materialize_dense`
- **Ancla:** `src/learning/solver_portfolio.rs::materialize_dense`
- **Forma / líneas:** || left.len() != rows.checked_mul(*rank).unwrap_or(usize::MAX); || right.len() != rank.checked_mul(*columns).unwrap_or(usize::MAX); let value = compensated_sum((0..*rank).map(|component| {
- **Fragmento:**
```rust
            .ok_or_else(|| BrainError::Invalid("solver_candidate_shape_overflow".into()))?;
            return Err(BrainError::Invalid("solver_candidate_dense_materialization_limit".into()));
                    .ok_or_else(|| BrainError::Invalid("solver_candidate_shape_overflow".into()))?;
                    return Err(BrainError::Integrity("solver_dense_candidate_invalid".into()));
            Self::LowRank {
                rank,
                if *rank == 0
                    || left.len() != rows.checked_mul(*rank).unwrap_or(usize::MAX)
                    || right.len() != rank.checked_mul(*columns).unwrap_or(usize::MAX)
                    return Err(BrainError::Integrity("solver_low_rank_candidate_invalid".into()));
                        let value = compensated_sum((0..*rank).map(|component| {
                            left[row * *rank + component] * right[component * *columns + column]
                            return Err(BrainError::Numerical(
                        return Err(BrainError::Integrity(
```

### 91. Jacobi SVD / portfolio: `problem_limit`
- **Ancla:** `src/learning/solver_portfolio.rs::problem_limit`
- **Forma / líneas:** let safe_gram_magnitude = (f64::MAX / energy_terms).sqrt();
- **Fragmento:**
```rust
        .saturating_mul(problem.input_dimension())
    let safe_gram_magnitude = (f64::MAX / energy_terms).sqrt();
    let working = problem
    match working {
        Some(value) if value <= limits.max_working_elements => None,
        _ => Some(EvaluationReason::ResourceLimit("solver_max_working_elements")),
```

### 92. Jacobi SVD / portfolio: `compensated_sum`
- **Ancla:** `src/learning/solver_portfolio.rs::compensated_sum`
- **Forma / líneas:** let updated = sum + value;; correction += (sum - updated) + value;; correction += (value - updated) + sum;; let result = sum + correction;
- **Fragmento:**
```rust
            return Err(BrainError::Numerical("solver_compensated_sum_input_nonfinite".into()));
            correction += (sum - updated) + value;
            correction += (value - updated) + sum;
        return Err(BrainError::Numerical("solver_compensated_sum_nonfinite".into()));
```

### 93. Jacobi SVD / portfolio: `scaled_compensated_dot`
- **Ancla:** `src/learning/solver_portfolio.rs::scaled_compensated_dot`
- **Forma / líneas:** let value = (normalized * left_scale) * right_scale;
- **Fragmento:**
```rust
fn scaled_compensated_dot(left: &[f64], right: &[f64]) -> BrainResult<f64> {
        return Err(BrainError::Invalid("solver_compensated_dot_shape_or_value".into()));
    let normalized = compensated_sum(
    let value = (normalized * left_scale) * right_scale;
        return Err(BrainError::Numerical("solver_compensated_dot_nonfinite".into()));
```

### 94. Jacobi SVD / portfolio: `direct_one_sided_jacobi_svd`
- **Ancla:** `src/learning/solver_portfolio.rs::direct_one_sided_jacobi_svd`
- **Forma / líneas:** let correlation = gamma.abs() / (alpha.sqrt() * beta.sqrt());
- **Fragmento:**
```rust
    // resulting component is normalized back to the same (input, case) pair.
            reconstruction_relative_error: 0.0,
            vector_orthogonality_error: 0.0,
```

### 95. Jacobi SVD / portfolio: `rotate_columns`
- **Ancla:** `src/learning/solver_portfolio.rs::rotate_columns`
- **Forma / líneas:** *first_value = cosine * old_first - sine * old_second;; *second_value = sine * old_first + cosine * old_second;

### 96. Jacobi SVD / portfolio: `maximum_column_correlation`
- **Ancla:** `src/learning/solver_portfolio.rs::maximum_column_correlation`
- **Forma / líneas:** maximum = maximum.max(gamma.abs() / (alpha.sqrt() * beta.sqrt()));
- **Fragmento:**
```rust
            let alpha = scaled_compensated_dot(&columns[first], &columns[first])?;
            let beta = scaled_compensated_dot(&columns[second], &columns[second])?;
            let gamma = scaled_compensated_dot(&columns[first], &columns[second])?;
            maximum = maximum.max(gamma.abs() / (alpha.sqrt() * beta.sqrt()));
        return Err(BrainError::Numerical("solver_direct_svd_correlation_nonfinite".into()));
```

### 97. Jacobi SVD / portfolio: `diagnose_problem`
- **Ancla:** `src/learning/solver_portfolio.rs::diagnose_problem`
- **Forma / líneas:** let singular_value_cutoff = largest * policy.relative_rank_tolerance.max(automatic_tolerance);
- **Fragmento:**
```rust
        return Err(BrainError::Numerical("solver_gram_spectrum_nonfinite".into()));
    let singular_value_cutoff = largest * policy.relative_rank_tolerance.max(automatic_tolerance);
    let numerical_rank = singular_values
    // Entropy and explained-energy ranks are scale-free. Feeding the raw
    // rank zero. Normalize before computing these diagnostics while retaining
    let normalized_spectrum = if largest == 0.0 {
    let effective_rank = effective_rank_from_spectrum(&normalized_spectrum)?;
    let energy_rank = if numerical_rank == 0 {
        choose_energy_rank(
            &normalized_spectrum,
            numerical_rank,
            policy.minimum_rank.min(numerical_rank),
    let (condition_kind, finite_singular_condition_number, finite_gram_condition_number) =
        if numerical_rank == 0 {
        } else if numerical_rank < gram.row_count() {
            (GramCondition::RankDeficient, None, None)
            let smallest = singular_values[numerical_rank - 1];
                // scale-normalized, so an infinite auxiliary estimate must
```

### 98. Jacobi SVD / portfolio: `direct_svd_factors`
- **Ancla:** `src/learning/solver_portfolio.rs::direct_svd_factors`
- **Forma / líneas:** let mut left = vec![0.0; problem.output_dimension() * rank];; let mut right = vec![0.0; rank * problem.input_dimension()];
- **Fragmento:**
```rust
    rank: usize,
    if rank == 0
        || rank > direct_svd.components.len()
        || direct_svd.components.iter().take(rank).any(|component| {
        return Err(BrainError::Numerical("solver_direct_svd_rank_invalid".into()));
    let mut left = vec![0.0; problem.output_dimension() * rank];
    let mut right = vec![0.0; rank * problem.input_dimension()];
    for (component_index, component) in direct_svd.components.iter().take(rank).enumerate() {
            return Err(BrainError::Numerical("solver_direct_svd_inverse_nonfinite".into()));
            left[output * rank + component_index] = compensated_sum(
        return Err(BrainError::Numerical("solver_direct_svd_factor_nonfinite".into()));
```

### 99. Jacobi SVD / portfolio: `evaluate_candidate`
- **Ancla:** `src/learning/solver_portfolio.rs::evaluate_candidate`
- **Forma / líneas:** let relative_allowance = policy.relative_residual_tolerance * metrics.target_norm;; let acceptance_threshold = policy.absolute_residual_tolerance + relative_allowance;
- **Fragmento:**
```rust
    constructed_rank: Option<usize>,
                constructed_rank,
            constructed_rank,
            let relative_allowance = policy.relative_residual_tolerance * metrics.target_norm;
            let acceptance_threshold = policy.absolute_residual_tolerance + relative_allowance;
                    reason: EvaluationReason::ResidualToleranceUnrepresentable,
                    constructed_rank,
            let accepted = metrics.absolute_residual <= acceptance_threshold;
                    EvaluationReason::ResidualWithinTolerance
                    EvaluationReason::ResidualExceedsTolerance
                constructed_rank,
        Err(error) => CandidateEvaluation {
            reason: EvaluationReason::InvalidCandidate(error.to_string()),
            constructed_rank,
```

### 100. Jacobi SVD / portfolio: `measure_candidate`
- **Ancla:** `src/learning/solver_portfolio.rs::measure_candidate`
- **Forma / líneas:** let root_mean_square_residual = absolute_residual / (residual_count as f64).sqrt();
- **Fragmento:**
```rust
        return Err(BrainError::Integrity("solver_candidate_problem_shape_mismatch".into()));
    let mut residuals = Vec::with_capacity(
            .ok_or_else(|| BrainError::Invalid("solver_residual_shape_overflow".into()))?,
            let predicted = scaled_compensated_dot(
            residuals.push(predicted - problem.targets.get(case, output));
    let absolute_residual = norm(&residuals)?;
    let residual_count = residuals.len();
    if residual_count == 0 {
        return Err(BrainError::Integrity("solver_candidate_residual_empty".into()));
    let root_mean_square_residual = absolute_residual / (residual_count as f64).sqrt();
    let maximum_absolute_residual = residuals
    let target_norm = norm(problem.targets.as_slice())?;
    let frobenius_norm = norm(&dense)?;
    let relative_residual = if target_norm > 0.0 {
        Some(absolute_residual / target_norm)
    } else if absolute_residual == 0.0 {
    if !absolute_residual.is_finite()
        || !root_mean_square_residual.is_finite()
        || !maximum_absolute_residual.is_finite()
        || !target_norm.is_finite()
```

### 101. Causal credit: `lower_confidence_bound`
- **Ancla:** `src/learning/causal_credit.rs::lower_confidence_bound`
- **Forma / líneas:** let bound = self.mean_interaction_effect - EFFECT_95_Z * self.standard_error;
- **Fragmento:**
```rust
            || !self.mean_interaction_effect.is_finite()
            || !self.standard_error.is_finite()
            || self.standard_error < 0.0
            return Err(BrainError::Integrity("causal_pair_interaction_unresolved".into()));
        let bound = self.mean_interaction_effect - EFFECT_95_Z * self.standard_error;
            return Err(BrainError::Numerical("causal_pair_interaction_bound_non_finite".into()));
```

### 102. Causal credit: `certified_causal_priority_weights`
- **Ancla:** `src/learning/causal_credit.rs::certified_causal_priority_weights`
- **Forma / líneas:** || report.pair_interactions.len() != expected.len() * expected.len().saturating_sub(1) / 2
- **Fragmento:**
```rust
    field_ids: &[SkillId],
        return Err(BrainError::Integrity("causal_credit_priority_report_unresolved".into()));
        return Err(BrainError::Invalid("causal_credit_priority_field_identity_invalid".into()));
        if field.skill_id.trim().is_empty()
            || !field.mean_marginal_effect.is_finite()
            || !field.standard_error.is_finite()
            || field.standard_error < 0.0
                .insert(field.skill_id.clone(), field.lower_confidence_bound)
            return Err(BrainError::Integrity("causal_credit_priority_field_unresolved".into()));
        || report.pair_interactions.len() != expected.len() * expected.len().saturating_sub(1) / 2
        return Err(BrainError::Integrity("causal_credit_priority_identity_mismatch".into()));
        let (left, right) = if pair.left_skill_id < pair.right_skill_id {
            (&pair.left_skill_id, &pair.right_skill_id)
            (&pair.right_skill_id, &pair.left_skill_id)
            || !pair.mean_interaction_effect.is_finite()
            || !pair.standard_error.is_finite()
            || pair.standard_error < 0.0
            return Err(BrainError::Integrity(
```

### 103. Causal credit: `mean_and_se`
- **Ancla:** `src/learning/causal_credit.rs::mean_and_se`
- **Forma / líneas:** let mean = values.iter().sum::<f64>() / values.len() as f64;
- **Fragmento:**
```rust
fn mean_and_se(values: &[f64]) -> (f64, f64) {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
        return (mean, f64::INFINITY);
    let variance = values
        .map(|value| (value - mean).powi(2))
    (mean, (variance / values.len() as f64).sqrt())
```

### 104. Causal credit: `compute_shapley_values`
- **Ancla:** `src/learning/causal_credit.rs::compute_shapley_values`
- **Forma / líneas:** let avg_diff = diffs.iter().sum::<f64>() / diffs.len() as f64;; let weight = (factorial(subset_len) * factorial(n - 1 - subset_len)) / factorial(n);; total_shapley += weight * avg_diff;
- **Fragmento:**
```rust
) -> BrainResult<BTreeMap<SkillId, f64>> {
    let mut all_skills = BTreeSet::new();
    let mut table = BTreeMap::<(String, BTreeSet<SkillId>), f64>::new();
        all_skills.extend(set.iter().cloned());
    let skills: Vec<SkillId> = all_skills.into_iter().collect();
    let n = skills.len();
    for (i_idx, field) in skills.iter().enumerate() {
        let other_skills: Vec<SkillId> = skills
        let m = other_skills.len();
            for (bit, s) in other_skills.iter().enumerate().take(20) {
                total_shapley += weight * avg_diff;
                total_weight += weight;
```

### 105. Applicability similarity: `validate_for`
- **Ancla:** `src/learning/procedural_memory.rs::validate_for`
- **Forma / líneas:** SolverParameters::LowRank { rank } => u64::from(*rank) <= minimum_dimension,
- **Fragmento:**
```rust
            SolverParameters::LowRank { rank } => u64::from(*rank) <= minimum_dimension,
            SolverParameters::RandomizedLowRank { rank, oversampling } => rank
                .is_some_and(|sketch_rank| u64::from(sketch_rank) <= minimum_dimension),
            | SolverParameters::FullRank
```

### 106. Applicability similarity: `rank`
- **Ancla:** `src/learning/procedural_memory.rs::rank`
- **Forma / líneas:** | SolverParameters::RandomizedLowRank { rank, .. } => Some(*rank),
- **Fragmento:**
```rust
    pub fn rank(&self) -> Option<u32> {
            SolverParameters::LowRank { rank }
            | SolverParameters::RandomizedLowRank { rank, .. } => Some(*rank),
```

### 107. Applicability similarity: `positive_utility`
- **Ancla:** `src/learning/procedural_memory.rs::positive_utility`
- **Forma / líneas:** (Some(true), Some(residual)) => Some(1.0 / (1.0 + residual.get())),
- **Fragmento:**
```rust
        match (self.independent_gate_advanced, self.solver_relative_residual) {
            (Some(true), Some(residual)) => Some(1.0 / (1.0 + residual.get())),
```

### 108. Applicability similarity: `applicability_similarity`
- **Ancla:** `src/learning/procedural_memory.rs::applicability_similarity`
- **Forma / líneas:** let mut weighted_score = policy.projection.row_weight.get() * row_score; weighted_score += policy.projection.rank_weight.get() * rank_score;
- **Fragmento:**
```rust
    let row_score = logarithmic_ratio(left.dimensions.rows, right.dimensions.rows);
    let column_score = logarithmic_ratio(left.dimensions.columns, right.dimensions.columns);
    let rank_score = match (
        left.dimensions.estimated_effective_rank,
        right.dimensions.estimated_effective_rank,
        (Some(left_rank), Some(right_rank)) => Some(ratio_similarity(
            left_rank,
            right_rank,
    let precision_score = if left.profile.precision == right.profile.precision {
    let structure_score = if left.profile.structure == right.profile.structure {
    let mut weighted_score = policy.projection.row_weight.get() * row_score
        + policy.projection.column_weight.get() * column_score
        + policy.projection.precision_weight.get() * precision_score
        + policy.projection.structure_weight.get() * structure_score;
    if let Some(rank_score) = rank_score {
        weighted_score += policy.projection.rank_weight.get() * rank_score;
        + policy.projection.rank_weight.get()
        weighted_score += policy.projection.condition_weight.get()
        &mut weighted_score,
        &mut weighted_score,
```

### 109. Applicability similarity: `add_optional_unit_similarity`
- **Ancla:** `src/learning/procedural_memory.rs::add_optional_unit_similarity`
- **Forma / líneas:** *weighted_score += weight * (1.0 - (left.get() - right.get()).abs());
- **Fragmento:**
```rust
    weighted_score: &mut f64,
        *weighted_score += weight * (1.0 - (left.get() - right.get()).abs());
```

### 110. PETFC / portfolio gov: `stable_mean`
- **Ancla:** `src/learning/portfolio_governance.rs::stable_mean`
- **Forma / líneas:** let updated = sum + value;; correction += (sum - updated) + value;; correction += (value - updated) + sum;; let mean = ((sum + correction) / count as f64) * scale;
- **Fragmento:**
```rust
fn stable_mean(values: impl IntoIterator<Item = f64>) -> BrainResult<f64> {
        return Err(invalid("governance_mean_input_invalid"));
        u64::try_from(values.len()).map_err(|_| invalid("governance_mean_count_overflow"))?;
            correction += (sum - updated) + value;
            correction += (value - updated) + sum;
    // merely while computing their equally large finite mean.
    let mean = ((sum + correction) / count as f64) * scale;
    if !mean.is_finite() {
        return Err(BrainError::Numerical("governance_mean_nonfinite".into()));
    Ok(mean)
```

### 111. PETFC / portfolio gov: `stable_nonnegative_sum`
- **Ancla:** `src/learning/portfolio_governance.rs::stable_nonnegative_sum`
- **Forma / líneas:** let updated = sum + value;; let result = sum + correction;
- **Fragmento:**
```rust
            return Err(BrainError::Numerical("petfc_sum_input_invalid".into()));
        correction += if sum.abs() >= value.abs() {
        return Err(BrainError::Numerical("petfc_sum_nonfinite".into()));
```

### 112. PETFC / portfolio gov: `adaptive_score`
- **Ancla:** `src/learning/portfolio_governance.rs::adaptive_score`
- **Forma / líneas:** let exploration = policy.exploration_strength.get() * (numerator.ln() / denominator).sqrt();; let score = empirical + exploration - policy.cost_penalty.get() * cost.ln_1p();
- **Fragmento:**
```rust
fn adaptive_score(
        stable_mean(history.iter().copied())?
    let exploration = policy.exploration_strength.get() * (numerator.ln() / denominator).sqrt();
    let score = empirical + exploration - policy.cost_penalty.get() * cost.ln_1p();
    if !score.is_finite() {
        return Err(BrainError::Numerical("adaptive_budget_score_nonfinite".into()));
    Ok(score)
```

### 113. PETFC / portfolio gov: `petfc_reproduces_pythagorean_staircase_and_enforces_time_chain`
- **Ancla:** `src/learning/portfolio_governance.rs::petfc_reproduces_pythagorean_staircase_and_enforces_time_chain`
- **Forma / líneas:** let expected = 2.0_f64.sqrt();
- **Fragmento:**
```rust
        let expected = 2.0_f64.sqrt();
        assert!((assessment.waste_upper().unwrap() - (1.0 - 1.0 / expected)).abs() < 1.0e-12);
```

### 114. PETFC / portfolio gov: `petfc_geometry_uses_incumbent_anchored_effects_and_rejects_duplicate_checkpoint`
- **Ancla:** `src/learning/portfolio_governance.rs::petfc_geometry_uses_incumbent_anchored_effects_and_rejects_duplicate_checkpoint`
- **Forma / líneas:** let expected = 2.0_f64.sqrt();
- **Fragmento:**
```rust
    fn petfc_geometry_uses_incumbent_anchored_effects_and_rejects_duplicate_checkpoint() {
        let expected = 2.0_f64.sqrt();
```

### 115. Risk weight: `generate_candidates`
- **Ancla:** `src/learning/learning_orchestrator.rs::generate_candidates`
- **Forma / líneas:** let desired = (2 + (next_random(&mut state) as usize % dimension.saturating_sub(1).max(1)))
- **Fragmento:**
```rust
        let desired = (2 + (next_random(&mut state) as usize % dimension.saturating_sub(1).max(1)))
                >= (1usize.checked_shl(dimension.min(20) as u32).unwrap_or(0)).saturating_sub(1)
```

### 116. Risk weight: `rank`
- **Ancla:** `src/learning/learning_orchestrator.rs::rank`
- **Forma / líneas:** let tolerance = max * f64::EPSILON.sqrt() * (gram.rows.max(1) as f64);
- **Fragmento:**
```rust
fn rank(rows: &[Vec<f64>]) -> BrainResult<usize> {
            .saturating_mul(gram.rows)
            .saturating_mul(200)
    let tolerance = max * f64::EPSILON.sqrt() * (gram.rows.max(1) as f64);
```

### 117. Risk weight: `same_f64`
- **Ancla:** `src/learning/learning_orchestrator.rs::same_f64`
- **Forma / líneas:** && (left - right).abs() <= f64::EPSILON.sqrt() * 64.0 * (1.0 + left.abs().max(right.abs()))
- **Fragmento:**
```rust
        && (left - right).abs() <= f64::EPSILON.sqrt() * 64.0 * (1.0 + left.abs().max(right.abs()))
```

### 118. Risk weight: `next_learning_aperture`
- **Ancla:** `src/learning/learning_orchestrator.rs::next_learning_aperture`
- **Forma / líneas:** let outcome_utility = direction * session.policy.outcome_utility_weight * predicted_outcome;; let objective = score.objective + outcome_utility;
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("adaptive_learning_plan_step_budget_exhausted".into()));
        return Err(BrainError::Invalid("adaptive_learning_no_remaining_apertures".into()));
    let covariance = Matrix::from_rows(&session.posterior.covariance)?;
    // Reuse the authoritative active-aperture scorer for information, cost and
    // risk. The explicit posterior-mean term turns the live cycle into a
        let score = choose_active_aperture(
            &covariance,
        let predicted_outcome = dot(&candidate.sensing_vector, &session.posterior.mean)?;
        let objective = score.objective + outcome_utility;
            return Err(BrainError::Numerical(
                score.information_gain,
    let (selected, information_gain, predicted_outcome, outcome_utility, objective) = selected
        .ok_or_else(|| BrainError::Integrity("adaptive_learning_selected_missing".into()))?;
```

### 119. Risk weight: `validate_experiment_evidence`
- **Ancla:** `src/learning/learning_orchestrator.rs::validate_experiment_evidence`
- **Forma / líneas:** let tolerance = f64::EPSILON.sqrt()
- **Fragmento:**
```rust
        return Err(BrainError::Integrity(
        return Err(BrainError::Integrity("adaptive_learning_observation_contract_invalid".into()));
        BrainError::Integrity("adaptive_learning_observation_dense_artifact_missing".into())
        return Err(BrainError::Integrity(
        return Err(BrainError::Integrity(
            BrainError::Integrity("adaptive_learning_observation_layout_digest_missing".into())
        BrainError::Integrity("adaptive_learning_observation_layout_artifact_invalid".into())
```

### 120. Numerical evolution: `derive_observations`
- **Ancla:** `src/learning/numerical_evolution.rs::derive_observations`
- **Forma / líneas:** let target_rms = baseline_metrics.target_norm() / (scalar_count as f64).sqrt();; let baseline_fit = 1.0 / (1.0 + baseline_metrics.root_mean_square_residual() / scale);; let candidate_fit = 1.0 / (1.0 + candidate_metrics.root_mean_square_residual() / scale);; let baseline_worst = baseline_metrics.maximum_absolute_residual() / scale;; let candidate_worst = candidate_metrics.maximum_absolute_residual() / scale;
- **Fragmento:**
```rust
        baseline_id: &VariantId,
        candidate_id: &VariantId,
            let target_rms = baseline_metrics.target_norm() / (scalar_count as f64).sqrt();
            let baseline_fit = 1.0 / (1.0 + baseline_metrics.root_mean_square_residual() / scale);
            let candidate_fit = 1.0 / (1.0 + candidate_metrics.root_mean_square_residual() / scale);
            let baseline_worst = baseline_metrics.maximum_absolute_residual() / scale;
            let candidate_worst = candidate_metrics.maximum_absolute_residual() / scale;
                return Err(BrainError::Numerical("numerical_evaluation_metric_nonfinite".into()));
                worst_error_metric_id()?,
```

### 121. Sleep evidence verify: `recompute_functional_replay`
- **Ancla:** `src/learning/sleep_evidence.rs::recompute_functional_replay`
- **Forma / líneas:** let mean_utility_delta = differences.iter().sum::<f64>() / count as f64;
- **Fragmento:**
```rust
    expected_ids: &[SkillId],
        return Err(BrainError::Integrity("functional_replay_source_digest_mismatch".into()));
                row.mean,
                mean_metrics(&row.metrics, &baseline.validation_tasks)?,
            return Err(BrainError::Integrity("functional_replay_candidate_row_invalid".into()));
        return Err(BrainError::Integrity(
        let baseline_mean = mean_metrics(
                BrainError::Integrity("functional_replay_baseline_seed_missing".into())
```

### 122. Sleep evidence verify: `verify_causal_credit_artifacts`
- **Ancla:** `src/learning/sleep_evidence.rs::verify_causal_credit_artifacts`
- **Forma / líneas:** != expected_ids.len() * expected_ids.len().saturating_sub(1) / 2
- **Fragmento:**
```rust
    expected_ids: &[SkillId],
            != expected_ids.len() * expected_ids.len().saturating_sub(1) / 2
        .ok_or_else(|| BrainError::Integrity("causal_replay_field_ids_missing".into()))?
                .ok_or_else(|| BrainError::Integrity("causal_replay_field_id_invalid".into()))?;
            SkillId::parse(value)
                .map_err(|_| BrainError::Integrity("causal_replay_field_id_invalid".into()))
        .ok_or_else(|| BrainError::Integrity("causal_replay_evaluations_missing".into()))?;
```

### 123. Sleep evidence verify: `dense_fields_from_observations`
- **Ancla:** `src/learning/sleep_evidence.rs::dense_fields_from_observations`
- **Forma / líneas:** let sum = *output + coefficient * f64::from(value);
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("sleep_dense_field_source_shape_invalid".into()));
                return Err(BrainError::Invalid(format!(
                BrainError::Integrity(format!(
                return Err(BrainError::Integrity(format!(
                return Err(BrainError::Integrity("sleep_dense_field_loaded_dimension".into()));
                    return Err(BrainError::Numerical(
            support += 1;
            return Err(BrainError::Integrity(format!(
```

### 124. Sleep evidence verify: `verify_sleep_evidence`
- **Ancla:** `src/learning/sleep_evidence.rs::verify_sleep_evidence`
- **Forma / líneas:** let tolerance = f64::EPSILON.sqrt() * map.parameter_dimension.max(1) as f64 * 8.0;
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("sleep_evidence_schema_invalid".into()));
        .map(|field| field.skill_id.clone())
                let tolerance = f64::EPSILON.sqrt() * map.parameter_dimension.max(1) as f64 * 8.0;
```

### 125. Low-rank shadow: `validate`
- **Ancla:** `src/materialization/low_rank_shadow_materializer.rs::validate`
- **Forma / líneas:** if self.schema != "cerebro.tidex.low_rank_shadow_policy/v1"
- **Fragmento:**
```rust
        if self.schema != "cerebro.tidex.low_rank_shadow_policy/v1"
            || self.maximum_rank == 0
            || self.maximum_rank > MAX_SHADOW_RANK
            return Err(BrainError::Invalid("low_rank_shadow_policy_invalid".into()));
```

### 126. Low-rank shadow: `factor_dense_delta_verified`
- **Ancla:** `src/materialization/low_rank_shadow_materializer.rs::factor_dense_delta_verified`
- **Forma / líneas:** let mut left = vec![0.0; rows * rank];; let mut right = vec![0.0; rank * columns];; let scaled_sigma = eigen[component].0.sqrt();; let balanced_scale = scale.sqrt() * scaled_sigma.sqrt();
- **Fragmento:**
```rust
    policy: &LowRankShadowPolicy,
) -> BrainResult<VerifiedLowRankFactors> {
        .ok_or_else(|| BrainError::Invalid("low_rank_dense_shape_overflow".into()))?;
        .ok_or_else(|| BrainError::Invalid("low_rank_gram_shape_overflow".into()))?;
        return Err(BrainError::Invalid("low_rank_dense_input_invalid".into()));
    let target_norm = norm(dense)?;
        return Err(BrainError::Numerical("low_rank_zero_delta_has_no_factors".into()));
        .ok_or_else(|| BrainError::Invalid("low_rank_svd_work_overflow".into()))?;
        return Err(BrainError::Invalid("low_rank_svd_work_limit".into()));
        return Err(BrainError::Invalid("low_rank_gram_work_limit".into()));
```

### 127. Low-rank shadow: `rectangular_orientations_and_finite_scaling_reconstruct`
- **Ancla:** `src/materialization/low_rank_shadow_materializer.rs::rectangular_orientations_and_finite_scaling_reconstruct`
- **Forma / líneas:** ver código
- **Fragmento:**
```rust
                .map(|index| (index as f64 + 0.5) / scale.sqrt())
            assert_eq!(factors.rank, 1);
            assert!(factors.relative_reconstruction_error < 1.0e-10);
```

### 128. Activation steering mat.: `normalize`
- **Ancla:** `src/materialization/activation_steering_materializer.rs::normalize`
- **Forma / líneas:** let rms = magnitude / (vector.len() as f64).sqrt();
- **Fragmento:**
```rust
fn normalize(mut vector: Vec<f64>, normalization: SteeringNormalization) -> BrainResult<Vec<f64>> {
    let magnitude = norm(&vector)?;
    match normalization {
        SteeringNormalization::None => {}
        SteeringNormalization::UnitL2 if magnitude > 0.0 => {
        SteeringNormalization::RootMeanSquare if magnitude > 0.0 => {
            let rms = magnitude / (vector.len() as f64).sqrt();
```

### 129. Activation steering mat.: `normalization_is_exact_and_finite`
- **Ancla:** `src/materialization/activation_steering_materializer.rs::normalization_is_exact_and_finite`
- **Forma / líneas:** ver código
- **Fragmento:**
```rust
    fn normalization_is_exact_and_finite() {
        let unit = normalize(vec![3.0, 4.0], SteeringNormalization::UnitL2).unwrap();
        assert!((norm(&unit).unwrap() - 1.0).abs() < 1e-12);
        let rms = normalize(vec![3.0, 4.0], SteeringNormalization::RootMeanSquare).unwrap();
        assert!((norm(&rms).unwrap() - 2.0_f64.sqrt()).abs() < 1e-12);
```

### 130. Backend selection scores: `select_materialization_backend`
- **Ancla:** `src/materialization/materialization_selector.rs::select_materialization_backend`
- **Forma / líneas:** let utility = (policy.functional_weight * e.functional_ci_lower
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("backend_evaluation_cardinality_invalid".into()));
            return Err(BrainError::Invalid("backend_evaluation_duplicate".into()));
        return Err(BrainError::Invalid("backend_complementarity_limit".into()));
            || !pair.held_out_gain.is_finite()
            || !(0.0..=1.0).contains(&pair.held_out_gain)
            return Err(BrainError::Invalid("backend_complementarity_invalid".into()));
                && e.preservation_score >= policy.minimum_preservation_score
                && e.normalized_risk <= policy.maximum_normalized_risk
        return Err(BrainError::Integrity("no_backend_passed_comparative_gates".into()));
```

### 131. Receiver compile / portability: `project_functional_signature`
- **Ancla:** `src/receiver/receiver_compiler.rs::project_functional_signature`
- **Forma / líneas:** let centered = *raw_value - *mean_value;
- **Fragmento:**
```rust
    mean: &[f64],
        || raw.len() != mean.len()
        || raw.iter().chain(mean).any(|value| !value.is_finite())
        return Err(BrainError::Invalid("receiver_weight_functional_projection_shape".into()));
        for ((raw_value, mean_value), coefficient) in raw.iter().zip(mean).zip(component) {
            let centered = *raw_value - *mean_value;
            sum += product;
            return Err(BrainError::Invalid(
```

### 132. Receiver compile / portability: `validate_functional_anchor_identity`
- **Ancla:** `src/receiver/receiver_compiler.rs::validate_functional_anchor_identity`
- **Forma / líneas:** let relative = norm(&difference)? / scale;
- **Fragmento:**
```rust
            let scale = norm(&rows[i])?.max(norm(&rows[j])?).max(f64::MIN_POSITIVE);
            let relative = norm(&difference)? / scale;
                return Err(BrainError::Invalid(
```

### 133. Receiver compile / portability: `evaluate_portability`
- **Ancla:** `src/receiver/receiver_compiler.rs::evaluate_portability`
- **Forma / líneas:** let recovered_gain = (transferred_score - virgin_score) / direct_gain;
- **Fragmento:**
```rust
        || norm(expected_functional_signature)? <= 1e-15
        return Err(BrainError::Invalid("receiver_portability_signature_invalid".into()));
            return Err(BrainError::Invalid("receiver_portability_signature_invalid".into()));
    let score = |observed: &[f64]| -> BrainResult<f64> {
        let residual = observed
        Ok(1.0 - norm(&residual)? / norm(expected_functional_signature)?.max(1e-15))
    let virgin_score = score(virgin_functional_signature)?;
    let direct_score = score(direct_functional_signature)?;
    let transferred_score = score(transferred_functional_signature)?;
    let wrong_score = score(wrong_functional_signature)?;
    let direct_gain = direct_score - virgin_score;
    if direct_gain <= 1e-12 {
        return Err(BrainError::Invalid("receiver_portability_direct_oracle_has_no_gain".into()));
    let recovered_gain = (transferred_score - virgin_score) / direct_gain;
        virgin_score,
        direct_score,
        transferred_score,
        wrong_score,
        recovered_gain,
        correct_wrong_advantage: transferred_score - wrong_score,
```

### 134. Receiver compile / portability: `receiver_basis_adapter_reports_unweighted_raw_reconstruction_energy`
- **Ancla:** `src/receiver/receiver_compiler.rs::receiver_basis_adapter_reports_unweighted_raw_reconstruction_energy`
- **Forma / líneas:** ver código
- **Fragmento:**
```rust
        input.target_explained_variance = 0.5;
        input.max_rank = 1;
        assert_eq!(result.selected_rank, 1);
                sse += (reconstructed - value).powi(2);
                total_energy += value * value;
        assert!((result.retained_energy - (1.0 - sse / total_energy)).abs() < 1e-12);
        assert!((result.reconstruction_rms - (sse / 12.0).sqrt()).abs() < 1e-12);
```

### 135. Receiver compile / portability: `toggle_contract`
- **Ancla:** `src/receiver/receiver_compiler.rs::toggle_contract`
- **Forma / líneas:** let pre = 2.0_f64.sqrt();
- **Fragmento:**
```rust
        let pre = 2.0_f64.sqrt();
                    pre_target_error: pre,
                    post_target_error: 0.0,
                    pre_target_error: pre,
                    post_target_error: 0.0,
            maximum_closure_error: 1e-5,
```

### 136. Linear readout verify: `validate_behavioral_calibration_evidence`
- **Ancla:** `src/receiver/receiver_weight_binding.rs::validate_behavioral_calibration_evidence`
- **Forma / líneas:** let mean_base_accuracy = evidence.base_accuracies.iter().sum::<f64>() / count as f64;
- **Fragmento:**
```rust
        || !policy.minimum_mean_accuracy.is_finite()
        || !(0.0..=1.0).contains(&policy.minimum_mean_accuracy)
        || !policy.minimum_mean_gain.is_finite()
        || !(0.0..=1.0).contains(&policy.minimum_mean_gain)
    let mean_base_accuracy = evidence.base_accuracies.iter().sum::<f64>() / count as f64;
    let mean_compiled_accuracy =
    let mean_gain = mean_compiled_accuracy - mean_base_accuracy;
    if mean_compiled_accuracy < policy.minimum_mean_accuracy
        || mean_gain < policy.minimum_mean_gain
        mean_base_accuracy,
        mean_compiled_accuracy,
        mean_gain,
```

### 137. Readout roundoff bound: `readout_dot_roundoff_bound`
- **Ancla:** `src/receiver/validation.rs::readout_dot_roundoff_bound`
- **Forma / líneas:** let upper_sum = absolute_sum / (1.0 - gamma64);; let bound = (gamma32 + gamma64) * upper_sum + underflow;
- **Fragmento:**
```rust
pub fn readout_dot_roundoff_bound(weights: &[f64], input: &[f64]) -> BrainResult<f64> {
        return Err(BrainError::Invalid("readout_dot_bound_shape".into()));
    let absolute_sum = dot(&absolute_weights, &absolute_input)?;
    let gamma32 = (count32 * unit32) / (1.0 - count32 * unit32);
    let gamma64 = (count64 * unit64) / (1.0 - count64 * unit64);
    let upper_sum = absolute_sum / (1.0 - gamma64);
        return Err(BrainError::Invalid("readout_dot_bound_nonfinite".into()));
```

### 138. Learned controller: `train_learned_controller`
- **Ancla:** `src/engine/learned_controller.rs::train_learned_controller`
- **Forma / líneas:** squared_error += (target - predicted) * (target - predicted);
- **Fragmento:**
```rust
    ridge: f64,
    if !ridge.is_finite()
        || ridge <= 0.0
        return Err(BrainError::Invalid("learned_controller_training_config".into()));
    let weights = fit_weights(examples, coefficient_dim, ridge)?;
    let mut squared_error = 0.0;
            squared_error += (target - predicted) * (target - predicted);
            values += 1;
        training_rms: (squared_error / values.max(1) as f64).sqrt(),
        grouped_cv_r2: grouped_cv_r2(examples, coefficient_dim, ridge)?,
```

### 139. Learned controller: `same_supervision_coefficients`
- **Ancla:** `src/engine/learned_controller.rs::same_supervision_coefficients`
- **Forma / líneas:** (left - right).abs() <= f64::EPSILON.sqrt() * 64.0 * (1.0 + left.abs().max(right.abs()))
- **Fragmento:**
```rust
            (left - right).abs() <= f64::EPSILON.sqrt() * 64.0 * (1.0 + left.abs().max(right.abs()))
```

### 140. Learned controller: `same_controller_float`
- **Ancla:** `src/engine/learned_controller.rs::same_controller_float`
- **Forma / líneas:** && (left - right).abs() <= f64::EPSILON.sqrt() * 64.0 * (1.0 + left.abs().max(right.abs()))
- **Fragmento:**
```rust
        && (left - right).abs() <= f64::EPSILON.sqrt() * 64.0 * (1.0 + left.abs().max(right.abs()))
```

### 141. Cognitive field: `curvature_similarity`
- **Ancla:** `src/engine/cognitive_field.rs::curvature_similarity`
- **Forma / líneas:** let denom = (curvature.get(left, left).max(0.0) * curvature.get(right, right).max(0.0)).sqrt();
- **Fragmento:**
```rust
    let denom = (curvature.get(left, left).max(0.0) * curvature.get(right, right).max(0.0)).sqrt();
```

### 142. Cognitive field: `build`
- **Ancla:** `src/engine/cognitive_field.rs::build`
- **Forma / líneas:** let expected_pairs = n * n.saturating_sub(1) / 2;
- **Fragmento:**
```rust
        fields: &[SkillField],
            return Err(BrainError::Invalid("cognitive_field_input_shape".into()));
            return Err(BrainError::Invalid(
                return Err(BrainError::Integrity(
            if field.skill_id.trim().is_empty() || !unique.insert(field.skill_id.clone()) {
                return Err(BrainError::Invalid("cognitive_field_field_identity".into()));
            field_ids.push(field.skill_id.clone());
            .map(|field| field.skill_id.clone())
            return Err(BrainError::Integrity("cognitive_field_causal_identity_unresolved".into()));
            robust_effect_scale(causal.fields.iter().map(|row| row.mean_marginal_effect));
        let mut pair_effects = BTreeMap::<(SkillId, SkillId), f64>::new();
                || !pair.mean_interaction_effect.is_finite()
                || !unique.contains(&pair.left_skill_id)
                || !unique.contains(&pair.right_skill_id)
                || pair.left_skill_id == pair.right_skill_id
                return Err(BrainError::Integrity(
            let key = if pair.left_skill_id < pair.right_skill_id {
                (pair.left_skill_id.clone(), pair.right_skill_id.clone())
                (pair.right_skill_id.clone(), pair.left_skill_id.clone())
                return Err(BrainError::Integrity(
```

### 143. Cognitive field: `evolve`
- **Ancla:** `src/engine/cognitive_field.rs::evolve`
- **Forma / líneas:** let derivative = self.config.evidence_gain * drive.evidence[index]
- **Fragmento:**
```rust
            || drive.prediction_error.len() != n
                .chain(&drive.prediction_error)
            return Err(BrainError::Invalid("cognitive_field_drive_shape_or_values".into()));
                let derivative = self.config.evidence_gain * drive.evidence[index]
                    + self.config.prediction_error_gain * drive.prediction_error[index]
                    - self.config.decay * state[index]
                    - self.config.laplacian_gain * diffusion[index]
                    - self.config.inhibition_gain * drive.inhibition[index]
                    - self.config.risk_gain * drive.risk[index];
                return Err(BrainError::Numerical("cognitive_field_state_non_finite".into()));
```

### 144. Cognitive field: `coalitions`
- **Ancla:** `src/engine/cognitive_field.rs::coalitions`
- **Forma / líneas:** let salience = mean_activation.max(0.0) * (0.5 + 0.5 * internal_coherence);
- **Fragmento:**
```rust
            return Err(BrainError::Invalid("cognitive_field_activation_shape".into()));
            let mean_activation = component
            let salience = mean_activation.max(0.0) * (0.5 + 0.5 * internal_coherence);
                mean_activation,
```

### 145. Parametric program: `compile_operator_to_fields`
- **Ancla:** `src/engine/parametric_program.rs::compile_operator_to_fields`
- **Forma / líneas:** let relative_residual = norm(&residual)? / norm(operator)?.max(1e-15);
- **Fragmento:**
```rust
    fields: &[SkillField],
    ridge: f64,
    max_relative_residual: f64,
        || !ridge.is_finite()
        || ridge < 0.0
        || !max_relative_residual.is_finite()
        || max_relative_residual < 0.0
        return Err(BrainError::Invalid("parametric_program_operator_shape".into()));
    let rank = fields.len();
    let mut gram = Matrix::zeros(rank, rank);
    let mut rhs = vec![0.0; rank];
    for i in 0..rank {
        rhs[i] = dot(&fields[i].direction, operator)?;
        for j in 0..rank {
            gram.set(i, j, dot(&fields[i].direction, &fields[j].direction)?);
    for i in 0..rank {
        gram.set(i, i, gram.get(i, i) + ridge);
    let reconstructed = compose_skill_fields(fields, &coefficients)?;
    let residual = reconstructed
    let relative_residual = norm(&residual)? / norm(operator)?.max(1e-15);
```

### 146. Parametric program: `apply_parametric_transition`
- **Ancla:** `src/engine/parametric_program.rs::apply_parametric_transition`
- **Forma / líneas:** scores[row] += operator[row * state_dim + col] * state[col];
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("parametric_program_transition_shape".into()));
    let mut scores = vec![0.0; state_dim];
            scores[row] += operator[row * state_dim + col] * state[col];
    let winner = argmax(&scores)?;
```

### 147. Parametric program: `ties_merge`
- **Ancla:** `src/engine/parametric_program.rs::ties_merge`
- **Forma / líneas:** merged[coordinate] = agreeing.iter().sum::<f64>() / agreeing.len() as f64;
- **Fragmento:**
```rust
        return Err(BrainError::Invalid("parametric_program_ties_input".into()));
```

### 148. Engine analysis: `analyze_canonical`
- **Ancla:** `src/engine/analysis.rs::analyze_canonical`
- **Forma / líneas:** let spectral_normalized_rms = spectral.reconstruction_rms / input_rms;
- **Fragmento:**
```rust
        let sbas = reconstruct_trajectory(obs, self.config.ridge)?;
            analyze_weight_dynamics(obs, sbas.cycle_rms, sbas.max_edge_residual)?;
        let conf = remove_confounders(obs, self.config.ridge)?;
                .or_default() += 1;
                        BrainError::Integrity("analysis_group_independence_weight_missing".into())
                Ok::<f64, BrainError>(
                        / (group_counts[o.independence_group.as_str()] as f64).sqrt(),
        // Hypothesis A: energy/spectral tomography over confounder-residualized
            reconstruct_skill_fields(&conf.residuals, &weights, &groups, generation, &self.config)?;
            self.config.ridge,
```

### 149. Operator plasticity advice: `discovery_binding_from_value`
- **Ancla:** `src/operator/control_plane.rs::discovery_binding_from_value`
- **Forma / líneas:** let all_scores_zero = !scores.is_empty() && scores.iter().all(|score| *score == 0.0);
- **Fragmento:**
```rust
    let scores = evaluations
            row.get("weighted_score")
    let all_scores_zero = !scores.is_empty() && scores.iter().all(|score| *score == 0.0);
        all_scores_zero,
```

### 150. Operator plasticity advice: `model_scan_rejects_roots_outside_hub`
- **Ancla:** `src/operator/control_plane.rs::model_scan_rejects_roots_outside_hub`
- **Forma / líneas:** let error = discover_local_models(Path::new("/tmp"))
- **Fragmento:**
```rust
        let error = discover_local_models(Path::new("/tmp"))
            error.contains("operator_model_scan_root_outside_hub")
                || error.contains("operator_model_hub_missing")
                || error.contains("operator_model_hub_invalid")
```

### 151. Capability IR scoring: `verify_receiver_signature`
- **Ancla:** `src/capability/capability_ir.rs::verify_receiver_signature`
- **Forma / líneas:** Verify a receiver-produced functional signature against the same
- **Fragmento:**
```rust
        self.validate_against(ir)?;
            .map_err(|_| BrainError::Invalid("operational_state_dimension_overflow".into()))?;
            .ok_or_else(|| BrainError::Invalid("operational_signature_size_overflow".into()))?;
            return Err(BrainError::Invalid("receiver_operational_signature_invalid".into()));
                .ok_or_else(|| BrainError::Integrity("operator_ir_source_anchor_unknown".into()))?;
                .ok_or_else(|| BrainError::Integrity("operator_ir_target_anchor_unknown".into()))?;
                .sqrt();
                .sqrt();
                return Err(BrainError::Numerical("receiver_operational_metric_non_finite".into()));
        let closure_satisfied = max_closure <= self.maximum_closure_error;
            maximum_observed_closure_error: max_closure,
```

## Constantes de política (entran en las fórmulas)

### `config/plasticity.toml`
```
schema = "cerebro.cross_model.control_plane/v1"

[bcm]
initial_theta = 0.5
window_size = 100
learning_rate = 0.01
theta_decay = 0.001

[eligibility]
initial_trace = 0.0
decay_factor = 0.95
trace_update_rate = 0.1
max_trace_value = 1.0

[modulation]
reward_weight = 0.4
attention_weight = 0.3
novelty_weight = 0.2
stability_weight = 0.1
decay_factor = 0.99

[routing]
uncertainty_weight = 0.05
minimum_measured_score = 0.0

[content_drift]
similarity_threshold = 0.8
adaptation_rate = 0.1
maximum_pressure = 1.0

[pi]
proportional_gain = 0.5
integral_gain = 0.1
output_min = 0.0
output_max = 1.0
integral_windup_limit = 1.0

[elo]
initial_rating = 1500.0
k_factor = 32.0
rating_floor = 100.0
rating_ceiling = 3000.0
logistic_scale = 400.0

```

### `config/governance.toml`
```
schema = "cerebro.cross_model.governance/v1"

[discovery]
minimum_probes = 4
minimum_mean_gap = 0.10
significance_alpha = 0.05

[alignment]
ridge_lambda = 0.0001
maximum_validation_residual = 0.35
maximum_calibration_pairs = 256

[promotion]
minimum_behavioral_score = 0.55
minimum_preservation_score = 0.98
minimum_independent_replay_score = 0.98
require_domain_fitness = true
production_authority = "core_universal_promotion_gate_and_adapter_bank"

[policy]
auto_promote = false
auto_transfer = false
behavioral_evidence_may_create_steering = false
steering_may_create_weight_delta = false

```

### `config/tidex.toml`
```
schema = "cerebro.cross_model.system/v1"

[runtime]
fail_closed = true
maximum_models = 64
maximum_stored_capabilities = 100000

[features]
behavioral_discovery = true
internal_activation_extraction = true
activation_intervention = true
automatic_activation_steering = false
automatic_production_promotion = false
core_adapter_bank = true
core_shadow_evaluation = true
core_weight_materialization = true

[evidence]
require_sha256 = true
require_deterministic_verifier = true
require_independent_replay_for_promotion = true

```

### `config/materialization/backend-selection.json`
```
{
  "schema": "cerebro.tidex.backend_selection_policy/v1",
  "minimum_functional_ci_lower": 0.8,
  "minimum_preservation_score": 0.95,
  "minimum_identity_margin": 0.05,
  "minimum_numerical_stability": 0.99,
  "maximum_normalized_risk": 0.1,
  "maximum_latency_micros": 1000000,
  "maximum_resident_bytes": 1073741824,
  "functional_weight": 0.35,
  "preservation_weight": 0.25,
  "stability_weight": 0.15,
  "risk_weight": 0.1,
  "latency_weight": 0.075,
  "memory_weight": 0.075,
  "required_controls": [
    "unmodified_receiver",
    "dense_delta",
    "conventional_low_rank",
    "wrong_capability_ir",
    "random_delta",
    "mean_capability",
    "nearest_capability",
    "alternative_backend",
    "non_target_preservation"
  ],
  "allow_hybrid": true,
  "minimum_hybrid_complementarity": 0.05
}

```

### `config/materialization/low-rank-policy.json`
```
{
  "schema": "cerebro.tidex.low_rank_shadow_policy/v1",
  "maximum_rank": 32,
  "relative_reconstruction_tolerance": 0.001,
  "absolute_reconstruction_tolerance": 1e-8,
  "minimum_parameter_reduction_ratio": 0.5,
  "maximum_svd_sweeps": 250
}

```

### `config/materialization/sparse-policy.json`
```
{
  "schema": "cerebro.tidex.sparse_shadow_policy/v1",
  "maximum_nonzero_count": 1048576,
  "maximum_density": 0.1,
  "absolute_zero_threshold": 1e-10,
  "relative_reconstruction_tolerance": 0.01,
  "absolute_reconstruction_tolerance": 1e-8,
  "minimum_storage_reduction_ratio": 0.5
}

```

### `config/materialization/steering-policy.json`
```
{
  "schema": "cerebro.tidex.activation_steering_policy/v1",
  "maximum_vector_l2": 20.0,
  "maximum_absolute_component": 5.0,
  "maximum_gain": 2.0,
  "allow_zero_vector": false
}

```
