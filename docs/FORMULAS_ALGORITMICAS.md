# Catálogo completo de fórmulas algorítmicas — TIDE-X / `/home/yo/Future`

Inventario exhaustivo de fórmulas, reglas de actualización y métricas implementadas en código (Rust/Python) y políticas numéricas en `config/`.

**Alcance:** `src/**`, `quality/experiments/**`, `config/**`, docs de diseño con reglas. Excluido: `runtime/` (artefactos), `.git`, locks.

**Funciones algorítmicas catalogadas:** 849 en 94 módulos.

## Fórmulas canónicas (forma cerrada destacada)

| ID | Fórmula | Dónde |
|----|---------|-------|
| F01 | `y = Wx` dot / matvec | `foundation/linalg` |
| F02 | `‖x‖₂ = sqrt(Σ xᵢ²)` (+ RMS estable) | `linalg::norm`, `stable_rms` |
| F03 | `cos(a,b) = (a·b)/(‖a‖‖b‖)` | `linalg::cosine` |
| F04 | Ridge: `(AᵀA + λI)⁻¹` | `linalg::inverse_with_ridge` |
| F05 | Rank-1 min-norm amortiguado | `low_rank_math::solve_minimum_norm_rank_one` |
| F06 | Multi-caso: `Y (XXᵀ + λI)⁻¹ X` | `low_rank_math::solve_regularized_multi_case_low_rank` |
| F07 | Residual relativo `‖Ŷ-Y‖/‖Y‖` | `dense_multi_case_relative_residual` |
| F08 | R² regresión | `validation::regression_r2` |
| F09 | Rango efectivo / energy rank del espectro | `effective_rank_from_spectrum`, `choose_energy_rank` |
| F10 | Jacobi SVD / eigen simétrico | `solver_portfolio::direct_one_sided_jacobi_svd`, `symmetric_eigen_jacobi_raw` |
| F11 | Transporte relacional / mapa afín | `analysis/transport` |
| F12 | SBAS / inversión temporal | `sbas`, `temporal_tracking::sbas_inversion` |
| F13 | Proyección geodésica Pythagoras | `pythagoras_topology` |
| F14 | Pearson correlación | `protected_map::pearson` |
| F15 | BCM metaplasticidad (θ, Δw) | `plasticity/bcm_metaplasticity` + `config/plasticity.toml` |
| F16 | Eligibility trace decay/update | `plasticity/eligibility_traces` |
| F17 | Neuromodulación ponderada R/A/N/S | `neuromodulation` + config weights |
| F18 | Elo expected/update | `plasticity/elo_system` |
| F19 | PI controller | `plasticity/pi_controller` |
| F20 | Gap detection (mean gap, α significancia) | `gap_detector` + `governance.toml` |
| F21 | Promotion gates (behavioral/preservation/replay) | `promotion_gates` + governance thresholds |
| F22 | LoRA / steering jerárquico | `lora_synthesizer`, `hierarchical_steering_extractor` |
| F23 | Shadow low-rank / sparse / steering materialization | `materialization/*` |
| F24 | Applicability similarity (procedural memory) | `procedural_memory` |
| F25 | PETFC / portfolio risk weights | `portfolio_governance`, `learning_orchestrator` |
| F26 | Digest SHA-256 domain-separated | `foundation/digest`, `build.rs` |
| F27 | Readout dot roundoff bound | `receiver/validation` |
| F28 | Neumaier compensated sum | `linalg::compensated_sum` |

## Fundamentos lineales y numéricos

### `src/foundation/linalg.rs` (18 fns)

#### `dot`  ·  L115
- **Ecuaciones / líneas clave:**
  - L115: `pub fn dot(a: &[f64], b: &[f64]) -> BrainResult<f64> {`
  - L117: `return Err(BrainError::Invalid("dot_shape_or_value".into()));`
  - L119: `let left_scale = a.iter().map(\|value\| value.abs()).fold(0.0_f64, f64::max);`
  - L120: `let right_scale = b.iter().map(\|value\| value.abs()).fold(0.0_f64, f64::max);`
  - L124: `let normalized = compensated_sum(`
  - L129: `let value = (normalized * left_scale) * right_scale;`
  - L131: `return Err(BrainError::Numerical("dot_non_finite_result".into()));`

#### `compensated_sum`  ·  L138
- **Doc:** Neumaier compensated summation for finite values. This is the shared numerical reduction primitive for persisted metrics and decisions.
- **Ecuaciones / líneas clave:**
  - L146: `if sum.abs() >= value.abs() {`
  - L147: `correction += (sum - updated) + value;`
  - L149: `correction += (value - updated) + sum;`

#### `stable_rms`  ·  L161
- **Doc:** Overflow-resistant RMS using a scaled sum-of-squares recurrence.
- **Ecuaciones / líneas clave:**
  - L166: `let absolute = value.abs();`
  - L182: `sum_squares += ratio * ratio;`
  - L191: `scale * (sum_squares / count as f64).sqrt()`

#### `norm`  ·  L198
- **Ecuaciones / líneas clave:**
  - L198: `pub fn norm(a: &[f64]) -> BrainResult<f64> {`
  - L200: `return Err(BrainError::Invalid("norm_input_invalid".into()));`
  - L202: `let scale = a.iter().map(\|value\| value.abs()).fold(0.0_f64, f64::max);`
  - L206: `let scaled_square_sum = a.iter().map(\|value\| (value / scale).powi(2)).sum::<f64>();`
  - L207: `let value = scale * scaled_square_sum.sqrt();`
  - L209: `return Err(BrainError::Numerical("norm_non_finite_result".into()));`

#### `normalize`  ·  L213
- **Ecuaciones / líneas clave:**
  - L213: `pub fn normalize(a: &[f64]) -> BrainResult<Vec<f64>> {`
  - L214: `let n = norm(a)?;`
  - L216: `return Err(BrainError::Numerical("normalize_zero_norm".into()));`
  - L218: `let normalized = a.iter().map(\|value\| value / n).collect::<Vec<_>>();`
  - L219: `if normalized.iter().any(\|value\| !value.is_finite()) {`
  - L220: `return Err(BrainError::Numerical("normalize_non_finite_result".into()));`
  - L222: `Ok(normalized)`

#### `cosine`  ·  L224
- **Ecuaciones / líneas clave:**
  - L224: `pub fn cosine(a: &[f64], b: &[f64]) -> BrainResult<f64> {`
  - L226: `return Err(BrainError::Invalid("cosine_dimension_mismatch".into()));`
  - L228: `let left_norm = norm(a)?;`
  - L229: `let right_norm = norm(b)?;`
  - L230: `if left_norm == 0.0 \|\| right_norm == 0.0 {`
  - L231: `return Err(BrainError::Numerical("cosine_zero_norm".into()));`
  - L236: `.map(\|(left, right)\| (left / left_norm) * (right / right_norm))`
  - L239: `return Err(BrainError::Numerical("cosine_non_finite_result".into()));`

#### `solve`  ·  L265
- **Ecuaciones / líneas clave:**
  - L278: `.map(\|value\| value.abs())`
  - L283: `let mut best = a.get(k, k).abs();`
  - L285: `let v = a.get(r, k).abs();`
  - L312: `if f.abs() < 1e-18 {`

#### `inverse_with_ridge`  ·  L331
- **Ecuaciones / líneas clave:**
  - L331: `pub fn inverse_with_ridge(a: &Matrix, ridge: f64) -> BrainResult<Matrix> {`
  - L333: `if a.rows != a.cols \|\| a.rows == 0 \|\| !ridge.is_finite() \|\| ridge < 0.0 {`
  - L339: `base.data[i * n + i] += ridge;`

#### `weighted_normal_solve`  ·  L353
- **Ecuaciones / líneas clave:**
  - L353: `pub fn weighted_normal_solve(`
  - L357: `ridge: f64,`
  - L368: `\|\| !ridge.is_finite()`
  - L369: `\|\| ridge < 0.0`
  - L379: `b[i] += w * xi * y[r];`
  - L381: `a.data[i * x.cols + j] += w * xi * x.get(r, j);`
  - L386: `a.data[i * x.cols + i] += ridge;`

#### `symmetric_top_eigen`  ·  L391
- **Ecuaciones / líneas clave:**
  - L404: `.map(\|i\| (((i + 1) * (comp + 3)) as f64 * 0.731).sin() + 0.17)`
  - L406: `v = normalize(&v)?;`
  - L410: `let p = dot(&w, q)?;`
  - L413: `let wn = norm(&w)?;`
  - L423: `let lambda = dot(&v, &av)?.max(0.0);`
  - L424: `if lambda < 1e-12 {`
  - L428: `out.push((lambda, v));`

#### `weighted_row_gram`  ·  L434
- **Ecuaciones / líneas clave:**
  - L446: `let v = weights[i].sqrt() * weights[j].sqrt() * dot(d.row(i), d.row(j))?;`

#### `symmetric_eigen_jacobi_raw`  ·  L467
- **Ecuaciones / líneas clave:**
  - L483: `.map(\|value\| value.abs())`
  - L486: `let symmetry_tolerance = f64::EPSILON.sqrt() * scale * n as f64;`
  - L489: `if (a.get(row, column) - a.get(column, row)).abs() > symmetry_tolerance {`
  - L504: `let x = d.get(i, j).abs();`
  - L520: `let c = phi.cos();`
  - L521: `let s = phi.sin();`
  - L554: `Ok((d.get(i, i), normalize(&vec)?))`

#### `symmetric_eigen_jacobi_signed`  ·  L565
- **Doc:** Signed eigen-decomposition for symmetric matrices. Unlike the historical energy helper, this preserves negative and near-zero eigenvalues so callers can validate positive semidefiniteness instead of silently truncating dangerous negative curvature.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `symmetric_eigen_jacobi`  ·  L575
- **Doc:** Energy-oriented symmetric eigendecomposition. Negative eigenvalues are intentionally discarded because Gram/energy callers require a PSD spectrum.
- **Ecuaciones / líneas clave:**
  - L583: `.map(\|(value, _)\| value.abs())`
  - L586: `let negative_tolerance = f64::EPSILON.sqrt() * scale * a.rows.max(1) as f64;`

#### `signed_eigensolver_preserves_negative_eigenvalues`  ·  L604
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `direction_operations_reject_zero_norm_instead_of_fabricating_geometry`  ·  L617
- **Ecuaciones / líneas clave:**
  - L617: `fn direction_operations_reject_zero_norm_instead_of_fabricating_geometry() {`
  - L619: `normalize(&[0.0, 0.0]),`
  - L620: `Err(BrainError::Numerical(message)) if message == "normalize_zero_norm"`
  - L623: `cosine(&[1.0, 0.0], &[0.0, 0.0]),`
  - L624: `Err(BrainError::Numerical(message)) if message == "cosine_zero_norm"`
  - L626: `let tiny = normalize(&[1e-300, 0.0]).unwrap();`
  - L627: `assert!((tiny[0] - 1.0).abs() <= 4.0 * f64::EPSILON);`
  - L629: `assert!((cosine(&[1e300, 1e300], &[1e300, 1e300]).unwrap() - 1.0).abs() < 1e-12);`

#### `stable_reductions_survive_large_finite_inputs_and_cancellation`  ·  L641
- **Ecuaciones / líneas clave:**
  - L642: `let scale = f64::MAX.sqrt();`
  - L645: `assert!((rms / scale - 1.0).abs() < 1e-12);`
  - L650: `let stable_dot = dot(&[1.0e16, 1.0, -1.0e16], &[1.0, 1.0, 1.0]).unwrap();`
  - L651: `assert!((stable_dot - 1.0).abs() < 1e-9);`

#### `weighted_algebra_rejects_invalid_weights_and_regularization`  ·  L655
- **Ecuaciones / líneas clave:**
  - L657: `assert!(weighted_normal_solve(&design, &[1.0, 2.0], &[1.0, -0.1], 1e-6).is_err());`
  - L658: `assert!(weighted_normal_solve(&design, &[1.0, 2.0], &[1.0, f64::NAN], 1e-6).is_err());`
  - L659: `assert!(weighted_normal_solve(&design, &[1.0, 2.0], &[1.0, 1.0], -1.0).is_err());`
  - L661: `assert!(inverse_with_ridge(&Matrix::identity(1), f64::NAN).is_err());`

### `src/foundation/low_rank_math.rs` (7 fns)

#### `solve_minimum_norm_rank_one`  ·  L40
- **Doc:** Solve the damped minimum-norm rank-one update for one observation.  For an input `x` and requested local output shift `y`, this returns `delta-W = y x^T / (x^T x + damping)`. It is a numerical solution, not a capability, causal, equivalence, or promotion claim.
- **Ecuaciones / líneas clave:**
  - L40: `pub fn solve_minimum_norm_rank_one(`
  - L44: `) -> BrainResult<MinimumNormRankOneSolution> {`
  - L54: `return Err(BrainError::Invalid("minimum_norm_rank_one_input_invalid".into()));`
  - L56: `let input_squared_norm = input_activation`
  - L58: `.map(\|value\| f64::from(*value).powi(2))`
  - L60: `if !input_squared_norm.is_finite() \|\| input_squared_norm <= f64::EPSILON {`
  - L61: `return Err(BrainError::Numerical("minimum_norm_rank_one_activation_degenerate".into()));`
  - L63: `let denominator = input_squared_norm + damping;`
  - L65: `return Err(BrainError::Numerical("minimum_norm_rank_one_denominator_invalid".into()));`
  - L72: `return Err(BrainError::Numerical("minimum_norm_rank_one_factor_non_finite".into()));`
  - L83: `let residual_norm = predicted_shift`
  - L86: `.map(\|(predicted, desired)\| f64::from(*predicted - *desired).powi(2))`
  - L88: `.sqrt();`
  - L89: `let left_squared_norm = desired_output_shift`
  - L91: `.map(\|value\| f64::from(*value).powi(2))`
  - L93: `let right_squared_norm = right`
  - L95: `.map(\|value\| f64::from(*value).powi(2))`
  - L97: `let frobenius_norm = (left_squared_norm * right_squared_norm).sqrt();`
  - L98: `if !residual_norm.is_finite() \|\| !frobenius_norm.is_finite() \|\| frobenius_norm == 0.0 {`
  - L99: `return Err(BrainError::Numerical("minimum_norm_rank_one_solution_invalid".into()));`
  - L101: `Ok(MinimumNormRankOneSolution {`
  - L105: `residual_norm,`
  - L106: `frobenius_norm,`

#### `solve_regularized_multi_case_low_rank`  ·  L115
- **Doc:** Fit the minimum-norm ridge update `Y (X X^T + lambda I)^-1 X`.  The returned factorization has one component per independently supplied case. A singleton or duplicate activation set is rejected so it cannot be mislabeled as multi-case evidence.
- **Ecuaciones / líneas clave:**
  - L115: `pub fn solve_regularized_multi_case_low_rank(`
  - L119: `) -> BrainResult<MultiCaseLowRankSolution> {`
  - L123: `\|\| cases as u64 > MAX_LOW_RANK`
  - L127: `return Err(BrainError::Invalid("multi_case_low_rank_input_invalid".into()));`
  - L144: `return Err(BrainError::Invalid("multi_case_low_rank_examples_invalid".into()));`
  - L156: `gram[row * cases + row] += damping;`
  - L185: `let mut residual_squared = 0.0_f64;`
  - L201: `residual_squared +=`
  - L202: `(predicted - f64::from(desired_output_shifts[case_index][output])).powi(2);`
  - L217: `let residual_norm = residual_squared.sqrt();`
  - L218: `let frobenius_norm = frobenius_squared.sqrt();`
  - L219: `if !residual_norm.is_finite() \|\| !frobenius_norm.is_finite() \|\| frobenius_norm <= 0.0 {`
  - L220: `return Err(BrainError::Numerical("multi_case_low_rank_solution_invalid".into()));`
  - L222: `Ok(MultiCaseLowRankSolution {`
  - L223: `rank: cases as u64,`
  - L226: `residual_norm,`
  - L227: `frobenius_norm,`

#### `dense_multi_case_relative_residual`  ·  L234
- **Doc:** Recompute the relative residual of a dense linear update against an exact multi-case contract. Integrations use this independent recomputation rather than trusting the residual reported by a producer.
- **Ecuaciones / líneas clave:**
  - L234: `pub fn dense_multi_case_relative_residual(`
  - L251: `return Err(BrainError::Integrity("multi_case_low_rank_residual_inputs_invalid".into()));`
  - L253: `let mut residual_squared = 0.0_f64;`
  - L262: `residual_squared += (predicted - shift[row]).powi(2);`
  - L263: `target_squared += shift[row].powi(2);`
  - L267: `return Err(BrainError::Numerical("multi_case_low_rank_relative_residual_invalid".into()));`
  - L269: `let relative = residual_squared.sqrt() / target_squared.sqrt();`
  - L271: `return Err(BrainError::Numerical("multi_case_low_rank_relative_residual_invalid".into()));`

#### `cholesky_spd`  ·  L276
- **Ecuaciones / líneas clave:**
  - L287: `"multi_case_low_rank_gram_indefinite".into(),`
  - L290: `lower[row * dimension + column] = sum.sqrt();`

#### `solve_cholesky`  ·  L299
- **Ecuaciones / líneas clave:**
  - L317: `return Err(BrainError::Numerical("multi_case_low_rank_linear_solve_invalid".into()));`

#### `rank_one_solution_is_exact_without_damping`  ·  L327
- **Ecuaciones / líneas clave:**
  - L327: `fn rank_one_solution_is_exact_without_damping() {`
  - L328: `let solution = solve_minimum_norm_rank_one(&[3.0, 4.0], &[2.0, -1.0], 0.0).unwrap();`
  - L329: `assert!(solution.residual_norm <= 1e-6);`

#### `degenerate_and_invalid_rank_one_inputs_fail_closed`  ·  L335
- **Ecuaciones / líneas clave:**
  - L335: `fn degenerate_and_invalid_rank_one_inputs_fail_closed() {`
  - L336: `assert!(solve_minimum_norm_rank_one(&[0.0, 0.0], &[1.0], 0.0).is_err());`
  - L337: `assert!(solve_minimum_norm_rank_one(&[1.0], &[1.0], -1.0).is_err());`
  - L338: `assert!(solve_minimum_norm_rank_one(&[f32::NAN], &[1.0], 0.0).is_err());`

### `src/foundation/validation.rs` (9 fns)

#### `validate_identifier`  ·  L19
- **Doc:** Canonical identifier validator shared by all persistent manifests. This keeps stable naming rules in one place, preventing divergent checks in different subsystems of the same runtime.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `validate_http_endpoint`  ·  L34
- **Doc:** Validate a network endpoint used by external model providers.
- **Ecuaciones / líneas clave:**
  - L38: `let normalized = value`
  - L42: `if normalized.is_empty() \|\| normalized.contains(char::is_whitespace) {`

#### `source_support_indices`  ·  L69
- **Doc:** Indices whose coefficient is numerically material in a source mixture. The tolerance is scale-relative and shared by memory, dense materialization, dual-space reconstruction and structured geometry so those authorities do not disagree about which observations support a field.
- **Ecuaciones / líneas clave:**
  - L75: `.map(\|value\| value.abs())`
  - L80: `let tolerance = max_abs * f64::EPSILON.sqrt();`
  - L84: `.filter_map(\|(index, value)\| (value.abs() > tolerance).then_some(index))`

#### `regression_r2`  ·  L137
- **Ecuaciones / líneas clave:**
  - L158: `sse += (actual_row[column] - predicted_row[column]).powi(2);`
  - L159: `sst += (actual_row[column] - means[column]).powi(2);`
  - L162: `Ok(if sst <= 1e-18 { 0.0 } else { 1.0 - sse / sst })`

#### `validate_symmetric_psd`  ·  L165
- **Ecuaciones / líneas clave:**
  - L173: `.map(\|value\| value.abs())`
  - L176: `let tolerance = f64::EPSILON.sqrt() * scale * matrix.rows as f64;`
  - L179: `if (matrix.get(i, j) - matrix.get(j, i)).abs() > tolerance {`

#### `symmetric_psd_condition`  ·  L191
- **Ecuaciones / líneas clave:**
  - L200: `let tolerance = largest * f64::EPSILON.sqrt() * matrix.rows.max(1) as f64;`

#### `effective_rank_from_spectrum`  ·  L213
- **Ecuaciones / líneas clave:**
  - L213: `pub fn effective_rank_from_spectrum(eigenvalues: &[f64]) -> BrainResult<f64> {`
  - L215: `return Err(BrainError::Invalid("effective_rank_spectrum_nonfinite".into()));`
  - L225: `(probability > 1e-15).then_some(-probability * probability.ln())`
  - L228: `Ok(entropy.exp())`

#### `choose_energy_rank`  ·  L231
- **Ecuaciones / líneas clave:**
  - L231: `pub fn choose_energy_rank(`
  - L234: `max_rank: usize,`
  - L235: `minimum_rank: usize,`
  - L241: `\|\| max_rank == 0`
  - L242: `\|\| minimum_rank > max_rank`
  - L244: `return Err(BrainError::Invalid("energy_rank_input_invalid".into()));`
  - L248: `return Ok(minimum_rank.min(eigenvalues.len()));`
  - L250: `let cap = max_rank.min(eigenvalues.len());`
  - L253: `accumulated += value.max(0.0);`
  - L255: `return Ok((index + 1).max(minimum_rank).min(cap));`
  - L258: `Ok(cap.max(minimum_rank.min(eigenvalues.len())))`

#### `identifier_validator_matches_managed_name_policy`  ·  L305
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/foundation/finite.rs` (1 fns)

#### `rejects_nonfinite_and_noncanonical_wire_values`  ·  L74
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/foundation/digest.rs` (9 fns)

#### `digest_bytes`  ·  L31
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest_domain`  ·  L36
- **Doc:** Hash a domain-separated payload in exactly one SHA-256 round.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `as_digest`  ·  L182
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest_requires_canonical_lowercase_without_changing_wire_shape`  ·  L459
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest_rejects_missing_or_malformed_identity`  ·  L468
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `sha256_digest_traits_file_hash_and_conversions_are_canonical`  ·  L486
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `every_compatibility_semantic_digest_exercises_its_full_trait_surface`  ·  L545
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `sealed_semantic_digests_round_trip_without_raw_public_conversions`  ·  L597
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `semantic_digest_domains_are_distinct_but_wire_compatible`  ·  L617
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/foundation/security.rs`
_Sin funciones algorítmicas filtradas (revisar a mano si es glue)._

## Análisis (tomografía, transporte, topología)

### `src/analysis/transport.rs` (29 fns)

#### `validate`  ·  L23
- **Ecuaciones / líneas clave:**
  - L25: `Self::FixedRidge { ridge } => *ridge,`
  - L26: `Self::CenteredTraceRidge { relative_ridge } => *relative_ridge,`
  - L49: `pub effective_ridge: f64,`
  - L79: `pub mean_loo_cosine: f64,`
  - L80: `pub min_loo_cosine: f64,`
  - L104: `pub mean_loo_cosine: f64,`
  - L105: `pub min_loo_cosine: f64,`

#### `fit_affine`  ·  L143
- **Ecuaciones / líneas clave:**
  - L143: `fn fit_affine(source: &[Vec<f64>], target: &[Vec<f64>], ridge: f64) -> BrainResult<TransportMap> {`
  - L149: `if !ridge.is_finite() \|\| ridge <= 0.0 {`
  - L150: `return Err(BrainError::Invalid("transport_ridge_invalid".into()));`
  - L160: `let beta = weighted_normal_solve(&design, &y, &row_weights, ridge)?;`
  - L170: `squared_error += (prediction - target[row][output]).powi(2);`
  - L178: `training_rms: (squared_error / (source.len() * target_dim) as f64).sqrt(),`

#### `fit_affine_with_policy`  ·  L182
- **Ecuaciones / líneas clave:**
  - L188: `let relative_ridge = match policy {`
  - L189: `AffineTransportPolicy::FixedRidge { ridge } => {`
  - L190: `let map = fit_affine(source, target, *ridge)?;`
  - L197: `effective_ridge: *ridge,`
  - L201: `AffineTransportPolicy::CenteredTraceRidge { relative_ridge } => *relative_ridge,`
  - L210: `effective_ridge: *reg,`
  - L240: `let unit_scale = stable_rms(centered.iter().flatten().copied())? * count.sqrt();`
  - L243: `let effective_ridge = relative_ridge * regularization_scale;`
  - L249: `\|\| !effective_ridge.is_finite()`
  - L250: `\|\| effective_ridge <= 0.0`
  - L254: `let normalized_rows = centered`
  - L262: `let design = Matrix::from_rows(&normalized_rows)?;`
  - L271: `let beta = weighted_normal_solve(&design, &centered_target, &row_weights, relative_ridge)?;`
  - L275: `bias[output] = target_mean[output] - dot(weights.row(output), &source_mean)?;`
  - L288: `let residuals = source`
  - L300: `map.training_rms = stable_rms(residuals.iter().flatten().copied())?;`
  - L309: `effective_ridge,`

#### `learn_transport`  ·  L316
- **Doc:** Backwards-compatible generation transport, now affine rather than forced through the origin. Use `learn_transport_validated` before promotion.
- **Ecuaciones / líneas clave:**
  - L319: `ridge: f64,`
  - L321: `fit_affine(source, target, ridge)`

#### `global_r2`  ·  L340
- **Ecuaciones / líneas clave:**
  - L357: `sse += (actual_row[column] - predicted_row[column]).powi(2);`
  - L358: `sst += (actual_row[column] - means[column]).powi(2);`
  - L361: `Ok(if sst <= 1e-18 { 0.0 } else { 1.0 - sse / sst })`

#### `leave_one_out_predictions`  ·  L364
- **Ecuaciones / líneas clave:**
  - L367: `ridge: f64,`
  - L386: `let map = fit_affine(&train_source, &train_target, ridge)?;`

#### `learn_transport_validated`  ·  L392
- **Ecuaciones / líneas clave:**
  - L395: `ridge: f64,`
  - L400: `let map = fit_affine(source, target, ridge)?;`
  - L401: `let predicted = leave_one_out_predictions(source, target, ridge)?;`

#### `compute_sinkhorn_optimal_transport`  ·  L417
- **Doc:** Computes formal Entropic Regularized Sinkhorn Optimal Transport between two representation point clouds: \min_{P \in U(a,b)} \langle P, C \rangle + \epsilon \Omega(P)
- **Ecuaciones / líneas clave:**
  - L441: `sq_dist += (source[i][d] - target[j][d]).powi(2);`
  - L450: `kernel[i][j] = (-cost_matrix[i][j] / reg).exp();`
  - L470: `kb += kernel[i][j] * b[j];`
  - L479: `kt_a += kernel[i][j] * next_a[i];`
  - L486: `max_diff = max_diff.max((next_a[i] - a[i]).abs());`
  - L503: `wasserstein_distance += p_ij * cost_matrix[i][j];`

#### `validated_from_predictions`  ·  L516
- **Ecuaciones / líneas clave:**
  - L529: `.map(\|(left, right)\| (left - right).powi(2))`
  - L532: `let loo_cv_rms = (squared_error / (target.len() * target[0].len()) as f64).sqrt();`
  - L533: `let cosines = target`
  - L536: `.map(\|(actual, prediction)\| cosine(actual, prediction))`
  - L538: `let mean_loo_cosine = cosines.iter().sum::<f64>() / cosines.len() as f64;`
  - L539: `let min_loo_cosine = cosines.iter().copied().fold(f64::INFINITY, f64::min);`
  - L540: `let resolved = loo_cv_r2 > 0.0 && min_loo_cosine > 0.0;`
  - L547: `mean_loo_cosine,`
  - L548: `min_loo_cosine,`

#### `functional_leverage`  ·  L560
- **Ecuaciones / líneas clave:**
  - L563: `ridge: f64,`
  - L567: `\|\| !ridge.is_finite()`
  - L568: `\|\| ridge <= 0.0`
  - L591: `gram.set(index, index, gram.get(index, index) + ridge);`
  - L595: `let leverage = dot(&query, &solved)?;`

#### `functional_support_envelope`  ·  L603
- **Doc:** Maximum leave-one-capability-out leverage is fixed from calibration alone.
- **Ecuaciones / líneas clave:**
  - L606: `ridge: f64,`
  - L619: `maximum_loo = maximum_loo.max(functional_leverage(&train, &calibration[holdout], ridge)?);`
  - L621: `Ok((functional_leverage(calibration, query, ridge)?, maximum_loo))`

#### `validate_transport_with_topology`  ·  L626
- **Doc:** Validates a transport map by checking both quantitative leave-one-out metrics (R^2, RMS, Cosine) and qualitative manifold topology (Betti numbers, homotopy score).
- **Ecuaciones / líneas clave:**
  - L647: `(target_topo.topological_homotopy_score - pred_topo.topological_homotopy_score).abs();`

#### `learn_transport_validated_with_policy`  ·  L696
- **Doc:** Explicit policy entry point. Existing callers of learn_transport_validated retain the original fixed-ridge implementation. Numerical stabilization is not evidence of semantic generalization: the same held-out gates still apply.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `learn_functional_transplant_with_policy`  ·  L719
- **Doc:** Functional-signature to receiver-coordinate compilation using an explicitly selected affine policy. No target update or target training data is accepted.
- **Ecuaciones / líneas clave:**
  - L742: `mean_loo_cosine: validated.mean_loo_cosine,`
  - L743: `min_loo_cosine: validated.min_loo_cosine,`

#### `learn_functional_transplant`  ·  L754
- **Doc:** Functional transplantation does not require source and target parameter dimensions to match. Common functional signatures are the bridge. The map learns functional_signature -> target capability vector from matched target anchors and is leave-one-anchor-out validated before it can be resolved.
- **Ecuaciones / líneas clave:**
  - L757: `ridge: f64,`
  - L765: `learn_transport_validated(functional_anchors, target_capability_anchors, ridge)?;`
  - L773: `mean_loo_cosine: validated.mean_loo_cosine,`
  - L774: `min_loo_cosine: validated.min_loo_cosine,`

#### `transplant`  ·  L780
- **Ecuaciones / líneas clave:**
  - L806: `pub ridge: f64,`
  - L807: `pub loo_source_cosines: Vec<f64>,`
  - L808: `pub loo_target_cosines: Vec<f64>,`
  - L809: `pub loo_coefficient_norms: Vec<f64>,`
  - L810: `pub min_loo_source_cosine: f64,`
  - L811: `pub min_loo_target_cosine: f64,`
  - L812: `pub mean_loo_source_cosine: f64,`
  - L813: `pub mean_loo_target_cosine: f64,`
  - L814: `pub max_loo_coefficient_norm: f64,`
  - L827: `pub source_projection_cosine: Option<f64>,`
  - L828: `pub coefficient_norm: f64,`

#### `normalize_anchor_rows`  ·  L833
- **Ecuaciones / líneas clave:**
  - L833: `fn normalize_anchor_rows(rows: &[Vec<f64>], label: &str) -> BrainResult<Vec<Vec<f64>>> {`
  - L835: `let mut normalized = Vec::with_capacity(rows.len());`
  - L837: `if norm(row)? <= 1e-15 {`
  - L838: `return Err(BrainError::Invalid(format!("{label}_zero_norm")));`
  - L840: `normalized.push(normalize(row)?);`
  - L842: `Ok(normalized)`

#### `relational_coefficients`  ·  L845
- **Ecuaciones / líneas clave:**
  - L848: `ridge: f64,`
  - L853: `\|\| !ridge.is_finite()`
  - L854: `\|\| ridge <= 0.0`
  - L861: `rhs[i] = dot(&anchors[i], query)?;`
  - L863: `let value = dot(&anchors[i], &anchors[j])?;`
  - L867: `gram.set(i, i, gram.get(i, i) + ridge);`

#### `learn_relational_transport`  ·  L901
- **Doc:** Learn a transport from *relations between matched capabilities*, not from arbitrary target basis IDs. Each source holdout anchor is reconstructed from the remaining source anchors; the same barycentric coefficients are applied to the matched target anchors and compared with the true target holdout.  The map is resolved only when target leave-one-out reconstruction is at least as strong as the weakest source leave-one-out reconstruction. This is a data-derived gate: cross-backbone transport may n
- **Ecuaciones / líneas clave:**
  - L904: `ridge: f64,`
  - L909: `if !ridge.is_finite() \|\| ridge <= 0.0 {`
  - L910: `return Err(BrainError::Invalid("relational_transport_ridge".into()));`
  - L912: `let source = normalize_anchor_rows(source_anchors, "relational_source")?;`
  - L913: `let target = normalize_anchor_rows(target_anchors, "relational_target")?;`
  - L914: `let mut loo_source_cosines = Vec::with_capacity(source.len());`
  - L915: `let mut loo_target_cosines = Vec::with_capacity(source.len());`
  - L916: `let mut loo_coefficient_norms = Vec::with_capacity(source.len());`
  - L930: `let coefficients = relational_coefficients(&train_source, &source[holdout], ridge)?;`
  - L933: `let source_cosine = cosine(&source_prediction, &source[holdout])?;`
  - L934: `let target_cosine = cosine(&target_prediction, &target[holdout])?;`
  - L935: `let coefficient_norm = norm(&coefficients)?;`
  - L936: `if !source_cosine.is_finite() \|\| !target_cosine.is_finite() \|\| !coefficient_norm.is_finite()`
  - L940: `loo_source_cosines.push(source_cosine);`
  - L941: `loo_target_cosines.push(target_cosine);`
  - L942: `loo_coefficient_norms.push(coefficient_norm);`
  - L944: `let min_loo_source_cosine = loo_source_cosines`
  - L948: `let min_loo_target_cosine = loo_target_cosines`
  - L952: `let mean_loo_source_cosine =`
  - L953: `loo_source_cosines.iter().sum::<f64>() / loo_source_cosines.len() as f64;`
  - L954: `let mean_loo_target_cosine =`
  - L955: `loo_target_cosines.iter().sum::<f64>() / loo_target_cosines.len() as f64;`
  - L956: `let max_loo_coefficient_norm = loo_coefficient_norms`
  - L960: `let numerical_tolerance = f64::EPSILON.sqrt();`
  - L961: `let resolved = min_loo_source_cosine > 0.0`

#### `transplant`  ·  L985
- **Ecuaciones / líneas clave:**
  - L988: `\|\| norm(source_signature)? <= 1e-15`
  - L992: `let normalized = normalize(source_signature)?;`
  - L994: `relational_coefficients(&self.source_anchors, &normalized, self.ridge)?;`
  - L998: `let source_projection_cosine = if norm(&source_prediction)? <= 1e-15 {`
  - L1001: `Some(cosine(&source_prediction, &normalized)?)`
  - L1003: `let coefficient_norm = norm(&target_coefficients)?;`
  - L1004: `let numerical_tolerance = f64::EPSILON.sqrt();`
  - L1005: `let within_training_support = source_projection_cosine.is_some_and(\|cosine\| {`
  - L1006: `cosine + numerical_tolerance >= self.min_loo_source_cosine`
  - L1007: `&& coefficient_norm <= self.max_loo_coefficient_norm + numerical_tolerance`
  - L1012: `source_projection_cosine,`
  - L1013: `coefficient_norm,`

#### `centered_trace_ridge_preserves_source_units_and_affine_origins`  ·  L1025
- **Ecuaciones / líneas clave:**
  - L1025: `fn centered_trace_ridge_preserves_source_units_and_affine_origins() {`
  - L1043: `let policy = AffineTransportPolicy::CenteredTraceRidge {`
  - L1044: `relative_ridge: 0.2,`
  - L1082: `assert!((actual[output] - target_offset[output] - expected[output]).abs() < 1e-9);`
  - L1084: `let expected_ridge = reference_diagnostics.full_fit.effective_ridge * scale * scale;`
  - L1085: `assert!((diagnostics.full_fit.effective_ridge / expected_ridge - 1.0).abs() < 1e-12);`
  - L1086: `assert!((changed.loo_cv_r2 - reference.loo_cv_r2).abs() < 1e-10);`

#### `trace_ridge_loo_refits_mean_and_scale_without_holdout_values`  ·  L1091
- **Ecuaciones / líneas clave:**
  - L1091: `fn trace_ridge_loo_refits_mean_and_scale_without_holdout_values() {`
  - L1092: `let policy = AffineTransportPolicy::CenteredTraceRidge {`
  - L1093: `relative_ridge: 0.5,`
  - L1101: `// slope=(2*5)/(5+0.5*5), bias=6-slope*1.5.`
  - L1104: `assert!((predicted[4][0] - expected).abs() < 1e-10);`
  - L1106: `assert!((diagnostics[4].regularization_scale - 5.0).abs() < 1e-12);`
  - L1107: `assert!((diagnostics[4].effective_ridge - 2.5).abs() < 1e-12);`
  - L1116: `assert!((changed[4][0] - (6.0 + slope * (1e6 - 1.5))).abs() < 1e-8);`

#### `explicit_fixed_policy_preserves_legacy_maps_and_diagnostics_are_honest`  ·  L1121
- **Ecuaciones / líneas clave:**
  - L1124: `let policy = AffineTransportPolicy::FixedRidge { ridge: 0.75 };`
  - L1135: `.all(\|fit\| fit.effective_ridge == 0.75)`

#### `centered_trace_ridge_rejects_unidentified_design_in_full_fit_or_any_fold`  ·  L1143
- **Ecuaciones / líneas clave:**
  - L1143: `fn centered_trace_ridge_rejects_unidentified_design_in_full_fit_or_any_fold() {`
  - L1144: `let policy = AffineTransportPolicy::CenteredTraceRidge {`
  - L1145: `relative_ridge: 0.5,`
  - L1164: `for relative_ridge in [0.0, -1.0, f64::NAN, f64::INFINITY] {`
  - L1165: `let invalid = AffineTransportPolicy::CenteredTraceRidge { relative_ridge };`

#### `centered_trace_ridge_does_not_penalize_the_intercept_or_claim_constant_targets_resolved`  ·  L1171
- **Ecuaciones / líneas clave:**
  - L1171: `fn centered_trace_ridge_does_not_penalize_the_intercept_or_claim_constant_targets_resolved() {`
  - L1174: `let policy = AffineTransportPolicy::CenteredTraceRidge {`
  - L1175: `relative_ridge: 100.0,`
  - L1183: `r#"{"kind":"centered_trace_ridge","relative_ridge":1.0,"hidden_alternate":true}"#;`

#### `relational_transport_validates_geometry_and_rejects_ood_query`  ·  L1188
- **Ecuaciones / líneas clave:**
  - L1203: `assert!(map.min_loo_target_cosine + 1e-10 >= map.min_loo_source_cosine);`
  - L1208: `assert_eq!(ood.source_projection_cosine, None);`

#### `validated_affine_transport_generalizes_across_generation_anchors`  ·  L1214
- **Ecuaciones / líneas clave:**
  - L1235: `assert!(map.min_loo_cosine > 0.999999);`

#### `functional_transplant_crosses_incompatible_parameter_dimensions`  ·  L1240
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `topological_transport_validation_preserves_manifold_homology`  ·  L1271
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/analysis/tomography.rs` (12 fns)

#### `reconstruct`  ·  L28
- **Ecuaciones / líneas clave:**
  - L34: `let a = dot(d.row(r), h)?;`
  - L37: `rec.data[r * d.cols + p] += a * h[p];`
  - L42: `err += e * e;`
  - L45: `Ok((coeff, rec, (err / (d.rows * d.cols).max(1) as f64).sqrt()))`

#### `reconstruct_skill_fields`  ·  L48
- **Ecuaciones / líneas clave:**
  - L83: `let rank =`
  - L84: `choose_energy_rank(&eigenvalues, cfg.target_explained_variance, cfg.max_rank, 1)?;`
  - L87: `for (lambda, u) in eigs.iter().take(rank) {`
  - L89: `let denom = lambda.sqrt();`
  - L94: `.map(\|r\| weights[r].sqrt() * u[r] / denom)`
  - L98: `h[p] += mix[r] * d.get(r, p);`
  - L101: `let h_norm = norm(&h)?;`
  - L102: `if h_norm == 0.0 {`
  - L108: `*coefficient /= h_norm;`
  - L110: `dirs.push(normalize(&h)?);`
  - L114: `let residual_norms = (0..d.rows)`
  - L119: `s += e * e;`
  - L121: `s.sqrt()`
  - L125: `let med = median(residual_norms.clone())?;`
  - L126: `let mad = median(residual_norms.iter().map(\|x\| (x - med).abs()).collect())?.max(1e-9);`
  - L129: `let z = (residual_norms[r] - med).abs() / scale;`
  - L144: `let rank = final_dirs.len();`
  - L148: `.take(rank)`
  - L154: `for k in 0..rank {`
  - L156: `let lambda = final_eigs[k].0;`
  - L162: `if a.abs() > 0.15 * lambda.sqrt().max(1e-9) {`
  - L163: `support += 1;`
  - L164: `aligned.push((a.signum() * cosine(d.row(r), h)?).abs());`
  - L190: `singular_value: lambda.sqrt(),`
  - L191: `explained_variance: (lambda / total).clamp(0.0, 1.0),`

#### `global_field_alignment`  ·  L218
- **Ecuaciones / líneas clave:**
  - L246: `let similarity = cosine(&old[old_index].direction, &incoming[incoming_index].direction)?;`
  - L247: `if similarity.abs() >= threshold {`

#### `align_incoming_identities`  ·  L295
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `assimilate_bank`  ·  L322
- **Ecuaciones / líneas clave:**
  - L386: `merged = normalize(&merged)?;`

#### `reconcile_full_corpus`  ·  L438
- **Doc:** Reconcile a complete-corpus reconstruction against durable capabilities with one global Hungarian assignment. This makes identity independent of incoming field order and prevents two candidates from greedily claiming one prior.
- **Ecuaciones / líneas clave:**
  - L471: `prior.persistence *= 0.85;`
  - L472: `prior.coherence *= 0.90;`
  - L473: `prior.uncertainty *= 1.15;`

#### `tomography_sources_exactly_recreate_normalized_fields_and_report_used_weights`  ·  L517
- **Ecuaciones / líneas clave:**
  - L517: `fn tomography_sources_exactly_recreate_normalized_fields_and_report_used_weights() {`
  - L523: `max_rank: 2,`
  - L533: `reconstructed[column] += mixture[row] * matrix.get(row, column);`
  - L537: `assert!((actual - expected).abs() < 1e-10);`

#### `align_incoming_identities_preserves_prior_and_aligns_sign`  ·  L578
- **Ecuaciones / líneas clave:**
  - L589: `assert!((aligned[0].direction[0] - 1.0).abs() < 1e-10);`
  - L590: `assert!((aligned[0].direction[1] - 0.0).abs() < 1e-10);`

#### `assimilate_bank_replaces_on_exact_evidence_match`  ·  L594
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `assimilate_bank_rejects_partial_evidence_overlap`  ·  L609
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `assimilate_bank_merges_disjoint_evidence_and_adds_unmatched`  ·  L623
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `reconcile_full_corpus_decays_or_drops_unassigned_priors`  ·  L656
- **Ecuaciones / líneas clave:**
  - L656: `fn reconcile_full_corpus_decays_or_drops_unassigned_priors() {`
  - L658: `let f2_decaying = test_field("f2", vec![0.0, 1.0], &[b"ev2"], 0.5, 0.5);`
  - L671: `fields: vec![f1, f2_decaying, f3_dropping],`
  - L688: `// f1 is matched, f2 decayed (0.5 * 0.85 = 0.425 >= 0.15), f3 dropped (0.15 * 0.85 = 0.1275 < 0.15)`

### `src/analysis/weight_tomography.rs` (14 fns)

#### `clamp01`  ·  L115
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `weighted_nonnegative_mean`  ·  L126
- **Ecuaciones / líneas clave:**
  - L143: `let normalized = compensated_sum(`
  - L149: `let result = scale * normalized / weight_sum;`

#### `detrend_scale_normalized`  ·  L169
- **Ecuaciones / líneas clave:**
  - L169: `fn detrend_scale_normalized(values: &[f64]) -> BrainResult<(Vec<f64>, f64)> {`
  - L175: `.map(\|value\| value.abs())`
  - L180: `let normalized = values.iter().map(\|value\| value / scale).collect::<Vec<_>>();`
  - L181: `if normalized.len() < 2 {`
  - L182: `return Ok((normalized, 0.0));`
  - L184: `let n = normalized.len() as f64;`
  - L186: `let y_mean = stable_mean(&normalized)?;`
  - L187: `let numerator = compensated_sum(normalized.iter().enumerate().map(\|(index, value)\| {`
  - L191: `let denominator = compensated_sum((0..normalized.len()).map(\|index\| {`
  - L200: `let detrended = normalized`

#### `neumaier_add`  ·  L226
- **Ecuaciones / líneas clave:**
  - L231: `if sum.abs() >= value.abs() {`
  - L232: `*correction += (*sum - updated) + value;`
  - L234: `*correction += (value - updated) + *sum;`

#### `positive_dft`  ·  L245
- **Doc:** Bounded direct DFT.  History is capped at 128 samples, so avoiding another numerical dependency keeps the production surface small while bounding work.
- **Ecuaciones / líneas clave:**
  - L262: `re += re_correction;`
  - L263: `im += im_correction;`

#### `normalized_power`  ·  L272
- **Ecuaciones / líneas clave:**
  - L272: `fn normalized_power(values: &[f64]) -> BrainResult<(Vec<f64>, f64)> {`
  - L273: `let (detrended, slope) = detrend_scale_normalized(values)?;`

#### `normalized_spectral_entropy`  ·  L288
- **Ecuaciones / líneas clave:**
  - L288: `fn normalized_spectral_entropy(power: &[f64]) -> BrainResult<f64> {`
  - L299: `-probability * probability.ln()`
  - L302: `Ok(clamp01(entropy / (power.len() as f64).ln()))`

#### `positive_autocorrelation`  ·  L305
- **Ecuaciones / líneas clave:**
  - L311: `.map(\|value\| value.abs())`
  - L316: `let normalized = values.iter().map(\|value\| value / scale).collect::<Vec<_>>();`
  - L317: `let left = &normalized[..normalized.len() - lag];`
  - L318: `let right = &normalized[lag..];`
  - L326: `let left_energy = compensated_sum(left.iter().map(\|value\| (value - left_mean).powi(2)))?;`
  - L327: `let right_energy = compensated_sum(right.iter().map(\|value\| (value - right_mean).powi(2)))?;`
  - L331: `let correlation = numerator / (left_energy.sqrt() * right_energy.sqrt());`

#### `scaled_cosine`  ·  L354
- **Ecuaciones / líneas clave:**
  - L354: `fn scaled_cosine(left: &[f64], right: &[f64]) -> BrainResult<Option<f64>> {`
  - L361: `let left_scale = left.iter().map(\|value\| value.abs()).fold(0.0, f64::max);`
  - L362: `let right_scale = right.iter().map(\|value\| value.abs()).fold(0.0, f64::max);`
  - L371: `let left_energy = compensated_sum(left.iter().map(\|value\| (value / left_scale).powi(2)))?;`
  - L372: `let right_energy = compensated_sum(right.iter().map(\|value\| (value / right_scale).powi(2)))?;`
  - L376: `let value = numerator / (left_energy.sqrt() * right_energy.sqrt());`

#### `directional_consistency`  ·  L383
- **Ecuaciones / líneas clave:**
  - L386: `if let Some(cosine) = scaled_cosine(&pair[0], &pair[1])? {`
  - L387: `scores.push(cosine.max(0.0));`

#### `normalized_window`  ·  L397
- **Ecuaciones / líneas clave:**
  - L397: `fn normalized_window(values: &[f64]) -> BrainResult<Vec<f64>> {`
  - L398: `let (detrended, _) = detrend_scale_normalized(values)?;`

#### `subaperture_metrics`  ·  L402
- **Ecuaciones / líneas clave:**
  - L421: `let master = normalized_window(&channel[..window])?;`
  - L422: `let slave = normalized_window(&channel[length - window..])?;`
  - L430: `let weight = (master_power * slave_power).sqrt();`
  - L431: `cross_re += cross_r;`
  - L432: `cross_im += cross_i;`
  - L433: `master_energy += master_power;`
  - L434: `slave_energy += slave_power;`
  - L437: `phase_x += weight * phase.cos();`
  - L438: `phase_y += weight * phase.sin();`
  - L439: `phase_weight_sum += weight;`
  - L458: `clamp01(cross_magnitude / (master_energy.sqrt() * slave_energy.sqrt()))`

#### `analyze_weight_dynamics`  ·  L475
- **Ecuaciones / líneas clave:**
  - L478: `max_edge_residual: f64,`
  - L483: `\|\| !max_edge_residual.is_finite()`
  - L484: `\|\| max_edge_residual < 0.0`
  - L612: `let mut normalized_spectral_energy_sum = 0.0_f64;`
  - L618: `let (power, slope) = normalized_power(channel)?;`
  - L624: `let normalization = (channel.len() as f64).powi(2).max(1.0);`

#### `coordinate_sampling_is_bounded_and_uses_the_full_span`  ·  L986
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/analysis/block_tomography.rs` (5 fns)

#### `parameter_layout_digest`  ·  L204
- **Doc:** Hash the canonical semantic projection of a validated layout. Serde field order is fixed by the Rust structs, while offsets and totals are required to be contiguous and exact by `validate`; insignificant artifact whitespace is therefore outside this identity.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `reconstruct_block`  ·  L361
- **Ecuaciones / líneas clave:**
  - L393: `Ok::<f64, BrainError>(total + weights[row] * dot(data.row(row), data.row(row))?)`
  - L401: `selected_rank: 0,`
  - L402: `effective_rank: 0.0,`
  - L405: `normalized_block_energy: 0.0,`
  - L413: `let rank = choose_energy_rank(`
  - L416: `cfg.max_rank.min(support.len()),`
  - L420: `let retained_energy = eigenvalues.iter().take(rank).sum::<f64>() / total_energy;`
  - L422: `let mut basis = Vec::<Vec<f64>>::with_capacity(rank);`
  - L423: `let mut axes = Vec::with_capacity(rank);`
  - L424: `for (lambda, eigenvector) in eigs.iter().take(rank) {`
  - L425: `let denom = lambda.sqrt().max(1e-15);`
  - L429: `let local_coefficient = weights[local_row].sqrt() * eigenvector[local_row] / denom;`
  - L431: `axis[parameter] += local_coefficient * data.get(local_row, parameter);`
  - L438: `let axis_norm = norm(&axis)?;`
  - L439: `if axis_norm <= 1e-15 {`
  - L443: `*value /= axis_norm;`
  - L446: `*coefficient /= axis_norm;`
  - L450: `singular_value: lambda.sqrt(),`
  - L459: `let coefficient = dot(data.row(row), axis)?;`
  - L461: `reconstructed[parameter] += coefficient * axis[parameter];`
  - L465: `squared_error += (data.get(row, parameter) - reconstructed[parameter]).powi(2);`
  - L468: `let reconstruction_rms = (squared_error / (data.rows * data.cols).max(1) as f64).sqrt();`
  - L474: `selected_rank: rank,`
  - L475: `effective_rank: effective_rank_from_spectrum(&eigenvalues)?,`
  - L478: `normalized_block_energy: 0.0,`

#### `reconstruct_structured_geometry`  ·  L484
- **Ecuaciones / líneas clave:**
  - L511: `block.normalized_block_energy = block.block_energy / total_block_energy;`
  - L514: `let max_local_rank = blocks`
  - L516: `.map(\|block\| block.selected_rank)`
  - L521: `.filter(\|block\| block.selected_rank > 0)`
  - L523: `let mean_effective_rank = if active_blocks == 0 {`
  - L528: `.filter(\|block\| block.selected_rank > 0)`
  - L529: `.map(\|block\| block.effective_rank)`
  - L537: `max_local_rank,`
  - L538: `mean_effective_rank,`

#### `block_geometry_preserves_local_rank_and_source_coefficients`  ·  L658
- **Ecuaciones / líneas clave:**
  - L658: `fn block_geometry_preserves_local_rank_and_source_coefficients() {`
  - L711: `assert!(geometry.skills[0].max_local_rank >= 2);`

#### `layout_complexity_limits_fail_closed_before_geometry_work`  ·  L896
- **Ecuaciones / líneas clave:**
  - L901: `let mut excessive_rank = minimal_layout();`
  - L902: `excessive_rank.blocks[0].shape = vec![1; MAX_PARAMETER_BLOCK_RANK + 1];`
  - L903: `excessive_rank.blocks[0].count = 1;`
  - L904: `excessive_rank.blocks[1].offset = 1;`
  - L905: `excessive_rank.total_parameter_count = 3;`
  - L906: `assert!(excessive_rank.validate().is_err());`

### `src/analysis/protected_map.rs` (13 fns)

#### `persist_protected_map`  ·  L73
- **Ecuaciones / líneas clave:**
  - L81: `\|\| report.cortex.directions.len() != report.selected_rank`
  - L82: `\|\| !report.effective_rank.is_finite()`
  - L83: `\|\| report.effective_rank <= 0.0`
  - L107: `\|\| norm(&direction.direction)? == 0.0`
  - L124: `selected_rank: report.selected_rank,`
  - L125: `effective_rank: report.effective_rank,`

#### `load_protected_cortex`  ·  L138
- **Ecuaciones / líneas clave:**
  - L145: `\|\| report.selected_rank != report.cortex.directions.len()`
  - L167: `\|\| norm(&direction)? == 0.0`

#### `validate_evidence`  ·  L187
- **Ecuaciones / líneas clave:**
  - L197: `let sensitivity_norm = norm(&probe.sensitivity)?;`
  - L201: `\|\| sensitivity_norm == 0.0`

#### `pearson`  ·  L215
- **Ecuaciones / líneas clave:**
  - L225: `numerator += (l - lm) * (r - rm);`
  - L226: `ld += (l - lm).powi(2);`
  - L227: `rd += (r - rm).powi(2);`
  - L229: `let denom = (ld * rd).sqrt();`

#### `metric_dual_direction`  ·  L238
- **Doc:** Represent a sensitivity covector g in the metric used by protected.rs. Its projector enforces u^T D delta = 0, so u must be proportional to D^+ g, not g. Normalize in D so its constraint is not discarded solely because of the projector's absolute Gram tolerance. Zero metric entries are permitted only outside the covector support; no epsilon invents missing sensitivity.
- **Ecuaciones / líneas clave:**
  - L258: `whitened.push(gradient / importance.sqrt());`
  - L261: `let dual_norm = norm(&whitened)?;`
  - L262: `if dual_norm == 0.0 {`
  - L271: `(value / dual_norm) / importance.sqrt()`
  - L282: `metric_energy += energy;`
  - L286: `\|\| (metric_energy - 1.0).abs() > f64::EPSILON.sqrt() * covector.len() as f64`
  - L288: `return Err(BrainError::Numerical("protected_map_metric_dual_normalization".into()));`

#### `build_protected_cortex_map`  ·  L293
- **Ecuaciones / líneas clave:**
  - L320: `let selected_rank =`
  - L321: `choose_energy_rank(&eigenvalues, target_explained_sensitivity, eigenvalues.len(), 0)?;`
  - L327: `eigenvalues.iter().take(selected_rank).sum::<f64>() / total_energy;`
  - L333: `parameter_importance[parameter] += weight * probe.sensitivity[parameter].powi(2);`
  - L358: `let mut directions = Vec::with_capacity(selected_rank);`
  - L359: `for (component, (lambda, eigenvector)) in eigs.iter().take(selected_rank).enumerate() {`
  - L360: `let denom = lambda.sqrt();`
  - L366: `let coefficient = weights[row].sqrt() * eigenvector[row] / denom;`
  - L368: `direction[parameter] += coefficient * rows.get(row, parameter);`
  - L371: `let direction_norm = norm(&direction)?;`
  - L372: `if direction_norm == 0.0 {`
  - L376: `*value /= direction_norm;`
  - L382: `importance: (*lambda / eigenvalues[0]).clamp(0.0, 1.0),`
  - L393: `.map(\|(probe, _)\| Ok(dot(&probe.sensitivity, &probe.sensitivity)?.sqrt()))`
  - L404: `selected_rank,`
  - L405: `effective_rank: effective_rank_from_spectrum(&eigenvalues)?,`

#### `persistent_scatterer_cortex_protection_identifies_invariants`  ·  L471
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `protected_map_learns_load_bearing_subspace`  ·  L503
- **Ecuaciones / líneas clave:**
  - L525: `assert!(map.selected_rank >= 1);`
  - L530: `assert!(norm(&damaging.projected).unwrap() < norm(&orthogonal.projected).unwrap());`

#### `protected_map_rejects_zero_reliability_instead_of_fabricating_a_weight`  ·  L534
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `protected_map_preserves_probe_responses_in_anisotropic_metric`  ·  L577
- **Ecuaciones / líneas clave:**
  - L595: `assert_eq!(map.selected_rank, 1);`
  - L603: `assert!(dot(&probe.sensitivity, &result.projected).unwrap().abs() < 1e-12);`
  - L606: `assert!(result.projected[0].abs() > 0.1);`
  - L607: `assert!(result.projected[1].abs() > 0.1);`
  - L614: `assert!(wrong.max_weighted_residual < 1e-12);`
  - L616: `dot(&evidence[0].sensitivity, &wrong.projected)`
  - L618: `.abs()`

#### `protected_map_preserves_all_independent_retained_covectors`  ·  L624
- **Ecuaciones / líneas clave:**
  - L640: `assert_eq!(map.selected_rank, 2);`
  - L648: `assert!(dot(&probe.sensitivity, &result.projected).unwrap().abs() < 1e-10);`

#### `protected_map_rejects_metric_underflow_that_loses_sensitivity`  ·  L655
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `protected_map_roundtrip_keeps_covector_contract_and_rejects_legacy_schema`  ·  L682
- **Ecuaciones / líneas clave:**
  - L718: `dot(&evidence[0].sensitivity, &result.projected)`
  - L720: `.abs()`

### `src/analysis/protected.rs` (2 fns)

#### `wdot`  ·  L16
- **Ecuaciones / líneas clave:**
  - L16: `fn wdot(a: &[f64], b: &[f64], weights: &[f64]) -> BrainResult<f64> {`
  - L25: `return Err(BrainError::Invalid("protected_weighted_dot_input".into()));`
  - L34: `return Err(BrainError::Numerical("protected_weighted_dot_non_finite".into()));`

#### `project_to_safe_subspace`  ·  L39
- **Ecuaciones / líneas clave:**
  - L67: `.map(\|(value, importance)\| value / (1.0 + importance))`
  - L75: `let mut protected_rank = 0usize;`
  - L76: `let mut max_weighted_residual = 0.0f64;`
  - L79: `if wdot(&direction.direction, &direction.direction, &cortex.parameter_importance)?`
  - L92: `wdot(&active[i].direction, &active[j].direction, &cortex.parameter_importance)?;`
  - L100: `.map(\|(value, _)\| value.abs())`
  - L103: `let tolerance = f64::EPSILON.sqrt() * scale * m.max(1) as f64;`
  - L112: `protected_rank += 1;`
  - L121: `if protected_rank == 0 {`
  - L128: `.map(\|direction\| wdot(&braked, &direction.direction, &cortex.parameter_importance))`
  - L137: `let reference_energy = wdot(delta, delta, &cortex.parameter_importance)?.sqrt();`
  - L140: `wdot(&direction.direction, &direction.direction, &cortex.parameter_importance)?`
  - L141: `.sqrt();`
  - L143: `wdot(&braked, &direction.direction, &cortex.parameter_importance)?.abs();`
  - L144: `let residual = if reference_energy == 0.0 {`
  - L147: `numerator / (reference_energy * direction_energy)`
  - L149: `max_weighted_residual = max_weighted_residual.max(residual);`
  - L151: `let residual_tolerance = f64::EPSILON.sqrt() * (m.max(1) as f64).sqrt() * 16.0;`
  - L152: `if max_weighted_residual > residual_tolerance {`
  - L154: `"protected_joint_projection_residual:{max_weighted_residual:.6e}"`
  - L160: `let removed_norm = norm(&removed_vector)?;`
  - L161: `let removed = removed_norm * removed_norm;`
  - L165: `let delta_norm = norm(delta)?;`
  - L166: `let damage = if delta_norm == 0.0 {`
  - L169: `removed_norm / delta_norm`

### `src/analysis/persistent.rs` (16 fns)

#### `union`  ·  L64
- **Ecuaciones / líneas clave:**
  - L70: `if self.rank[ra] < self.rank[rb] {`
  - L74: `if self.rank[ra] == self.rank[rb] {`
  - L75: `self.rank[ra] = self.rank[ra].saturating_add(1);`

#### `normalized_rows`  ·  L80
- **Ecuaciones / líneas clave:**
  - L80: `fn normalized_rows(d: &Matrix) -> BrainResult<Vec<Vec<f64>>> {`
  - L87: `let n = norm(row)?;`

#### `components_at_threshold`  ·  L96
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `clustering_silhouette`  ·  L119
- **Ecuaciones / líneas clave:**
  - L139: `.map(\|other\| 1.0 - scores.get(row, other))`
  - L141: `/ (own.len() - 1) as f64;`
  - L149: `.map(\|other\| 1.0 - scores.get(row, *other))`
  - L155: `total += (nearest_other - intra) / denom;`

#### `recurrence_metrics`  ·  L160
- **Ecuaciones / líneas clave:**
  - L169: `singletons += 1;`
  - L176: `recurrent_members += component.len();`
  - L178: `weighted_persistence += component.len() as f64 * seen.len() as f64 / group_count as f64;`

#### `better_cluster_candidate`  ·  L189
- **Ecuaciones / líneas clave:**
  - L198: `if (candidate.0 - current.0).abs() > EPS {`
  - L201: `if (candidate.1 - current.1).abs() > EPS {`
  - L204: `if (candidate.2 - current.2).abs() > EPS {`
  - L210: `if (candidate.4 - current.4).abs() > EPS {`

#### `centroid_for_members`  ·  L310
- **Ecuaciones / líneas clave:**
  - L323: `let sign = if cosine(reference, &rows[index])? >= 0.0 {`
  - L329: `total_weight += weight;`
  - L331: `pre[p] += weight * sign * rows[index][p];`
  - L340: `let pre_norm = norm(&pre)?;`
  - L341: `if !pre_norm.is_finite() \|\| pre_norm <= 1e-15 {`
  - L344: `Ok(pre.iter().map(\|value\| value / pre_norm).collect())`

#### `functional_dimension`  ·  L347
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `weighted_functional_signature`  ·  L366
- **Ecuaciones / líneas clave:**
  - L380: `let sign = if cosine(reference, &rows[index])? >= 0.0 {`
  - L385: `total += w;`
  - L387: `signature[out] += w * sign * observations[index].functional_response[out];`

#### `assignment_min_margin`  ·  L410
- **Ecuaciones / líneas clave:**
  - L423: `let matched = cosine(&reference[reference_index], &candidate[matched_candidate])?.abs();`
  - L430: `.max(cosine(&reference[reference_index], &candidate[candidate_index])?.abs());`

#### `cross_aperture_functional_cv`  ·  L440
- **Ecuaciones / líneas clave:**
  - L488: `let similarity = cosine(&rows[index], centroid)?;`
  - L489: `if best.is_none_or(\|(_, current)\| similarity.abs() > current.abs()) {`
  - L496: `min_holdout_similarity = min_holdout_similarity.min(similarity.abs());`
  - L523: `sse += error * error;`
  - L525: `sst += centered * centered;`
  - L528: `let cv_r2 = if sst <= 1e-18 { 0.0 } else { 1.0 - sse / sst };`
  - L552: `parametric_cluster_stability = parametric_cluster_stability.min(parametric.mean_abs_cosine);`
  - L553: `functional_cluster_stability = functional_cluster_stability.min(functional.mean_abs_cosine);`

#### `effective_rank_from_energies`  ·  L578
- **Ecuaciones / líneas clave:**
  - L578: `fn effective_rank_from_energies(energies: &[f64]) -> f64 {`
  - L584: `(probability > 1e-15).then_some(-probability * probability.ln())`
  - L587: `entropy.exp()`

#### `reconstruct_persistent_skill_fields`  ·  L590
- **Ecuaciones / líneas clave:**
  - L614: `let rows = normalized_rows(d)?;`
  - L633: `Ok::<f64, BrainError>(total + reliability[row] * dot(d.row(row), d.row(row))?)`
  - L658: `let sign = if cosine(reference, &rows[member])? >= 0.0 {`
  - L667: `Ok::<f64, BrainError>(total + reliability[row] * dot(d.row(row), d.row(row))?)`
  - L675: `Ok::<f64, BrainError>(total + reliability[row] * cosine(&rows[row], &direction)?.abs())`
  - L678: `let residual = 1.0 - cosine(&rows[row], &direction)?.abs().clamp(0.0, 1.0);`
  - L679: `Ok::<f64, BrainError>(total + reliability[row] * residual * residual)`
  - L697: `singular_value: cluster_energy.sqrt(),`
  - L701: `uncertainty: angular_variance.sqrt() / (members.len() as f64).sqrt().max(1.0),`
  - L717: `let value = dot(&directions[left], &directions[right])?;`
  - L731: `return Err(BrainError::Numerical("persistent_field_geometry_rank_deficient".into()));`
  - L739: `return Err(BrainError::Numerical("persistent_field_geometry_rank_deficient".into()));`

#### `structural_threshold_selection_avoids_largest_gap_fragmentation`  ·  L836
- **Ecuaciones / líneas clave:**
  - L837: `let g = 0.3_f64.sqrt();`
  - L838: `let skill = 0.2_f64.sqrt();`
  - L839: `let pair = 0.4_f64.sqrt();`
  - L840: `let unique_pair = 0.1_f64.sqrt();`
  - L841: `let unique_single = 0.5_f64.sqrt();`

#### `persistent_inverse_discovers_recurrent_skills_without_task_labels`  ·  L867
- **Ecuaciones / líneas clave:**
  - L868: `let skill_a = normalize(&[1.0, 0.2, 0.0, 0.0, 0.0, 0.0]).unwrap();`
  - L869: `let skill_b = normalize(&[0.0, 0.0, 1.0, 0.2, 0.0, 0.0]).unwrap();`
  - L870: `let skill_c = normalize(&[0.0, 0.0, 0.0, 0.0, 1.0, 0.2]).unwrap();`
  - L881: `*value += 0.03`
  - L882: `* (((aperture + 1) * (parameter + 2) * (index + 3)) as f64 * 0.37).sin();`
  - L885: `index += 1;`
  - L896: `assert_eq!(result.selected_rank, 3);`
  - L903: `let nonzero = mixture.iter().filter(\|value\| value.abs() > 1e-12).count();`
  - L905: `assert!((mixture.iter().map(\|value\| value.abs()).sum::<f64>() - 1.0).abs() < 1e-12);`
  - L929: `assert!(cosine(&left.direction, &right.direction).unwrap().abs() > 1.0 - 1e-12);`

#### `persistent_inverse_solves_nonorthogonal_field_coefficients_jointly`  ·  L938
- **Ecuaciones / líneas clave:**
  - L940: `let second = vec![0.5, 0.75_f64.sqrt()];`
  - L945: `index += 1;`
  - L947: `index += 1;`
  - L957: `assert_eq!(result.selected_rank, 2);`
  - L959: `assert!((result.condition_estimate - 3.0).abs() < 1e-10);`
  - L965: `.filter(\|coefficient\| coefficient.abs() > 1e-10)`

### `src/analysis/pythagoras_topology.rs` (9 fns)

#### `evaluate_and_correct`  ·  L58
- **Doc:** Correct a discrete step update `delta_w` to ensure trust region calculations reflect true geodesic manifold distance rather than Manhattan step inflation.
- **Ecuaciones / líneas clave:**
  - L65: `let l1_sum = crate::foundation::linalg::compensated_sum(delta_w.iter().map(\|v\| v.abs()))?;`
  - L68: `let l2_norm = norm(delta_w)?;`
  - L71: `if l2_norm <= f64::EPSILON * (dim as f64).sqrt() \|\| l1_sum <= f64::EPSILON * dim as f64 {`
  - L76: `geodesic_l2_length: l2_norm,`
  - L78: `corrected_geodesic_norm: 0.0,`
  - L84: `if !l1_sum.is_finite() \|\| !l2_norm.is_finite() {`
  - L85: `return Err(BrainError::Numerical("pythagoras_norm_overflow".into()));`
  - L88: `let inflation_ratio = l1_sum / l2_norm;`
  - L90: `let correction_factor = l2_norm / l1_sum;`
  - L91: `let corrected_norm = l2_norm; // Corrected norm IS the L2 norm`
  - L101: `geodesic_l2_length: l2_norm,`
  - L103: `corrected_geodesic_norm: corrected_norm,`

#### `project_to_geodesic`  ·  L109
- **Doc:** Rescale a discrete weight shift to match the true geodesic constraint.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `analyze_topology`  ·  L146
- **Doc:** Analyze the persistent topology of a set of skill representation vectors.
- **Ecuaciones / líneas clave:**
  - L192: `let dist = sum_sq.sqrt();`
  - L195: `j += 1;`
  - L197: `i += 1;`

#### `find`  ·  L202
- **Ecuaciones / líneas clave:**
  - L227: `edge_count += 1;`
  - L229: `j += 1;`
  - L231: `i += 1;`
  - L252: `let homotopy_score = (1.0 / (betti_0 as f64)).clamp(0.0, 1.0);`
  - L261: `j += 1;`
  - L263: `i += 1;`
  - L276: `k += 1;`
  - L278: `j += 1;`
  - L280: `i += 1;`

#### `process_range_doppler`  ·  L370
- **Doc:** Decompose a weight matrix into Doppler subapertures to focus multi-layer drift. This upgraded version uses true sub-pixel phase correlation (from temporal_tracking) to accurately track range migration across matrix rows, achieving rigorous integration between structural topology and temporal coherence systems.
- **Ecuaciones / líneas clave:**
  - L394: `sub_energies[sub_idx] += val * val;`
  - L427: `let shift = corr.estimated_shift.iter().sum::<f64>() / (cols as f64);`
  - L428: `cumulative_shift += shift;`
  - L431: `coherence_sum += corr.peak_magnitude;`
  - L432: `coherence_count += 1;`
  - L440: `coherence_sum / (coherence_count as f64)`

#### `separate_carrier_and_detail`  ·  L467
- **Doc:** Decomposes a weight matrix into its low-rank universal carrier component (capturing the dominant spectral structure transferable across architectures) and the high-frequency detail residual.  The carrier is computed as a truncated SVD low-rank approximation: W ≈ U_k Σ_k V_k^T where k is the smallest rank capturing ≥ 80% of the Frobenius energy. The detail is the exact residual: W - carrier.  This is a genuine spectral decomposition: the carrier spans the dominant singular subspace and the detail
- **Ecuaciones / líneas clave:**
  - L501: `let total_energy: f64 = eigen_sorted.iter().map(\|(val, _)\| val.abs()).sum();`
  - L507: `.filter(\|(val, _)\| val.abs() > relative_threshold)`
  - L518: `// Choose rank k: smallest k such that sum(σ²_1..k) >= 0.80 * retained energy`
  - L521: `let mut rank = 0;`
  - L523: `cumulative += val;`
  - L524: `rank += 1;`
  - L529: `rank = rank.max(1).min(eigen_positive.len());`
  - L540: `for (_, v_j) in eigen_positive.iter().take(rank) {`
  - L541: `let projection = dot(w_row, v_j)?;`
  - L542: `val += projection * v_j[c];`
  - L548: `// Detail = W - carrier (exact residual)`

#### `pythagoras_staircase_corrects_high_dim_step_inflation`  ·  L565
- **Ecuaciones / líneas clave:**
  - L572: `assert!((report.geodesic_l2_length - 10.0).abs() < 1e-10);`
  - L574: `assert!((report.staircase_inflation_ratio - 10.0).abs() < 1e-10);`
  - L577: `let corrected_l2 = norm(&corrected).unwrap();`
  - L578: `assert!((corrected_l2 - 1.0).abs() < 1e-10);`

#### `sar_doppler_subapertures_decomposes_weight_matrix`  ·  L597
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `sar_carrier_and_detail_reconstructs_original`  ·  L613
- **Ecuaciones / líneas clave:**
  - L630: `(sum - mat.get(r, c)).abs() < 1e-12,`

### `src/analysis/trust_region.rs` (11 fns)

#### `validate_quadratic_metric`  ·  L54
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `quadratic_cost`  ·  L59
- **Ecuaciones / líneas clave:**
  - L68: `let cost = dot(coefficients, &product)?;`
  - L69: `if cost < -f64::EPSILON.sqrt() {`

#### `apply_quadratic_trust_region`  ·  L75
- **Ecuaciones / líneas clave:**
  - L89: `(max_quadratic_cost / proposed).sqrt().clamp(0.0, 1.0)`

#### `apply_pythagoras_geodesic_trust_region`  ·  L112
- **Doc:** Applies a trust region constraint that eliminates the Pythagoras Staircase metric inflation in discrete high-dimensional parameter updates.
- **Ecuaciones / líneas clave:**
  - L141: `(max_quadratic_cost / proposed).sqrt().clamp(0.0, 1.0)`
  - L168: `constrained: scale < 1.0 \|\| (pythagoras_report.metric_correction_factor - 1.0).abs() > 1e-9,`
  - L169: `allocation_policy: if (pythagoras_report.metric_correction_factor - 1.0).abs() < 1e-9 {`

#### `signed_magnitude_metric`  ·  L182
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `causal_priority_candidate`  ·  L209
- **Ecuaciones / líneas clave:**
  - L215: `let tolerance = f64::EPSILON.sqrt()`
  - L217: `+ magnitudes.iter().map(\|value\| value.abs()).sum::<f64>()`
  - L218: `+ gradient.iter().map(\|value\| value.abs()).sum::<f64>());`
  - L226: `let ratio = priorities[index] / (2.0 * gradient[index]);`
  - L232: `\|\| ((ratio - best_ratio).abs() <= tolerance && index < best_index)`

#### `apply_causal_priority_trust_region`  ·  L247
- **Doc:** Contract a proposed composition to a verified quadratic budget while preserving the fields with the largest *lower-confidence* causal benefit. The routine is intentionally not a generic optimiser: it executes the deterministic authority rule used by TIDE-X, reducing the field with the least causal utility per marginal curvature relief until the certified budget is met.  If causal evidence cannot determine a lawful contraction, it fails closed rather than reverting to uniform scaling.
- **Ecuaciones / líneas clave:**
  - L266: `let tolerance = f64::EPSILON.sqrt() * (1.0 + proposed.abs() + max_quadratic_cost.abs());`
  - L270: `.map(\|value\| value.abs())`

#### `trust_region_rejects_indefinite_metric`  ·  L370
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `trust_region_scales_exactly_to_quadratic_budget`  ·  L378
- **Ecuaciones / líneas clave:**
  - L384: `assert!((result.proposed_quadratic_cost - 5.0).abs() < 1e-12);`
  - L385: `assert!((result.accepted_quadratic_cost - 1.25).abs() < 1e-10);`
  - L386: `assert!((result.scale - 0.5).abs() < 1e-12);`

#### `causal_priority_preserves_high_value_field_before_uniform_scaling`  ·  L396
- **Ecuaciones / líneas clave:**
  - L407: `assert!((result.accepted_coefficients[0] - 1.0).abs() < 1e-10);`
  - L408: `assert!((result.accepted_coefficients[1] - 0.5).abs() < 1e-10);`
  - L409: `assert!((result.accepted_quadratic_cost - 2.0).abs() < 1e-9);`

#### `pythagoras_geodesic_trust_region_corrects_step_inflation`  ·  L423
- **Ecuaciones / líneas clave:**
  - L437: `assert!((result.accepted_quadratic_cost - 1.0).abs() < 1e-10);`

### `src/analysis/sbas.rs` (3 fns)

#### `reconstruct_trajectory`  ·  L16
- **Ecuaciones / líneas clave:**
  - L18: `ridge: f64,`
  - L23: `if !ridge.is_finite() \|\| ridge <= 0.0 {`
  - L24: `return Err(BrainError::Invalid("sbas_ridge_invalid".into()));`
  - L102: `lap.data[a * (n - 1) + a] += w;`
  - L108: `lap.data[b * (n - 1) + b] += w;`
  - L110: `rhs[b][p] += w * o.delta[p];`
  - L119: `lap.data[i * (n - 1) + i] += ridge;`
  - L129: `let mut edge_residual_norms = Vec::new();`
  - L137: `let residual = sub(&predicted, &o.delta)?;`
  - L138: `let r = norm(&residual)? / (norm(&o.delta)? + 1e-12);`
  - L140: `sq += w * r * r;`
  - L141: `weight_sum += w;`
  - L143: `edge_residual_norms.push(r);`
  - L148: `edge_residual_norms,`
  - L149: `cycle_rms: (sq / weight_sum.max(1e-12)).sqrt(),`
  - L150: `max_edge_residual: maxr,`

#### `digest`  ·  L159
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `parallel_base_to_variant_edges_have_near_zero_cycle_residual`  ·  L196
- **Ecuaciones / líneas clave:**
  - L196: `fn parallel_base_to_variant_edges_have_near_zero_cycle_residual() {`
  - L204: `assert!(result.max_edge_residual < 1e-4, "max={}", result.max_edge_residual);`

### `src/analysis/temporal_tracking.rs` (7 fns)

#### `phase_correlation`  ·  L56
- **Doc:** Compute normalised phase correlation between two weight vectors.  Analogous to InSAR interferogram formation:  ```text Z_k = a_k · conj(b_k) / |a_k · conj(b_k)| phase_k = atan2(Im(Z_k), Re(Z_k)) ```  Since neural weights are real-valued, we treat each element pair as a unit-amplitude phasor whose "phase" is the arctangent ratio of the difference to the mean, giving sub-element sensitivity to distributed perturbations that L2 norm would miss.
- **Ecuaciones / líneas clave:**
  - L65: `let mut phase_residuals = Vec::with_capacity(n);`
  - L83: `let phase = diff.atan2(mean.abs() + 1e-15);`
  - L84: `phase_residuals.push(phase);`
  - L88: `cross_real_sum += phase.cos();`
  - L89: `cross_imag_sum += phase.sin();`
  - L91: `if phase.abs() > PI * 0.25 {`
  - L92: `incoherent_count += 1;`
  - L99: `let peak_magnitude = (peak_real * peak_real + peak_imag * peak_imag).sqrt();`
  - L101: `let phase_rms = stable_rms(phase_residuals.iter().copied())?;`
  - L108: `phase_residuals,`
  - L137: `pub residual_rms: f64,`
  - L149: `///   Solve via SVD:           Ax = δ  →  x = A† δ`

#### `sbas_inversion`  ·  L155
- **Doc:** Reconstruct a time-series of parameter drift from pairwise differentials.  This is the SBAS (Small BAseline Subset) algorithm adapted from InSAR:  ```text For each pair (a, b):   δ_i = u(t_b) - u(t_a) Design matrix A:        A[i, epoch_b] = +1, A[i, epoch_a] = -1 Solve via SVD:           Ax = δ  →  x = A† δ ```  The result `x` is the cumulative displacement at each epoch. Regularisation via Tikhonov ensures stability when the graph is not fully connected.
- **Ecuaciones / líneas clave:**
  - L203: `ata[cb * unknowns + cb] += 1.0;`
  - L204: `atb[cb] += pair.differential;`
  - L207: `ata[ca * unknowns + ca] += 1.0;`
  - L218: `ata[i * unknowns + i] += regularisation;`
  - L225: `let d = ata[i * unknowns + i].abs();`
  - L247: `// Velocity: linear fit  u(t) = v·t + c  →  v = (n·Σ(t·u) - Σt·Σu) / (n·Σt² - (Σt)²)`
  - L250: `let sum_t2: f64 = (0..num_epochs).map(\|i\| (i as f64).powi(2)).sum();`
  - L258: `let velocity = if denom.abs() > 1e-15 {`
  - L263: `let intercept = if denom.abs() > 1e-15 {`
  - L270: `let residuals: Vec<f64> = cumulative`
  - L275: `let residual_rms = stable_rms(residuals)?;`
  - L281: `residual_rms,`

#### `cholesky_solve`  ·  L287
- **Doc:** Cholesky LLT solve for SPD system.
- **Ecuaciones / líneas clave:**
  - L293: `sum += l[j * n + k] * l[j * n + k];`
  - L299: `l[j * n + j] = diag.sqrt();`
  - L304: `s += l[i * n + k] * l[j * n + k];`
  - L315: `s += l[i * n + k] * y[k];`
  - L325: `s += l[k * n + i] * x[k];`

#### `identify_persistent_scatterers`  ·  L389
- **Doc:** Identify persistent scatterers — parameters that remain structurally invariant across multiple learning epochs.  Adapted from PS-InSAR: for each parameter, we compute:  1. **Amplitude dispersion** $D_A = \sigma_A / \bar{A}$ — ratio of temporal standard deviation to mean amplitude. PS candidates have $D_A < 0.25$.  2. **Temporal coherence** $\gamma_t = |\frac{1}{N}\sum_k e^{j\phi_k}|$ — the mean phasor over all differential phase measurements. High coherence ($\gamma_t > 0.85$) means the paramete
- **Ecuaciones / líneas clave:**
  - L427: `let variance = values.iter().map(\|v\| (v - mean).powi(2)).sum::<f64>() / nf;`
  - L428: `let std_dev = variance.sqrt();`
  - L431: `let amplitude_dispersion = if mean.abs() > 1e-15 {`
  - L432: `std_dev / mean.abs()`
  - L443: `let phase = diff.atan2(mean.abs() + 1e-15);`
  - L444: `cos_sum += phase.cos();`
  - L445: `sin_sum += phase.sin();`
  - L448: `let temporal_coherence = ((cos_sum / npf).powi(2) + (sin_sum / npf).powi(2)).sqrt();`
  - L454: `invariant_count += 1;`
  - L459: `quasi_stable_count += 1;`
  - L462: `unstable_count += 1;`

#### `sbas_recovers_linear_drift`  ·  L532
- **Ecuaciones / líneas clave:**
  - L572: `(result.cumulative[i] - (i as f64 * 2.0)).abs() < 0.01,`
  - L579: `assert!((result.velocity - 2.0).abs() < 0.01);`
  - L580: `assert!(result.residual_rms < 0.01);`

#### `sbas_rejects_insufficient_epochs`  ·  L584
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `persistent_scatterers_identify_stable_parameters`  ·  L589
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/analysis/identifiability.rs` (7 fns)

#### `gram_of_fields`  ·  L37
- **Ecuaciones / líneas clave:**
  - L43: `let norm_tolerance = f64::EPSILON.sqrt() * (dim.max(1) as f64).sqrt() * 16.0;`
  - L52: `\|\| (norm(&field.direction)? - 1.0).abs() > norm_tolerance`
  - L60: `let value = crate::foundation::linalg::dot(&fields[i].direction, &fields[j].direction)?;`

#### `spectrum_and_rank`  ·  L93
- **Ecuaciones / líneas clave:**
  - L93: `fn spectrum_and_rank(matrix: &Matrix) -> BrainResult<(Vec<f64>, usize, f64)> {`
  - L97: `.map(\|(eigenvalue, _)\| eigenvalue.max(0.0).sqrt())`
  - L105: `let tolerance = largest * f64::EPSILON.sqrt() * matrix.rows.max(1) as f64;`
  - L106: `let rank = spectrum.iter().filter(\|value\| **value > tolerance).count();`
  - L112: `let condition = if rank == 0 \|\| !smallest_resolved.is_finite() {`
  - L117: `Ok((spectrum, rank, condition))`

#### `principal_angle_min`  ·  L120
- **Ecuaciones / líneas clave:**
  - L127: `let c = cosine(&fields[i].direction, &fields[j].direction)?`
  - L128: `.abs()`

#### `resolution_map`  ·  L136
- **Ecuaciones / líneas clave:**
  - L141: `ridge: f64,`
  - L148: `\|\| !ridge.is_finite()`
  - L149: `\|\| ridge <= 0.0`
  - L157: `let (geometry_spectrum, geometry_rank, geometry_condition) = spectrum_and_rank(&geometry)?;`
  - L158: `let (excitation_spectrum, excitation_rank, excitation_condition) =`
  - L159: `spectrum_and_rank(&excitation)?;`
  - L160: `let resolved_rank = geometry_rank.min(excitation_rank);`
  - L162: `let covariance = inverse_with_ridge(&excitation, ridge)?;`
  - L163: `let noise_variance = reconstruction_rms.powi(2).max(f64::EPSILON);`
  - L168: `.map(\|row\| observation_weights[row] * coefficients.get(row, field_index).powi(2))`
  - L177: `let coefficient_rms = (weighted_squared_coefficient / total_observation_weight).sqrt();`
  - L179: `let covariance_tolerance = f64::EPSILON.sqrt()`
  - L183: `.map(\|value\| value.abs())`
  - L191: `let posterior_std = (covariance_diagonal.max(0.0) * noise_variance).sqrt();`
  - L196: `let resolved = resolved_rank == fields.len()`
  - L216: `field_geometry_numerical_rank: geometry_rank,`
  - L217: `excitation_numerical_rank: excitation_rank,`
  - L218: `resolved_rank,`

#### `resolution_map_accepts_independent_excited_fields`  ·  L259
- **Ecuaciones / líneas clave:**
  - L269: `assert_eq!(map.resolved_rank, 2);`

#### `resolution_map_marks_collinear_fields_unresolved`  ·  L275
- **Ecuaciones / líneas clave:**
  - L285: `assert!(map.resolved_rank < 2);`

#### `resolution_map_rejects_zero_weight_and_noncanonical_field_geometry`  ·  L291
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/analysis/aperture_independence.rs` (7 fns)

#### `confounder_names`  ·  L32
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `group_design_profiles`  ·  L41
- **Ecuaciones / líneas clave:**
  - L79: `present += 1;`
  - L106: `.map(\|(value, weight)\| weight * (value - mean).powi(2))`

#### `standardized_profiles`  ·  L117
- **Ecuaciones / líneas clave:**
  - L138: `.map(\|row\| (row[col] - means[col]).powi(2))`
  - L141: `stds[col] = variance.sqrt();`
  - L144: `.map(\|row\| row[col].abs())`
  - L147: `if stds[col] > f64::EPSILON.sqrt() * scale {`

#### `design_numerical_rank`  ·  L163
- **Ecuaciones / líneas clave:**
  - L163: `fn design_numerical_rank(profiles: &[Vec<f64>]) -> BrainResult<usize> {`
  - L175: `let tolerance = largest * f64::EPSILON.sqrt() * gram.rows.max(1) as f64;`

#### `valid_digest`  ·  L179
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `estimate_aperture_independence`  ·  L241
- **Ecuaciones / líneas clave:**
  - L252: `let numerical_design_rank = design_numerical_rank(&profiles)?;`
  - L315: `let effective_group_rank = if total <= 1e-18 \|\| square_sum <= 1e-18 {`
  - L332: `effective_group_rank,`
  - L334: `numerical_design_rank,`

#### `many_well_separated_designs_exceed_three_effective_apertures`  ·  L397
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/analysis/dual_space.rs` (8 fns)

#### `fit_output`  ·  L156
- **Ecuaciones / líneas clave:**
  - L161: `ridge: f64,`
  - L179: `betas.push(weighted_normal_solve(&x, &target, &weights, ridge)?);`

#### `fit_representation_map`  ·  L191
- **Ecuaciones / líneas clave:**
  - L195: `ridge: f64,`
  - L207: `\|\| !ridge.is_finite()`
  - L208: `\|\| ridge <= 0.0`
  - L220: `fit_output(coefficients, representation_rows, &fold.train, observations, ridge)?;`
  - L227: `let betas = fit_output(coefficients, representation_rows, &all, observations, ridge)?;`
  - L238: `mean_matched_cosine: 0.0,`

#### `representation_centroid`  ·  L244
- **Ecuaciones / líneas clave:**
  - L267: `total += weight;`
  - L269: `centroid[dimension] += weight * sign * rows[index][dimension];`
  - L278: `normalize(&centroid).map_err(\|error\| match error {`

#### `expected_field_for_observation`  ·  L286
- **Ecuaciones / líneas clave:**
  - L290: `let mut ranked = Vec::with_capacity(model.fields.len());`
  - L293: `ranked.push((field_index, coefficient.abs(), coefficient.signum()));`
  - L295: `ranked.sort_by(\|left, right\| right.1.total_cmp(&left.1).then(left.0.cmp(&right.0)));`
  - L296: `let (field_index, magnitude, sign) = ranked`
  - L303: `let ambiguity_tolerance = magnitude * f64::EPSILON.sqrt();`
  - L304: `if ranked`

#### `representation_recurrence`  ·  L315
- **Ecuaciones / líneas clave:**
  - L327: `let mut cosine_sum = 0.0;`
  - L345: `let normalized = normalize(&aligned).map_err(\|error\| match error {`
  - L353: `.map(\|centroid\| crate::foundation::linalg::cosine(&normalized, centroid))`
  - L371: `cosine_sum += expected_similarity;`
  - L372: `correct += usize::from(predicted == expected);`
  - L373: `evaluated += 1;`
  - L384: `cosine_sum / evaluated as f64,`

#### `analyze_dual_space`  ·  L390
- **Ecuaciones / líneas clave:**
  - L398: `\|\| !config.ridge.is_finite()`
  - L399: `\|\| config.ridge < 0.0`
  - L416: `config.ridge,`
  - L419: `let (match_accuracy, mean_matched_cosine, min_match_margin_observed, recurrence_centroids) =`
  - L436: `.map(\|value\| value.abs())`
  - L438: `let tolerance = max_abs * f64::EPSILON.sqrt();`
  - L442: `.filter(\|(_, coefficient)\| coefficient.abs() > tolerance)`
  - L464: `representation_mean_matched_cosine: mean_matched_cosine,`

#### `representation_map_recovers_cross_aperture_linear_signatures`  ·  L538
- **Ecuaciones / líneas clave:**
  - L558: `assert!((fit.field_signatures[0][0] - 2.0).abs() < 1e-6);`
  - L559: `assert!((fit.field_signatures[1][1] - 3.0).abs() < 1e-6);`

#### `directional_matching_cannot_hide_failed_cross_aperture_generalization`  ·  L568
- **Ecuaciones / líneas clave:**
  - L609: `ridge: 1e-9,`

### `src/analysis/functional.rs` (1 fns)

#### `fit_functional_map`  ·  L26
- **Ecuaciones / líneas clave:**
  - L29: `ridge: f64,`
  - L38: `if !ridge.is_finite() \|\| ridge <= 0.0 {`
  - L39: `return Err(BrainError::Invalid("functional_ridge_invalid".into()));`
  - L83: `let beta = weighted_normal_solve(&x, &y, &weights, ridge)?;`
  - L91: `sse += (actual - pred).powi(2);`
  - L92: `sst += (actual - mean).powi(2);`
  - L96: `let cv_r2 = if sst <= 1e-18 { 0.0 } else { 1.0 - sse / sst };`
  - L109: `let beta = weighted_normal_solve(&x, &y, &weights, ridge)?;`

### `src/analysis/gauge.rs` (1 fns)

#### `align_bases`  ·  L14
- **Ecuaciones / líneas clave:**
  - L26: `cost[i][j] = 1.0 - cosine(&reference[i], &candidate[j])?.abs().clamp(0.0, 1.0);`
  - L58: `u[p[j]] += delta;`
  - L90: `let c = cosine(&reference[i], &candidate[j])?;`
  - L92: `sum += c.abs();`
  - L93: `count += 1;`
  - L99: `mean_abs_cosine: sum / count.max(1) as f64,`

### `src/analysis/confounders.rs` (2 fns)

#### `raw_design`  ·  L18
- **Ecuaciones / líneas clave:**
  - L66: `.map(\|row\| (x.get(row, column) - mean).powi(2))`
  - L69: `let std = variance.sqrt();`

#### `remove_confounders`  ·  L83
- **Ecuaciones / líneas clave:**
  - L85: `ridge: f64,`
  - L90: `if !ridge.is_finite() \|\| ridge <= 0.0 {`
  - L91: `return Err(BrainError::Invalid("confounder_ridge_invalid".into()));`
  - L117: `residuals: d,`
  - L130: `let mut normal = Matrix::zeros(q, q);`
  - L134: `normal.data[i * q + j] += weights[r] * x.get(r, i) * x.get(r, j);`
  - L138: `let inv = inverse_with_ridge(&normal, ridge)?;`
  - L145: `projection += x.get(r, i) * inv.get(i, j) * x.get(sidx, j) * weights[sidx];`
  - L151: `let residuals = transform.matmul(&d)?;`
  - L153: `let mut residual_energy = 0.0;`
  - L156: `total_energy += weights[r] * d.get(r, p).powi(2);`
  - L157: `residual_energy += weights[r] * residuals.get(r, p).powi(2);`
  - L163: `(1.0 - residual_energy / total_energy).clamp(0.0, 1.0)`
  - L166: `residuals,`

### `src/analysis/active.rs` (9 fns)

#### `trace`  ·  L70
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `choose_active_aperture`  ·  L74
- **Ecuaciones / líneas clave:**
  - L96: `let snr = dot(&candidate.sensing_vector, &q)?.max(0.0) / candidate.noise_variance;`
  - L97: `let information_gain = 0.5 * (1.0 + snr).ln();`
  - L100: `let score = ApertureScore {`
  - L116: `/// Σ' = Σ - Σ a aᵀ Σ / (σ² + aᵀ Σ a).`

#### `assimilate_aperture_result`  ·  L170
- **Doc:** Assimilate the realized scalar result y = a^T x + ε after an aperture has actually run. This is the exact Gaussian linear update of both posterior mean and covariance. Planning uses covariance only; execution closes the loop with this function once evidence arrives.
- **Ecuaciones / líneas clave:**
  - L187: `let predictive_variance = dot(&candidate.sensing_vector, &projected)?.max(0.0);`
  - L192: `let predicted_value = dot(&candidate.sensing_vector, &posterior.mean)?;`

#### `plan_active_apertures`  ·  L212
- **Doc:** Sequential experiment design. Candidates are used at most once. After each selected aperture the posterior covariance is updated, so the next choice reflects information already expected from prior planned experiments.
- **Ecuaciones / líneas clave:**
  - L255: `total_information_gain += best.information_gain;`

#### `aperture_candidate_wire_rejects_paths_and_digests`  ·  L290
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `active_aperture_rejects_indefinite_covariance`  ·  L310
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `active_aperture_rejects_zero_sensing_and_breaks_ties_canonically`  ·  L325
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `active_aperture_prefers_information_when_costs_match`  ·  L352
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `realized_aperture_updates_mean_and_covariance`  ·  L390
- **Ecuaciones / líneas clave:**
  - L404: `assert!(updated.mean[1].abs() < 1e-12);`
  - L406: `assert!((updated.covariance[1][1] - 1.0).abs() < 1e-12);`

### `src/analysis/interaction.rs` (1 fns)

#### `mvdr_weights`  ·  L45
- **Ecuaciones / líneas clave:**
  - L45: `pub fn mvdr_weights(covariance: &Matrix, desired: &[f64], ridge: f64) -> BrainResult<Vec<f64>> {`
  - L49: `\|\| !ridge.is_finite()`
  - L50: `\|\| ridge < 0.0`
  - L55: `let inv = crate::foundation::linalg::inverse_with_ridge(covariance, ridge)?;`
  - L57: `let denom = crate::foundation::linalg::dot(desired, &x)?;`
  - L58: `if denom.abs() < 1e-15 {`

## Plasticidad cross-model

### `src/cross_model/plasticity/bcm_metaplasticity.rs` (6 fns)

#### `default`  ·  L26
- **Ecuaciones / líneas clave:**
  - L28: `initial_theta: 0.5,`
  - L31: `theta_decay: 0.001,`

#### `validate`  ·  L37
- **Ecuaciones / líneas clave:**
  - L38: `if !self.initial_theta.is_finite()`
  - L39: `\|\| !(0.0..=1.0).contains(&self.initial_theta)`
  - L45: `\|\| !self.theta_decay.is_finite()`
  - L46: `\|\| !(0.0..1.0).contains(&self.theta_decay)`

#### `update_threshold`  ·  L86
- **Ecuaciones / líneas clave:**
  - L108: `state.theta_m =`
  - L109: `(state.theta_m + state.learning_rate * (mean_squared - state.theta_m)).clamp(0.0, 1.0);`
  - L110: `Ok(state.theta_m)`

#### `calculate_weight_change`  ·  L113
- **Ecuaciones / líneas clave:**
  - L126: `Ok(state.learning_rate * pre_synaptic * post_synaptic * (post_synaptic - state.theta_m))`

#### `get_threshold`  ·  L129
- **Ecuaciones / líneas clave:**
  - L130: `self.states.get(capability_name).map(\|state\| state.theta_m)`

#### `apply_decay`  ·  L150
- **Ecuaciones / líneas clave:**
  - L150: `pub fn apply_decay(&mut self) {`
  - L152: `state.theta_m = (state.theta_m * (1.0 - self.config.theta_decay)).clamp(0.0, 1.0);`

### `src/cross_model/plasticity/eligibility_traces.rs` (9 fns)

#### `validate`  ·  L35
- **Ecuaciones / líneas clave:**
  - L38: `\|\| !self.decay_factor.is_finite()`
  - L39: `\|\| !(0.0..=1.0).contains(&self.decay_factor)`

#### `initialize_trace`  ·  L66
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `update_trace`  ·  L81
- **Ecuaciones / líneas clave:**
  - L89: `trace.trace_value = (trace.trace_value * self.config.decay_factor`

#### `accumulate_credit`  ·  L96
- **Ecuaciones / líneas clave:**
  - L104: `trace.credit_accumulated += credit * trace.trace_value;`

#### `get_trace`  ·  L111
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `get_accumulated_credit`  ·  L116
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `reset_trace`  ·  L122
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `decay_all`  ·  L133
- **Ecuaciones / líneas clave:**
  - L133: `pub fn decay_all(&mut self) {`
  - L135: `trace.trace_value *= self.config.decay_factor;`

#### `get_all_traces`  ·  L141
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/cross_model/plasticity/neuromodulation.rs` (4 fns)

#### `validate`  ·  L65
- **Ecuaciones / líneas clave:**
  - L78: `\|\| (self.weights.values().sum::<f64>() - 1.0).abs() > 1e-12`
  - L79: `\|\| !self.decay_factor.is_finite()`
  - L80: `\|\| !(0.0..=1.0).contains(&self.decay_factor)`

#### `calculate_plasticity_modulation`  ·  L118
- **Ecuaciones / líneas clave:**
  - L122: `modulation += *weight * self.get_level(*modulator)?;`

#### `modulate_learning_rate`  ·  L130
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `apply_decay`  ·  L137
- **Ecuaciones / líneas clave:**
  - L137: `pub fn apply_decay(&mut self) {`
  - L139: `*level *= self.config.decay_factor;`

### `src/cross_model/plasticity/content_plasticity.rs` (5 fns)

#### `default`  ·  L34
- **Ecuaciones / líneas clave:**
  - L39: `temporal_decay: HashMap::new(),`
  - L42: `eligibility_lambda_content: 0.95,`
  - L43: `eligibility_decay_content: 0.99,`

#### `update_source_trust`  ·  L84
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `consolidate_fact`  ·  L124
- **Ecuaciones / líneas clave:**
  - L134: `let decay = self.temporal_decay.get(fact_id).copied().unwrap_or(0.0);`
  - L140: `confidence = (confidence * (1.0 - decay) + evidence_strength * self.consolidation_rate)`
  - L144: `self.temporal_decay`
  - L145: `.insert(fact_id.to_string(), (decay + self.eligibility_decay_content).min(0.99));`

#### `update_similarity`  ·  L224
- **Ecuaciones / líneas clave:**
  - L241: `let evidence_strength = (1.0 - measured_similarity.abs()).clamp(0.0, 1.0);`
  - L244: `measured_similarity.abs().clamp(0.0, 1.0),`
  - L255: `* (1.0 - self.config.adaptation_rate)`

#### `advanced_content_plasticity_tracks_fact_confidence_and_source_trust`  ·  L304
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/cross_model/plasticity/routing_plasticity.rs` (5 fns)

#### `update_weight`  ·  L62
- **Ecuaciones / líneas clave:**
  - L73: `let delta = self.learning_rate * correlation - self.decay_rate;`

#### `stability_adjusted_score`  ·  L91
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `decay_tick`  ·  L113
- **Ecuaciones / líneas clave:**
  - L113: `pub fn decay_tick(&mut self) -> Result<(), String> {`
  - L114: `if !self.decay_rate.is_finite() \|\| !(0.0..=1.0).contains(&self.decay_rate) {`
  - L115: `return Err("routing_plasticity_decay_rate_must_be_within_0_1".into());`
  - L117: `let factor = 1.0 - self.decay_rate;`
  - L120: `*weight *= factor;`

#### `route_capability`  ·  L184
- **Ecuaciones / líneas clave:**
  - L214: `* ((total_samples.max(1.0).ln() / row.sample_size as f64).max(0.0)).sqrt();`
  - L220: `.map(\|decision\| (decision.routing_score - row.score).abs())`
  - L223: `(1.0 - mean_gap.clamp(0.0, 1.0)).clamp(0.0, 1.0)`
  - L225: `let adjusted_score = self.matrix.stability_adjusted_score(`

#### `routing_capability_prefers_stability_weighted_model_when_scores_are_equal`  ·  L353
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/cross_model/plasticity/pi_controller.rs` (0 fns)

### `src/cross_model/plasticity/elo_system.rs` (1 fns)

#### `update_observed`  ·  L88
- **Ecuaciones / líneas clave:**
  - L114: `/ (1.0 + 10.0_f64.powf((second_rating - first_rating) / self.config.logistic_scale));`
  - L115: `let expected_second = 1.0 - expected_first;`
  - L116: `let observed_second = 1.0 - first_observed_score;`

### `src/cross_model/plasticity_engine.rs`
_Sin funciones algorítmicas filtradas (revisar a mano si es glue)._

## Extracción / alineación / LoRA / contrafácticos

### `src/cross_model/extraction/cross_model_aligner.rs` (10 fns)

#### `validate`  ·  L38
- **Ecuaciones / líneas clave:**
  - L43: `\|\| !self.ridge_lambda.is_finite()`
  - L44: `\|\| self.ridge_lambda <= 0.0`
  - L84: `pub ridge_lambda: f64,`
  - L85: `pub maximum_validation_residual: f64,`

#### `default`  ·  L90
- **Ecuaciones / líneas clave:**
  - L92: `ridge_lambda: 1e-4,`
  - L93: `maximum_validation_residual: 0.35,`

#### `validate`  ·  L100
- **Ecuaciones / líneas clave:**
  - L101: `if !self.ridge_lambda.is_finite()`
  - L102: `\|\| self.ridge_lambda <= 0.0`
  - L103: `\|\| !self.maximum_validation_residual.is_finite()`
  - L104: `\|\| self.maximum_validation_residual < 0.0`
  - L115: `struct FittedRidgeMap {`
  - L121: `validation_residual: f64,`

#### `calibrate_from_prompts`  ·  L134
- **Ecuaciones / líneas clave:**
  - L180: `ridge_lambda: self.config.ridge_lambda,`
  - L188: `if fitted.validation_residual > self.config.maximum_validation_residual {`
  - L190: `"alignment_validation_residual_exceeded:{:.6}",`
  - L191: `fitted.validation_residual`

#### `align`  ·  L198
- **Ecuaciones / líneas clave:**
  - L211: `if fitted.validation_residual > self.config.maximum_validation_residual {`
  - L218: `alignment_score: (1.0 - fitted.validation_residual).clamp(0.0, 1.0),`
  - L219: `normalized_residual: fitted.validation_residual,`
  - L221: `method: AlignmentMethod::CalibratedLinearRidge,`

#### `fit`  ·  L244
- **Ecuaciones / líneas clave:**
  - L247: `) -> Result<FittedRidgeMap, Box<dyn Error + Send + Sync>> {`
  - L264: `let value = dot(&source_training[row], &source_training[column])`
  - L266: `calibration.ridge_lambda`
  - L290: `let mut fitted = FittedRidgeMap {`
  - L296: `validation_residual: 0.0,`
  - L298: `fitted.validation_residual = validation_residual(&fitted, &calibration.validation_pairs)?;`

#### `predict`  ·  L303
- **Ecuaciones / líneas clave:**
  - L303: `fn predict(map: &FittedRidgeMap, source: &Tensor) -> Result<Tensor, String> {`
  - L320: `.map(\|row\| dot(&source.data, row))`

#### `validation_residual`  ·  L333
- **Ecuaciones / líneas clave:**
  - L333: `fn validation_residual(map: &FittedRidgeMap, pairs: &[ActivationPair]) -> Result<f64, String> {`
  - L342: `error_sq += (prediction - target).powi(2);`
  - L343: `target_sq += target.powi(2);`
  - L349: `Ok((error_sq / target_sq).sqrt())`

#### `dot`  ·  L352
- **Ecuaciones / líneas clave:**
  - L352: `fn dot(left: &[f64], right: &[f64]) -> f64 {`

#### `calibration_digest`  ·  L356
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/cross_model/extraction/hierarchical_steering_extractor.rs` (4 fns)

#### `validate`  ·  L72
- **Ecuaciones / líneas clave:**
  - L80: `\|\| !self.minimum_mean_direction_cosine.is_finite()`
  - L81: `\|\| !(-1.0..=1.0).contains(&self.minimum_mean_direction_cosine)`

#### `extract`  ·  L99
- **Ecuaciones / líneas clave:**
  - L140: `&& component.quality.mean_direction_cosine`
  - L141: `>= self.config.minimum_mean_direction_cosine`
  - L168: `+ ((best.quality.mean_direction_cosine + 1.0) / 2.0))`

#### `extract_layer`  ·  L220
- **Ecuaciones / líneas clave:**
  - L262: `*target += *value / pair_differences.len() as f64;`
  - L267: `let norm = vector.l2_norm();`
  - L268: `if norm <= f64::EPSILON {`
  - L271: `let mut cosine_sum = 0.0;`
  - L277: `let cosine = row_tensor`
  - L278: `.cosine_similarity(&vector)`
  - L279: `.ok_or("activation_pair_zero_norm")?;`
  - L280: `cosine_sum += cosine;`
  - L281: `if cosine > 0.0 {`
  - L282: `persistent += 1;`
  - L284: `squared_error += row`
  - L287: `.map(\|(value, mean)\| (value - mean).powi(2))`
  - L290: `let mean_direction_cosine = cosine_sum / pair_differences.len() as f64;`
  - L292: `let rmse = (squared_error / (pair_differences.len() * dimension) as f64).sqrt();`
  - L293: `let relative_dispersion = rmse / (norm / (dimension as f64).sqrt());`
  - L297: `.filter(\|value\| value.abs() <= self.config.sparsity_epsilon)`
  - L304: `norm,`
  - L306: `mean_direction_cosine,`

#### `quality_score`  ·  L314
- **Ecuaciones / líneas clave:**
  - L315: `metrics.direction_persistence * ((metrics.mean_direction_cosine + 1.0) / 2.0)`
  - L316: `/ (1.0 + metrics.relative_dispersion)`

### `src/cross_model/extraction/lora_synthesizer.rs` (4 fns)

#### `validate`  ·  L46
- **Ecuaciones / líneas clave:**
  - L50: `\|\| self.rank == 0`
  - L55: `\|\| self.a.shape[0] != self.rank`
  - L56: `\|\| self.b.shape[1] != self.rank`
  - L59: `\|\| self.alpha != self.rank as f64`

#### `materialize_dense`  ·  L68
- **Ecuaciones / líneas clave:**
  - L77: `let scale = self.alpha / self.rank as f64;`
  - L81: `for component in 0..self.rank {`
  - L82: `value += self.b.data[row * self.rank + component]`

#### `build_from_verified_factors`  ·  L164
- **Ecuaciones / líneas clave:**
  - L169: `factors: VerifiedLowRankFactors,`
  - L176: `// Core factors are dense = left @ right. PEFT uses B @ A * alpha/rank.`
  - L177: `// alpha=rank therefore preserves the verified core factorization exactly.`
  - L181: `rank: factors.rank,`
  - L182: `alpha: factors.rank as f64,`
  - L185: `vec![factors.rank, factors.columns],`
  - L192: `vec![factors.rows, factors.rank],`
  - L205: `.map(\|(a, b)\| (a - b).abs())`
  - L212: `.map(\|v\| v.abs())`

#### `synthesis_digest`  ·  L240
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/cross_model/extraction/counterfactual_analyzer.rs` (3 fns)

#### `validate`  ·  L73
- **Ecuaciones / líneas clave:**
  - L86: `\|\| !row.cosine_similarity.is_finite()`
  - L87: `\|\| !(-1.0..=1.0).contains(&row.cosine_similarity)`

#### `analyze_one`  ·  L156
- **Ecuaciones / líneas clave:**
  - L165: `let original_score = scenario.verifier.score(&original.text)?;`
  - L166: `let perturbed_score = scenario.verifier.score(&perturbed.text)?;`
  - L179: `let difference_norm = before`
  - L183: `.map(\|(a, b)\| (a - b).powi(2))`
  - L185: `.sqrt();`
  - L186: `let denominator = before.l2_norm().max(f64::EPSILON);`
  - L187: `let cosine_similarity = before`
  - L188: `.cosine_similarity(&after)`
  - L189: `.ok_or("counterfactual_activation_zero_norm")?;`
  - L192: `relative_activation_change: difference_norm / denominator,`
  - L193: `cosine_similarity,`

#### `counterfactual_digest`  ·  L261
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

## Discovery / promoción

### `src/cross_model/discovery/gap_detector.rs` (4 fns)

#### `detect_gaps`  ·  L102
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `detect_from_evaluations`  ·  L119
- **Ecuaciones / líneas clave:**
  - L158: `\|\| (source_row.weight - probe.weight).abs() > f64::EPSILON`
  - L159: `\|\| (target_row.weight - probe.weight).abs() > f64::EPSILON`
  - L163: `weighted_difference += probe.weight * (source_row.score - target_row.score);`
  - L164: `sum_weight += probe.weight;`
  - L165: `sum_weight_sq += probe.weight * probe.weight;`
  - L174: `let radius = ((1.0 / benchmark.significance_alpha).ln() * sum_weight_sq`
  - L175: `/ (2.0 * sum_weight * sum_weight))`
  - L176: `.sqrt();`
  - L192: `confidence_level: 1.0 - benchmark.significance_alpha,`

#### `gap_digest`  ·  L203
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `conservative_gap_requires_real_margin`  ·  L284
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/cross_model/discovery/prioritizer.rs` (2 fns)

#### `score_gap`  ·  L106
- **Ecuaciones / líneas clave:**
  - L110: `let receiver_deficit = (1.0 - gap.target_score).clamp(0.0, 1.0);`
  - L112: `let overall_score = self.config.effect_weight * measured_effect`

#### `priority_digest`  ·  L152
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/cross_model/discovery/domain_analyzer.rs` (1 fns)

#### `from_label`  ·  L22
- **Ecuaciones / líneas clave:**
  - L23: `let normalized = label.trim().to_ascii_lowercase();`
  - L24: `if normalized.is_empty() {`
  - L27: `Ok(match normalized.as_str() {`
  - L35: `_ => Self::Other(normalized),`

### `src/cross_model/discovery/emergent_detector.rs`
_Sin funciones algorítmicas filtradas (revisar a mano si es glue)._

### `src/cross_model/promotion/promotion_gates.rs` (2 fns)

#### `run_all_gates`  ·  L79
- **Ecuaciones / líneas clave:**
  - L124: `let preservation_score = minimum_score_for(validation, EvidenceType::Preservation);`
  - L138: `let replay_score = minimum_score_for(validation, EvidenceType::IndependentReplay);`

#### `minimum_score_for`  ·  L204
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/cross_model/promotion/domain_fitness.rs` (1 fns)

#### `evaluate_fitness`  ·  L60
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/cross_model/promotion/evidence_validator.rs` (1 fns)

#### `validate`  ·  L142
- **Ecuaciones / líneas clave:**
  - L162: `*by_type.entry(item.evidence_type).or_insert(0usize) += 1;`
  - L164: `let minimum_score = items`

### `src/cross_model/promotion/promoter.rs`
_Sin funciones algorítmicas filtradas (revisar a mano si es glue)._

## Aprendizaje / portfolio / crédito

### `src/learning/solver_portfolio.rs` (99 fns)

#### `as_digest`  ·  L40
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `as_digest`  ·  L62
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `as_digest`  ·  L77
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `as_digest`  ·  L96
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L159
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `max_numerical_work_units`  ·  L286
- **Ecuaciones / líneas clave:**
  - L298: `relative_rank_tolerance: f64,`
  - L302: `minimum_rank: usize,`
  - L303: `maximum_rank: usize,`
  - L304: `relative_residual_tolerance: f64,`
  - L305: `absolute_residual_tolerance: f64,`
  - L306: `svd_orthogonality_tolerance: f64,`
  - L307: `max_svd_sweeps: usize,`

#### `default`  ·  L314
- **Ecuaciones / líneas clave:**
  - L318: `relative_rank_tolerance: 1.0e-10,`
  - L320: `minimum_rank: 1,`
  - L321: `maximum_rank: 64,`
  - L322: `relative_residual_tolerance: 1.0e-8,`
  - L323: `absolute_residual_tolerance: 1.0e-10,`
  - L324: `svd_orthogonality_tolerance: 1.0e-12,`
  - L325: `max_svd_sweeps: 100,`

#### `validate`  ·  L333
- **Ecuaciones / líneas clave:**
  - L338: `\|\| !self.relative_rank_tolerance.is_finite()`
  - L339: `\|\| !(0.0..1.0).contains(&self.relative_rank_tolerance)`
  - L343: `\|\| self.minimum_rank == 0`
  - L344: `\|\| self.maximum_rank < self.minimum_rank`
  - L345: `\|\| !self.relative_residual_tolerance.is_finite()`
  - L346: `\|\| self.relative_residual_tolerance < 0.0`
  - L347: `\|\| !self.absolute_residual_tolerance.is_finite()`
  - L348: `\|\| self.absolute_residual_tolerance < 0.0`
  - L349: `\|\| self.relative_residual_tolerance == 0.0 && self.absolute_residual_tolerance == 0.0`
  - L350: `\|\| !self.svd_orthogonality_tolerance.is_finite()`
  - L351: `\|\| !(0.0..1.0).contains(&self.svd_orthogonality_tolerance)`
  - L352: `\|\| self.max_svd_sweeps == 0`

#### `with_rank_policy`  ·  L374
- **Ecuaciones / líneas clave:**
  - L374: `pub fn with_rank_policy(`
  - L378: `minimum_rank: usize,`
  - L379: `maximum_rank: usize,`
  - L381: `self.relative_rank_tolerance = relative_tolerance;`
  - L383: `self.minimum_rank = minimum_rank;`
  - L384: `self.maximum_rank = maximum_rank;`

#### `with_residual_tolerances`  ·  L389
- **Ecuaciones / líneas clave:**
  - L389: `pub fn with_residual_tolerances(mut self, relative: f64, absolute: f64) -> BrainResult<Self> {`
  - L390: `self.relative_residual_tolerance = relative;`
  - L391: `self.absolute_residual_tolerance = absolute;`

#### `with_svd_convergence`  ·  L396
- **Ecuaciones / líneas clave:**
  - L396: `pub fn with_svd_convergence(`
  - L401: `self.svd_orthogonality_tolerance = orthogonality_tolerance;`
  - L402: `self.max_svd_sweeps = max_sweeps;`

#### `relative_rank_tolerance`  ·  L427
- **Ecuaciones / líneas clave:**
  - L427: `pub fn relative_rank_tolerance(&self) -> f64 {`
  - L428: `self.relative_rank_tolerance`

#### `minimum_rank`  ·  L435
- **Ecuaciones / líneas clave:**
  - L435: `pub fn minimum_rank(&self) -> usize {`
  - L436: `self.minimum_rank`

#### `maximum_rank`  ·  L439
- **Ecuaciones / líneas clave:**
  - L439: `pub fn maximum_rank(&self) -> usize {`
  - L440: `self.maximum_rank`

#### `relative_residual_tolerance`  ·  L443
- **Ecuaciones / líneas clave:**
  - L443: `pub fn relative_residual_tolerance(&self) -> f64 {`
  - L444: `self.relative_residual_tolerance`

#### `absolute_residual_tolerance`  ·  L447
- **Ecuaciones / líneas clave:**
  - L447: `pub fn absolute_residual_tolerance(&self) -> f64 {`
  - L448: `self.absolute_residual_tolerance`

#### `svd_orthogonality_tolerance`  ·  L451
- **Ecuaciones / líneas clave:**
  - L451: `pub fn svd_orthogonality_tolerance(&self) -> f64 {`
  - L452: `self.svd_orthogonality_tolerance`

#### `max_svd_sweeps`  ·  L455
- **Ecuaciones / líneas clave:**
  - L455: `pub fn max_svd_sweeps(&self) -> usize {`
  - L456: `self.max_svd_sweeps`

#### `digest`  ·  L467
- **Ecuaciones / líneas clave:**
  - L478: `self.relative_rank_tolerance,`
  - L480: `self.relative_residual_tolerance,`
  - L481: `self.absolute_residual_tolerance,`
  - L482: `self.svd_orthogonality_tolerance,`
  - L488: `usize_value(self.minimum_rank, "minimum_rank")?,`
  - L489: `usize_value(self.maximum_rank, "maximum_rank")?,`
  - L490: `usize_value(self.max_svd_sweeps, "svd_sweeps")?,`
  - L511: `BuiltInCholeskyRidge,`
  - L512: `BuiltInDirectJacobiSvd,`

#### `values`  ·  L648
- **Ecuaciones / líneas clave:**
  - L662: `LowRank {`
  - L665: `rank: usize,`

#### `rank`  ·  L708
- **Ecuaciones / líneas clave:**
  - L708: `pub fn rank(&self) -> Option<usize> {`
  - L710: `Self::LowRank { rank, .. } => Some(*rank),`

#### `stored_parameter_count`  ·  L715
- **Ecuaciones / líneas clave:**
  - L717: `Self::LowRank {`
  - L720: `rank,`
  - L723: `.checked_mul(*rank)?`
  - L724: `.checked_add(rank.checked_mul(*columns)?),`

#### `materialize_dense`  ·  L733
- **Ecuaciones / líneas clave:**
  - L755: `Self::LowRank {`
  - L758: `rank,`
  - L762: `if *rank == 0`
  - L763: `\|\| left.len() != rows.checked_mul(*rank).unwrap_or(usize::MAX)`
  - L764: `\|\| right.len() != rank.checked_mul(*columns).unwrap_or(usize::MAX)`
  - L767: `return Err(BrainError::Integrity("solver_low_rank_candidate_invalid".into()));`
  - L772: `let value = compensated_sum((0..*rank).map(\|component\| {`
  - L773: `left[row * *rank + component] * right[component * *columns + column]`

#### `exact_digest`  ·  L863
- **Ecuaciones / líneas clave:**
  - L879: `Self::LowRank {`
  - L880: `rank, left, right, ..`
  - L883: `append_usize(&mut frame, *rank, "candidate_rank")?;`

#### `append_f64_slice`  ·  L926
- **Ecuaciones / líneas clave:**
  - L947: `absolute_residual: f64,`
  - L948: `root_mean_square_residual: f64,`
  - L949: `maximum_absolute_residual: f64,`
  - L951: `relative_residual: Option<f64>,`
  - L952: `target_norm: f64,`
  - L953: `frobenius_norm: f64,`

#### `absolute_residual`  ·  L958
- **Ecuaciones / líneas clave:**
  - L958: `pub fn absolute_residual(&self) -> f64 {`
  - L959: `self.absolute_residual`

#### `root_mean_square_residual`  ·  L962
- **Ecuaciones / líneas clave:**
  - L962: `pub fn root_mean_square_residual(&self) -> f64 {`
  - L963: `self.root_mean_square_residual`

#### `maximum_absolute_residual`  ·  L966
- **Ecuaciones / líneas clave:**
  - L966: `pub fn maximum_absolute_residual(&self) -> f64 {`
  - L967: `self.maximum_absolute_residual`

#### `relative_residual`  ·  L970
- **Ecuaciones / líneas clave:**
  - L970: `pub fn relative_residual(&self) -> Option<f64> {`
  - L971: `self.relative_residual`

#### `target_norm`  ·  L974
- **Ecuaciones / líneas clave:**
  - L974: `pub fn target_norm(&self) -> f64 {`
  - L975: `self.target_norm`

#### `frobenius_norm`  ·  L978
- **Ecuaciones / líneas clave:**
  - L978: `pub fn frobenius_norm(&self) -> f64 {`
  - L979: `self.frobenius_norm`

#### `stored_parameter_count`  ·  L982
- **Ecuaciones / líneas clave:**
  - L991: `RankDeficient,`
  - L1001: `numerical_rank: usize,`
  - L1002: `effective_rank: f64,`
  - L1003: `energy_rank: usize,`
  - L1008: `direct_svd_sweeps: usize,`
  - L1010: `direct_svd_reconstruction_relative_error: f64,`
  - L1011: `direct_svd_vector_orthogonality_error: f64,`

#### `gram_spectrum`  ·  L1019
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `numerical_rank`  ·  L1027
- **Ecuaciones / líneas clave:**
  - L1027: `pub fn numerical_rank(&self) -> usize {`
  - L1028: `self.numerical_rank`

#### `effective_rank`  ·  L1031
- **Ecuaciones / líneas clave:**
  - L1031: `pub fn effective_rank(&self) -> f64 {`
  - L1032: `self.effective_rank`

#### `energy_rank`  ·  L1035
- **Ecuaciones / líneas clave:**
  - L1035: `pub fn energy_rank(&self) -> usize {`
  - L1036: `self.energy_rank`

#### `finite_singular_condition_number`  ·  L1047
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `finite_gram_condition_number`  ·  L1051
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `direct_svd_sweeps`  ·  L1055
- **Ecuaciones / líneas clave:**
  - L1055: `pub fn direct_svd_sweeps(&self) -> usize {`
  - L1056: `self.direct_svd_sweeps`

#### `direct_svd_reconstruction_relative_error`  ·  L1063
- **Ecuaciones / líneas clave:**
  - L1063: `pub fn direct_svd_reconstruction_relative_error(&self) -> f64 {`
  - L1064: `self.direct_svd_reconstruction_relative_error`

#### `direct_svd_vector_orthogonality_error`  ·  L1067
- **Ecuaciones / líneas clave:**
  - L1067: `pub fn direct_svd_vector_orthogonality_error(&self) -> f64 {`
  - L1068: `self.direct_svd_vector_orthogonality_error`
  - L1077: `RankDeficient,`
  - L1087: `ResidualWithinTolerance,`
  - L1088: `ResidualExceedsTolerance,`
  - L1090: `RankBudgetInsufficient,`
  - L1096: `DirectSvdDidNotConverge,`
  - L1097: `ResidualToleranceUnrepresentable,`
  - L1107: `constructed_rank: Option<usize>,`

#### `constructed_rank`  ·  L1125
- **Ecuaciones / líneas clave:**
  - L1125: `pub fn constructed_rank(&self) -> Option<usize> {`
  - L1126: `self.constructed_rank`

#### `metrics`  ·  L1133
- **Ecuaciones / líneas clave:**
  - L1155: `absolute_residual_bits: u64,`
  - L1156: `root_mean_square_residual_bits: u64,`
  - L1157: `maximum_absolute_residual_bits: u64,`
  - L1158: `relative_residual_bits: Option<u64>,`
  - L1159: `target_norm_bits: u64,`
  - L1160: `frobenius_norm_bits: u64,`

#### `from`  ·  L1165
- **Ecuaciones / líneas clave:**
  - L1167: `absolute_residual_bits: metrics.absolute_residual.to_bits(),`
  - L1168: `root_mean_square_residual_bits: metrics.root_mean_square_residual.to_bits(),`
  - L1169: `maximum_absolute_residual_bits: metrics.maximum_absolute_residual.to_bits(),`
  - L1170: `relative_residual_bits: metrics.relative_residual.map(f64::to_bits),`
  - L1171: `target_norm_bits: metrics.target_norm.to_bits(),`
  - L1172: `frobenius_norm_bits: metrics.frobenius_norm.to_bits(),`
  - L1183: `numerical_rank: usize,`
  - L1184: `effective_rank_bits: u64,`
  - L1185: `energy_rank: usize,`
  - L1190: `direct_svd_sweeps: usize,`
  - L1192: `direct_svd_reconstruction_relative_error_bits: u64,`
  - L1193: `direct_svd_vector_orthogonality_error_bits: u64,`

#### `from`  ·  L1197
- **Ecuaciones / líneas clave:**
  - L1210: `numerical_rank: diagnostics.numerical_rank,`
  - L1211: `effective_rank_bits: diagnostics.effective_rank.to_bits(),`
  - L1212: `energy_rank: diagnostics.energy_rank,`
  - L1221: `direct_svd_sweeps: diagnostics.direct_svd_sweeps,`
  - L1225: `direct_svd_reconstruction_relative_error_bits: diagnostics`
  - L1226: `.direct_svd_reconstruction_relative_error`
  - L1228: `direct_svd_vector_orthogonality_error_bits: diagnostics`
  - L1229: `.direct_svd_vector_orthogonality_error`
  - L1240: `constructed_rank: Option<usize>,`

#### `problem_digest`  ·  L1283
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `policy_digest`  ·  L1287
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `exact_digest`  ·  L1293
- **Doc:** Return the exact run digest only after recomputing and authenticating the complete canonical projection.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `projection`  ·  L1375
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `seal_portfolio_report`  ·  L1438
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `bounded_unknown`  ·  L1505
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `solve_with_portfolio`  ·  L1536
- **Doc:** Diagnose, generate, verify, and deterministically select bounded solver candidates. This function never promotes or persists its selected proposal.
- **Ecuaciones / líneas clave:**
  - L1585: `EvaluationReason::ResourceLimit("solver_max_svd_work_units"),`
  - L1589: `let direct_svd = match direct_one_sided_jacobi_svd(problem, policy)? {`
  - L1596: `EvaluationReason::DirectSvdDidNotConverge,`
  - L1601: `let diagnostics = diagnose_problem(&gram, &direct_svd, policy)?;`
  - L1612: `NumericalBackendRole::BuiltInCholeskyRidge,`
  - L1617: `constructed_rank: None,`
  - L1623: `evaluations.push(run_direct_svd(problem, &direct_svd, &diagnostics, policy)?);`

#### `seal_with_unavailable_direct_solver`  ·  L1687
- **Ecuaciones / líneas clave:**
  - L1695: `NumericalBackendRole::BuiltInDirectJacobiSvd,`
  - L1696: `BUILTIN_DIRECT_SVD_NAME,`
  - L1700: `constructed_rank: None,`

#### `problem_limit`  ·  L1764
- **Ecuaciones / líneas clave:**
  - L1783: `.any(\|value\| value.abs() > policy.max_absolute_value)`
  - L1795: `let safe_gram_magnitude = (f64::MAX / energy_terms).sqrt();`
  - L1800: `.any(\|value\| value.abs() > safe_gram_magnitude)`

#### `direct_solver_work_upper_bound`  ·  L1839
- **Doc:** Conservative work bound for the complete built-in rank-revealing path: Jacobi sweeps (including convergence checks), decomposition verification, Gram diagnostics, construction of every permitted rank candidate, and independent residual recomputation for each candidate. Units deliberately over-count loop bodies; they are an execution budget, not reported FLOPs.
- **Ecuaciones / líneas clave:**
  - L1848: `let sweeps = u128::try_from(policy.max_svd_sweeps).ok()?;`
  - L1879: `let rank_cap = u128::try_from(`
  - L1881: `.maximum_rank`
  - L1885: `let rank_sum = rank_cap`
  - L1886: `.checked_mul(rank_cap.checked_add(1)?)?`
  - L1888: `let factor_construction = rank_sum`
  - L1891: `let candidate_materialization = rank_sum`
  - L1896: `rank_cap.checked_mul(single_candidate_evaluation_work_upper_bound(problem)?)?;`

#### `single_candidate_evaluation_work_upper_bound`  ·  L1917
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `row_gram`  ·  L1929
- **Ecuaciones / líneas clave:**
  - L1935: `let value = scaled_compensated_dot(inputs.row(row), inputs.row(column))?;`
  - L1938: `column += 1;`
  - L1940: `row += 1;`

#### `compensated_sum`  ·  L1945
- **Ecuaciones / líneas clave:**
  - L1953: `if sum.abs() >= value.abs() {`
  - L1954: `correction += (sum - updated) + value;`
  - L1956: `correction += (value - updated) + sum;`

#### `scaled_compensated_dot`  ·  L1971
- **Doc:** Scale-normalized Neumaier dot product used at the independent numerical boundary. The common linalg dot remains the general fast primitive; this path additionally protects verification and Jacobi rotations from avoidable overflow and cancellation.
- **Ecuaciones / líneas clave:**
  - L1971: `fn scaled_compensated_dot(left: &[f64], right: &[f64]) -> BrainResult<f64> {`
  - L1973: `return Err(BrainError::Invalid("solver_compensated_dot_shape_or_value".into()));`
  - L1975: `let left_scale = left.iter().map(\|value\| value.abs()).fold(0.0, f64::max);`
  - L1976: `let right_scale = right.iter().map(\|value\| value.abs()).fold(0.0, f64::max);`
  - L1980: `let normalized = compensated_sum(`
  - L1985: `let value = (normalized * left_scale) * right_scale;`
  - L1987: `return Err(BrainError::Numerical("solver_compensated_dot_nonfinite".into()));`
  - L1993: `struct DirectSvdComponent {`
  - L2000: `struct DirectJacobiSvd {`
  - L2002: `components: Vec<DirectSvdComponent>,`

#### `direct_one_sided_jacobi_svd`  ·  L2013
- **Doc:** Thin one-sided Jacobi SVD of X^T. It orthogonalizes the case columns directly instead of diagonalizing X X^T, so rank revelation does not first square the condition number. This is a bounded in-tree implementation, not a claim of LAPACK GELSD compatibility.
- **Ecuaciones / líneas clave:**
  - L2013: `fn direct_one_sided_jacobi_svd(`
  - L2016: `) -> BrainResult<Option<DirectJacobiSvd>> {`
  - L2028: `.map(\|value\| value.abs())`
  - L2031: `return Ok(Some(DirectJacobiSvd {`
  - L2034: `.map(\|component\| DirectSvdComponent {`
  - L2083: `for sweep in 0..policy.max_svd_sweeps {`
  - L2086: `let alpha = scaled_compensated_dot(&columns[first], &columns[first])?;`
  - L2087: `let beta = scaled_compensated_dot(&columns[second], &columns[second])?;`
  - L2091: `let gamma = scaled_compensated_dot(&columns[first], &columns[second])?;`
  - L2092: `let correlation = gamma.abs() / (alpha.sqrt() * beta.sqrt());`
  - L2095: `"solver_direct_svd_correlation_nonfinite".into(),`
  - L2098: `if correlation <= policy.svd_orthogonality_tolerance {`
  - L2101: `let zeta = (beta - alpha) / (2.0 * gamma);`
  - L2103: `1.0 / (zeta + (1.0 + zeta * zeta).sqrt())`
  - L2105: `-1.0 / (-zeta + (1.0 + zeta * zeta).sqrt())`
  - L2107: `let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();`
  - L2108: `let sine = cosine * tangent;`
  - L2109: `if !cosine.is_finite() \|\| !sine.is_finite() {`
  - L2111: `"solver_direct_svd_rotation_nonfinite".into(),`
  - L2114: `rotate_columns(&mut columns, first, second, cosine, sine);`
  - L2115: `rotate_columns(&mut rotations, first, second, cosine, sine);`
  - L2120: `if maximum_correlation <= policy.svd_orthogonality_tolerance {`
  - L2131: `let scaled_sigma = norm(&columns[index])?;`
  - L2134: `return Err(BrainError::Numerical("solver_direct_svd_singular_value_nonfinite".into()));`
  - L2136: `let normalized_column = if scaled_sigma > 0.0 {`

#### `verify_direct_svd`  ·  L2203
- **Ecuaciones / líneas clave:**
  - L2203: `fn verify_direct_svd(`
  - L2205: `components: &[DirectSvdComponent],`
  - L2221: `let input_dot =`
  - L2222: `scaled_compensated_dot(&active[first].input_vector, &active[second].input_vector)?;`
  - L2223: `let case_dot =`
  - L2224: `scaled_compensated_dot(&active[first].case_vector, &active[second].case_vector)?;`
  - L2226: `.max((input_dot - expected).abs())`
  - L2227: `.max((case_dot - expected).abs());`
  - L2231: `let mut residual = Vec::with_capacity(problem.inputs.as_slice().len());`
  - L2239: `residual.push(reconstructed - problem.inputs.get(case, input));`
  - L2242: `let input_norm = norm(problem.inputs.as_slice())?;`
  - L2243: `let reconstruction_relative_error = if input_norm == 0.0 {`
  - L2244: `if residual.iter().all(\|value\| *value == 0.0) {`
  - L2248: `"solver_direct_svd_zero_reconstruction_invalid".into(),`
  - L2252: `norm(&residual)? / input_norm`
  - L2255: `return Err(BrainError::Numerical("solver_direct_svd_verification_nonfinite".into()));`

#### `rotate_columns`  ·  L2260
- **Ecuaciones / líneas clave:**
  - L2260: `fn rotate_columns(columns: &mut [Vec<f64>], first: usize, second: usize, cosine: f64, sine: f64) {`
  - L2267: `*first_value = cosine * old_first - sine * old_second;`
  - L2268: `*second_value = sine * old_first + cosine * old_second;`

#### `maximum_column_correlation`  ·  L2272
- **Ecuaciones / líneas clave:**
  - L2276: `let alpha = scaled_compensated_dot(&columns[first], &columns[first])?;`
  - L2277: `let beta = scaled_compensated_dot(&columns[second], &columns[second])?;`
  - L2281: `let gamma = scaled_compensated_dot(&columns[first], &columns[second])?;`
  - L2282: `maximum = maximum.max(gamma.abs() / (alpha.sqrt() * beta.sqrt()));`
  - L2286: `return Err(BrainError::Numerical("solver_direct_svd_correlation_nonfinite".into()));`

#### `diagnose_problem`  ·  L2291
- **Ecuaciones / líneas clave:**
  - L2293: `direct_svd: &DirectJacobiSvd,`
  - L2297: `let singular_values = direct_svd`
  - L2311: `f64::EPSILON * gram.row_count().max(direct_svd.input_dimension).max(1) as f64;`
  - L2312: `let singular_value_cutoff = largest * policy.relative_rank_tolerance.max(automatic_tolerance);`
  - L2313: `let numerical_rank = singular_values`
  - L2322: `let normalized_spectrum = if largest == 0.0 {`
  - L2333: `let effective_rank = effective_rank_from_spectrum(&normalized_spectrum)?;`
  - L2334: `let energy_rank = if numerical_rank == 0 {`
  - L2337: `choose_energy_rank(`
  - L2338: `&normalized_spectrum,`
  - L2340: `numerical_rank,`
  - L2341: `policy.minimum_rank.min(numerical_rank),`
  - L2345: `if numerical_rank == 0 {`
  - L2347: `} else if numerical_rank < gram.row_count() {`
  - L2348: `(GramCondition::RankDeficient, None, None)`
  - L2350: `let smallest = singular_values[numerical_rank - 1];`
  - L2378: `numerical_rank,`
  - L2379: `effective_rank,`
  - L2380: `energy_rank,`
  - L2385: `direct_svd_sweeps: direct_svd.sweeps,`
  - L2386: `maximum_scaled_column_correlation: direct_svd.maximum_scaled_column_correlation,`
  - L2387: `direct_svd_reconstruction_relative_error: direct_svd.reconstruction_relative_error,`
  - L2388: `direct_svd_vector_orthogonality_error: direct_svd.vector_orthogonality_error,`

#### `cholesky_eligibility`  ·  L2392
- **Ecuaciones / líneas clave:**
  - L2400: `if problem.case_count() > MAX_LOW_RANK as usize {`
  - L2405: `GramCondition::RankDeficient => CholeskyGate::RankDeficient,`

#### `run_cholesky`  ·  L2428
- **Ecuaciones / líneas clave:**
  - L2430: `NumericalBackendRole::BuiltInCholeskyRidge,`
  - L2453: `match solve_regularized_multi_case_low_rank(&inputs, &targets, policy.cholesky_damping) {`
  - L2455: `let rank = solution.rank as usize;`
  - L2459: `rank,`
  - L2463: `evaluate_candidate(backend, Some(rank), candidate, problem, policy)`
  - L2469: `constructed_rank: None,`

#### `run_direct_svd`  ·  L2476
- **Ecuaciones / líneas clave:**
  - L2476: `fn run_direct_svd(`
  - L2478: `direct_svd: &DirectJacobiSvd,`
  - L2483: `NumericalBackendRole::BuiltInDirectJacobiSvd,`
  - L2484: `BUILTIN_DIRECT_SVD_NAME,`
  - L2486: `if diagnostics.numerical_rank == 0 {`
  - L2494: `let rank_cap = diagnostics.numerical_rank.min(policy.maximum_rank);`
  - L2495: `if rank_cap < diagnostics.energy_rank \|\| rank_cap < policy.minimum_rank {`
  - L2499: `reason: EvaluationReason::RankBudgetInsufficient,`
  - L2500: `constructed_rank: Some(rank_cap),`
  - L2506: `for rank in diagnostics.energy_rank.max(policy.minimum_rank)..=rank_cap {`
  - L2508: `direct_svd_factors(problem, direct_svd, rank, diagnostics.singular_value_cutoff)?;`
  - L2512: `rank,`
  - L2516: `let evaluated = evaluate_candidate(backend.clone(), Some(rank), candidate, problem, policy);`
  - L2523: `last.ok_or_else(\|\| BrainError::Numerical("solver_direct_svd_candidate_missing".into()))?;`
  - L2524: `if rank_cap < diagnostics.numerical_rank {`
  - L2526: `result.reason = EvaluationReason::RankBudgetInsufficient;`

#### `direct_svd_factors`  ·  L2531
- **Ecuaciones / líneas clave:**
  - L2531: `fn direct_svd_factors(`
  - L2533: `direct_svd: &DirectJacobiSvd,`
  - L2534: `rank: usize,`
  - L2537: `if rank == 0`
  - L2538: `\|\| rank > direct_svd.components.len()`
  - L2539: `\|\| direct_svd.components.iter().take(rank).any(\|component\| {`
  - L2551: `return Err(BrainError::Numerical("solver_direct_svd_rank_invalid".into()));`
  - L2553: `let mut left = vec![0.0; problem.output_dimension() * rank];`
  - L2554: `let mut right = vec![0.0; rank * problem.input_dimension()];`
  - L2555: `for (component_index, component) in direct_svd.components.iter().take(rank).enumerate() {`
  - L2558: `return Err(BrainError::Numerical("solver_direct_svd_inverse_nonfinite".into()));`
  - L2561: `left[output * rank + component_index] = compensated_sum(`
  - L2572: `return Err(BrainError::Numerical("solver_direct_svd_factor_nonfinite".into()));`

#### `compact_representation`  ·  L2577
- **Ecuaciones / líneas clave:**
  - L2580: `rank: usize,`
  - L2584: `let low_rank = CandidateRepresentation::LowRank {`
  - L2587: `rank,`
  - L2591: `let factor_parameters = low_rank.stored_parameter_count().unwrap_or(usize::MAX);`
  - L2594: `low_rank`
  - L2596: `match low_rank.materialize_dense() {`
  - L2602: `Err(_) => low_rank,`

#### `run_external`  ·  L2607
- **Ecuaciones / líneas clave:**
  - L2624: `constructed_rank: candidate.rank(),`
  - L2629: `let rank = candidate.rank();`
  - L2630: `evaluate_candidate(descriptor, rank, candidate.clone(), problem, policy)`

#### `evaluate_candidate`  ·  L2674
- **Ecuaciones / líneas clave:**
  - L2676: `constructed_rank: Option<usize>,`
  - L2690: `constructed_rank,`
  - L2701: `constructed_rank,`
  - L2708: `let relative_allowance = policy.relative_residual_tolerance * metrics.target_norm;`
  - L2709: `let acceptance_threshold = policy.absolute_residual_tolerance + relative_allowance;`
  - L2714: `reason: EvaluationReason::ResidualToleranceUnrepresentable,`
  - L2715: `constructed_rank,`
  - L2720: `let accepted = metrics.absolute_residual <= acceptance_threshold;`
  - L2729: `EvaluationReason::ResidualWithinTolerance`
  - L2731: `EvaluationReason::ResidualExceedsTolerance`
  - L2733: `constructed_rank,`
  - L2742: `constructed_rank,`

#### `measure_candidate`  ·  L2749
- **Ecuaciones / líneas clave:**
  - L2759: `let mut residuals = Vec::with_capacity(`
  - L2763: `.ok_or_else(\|\| BrainError::Invalid("solver_residual_shape_overflow".into()))?,`
  - L2768: `let predicted = scaled_compensated_dot(`
  - L2772: `residuals.push(predicted - problem.targets.get(case, output));`
  - L2775: `let absolute_residual = norm(&residuals)?;`
  - L2776: `let residual_count = residuals.len();`
  - L2777: `if residual_count == 0 {`
  - L2778: `return Err(BrainError::Integrity("solver_candidate_residual_empty".into()));`
  - L2780: `let root_mean_square_residual = absolute_residual / (residual_count as f64).sqrt();`
  - L2781: `let maximum_absolute_residual = residuals`
  - L2783: `.map(\|value\| value.abs())`
  - L2785: `let target_norm = norm(problem.targets.as_slice())?;`
  - L2786: `let frobenius_norm = norm(&dense)?;`
  - L2787: `let relative_residual = if target_norm > 0.0 {`
  - L2788: `Some(absolute_residual / target_norm)`
  - L2789: `} else if absolute_residual == 0.0 {`
  - L2794: `if !absolute_residual.is_finite()`
  - L2795: `\|\| !root_mean_square_residual.is_finite()`
  - L2796: `\|\| !maximum_absolute_residual.is_finite()`
  - L2797: `\|\| !target_norm.is_finite()`
  - L2798: `\|\| !frobenius_norm.is_finite()`
  - L2799: `\|\| relative_residual.is_some_and(\|value\| !value.is_finite())`
  - L2804: `absolute_residual,`
  - L2805: `root_mean_square_residual,`
  - L2806: `maximum_absolute_residual,`

#### `choose_accepted`  ·  L2849
- **Ecuaciones / líneas clave:**
  - L2881: `.absolute_residual`
  - L2882: `.total_cmp(&right_metrics.absolute_residual)`
  - L2886: `.frobenius_norm`
  - L2887: `.total_cmp(&right_metrics.frobenius_norm)`

#### `exact_policy`  ·  L2900
- **Ecuaciones / líneas clave:**
  - L2902: `.with_rank_policy(1.0e-10, 0.999999, 1, 64)`
  - L2904: `.with_residual_tolerances(1.0e-9, 1.0e-11)`

#### `rejects_wrong_shape_and_nonfinite_problem`  ·  L2909
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `exact_problem_digest_commits_to_shape_order_and_ieee_bits`  ·  L2919
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `exact_candidate_and_policy_digests_commit_to_semantics`  ·  L2946
- **Ecuaciones / líneas clave:**
  - L2957: `let factored_zero = CandidateRepresentation::LowRank {`
  - L2960: `rank: 1,`

#### `rank_deficiency_forbids_cholesky_and_uses_minimum_norm_spectral_path`  ·  L2972
- **Ecuaciones / líneas clave:**
  - L2972: `fn rank_deficiency_forbids_cholesky_and_uses_minimum_norm_spectral_path() {`
  - L2980: `assert_eq!(report.cholesky_gate, Some(CholeskyGate::RankDeficient));`
  - L2982: `assert_eq!(diagnostics.numerical_rank, 1);`
  - L2983: `assert_eq!(diagnostics.condition_kind, GramCondition::RankDeficient);`
  - L2984: `assert_eq!(report.selected().unwrap().constructed_rank, Some(1));`

#### `direct_svd_uses_tall_orientation_for_overdetermined_contracts`  ·  L2988
- **Ecuaciones / líneas clave:**
  - L2988: `fn direct_svd_uses_tall_orientation_for_overdetermined_contracts() {`
  - L3001: `assert_eq!(report.cholesky_gate, Some(CholeskyGate::RankDeficient));`
  - L3002: `assert_eq!(report.diagnostics.as_ref().unwrap().numerical_rank, 2);`
  - L3010: `.absolute_residual`

#### `ill_conditioned_full_rank_problem_selects_preplanned_spectral_backend`  ·  L3016
- **Ecuaciones / líneas clave:**
  - L3016: `fn ill_conditioned_full_rank_problem_selects_preplanned_spectral_backend() {`
  - L3028: `NumericalBackendRole::BuiltInDirectJacobiSvd`

#### `spectral_rank_expands_until_the_contract_residual_is_met`  ·  L3033
- **Ecuaciones / líneas clave:**
  - L3033: `fn spectral_rank_expands_until_the_contract_residual_is_met() {`
  - L3040: `.with_rank_policy(1.0e-10, 0.75, 1, 64)`
  - L3043: `assert_eq!(report.diagnostics.as_ref().unwrap().energy_rank, 1);`
  - L3048: `evaluation.backend.role() == NumericalBackendRole::BuiltInDirectJacobiSvd`
  - L3052: `assert_eq!(spectral.constructed_rank, Some(2));`

#### `repeated_solve_is_deterministic`  ·  L3056
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `zero_target_has_a_verified_zero_minimum_norm_solution`  ·  L3092
- **Ecuaciones / líneas clave:**
  - L3092: `fn zero_target_has_a_verified_zero_minimum_norm_solution() {`
  - L3101: `assert_eq!(selected.metrics.as_ref().unwrap().absolute_residual, 0.0);`
  - L3102: `assert_eq!(selected.metrics.as_ref().unwrap().target_norm, 0.0);`
  - L3103: `assert_eq!(selected.metrics.as_ref().unwrap().relative_residual, Some(0.0));`

#### `degenerate_zero_design_still_returns_the_verified_zero_solution`  ·  L3117
- **Ecuaciones / líneas clave:**
  - L3126: `assert_eq!(report.diagnostics().unwrap().numerical_rank(), 0);`
  - L3128: `assert_eq!(selected.constructed_rank(), Some(0));`
  - L3129: `assert_eq!(selected.metrics().unwrap().absolute_residual(), 0.0);`

#### `scale_free_rank_diagnostics_do_not_turn_tiny_data_into_rank_zero`  ·  L3142
- **Ecuaciones / líneas clave:**
  - L3142: `fn scale_free_rank_diagnostics_do_not_turn_tiny_data_into_rank_zero() {`
  - L3152: `assert_eq!(diagnostics.numerical_rank(), 2);`
  - L3153: `assert!((diagnostics.effective_rank() - 2.0).abs() <= 1.0e-12);`

#### `overflowing_tolerance_is_unknown_and_never_silently_accepts`  ·  L3159
- **Ecuaciones / líneas clave:**
  - L3162: `.with_residual_tolerances(f64::MAX, f64::MAX)`
  - L3166: `assert_eq!(report.reason(), &EvaluationReason::ResidualToleranceUnrepresentable);`

#### `direct_svd_and_verifier_handle_large_finite_scaling`  ·  L3171
- **Ecuaciones / líneas clave:**
  - L3171: `fn direct_svd_and_verifier_handle_large_finite_scaling() {`
  - L3183: `NumericalBackendRole::BuiltInDirectJacobiSvd`
  - L3192: `.relative_residual`

#### `compensated_verification_preserves_cancellation_residual`  ·  L3199
- **Ecuaciones / líneas clave:**
  - L3199: `fn compensated_verification_preserves_cancellation_residual() {`
  - L3200: `let value = scaled_compensated_dot(&[1.0e16, 1.0, -1.0e16], &[1.0, 1.0, 1.0]).unwrap();`
  - L3201: `assert!((value - 1.0).abs() <= 1.0e-12);`

#### `exhausted_direct_svd_sweeps_are_bounded_unknown`  ·  L3205
- **Ecuaciones / líneas clave:**
  - L3205: `fn exhausted_direct_svd_sweeps_are_bounded_unknown() {`
  - L3215: `let policy = exact_policy().with_svd_convergence(1.0e-16, 1).unwrap();`
  - L3218: `assert_eq!(report.reason, EvaluationReason::DirectSvdDidNotConverge);`

#### `external_proposal_cannot_self_report_or_bypass_residual_verification`  ·  L3230
- **Ecuaciones / líneas clave:**
  - L3230: `fn external_proposal_cannot_self_report_or_bypass_residual_verification() {`
  - L3237: `"test.external.full_rank",`
  - L3251: `assert_eq!(external.reason, EvaluationReason::ResidualExceedsTolerance);`
  - L3252: `assert!(external.metrics.as_ref().unwrap().absolute_residual > 1.0);`

#### `gram_overflow_risk_is_a_bounded_unknown`  ·  L3366
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `direct_svd_work_is_bounded_before_execution`  ·  L3375
- **Ecuaciones / líneas clave:**
  - L3375: `fn direct_svd_work_is_bounded_before_execution() {`
  - L3387: `assert_eq!(report.reason(), &EvaluationReason::ResourceLimit("solver_max_svd_work_units"));`

#### `bounded_builtin_work_does_not_hide_an_exact_external_candidate`  ·  L3393
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `aggregate_external_candidate_storage_is_bounded_before_cloning`  ·  L3449
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `direct_svd_work_limit_has_an_absolute_ceiling`  ·  L3477
- **Ecuaciones / líneas clave:**
  - L3477: `fn direct_svd_work_limit_has_an_absolute_ceiling() {`

#### `public_candidate_materialization_enforces_the_absolute_dense_limit`  ·  L3486
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `sparse_and_block_claims_are_verified_from_canonical_structure`  ·  L3500
- **Ecuaciones / líneas clave:**
  - L3539: `assert_eq!(external.metrics.as_ref().unwrap().absolute_residual, 0.0);`

#### `compact_solution_retains_low_rank_factors_when_they_are_smaller`  ·  L3575
- **Ecuaciones / líneas clave:**
  - L3575: `fn compact_solution_retains_low_rank_factors_when_they_are_smaller() {`
  - L3587: `Some(CandidateRepresentation::LowRank { rank: 1, .. })`

#### `rank_cap_reports_bounded_unknown_instead_of_false_impossibility`  ·  L3592
- **Ecuaciones / líneas clave:**
  - L3592: `fn rank_cap_reports_bounded_unknown_instead_of_false_impossibility() {`
  - L3599: `.with_rank_policy(1.0e-10, 0.75, 1, 1)`
  - L3606: `evaluation.backend.role() == NumericalBackendRole::BuiltInDirectJacobiSvd`
  - L3610: `assert_eq!(spectral.reason, EvaluationReason::RankBudgetInsufficient);`

### `src/learning/causal_credit.rs` (8 fns)

#### `lower_confidence_bound`  ·  L51
- **Doc:** Pessimistic 95% interaction effect used by governed routing. Positive synergy is admitted only when it survives its uncertainty margin; possible negative interference is deliberately retained.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `certified_causal_priority_weights`  ·  L84
- **Doc:** Return the conservative, evidence-backed causal utility weight for every runtime field in the caller's canonical order.  The lower confidence bound, rather than the point estimate, is the only quantity that may prioritize preservation inside a governed trust region.  Any missing, unresolved, or non-beneficial field is an authority failure: composition must not silently substitute a heuristic weight.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `mean_and_se`  ·  L166
- **Ecuaciones / líneas clave:**
  - L176: `.map(\|value\| (value - mean).powi(2))`
  - L178: `/ (values.len() - 1) as f64;`
  - L179: `(mean, (variance / values.len() as f64).sqrt())`

#### `estimate_causal_credit`  ·  L199
- **Ecuaciones / líneas clave:**
  - L262: `raw_pair_count += 1;`
  - L334: `raw_quad_count += 1;`

#### `factorial`  ·  L368
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `compute_shapley_values`  ·  L374
- **Doc:** Computes formal N-player Shapley Values for each skill field across contexts: \phi_i = \sum_{S \subseteq N \setminus \{i\}} \frac{|S|!(|N|-|S|-1)!}{|N|!} ( v(S \cup \{i\}) - v(S) )
- **Ecuaciones / líneas clave:**
  - L434: `total_shapley += weight * avg_diff;`
  - L435: `total_weight += weight;`

#### `causal_credit_recovers_main_and_interaction_effects`  ·  L455
- **Ecuaciones / líneas clave:**
  - L490: `assert!((a.mean_marginal_effect - 2.25).abs() < 1e-12);`
  - L491: `assert!((b.mean_marginal_effect - 1.25).abs() < 1e-12);`
  - L493: `assert!((pair.mean_interaction_effect - 0.5).abs() < 1e-12);`

#### `shapley_values_computed_correctly`  ·  L523
- **Ecuaciones / líneas clave:**
  - L552: `// \phi_b = 0.5 * (1.0 - 0.0) + 0.5 * (3.5 - 2.0) = 0.5 + 0.75 = 1.25`
  - L553: `assert!((shapley.get("a").unwrap() - 2.25).abs() < 1e-10);`
  - L554: `assert!((shapley.get("b").unwrap() - 1.25).abs() < 1e-10);`

### `src/learning/sleep_evidence.rs` (16 fns)

#### `verify_file`  ·  L279
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `finite_scalar_match`  ·  L300
- **Ecuaciones / líneas clave:**
  - L303: `&& (left - right).abs() <= 64.0 * f64::EPSILON * (1.0 + left.abs().max(right.abs()))`

#### `finite_vector_match`  ·  L306
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `exact_metric_schema`  ·  L324
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `mean_metrics`  ·  L330
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `verify_full_coalition_evaluations`  ·  L498
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `recompute_functional_replay`  ·  L557
- **Ecuaciones / líneas clave:**
  - L646: `.map(\|difference\| (difference - mean_utility_delta).powi(2))`
  - L648: `/ (count - 1) as f64;`
  - L649: `let standard_error = (sample_variance / count as f64).sqrt();`
  - L658: `mean_utility_delta.abs() / standard_error`
  - L682: `causal_damage_per_parameter_norm: f64,`

#### `verify_protection_artifacts`  ·  L735
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `verify_causal_credit_artifacts`  ·  L743
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `verified_causal_credit_with_weights`  ·  L1044
- **Doc:** Load causal credit only after its source replay and wrapper have both been recomputed and matched. The returned priority vector is the authoritative lower-confidence-bound ordering used by the trust region; absence of that vector is an evidence failure, never a uniform-scaling substitute.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `dense_fields_from_observations`  ·  L1071
- **Ecuaciones / líneas clave:**
  - L1096: `if coefficient.abs() <= f64::EPSILON {`
  - L1117: `if !sum.is_finite() \|\| sum.abs() > f32::MAX as f64 {`
  - L1124: `support += 1;`

#### `verify_sleep_evidence`  ·  L1136
- **Ecuaciones / líneas clave:**
  - L1204: `let tolerance = f64::EPSILON.sqrt() * map.parameter_dimension.max(1) as f64 * 8.0;`
  - L1207: `&& recomputed.selected_rank == map.selected_rank`
  - L1208: `&& (recomputed.effective_rank - map.effective_rank).abs() <= tolerance`
  - L1210: `.abs()`
  - L1212: `&& (recomputed.fisher_trace - map.fisher_trace).abs() <= tolerance`
  - L1219: `(Some(left), Some(right)) => (left - right).abs() <= tolerance,`
  - L1229: `.all(\|(left, right)\| (left - right).abs() <= tolerance);`
  - L1237: `&& (stored.importance - current.importance).abs() <= tolerance`
  - L1243: `.all(\|(left, right)\| (left - right).abs() <= tolerance)`
  - L1250: `&& map.selected_rank == bundle.protection.selected_rank;`
  - L1262: `&& bundle.protection.selected_rank > 0`

#### `load_sleep_evidence`  ·  L1551
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `load_sleep_evidence_rejects_symlink_or_directory_current_pointer`  ·  L1612
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `sensitivity_loader_rejects_artifact_under_symlinked_parent`  ·  L1629
- **Ecuaciones / líneas clave:**
  - L1647: `"causal_damage_per_parameter_norm":1.0,`
  - L1657: `"causal_damage_per_parameter_norm":1.0,`

#### `sleep_evidence_recomputes_replay_and_rejects_summary_forgery`  ·  L1694
- **Ecuaciones / líneas clave:**
  - L1784: `{"probe_id":"p1","artifact":gradient_a,"causal_damage_per_parameter_norm":1.0,"reliability":1.0},`
  - L1785: `{"probe_id":"p2","artifact":gradient_b,"causal_damage_per_parameter_norm":2.0,"reliability":1.0}`

### `src/learning/procedural_memory.rs` (68 fns)

#### `is_zero_digest`  ·  L77
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `canonical_finite`  ·  L81
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `is_canonical_finite`  ·  L85
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `bind_exact_digest`  ·  L103
- **Doc:** Bind an already authenticated byte identity into this semantic domain.  Equal raw hashes in different domains remain distinct Rust types and receive different semantic digest values.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `deserialize`  ·  L302
- **Ecuaciones / líneas clave:**
  - L334: `LowRankObserved,`
  - L356: `estimated_effective_rank: Option<u64>,`

#### `new`  ·  L363
- **Ecuaciones / líneas clave:**
  - L366: `estimated_effective_rank: Option<u64>,`
  - L372: `estimated_effective_rank,`

#### `validate_axes`  ·  L379
- **Ecuaciones / líneas clave:**
  - L386: `.estimated_effective_rank`
  - L387: `.is_some_and(\|rank\| rank > min_dimension)`

#### `digest`  ·  L535
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `estimated_effective_rank`  ·  L548
- **Ecuaciones / líneas clave:**
  - L548: `pub fn estimated_effective_rank(&self) -> Option<u64> {`
  - L549: `self.dimensions.estimated_effective_rank`

#### `unit_interval`  ·  L557
- **Ecuaciones / líneas clave:**
  - L564: `CholeskyRidgeLowRank,`
  - L568: `DirectJacobiSvd,`
  - L570: `DivideConquerSvd,`
  - L573: `RandomizedSvd,`
  - L578: `FullRankGradient,`
  - L579: `OrthogonalizedFullRank,`
  - L594: `LowRank {`
  - L595: `rank: u32,`
  - L597: `RandomizedLowRank {`
  - L598: `rank: u32,`
  - L615: `FullRank,`
  - L627: `LowRank,`

#### `validate`  ·  L662
- **Ecuaciones / líneas clave:**
  - L676: `(SolverFamily::CholeskyRidgeLowRank, SolverParameters::LowRank { rank }) => *rank > 0,`
  - L678: `SolverFamily::RandomizedSvd,`
  - L679: `SolverParameters::RandomizedLowRank { rank, oversampling },`
  - L680: `) => *rank > 0 && *oversampling > 0 && rank.checked_add(*oversampling).is_some(),`
  - L682: `SolverFamily::DirectJacobiSvd`
  - L684: `\| SolverFamily::DivideConquerSvd,`
  - L706: `SolverFamily::FullRankGradient \| SolverFamily::OrthogonalizedFullRank,`
  - L707: `SolverParameters::FullRank,`
  - L724: `SolverFamily::CholeskyRidgeLowRank => {`
  - L730: `SolverFamily::DirectJacobiSvd => {`
  - L735: `SolverFamily::PivotedQr \| SolverFamily::DivideConquerSvd => {`
  - L746: `\| SolverFamily::FullRankGradient`
  - L747: `\| SolverFamily::OrthogonalizedFullRank => {`
  - L752: `SolverFamily::RandomizedSvd => {`

#### `validate_for`  ·  L771
- **Ecuaciones / líneas clave:**
  - L779: `SolverParameters::LowRank { rank } => u64::from(*rank) <= minimum_dimension,`
  - L780: `SolverParameters::RandomizedLowRank { rank, oversampling } => rank`
  - L782: `.is_some_and(\|sketch_rank\| u64::from(sketch_rank) <= minimum_dimension),`
  - L798: `\| SolverParameters::FullRank`

#### `rank`  ·  L813
- **Ecuaciones / líneas clave:**
  - L813: `pub fn rank(&self) -> Option<u32> {`
  - L815: `SolverParameters::LowRank { rank }`
  - L816: `\| SolverParameters::RandomizedLowRank { rank, .. } => Some(*rank),`

#### `digest`  ·  L821
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `problem_digest`  ·  L926
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `candidate_digest`  ·  L930
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `receipt_digest`  ·  L1053
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `research_evaluation_design`  ·  L1062
- **Ecuaciones / líneas clave:**
  - L1092: `RankInsufficient,`
  - L1093: `ResidualTooLarge,`

#### `kind`  ·  L1150
- **Ecuaciones / líneas clave:**
  - L1158: `IncreaseRank,`
  - L1159: `ReduceRank,`
  - L1162: `SwitchToSvd,`
  - L1167: `EscalateFullRank,`

#### `new`  ·  L1213
- **Doc:** Test/internal constructor. Production code derives these fields from a selected solver evaluation and its step-local independent gate.
- **Ecuaciones / líneas clave:**
  - L1214: `solver_relative_residual: Option<f64>,`
  - L1219: `solver_relative_residual: solver_relative_residual.map(canonical_finite).transpose()?,`

#### `positive_utility`  ·  L1298
- **Ecuaciones / líneas clave:**
  - L1299: `match (self.independent_gate_advanced, self.solver_relative_residual) {`
  - L1300: `(Some(true), Some(residual)) => Some(1.0 / (1.0 + residual.get())),`

#### `projection`  ·  L1581
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `calculate_digest`  ·  L1596
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L1637
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `configuration_matches_backend`  ·  L1664
- **Ecuaciones / líneas clave:**
  - L1669: `NumericalBackendRole::BuiltInCholeskyRidge => {`
  - L1670: `configuration.family == SolverFamily::CholeskyRidgeLowRank`
  - L1672: `NumericalBackendRole::BuiltInDirectJacobiSvd => {`
  - L1673: `configuration.family == SolverFamily::DirectJacobiSvd`

#### `failure_from_solver_rejection`  ·  L1691
- **Ecuaciones / líneas clave:**
  - L1693: `EvaluationReason::ResidualExceedsTolerance => {`
  - L1694: `(FailureKind::ResidualTooLarge, FailureSeverity::Serious)`
  - L1697: `CholeskyGate::RankDeficient`
  - L1701: `\| EvaluationReason::RankBudgetInsufficient => {`
  - L1702: `(FailureKind::RankInsufficient, FailureSeverity::Serious)`
  - L1713: `EvaluationReason::DirectSvdDidNotConverge => {`
  - L1716: `EvaluationReason::ResidualToleranceUnrepresentable => {`
  - L1722: `EvaluationReason::ResidualWithinTolerance`

#### `projection`  ·  L1843
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `calculate_digest`  ·  L1856
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L1883
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `projection`  ·  L2080
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `calculate_digest`  ·  L2095
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L2118
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `solver_run_receipt`  ·  L2127
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `solver_run_failure_outcome`  ·  L2131
- **Ecuaciones / líneas clave:**
  - L2139: `CholeskyGate::RankDeficient \| CholeskyGate::Degenerate,`
  - L2140: `) => SolverRunRejectionReason::RankDeficientForRequiredContract,`
  - L2144: `EvaluationReason::ResidualExceedsTolerance`
  - L2146: `\| EvaluationReason::RankBudgetInsufficient`
  - L2148: `\| EvaluationReason::ResidualWithinTolerance`
  - L2151: `\| EvaluationReason::ResidualToleranceUnrepresentable`
  - L2152: `\| EvaluationReason::DirectSvdDidNotConverge => {`
  - L2163: `EvaluationReason::DirectSvdDidNotConverge => {`
  - L2170: `\| EvaluationReason::ResidualToleranceUnrepresentable`
  - L2171: `\| EvaluationReason::RankBudgetInsufficient => {`
  - L2176: `\| EvaluationReason::ResidualWithinTolerance`
  - L2177: `\| EvaluationReason::ResidualExceedsTolerance`

#### `classify_solver_resource`  ·  L2187
- **Ecuaciones / líneas clave:**
  - L2197: `\| "solver_max_svd_work_units"`

#### `feature_bounded`  ·  L2255
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L2331
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `priority_score`  ·  L2449
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `advice`  ·  L2479
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `solver_run_cautions`  ·  L2494
- **Doc:** Exact-policy cautions from runs that produced no candidate. These are separate from configuration advice because attributing a portfolio preflight failure to one solver configuration would be false.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `solver_run_failure_count`  ·  L2593
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `derive_functional_change`  ·  L2784
- **Doc:** Derive only non-causal functional drift from reports already reduced into this memory. Revisions and invalidation boundaries come from the registered report-to-attempt index; callers cannot choose either.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `record_solver_run_failure`  ·  L2876
- **Doc:** Preserve a candidate-free solver failure without allowing it to become candidate evidence or promotion authority.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `rebuild_with_solver_failures`  ·  L2919
- **Doc:** Canonical replay including failures that occurred before candidate materialization.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `retrieve`  ·  L2952
- **Ecuaciones / líneas clave:**
  - L3010: `report.considered_records += 1;`
  - L3016: `report.applicability_filtered_records += 1;`
  - L3024: `report.drift_filtered_records += 1;`
  - L3032: `report.evidence_filtered_records += 1;`
  - L3036: `report.evidence_filtered_records += 1;`
  - L3046: `1.0 - (-(evaluation.independent_group_count as f64)`
  - L3052: `.exp(),`
  - L3063: `report.evidence_filtered_records += 1;`
  - L3072: `report.applicability_filtered_records += 1;`
  - L3082: `report.applicability_filtered_records += 1;`

#### `solver_run_receipt_semantics_equal`  ·  L3284
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `correction_is_realized`  ·  L3299
- **Ecuaciones / líneas clave:**
  - L3305: `CorrectionKind::IncreaseRank => matches!(`
  - L3306: `(parent.configuration.rank(), child.configuration.rank()),`
  - L3307: `(Some(parent_rank), Some(child_rank)) if child_rank > parent_rank`
  - L3309: `CorrectionKind::ReduceRank => matches!(`
  - L3310: `(parent.configuration.rank(), child.configuration.rank()),`
  - L3311: `(Some(parent_rank), Some(child_rank)) if child_rank < parent_rank`
  - L3321: `CorrectionKind::SwitchToSvd => matches!(`
  - L3323: `SolverFamily::DirectJacobiSvd \| SolverFamily::DivideConquerSvd`
  - L3339: `CorrectionKind::EscalateFullRank => matches!(`
  - L3341: `SolverFamily::FullRankGradient \| SolverFamily::OrthogonalizedFullRank`

#### `applicability_similarity`  ·  L3370
- **Ecuaciones / líneas clave:**
  - L3380: `let row_score = logarithmic_ratio(left.dimensions.rows, right.dimensions.rows);`
  - L3381: `let column_score = logarithmic_ratio(left.dimensions.columns, right.dimensions.columns);`
  - L3382: `let rank_score = match (`
  - L3383: `left.dimensions.estimated_effective_rank,`
  - L3384: `right.dimensions.estimated_effective_rank,`
  - L3386: `(Some(left_rank), Some(right_rank)) => Some(ratio_similarity(`
  - L3387: `left_rank,`
  - L3389: `right_rank,`
  - L3394: `let precision_score = if left.profile.precision == right.profile.precision {`
  - L3399: `let structure_score = if left.profile.structure == right.profile.structure {`
  - L3404: `let mut weighted_score = policy.projection.row_weight.get() * row_score`
  - L3408: `if let Some(rank_score) = rank_score {`
  - L3409: `weighted_score += policy.projection.rank_weight.get() * rank_score;`
  - L3413: `+ policy.projection.rank_weight.get()`
  - L3424: `weighted_score += policy.projection.condition_weight.get()`
  - L3425: `* (-(left.get() - right.get()).abs() / 2.0).exp();`

#### `add_optional_unit_similarity`  ·  L3454
- **Ecuaciones / líneas clave:**
  - L3461: `*weighted_score += weight * (1.0 - (left.get() - right.get()).abs());`

#### `ratio_similarity`  ·  L3469
- **Ecuaciones / líneas clave:**
  - L3477: `(1.0 - (left - right).abs()).clamp(0.0, 1.0)`

#### `raw_digest`  ·  L3492
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `sealed_digest`  ·  L3496
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `config`  ·  L3551
- **Ecuaciones / líneas clave:**
  - L3553: `SolverFamily::CholeskyRidgeLowRank => {`
  - L3554: `(SolverParameters::LowRank { rank: 2 }, Some(1e-6), Some(1e-9), None)`
  - L3556: `SolverFamily::RandomizedSvd => (`
  - L3557: `SolverParameters::RandomizedLowRank {`
  - L3558: `rank: 2,`
  - L3565: `SolverFamily::DirectJacobiSvd`
  - L3567: `\| SolverFamily::DivideConquerSvd => (`
  - L3571: `(family == SolverFamily::DirectJacobiSvd).then_some(1_000),`
  - L3599: `SolverFamily::FullRankGradient \| SolverFamily::OrthogonalizedFullRank => {`
  - L3600: `(SolverParameters::FullRank, None, Some(1e-9), Some(1_000))`

#### `tamper_and_semantic_relabel_are_rejected`  ·  L3763
- **Ecuaciones / líneas clave:**
  - L3768: `config(SolverFamily::DivideConquerSvd),`
  - L3795: `config(SolverFamily::DivideConquerSvd),`

#### `exact_scope_prevents_cross_project_and_cross_capability_advice`  ·  L3841
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `functional_drift_uses_registered_scope_and_revision_and_makes_advice_stale`  ·  L3873
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `unregistered_drift_reports_are_bounded_unknown_not_free_invalidation`  ·  L3918
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `failed_attempt_is_negative_memory_but_never_a_blocking_authority`  ·  L3939
- **Ecuaciones / líneas clave:**
  - L3945: `config(SolverFamily::DivideConquerSvd),`
  - L3953: `config(SolverFamily::CholeskyRidgeLowRank),`
  - L3965: `.find(\|value\| value.configuration().family() == SolverFamily::DivideConquerSvd)`
  - L3970: `.find(\|value\| value.configuration().family() == SolverFamily::CholeskyRidgeLowRank)`

#### `declared_external_backend_identity_never_becomes_transferable_advice`  ·  L3979
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `ranking_and_rebuild_are_invariant_to_record_order`  ·  L3997
- **Ecuaciones / líneas clave:**
  - L3997: `fn ranking_and_rebuild_are_invariant_to_record_order() {`
  - L4011: `config(SolverFamily::DivideConquerSvd),`

#### `no_independent_evidence_means_no_advice`  ·  L4029
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `lineage_cannot_cross_scope_or_apply_an_unproposed_correction`  ·  L4130
- **Ecuaciones / líneas clave:**
  - L4147: `config(SolverFamily::DivideConquerSvd),`
  - L4150: `Some(CorrectionKind::SwitchToSvd),`
  - L4167: `CorrectionKind::SwitchToSvd,`

#### `lineage_cannot_claim_a_correction_that_configuration_did_not_realize`  ·  L4182
- **Ecuaciones / líneas clave:**
  - L4188: `config(SolverFamily::CholeskyRidgeLowRank),`
  - L4209: `FailureKind::ResidualTooLarge,`
  - L4213: `Some(CorrectionRecord::new(CorrectionKind::SwitchToSvd, CorrectionStatus::Refuted)),`
  - L4220: `config(SolverFamily::CholeskyRidgeLowRank),`
  - L4223: `Some(CorrectionKind::SwitchToSvd),`

#### `reducer_deduplicates_exact_replay_and_enforces_resource_bounds`  ·  L4240
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `rank_unknown_is_distinct_from_observed_zero_and_missing_work_is_allowed`  ·  L4287
- **Ecuaciones / líneas clave:**
  - L4287: `fn rank_unknown_is_distinct_from_observed_zero_and_missing_work_is_allowed() {`
  - L4288: `let unknown_rank = Applicability::new(`
  - L4298: `let observed_zero_rank = Applicability::new(`
  - L4308: `assert_ne!(unknown_rank.digest().unwrap(), observed_zero_rank.digest().unwrap());`

#### `candidate_free_solver_failure_is_sealed_and_replay_safe`  ·  L4336
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `materialized_solver_rejection_is_a_typed_negative_attempt_not_a_run_failure`  ·  L4412
- **Ecuaciones / líneas clave:**
  - L4419: `.with_residual_tolerances(1.0e-12, 1.0e-12)`
  - L4451: `config(SolverFamily::DirectJacobiSvd),`

#### `solver_run_receipt_replay_is_not_new_evidence_and_relabel_is_rejected`  ·  L4499
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/learning/portfolio_governance.rs` (44 fns)

#### `from_projection`  ·  L118
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `metric_id`  ·  L272
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `metric_catalog_digest`  ·  L304
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L441
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L550
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `metric_catalog_digest`  ·  L554
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `evaluation_policy_digest`  ·  L558
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `independence_design_digest`  ·  L578
- **Doc:** Identity of the declared independent-group set, without metric values, candidate identity, repetitions, or observation time. This proves stable binding and detects reuse; it does not by itself prove that a caller-chosen label denotes a statistically independent source.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `projection`  ·  L582
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `stable_mean`  ·  L863
- **Ecuaciones / líneas clave:**
  - L872: `.map(\|value\| value.abs())`
  - L882: `if sum.abs() >= value.abs() {`
  - L883: `correction += (sum - updated) + value;`
  - L885: `correction += (value - updated) + sum;`

#### `digest`  ·  L1003
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L1078
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `report_digest`  ·  L1082
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `metric_catalog_digest`  ·  L1086
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `evaluation_policy_digest`  ·  L1090
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `independence_design_digest`  ·  L1094
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `projection`  ·  L1110
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `decide_candidate`  ·  L1209
- **Ecuaciones / líneas clave:**
  - L1342: `normalization_scale: FiniteF64,`
  - L1343: `maximum_normalized_endpoint_degradation: FiniteF64,`

#### `new`  ·  L1347
- **Ecuaciones / líneas clave:**
  - L1349: `normalization_scale: f64,`
  - L1350: `maximum_normalized_endpoint_degradation: f64,`
  - L1352: `let normalization_scale = FiniteF64::new(normalization_scale)?;`
  - L1353: `let maximum_normalized_endpoint_degradation =`
  - L1354: `FiniteF64::new(maximum_normalized_endpoint_degradation)?;`
  - L1355: `if normalization_scale.get() <= 0.0 \|\| maximum_normalized_endpoint_degradation.get() < 0.0 {`
  - L1360: `normalization_scale,`
  - L1361: `maximum_normalized_endpoint_degradation,`

#### `digest`  ·  L1538
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `quality_metric_id`  ·  L1542
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `quality_normalization_scale`  ·  L1546
- **Ecuaciones / líneas clave:**
  - L1546: `pub fn quality_normalization_scale(&self) -> BrainResult<f64> {`
  - L1549: `.map(\|metric\| metric.normalization_scale.get())`

#### `digest`  ·  L1796
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `endpoint_distance_lower`  ·  L1998
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L2026
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `interval_distance_bounds`  ·  L2096
- **Ecuaciones / líneas clave:**
  - L2126: `let scale = policy.normalization_scale.get();`
  - L2127: `lower_components.push((difference.abs() - radius).max(0.0) / scale);`
  - L2128: `upper_components.push((difference.abs() + radius) / scale);`

#### `finite_option`  ·  L2133
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L2631
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L2887
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L2948
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `adaptive_score`  ·  L2973
- **Ecuaciones / líneas clave:**
  - L2991: `let exploration = policy.exploration_strength.get() * (numerator.ln() / denominator).sqrt();`
  - L2993: `let score = empirical + exploration - policy.cost_penalty.get() * cost.ln_1p();`

#### `allocate_adaptive_budget`  ·  L3000
- **Ecuaciones / líneas clave:**
  - L3118: `*count += 1;`
  - L3148: `let score =`

#### `digest`  ·  L3301
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L3402
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `metric`  ·  L4044
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `eligible_candidate`  ·  L4152
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `absolute_level_and_paired_effect_have_separate_empirical_envelopes`  ·  L4230
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `petfc_direct_path_has_unit_tortuosity_and_zero_waste`  ·  L4346
- **Ecuaciones / líneas clave:**
  - L4371: `assert!((assessment.path_length_upper().unwrap() - 0.4).abs() < 1.0e-12);`
  - L4372: `assert!((assessment.endpoint_distance_lower().unwrap() - 0.4).abs() < 1.0e-12);`
  - L4373: `assert!((assessment.tortuosity_upper().unwrap() - 1.0).abs() < 1.0e-12);`
  - L4374: `assert!(assessment.waste_upper().unwrap().abs() < 1.0e-12);`

#### `petfc_reproduces_pythagorean_staircase_and_enforces_time_chain`  ·  L4379
- **Ecuaciones / líneas clave:**
  - L4431: `let expected = 2.0_f64.sqrt();`
  - L4432: `assert!((assessment.tortuosity_upper().unwrap() - expected).abs() < 1.0e-12);`
  - L4433: `assert!((assessment.waste_upper().unwrap() - (1.0 - 1.0 / expected)).abs() < 1.0e-12);`

#### `petfc_geometry_uses_incumbent_anchored_effects_and_rejects_duplicate_checkpoint`  ·  L4437
- **Ecuaciones / líneas clave:**
  - L4483: `let expected = 2.0_f64.sqrt();`
  - L4484: `assert!((assessment.path_length_upper().unwrap() - expected).abs() < 1.0e-12);`
  - L4485: `assert!((assessment.endpoint_distance_lower().unwrap() - expected).abs() < 1.0e-12);`

#### `petfc_invariant_overlap_is_bounded_unknown_not_rollback`  ·  L4490
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `adaptive_budget_fails_closed_when_mandatory_coverage_does_not_fit`  ·  L4657
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `adaptive_budget_rejects_unbounded_history_and_work_before_planning`  ·  L4693
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `adapter_promotion_witnesses_accept_one_fully_bound_chain`  ·  L4820
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/learning/numerical_evolution.rs` (20 fns)

#### `fit_metric_id`  ·  L57
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `worst_error_metric_id`  ·  L61
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `numerical_metric_specs`  ·  L66
- **Doc:** Exact metric catalog implemented by this numerical evaluator.
- **Ecuaciones / líneas clave:**
  - L67: `minimum_normalized_fit: f64,`
  - L68: `maximum_normalized_worst_error: f64,`
  - L70: `if !(0.0..=1.0).contains(&minimum_normalized_fit)`
  - L71: `\|\| !maximum_normalized_worst_error.is_finite()`
  - L72: `\|\| maximum_normalized_worst_error < 0.0`
  - L81: `minimum_normalized_fit,`
  - L88: `maximum_normalized_worst_error,`

#### `solver`  ·  L229
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `exact_digest`  ·  L260
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `solver_run_failure`  ·  L387
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `solver_report`  ·  L391
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `derive_solver_run_failure`  ·  L715
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `observe_functional_change`  ·  L740
- **Doc:** Record only a non-causal functional change proven by two reports that this engine actually reduced. Detection and invalidation revisions are recovered from procedural memory; the caller cannot choose them or a causal drift label. This deterministic adapter suppresses exact replay, so temporal drift remains reserved for future instrumented frontends that can supply genuinely new authenticated observations.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `derive_observations`  ·  L777
- **Ecuaciones / líneas clave:**
  - L804: `let target_rms = baseline_metrics.target_norm() / (scalar_count as f64).sqrt();`
  - L806: `let baseline_fit = 1.0 / (1.0 + baseline_metrics.root_mean_square_residual() / scale);`
  - L807: `let candidate_fit = 1.0 / (1.0 + candidate_metrics.root_mean_square_residual() / scale);`
  - L808: `let baseline_worst = baseline_metrics.maximum_absolute_residual() / scale;`
  - L809: `let candidate_worst = candidate_metrics.maximum_absolute_residual() / scale;`

#### `variant_from_digest`  ·  L1034
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `research_trial_digest`  ·  L1067
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `experimental_input_digests`  ·  L1102
- **Doc:** Conservative structural identity for an experimental input. Targets are deliberately excluded: changing a label by one bit must not make a reused input appear independent. A future adapter may admit repeated measurements only by supplying an authenticated sample/provenance identity.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `applicability_from_report`  ·  L1129
- **Ecuaciones / líneas clave:**
  - L1153: `u64::try_from(diagnostics.numerical_rank())`
  - L1154: `.map_err(\|_\| invalid("numerical_rank_overflow"))`

#### `configuration_from_selected`  ·  L1175
- **Ecuaciones / líneas clave:**
  - L1183: `let tolerance = if policy.relative_residual_tolerance() > 0.0 {`
  - L1184: `policy.relative_residual_tolerance()`
  - L1186: `policy.absolute_residual_tolerance()`
  - L1189: `NumericalBackendRole::BuiltInCholeskyRidge => SolverConfiguration::new(`
  - L1190: `SolverFamily::CholeskyRidgeLowRank,`
  - L1191: `SolverParameters::LowRank {`
  - L1192: `rank: u32::try_from(`
  - L1194: `.constructed_rank()`
  - L1195: `.ok_or_else(\|\| integrity("numerical_cholesky_rank_missing"))?,`
  - L1197: `.map_err(\|_\| invalid("numerical_cholesky_rank_overflow"))?,`
  - L1203: `NumericalBackendRole::BuiltInDirectJacobiSvd => SolverConfiguration::new(`
  - L1204: `SolverFamily::DirectJacobiSvd,`
  - L1209: `u32::try_from(policy.max_svd_sweeps())`
  - L1210: `.map_err(\|_\| invalid("numerical_svd_sweep_overflow"))?,`
  - L1220: `CandidateRepresentation::LowRank { .. } => SolverRepresentationKind::LowRank,`

#### `evaluator_policy_digest`  ·  L1236
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `exact_sparse_proposal`  ·  L1401
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `terminal_solver_result_consumes_its_revision`  ·  L1608
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `pre_materialization_solver_limit_uses_candidate_free_record`  ·  L1637
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `deterministic_repeat_is_not_fresh_evidence_or_functional_drift`  ·  L1831
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/learning/learning_orchestrator.rs` (20 fns)

#### `default_cost_weight`  ·  L58
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `default_risk_weight`  ·  L61
- **Ecuaciones / líneas clave:**
  - L83: `pub design_rank: usize,`

#### `normalized`  ·  L254
- **Ecuaciones / líneas clave:**
  - L254: `fn normalized(mut values: Vec<f64>) -> BrainResult<Vec<f64>> {`
  - L255: `let magnitude = norm(&values)?;`

#### `rank`  ·  L375
- **Ecuaciones / líneas clave:**
  - L375: `fn rank(rows: &[Vec<f64>]) -> BrainResult<usize> {`
  - L394: `let tolerance = max * f64::EPSILON.sqrt() * (gram.rows.max(1) as f64);`

#### `plan_autonomous_learning`  ·  L398
- **Ecuaciones / líneas clave:**
  - L424: `if weight.abs() > 1e-12 {`
  - L425: `coverage[index] += 1;`
  - L437: `let design_rank = rank(&selected_rows)?;`
  - L440: `if design_rank < target.capability_ids.len() \|\| minimum_capability_coverage < 3 {`
  - L442: `"learning_plan_not_identifiable:rank={design_rank}:dimension={}:min_coverage={minimum_capability_coverage}",`
  - L453: `design_rank,`

#### `target_digest`  ·  L461
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `policy_digest`  ·  L467
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `same_f64`  ·  L530
- **Ecuaciones / líneas clave:**
  - L533: `&& (left - right).abs() <= f64::EPSILON.sqrt() * 64.0 * (1.0 + left.abs().max(right.abs()))`

#### `next_learning_aperture`  ·  L625
- **Ecuaciones / líneas clave:**
  - L656: `let score = choose_active_aperture(`
  - L662: `let predicted_outcome = dot(&candidate.sensing_vector, &session.posterior.mean)?;`
  - L694: `.filter(\|(_, value)\| value.abs() > 1e-12)`

#### `assimilate_learning_result`  ·  L712
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `validate_experiment_evidence`  ·  L881
- **Ecuaciones / líneas clave:**
  - L952: `let derived = dot(capability_weights, &observation.functional_response)?;`
  - L953: `let tolerance = f64::EPSILON.sqrt()`
  - L954: `* (1.0 + derived.abs().max(evidence.observed_value.abs()))`
  - L957: `if (derived - evidence.observed_value).abs() > tolerance {`

#### `verify_receipt_ledger_binding`  ·  L1077
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `issue_next_persistent_learning_aperture_under_root`  ·  L1429
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `issue_next_persistent_learning_aperture`  ·  L1464
- **Doc:** Atomically issue exactly one canonical next aperture. A second issue is rejected until an actual evidence envelope has been assimilated.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `assimilate_persistent_learning_evidence_under_root`  ·  L1502
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `assimilate_persistent_learning_evidence`  ·  L1572
- **Doc:** Assimilate a real, content-addressed experiment evidence envelope. Missing, changed or out-of-root evidence is a hard error; no outcome is synthesized.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `learning_plan_is_full_rank_and_covers_every_target`  ·  L1728
- **Ecuaciones / líneas clave:**
  - L1728: `fn learning_plan_is_full_rank_and_covers_every_target() {`
  - L1741: `assert_eq!(plan.design_rank, 10);`

#### `adaptive_learning_assimilates_realized_result_before_next_choice`  ·  L1767
- **Ecuaciones / líneas clave:**
  - L1795: `.any(\|value\| value.abs() > 1e-12)`

#### `realized_outcome_changes_the_next_canonical_aperture`  ·  L1800
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `persistent_cycle_rejects_unbound_outcomes_and_tampered_receipts`  ·  L1814
- **Ecuaciones / líneas clave:**
  - L1848: `malformed.observed_value += 1.0;`

### `src/learning/learning_finalization.rs` (1 fns)

#### `promoted_semantic_digest_is_not_the_staged_file_digest`  ·  L399
- **Ecuaciones / líneas clave:**
  - L401: `observation_id: ObservationId::parse("obs-bridge").unwrap(),`
  - L409: `independence_group: "aperture-bridge".into(),`
  - L416: `Sha256Digest::digest_bytes(b"obs-bridge"),`

### `src/learning/memory.rs` (1 fns)

#### `build_memory_snapshot`  ·  L93
- **Ecuaciones / líneas clave:**
  - L163: `total_weight += weight;`
  - L165: `*value += weight * observations[index].functional_response[output];`

### `src/learning/representation_evidence.rs`
_Sin funciones algorítmicas filtradas (revisar a mano si es glue)._

## Motor / campo cognitivo / controlador

### `src/engine/learned_controller.rs` (12 fns)

#### `fit_weights`  ·  L284
- **Ecuaciones / líneas clave:**
  - L287: `ridge: f64,`
  - L300: `weights.push(weighted_normal_solve(&design, &target, &reliability, ridge)?);`

#### `grouped_cv_r2`  ·  L320
- **Ecuaciones / líneas clave:**
  - L323: `ridge: f64,`
  - L348: `let weights = fit_weights(&train, coefficient_dim, ridge)?;`

#### `train_learned_controller`  ·  L358
- **Ecuaciones / líneas clave:**
  - L360: `ridge: f64,`
  - L364: `if !ridge.is_finite()`
  - L365: `\|\| ridge <= 0.0`
  - L371: `let weights = fit_weights(examples, coefficient_dim, ridge)?;`
  - L387: `squared_error += (target - predicted) * (target - predicted);`
  - L388: `values += 1;`
  - L406: `training_rms: (squared_error / values.max(1) as f64).sqrt(),`
  - L407: `grouped_cv_r2: grouped_cv_r2(examples, coefficient_dim, ridge)?,`

#### `train_runtime_learned_controller`  ·  L412
- **Ecuaciones / líneas clave:**
  - L415: `ridge: f64,`
  - L419: `let controller = train_learned_controller(examples, ridge, ood_margin_fraction)?;`

#### `controller_policy_digest`  ·  L448
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `validate_persisted_controller_policy`  ·  L462
- **Ecuaciones / líneas clave:**
  - L464: `\|\| !policy.ridge.is_finite()`
  - L465: `\|\| policy.ridge <= 0.0`

#### `verify_reconstruction_report_binding`  ·  L617
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `verify_finalization_binding_under_root`  ·  L635
- **Doc:** The engine-owned finalization receipt is the sole authority that can bridge the immutable raw experiment artifact to a semantic observation digest in the promoted runtime corpus. A controller never accepts that digest from a data set or supervision label.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `same_supervision_coefficients`  ·  L785
- **Ecuaciones / líneas clave:**
  - L788: `(left - right).abs() <= f64::EPSILON.sqrt() * 64.0 * (1.0 + left.abs().max(right.abs()))`

#### `same_controller_float`  ·  L792
- **Ecuaciones / líneas clave:**
  - L795: `&& (left - right).abs() <= f64::EPSILON.sqrt() * 64.0 * (1.0 + left.abs().max(right.abs()))`

#### `verify_controller_ledger_binding`  ·  L988
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `learned_controller_decide_and_training_validation_errors`  ·  L1877
- **Ecuaciones / líneas clave:**
  - L1911: `let neg_ridge_ex = vec![ControllerExample {`
  - L1918: `assert!(train_learned_controller(&neg_ridge_ex, -1.0, 0.25).is_err());`

### `src/engine/cognitive_field.rs` (7 fns)

#### `validate_config`  ·  L111
- **Ecuaciones / líneas clave:**
  - L129: `\|\| !config.decay.is_finite()`
  - L130: `\|\| config.decay <= 0.0`

#### `curvature_similarity`  ·  L159
- **Ecuaciones / líneas clave:**
  - L160: `let denom = (curvature.get(left, left).max(0.0) * curvature.get(right, right).max(0.0)).sqrt();`

#### `functional_similarity`  ·  L186
- **Ecuaciones / líneas clave:**
  - L195: `cosine(&left.functional_signature, &right.functional_signature)`

#### `attractor_digest`  ·  L200
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `build`  ·  L213
- **Ecuaciones / líneas clave:**
  - L308: `.map(\|effect\| (effect / pair_scale).tanh())`
  - L334: `degree += weight;`
  - L349: `config.causal_bias_weight * (effect / marginal_scale).tanh()`

#### `evolve`  ·  L379
- **Ecuaciones / líneas clave:**
  - L411: `.map(\|value\| (self.config.beta * value).tanh())`
  - L422: `- self.config.decay * state[index]`
  - L428: `final_max_delta = final_max_delta.max((candidate - state[index]).abs());`

#### `cognitive_field_rejects_indefinite_curvature`  ·  L744
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/engine/parametric_program.rs` (10 fns)

#### `compose_skill_fields`  ·  L41
- **Doc:** Compose one parameter-space operator from SkillFields.
- **Ecuaciones / líneas clave:**
  - L49: `*dst += alpha * value;`
  - L57: `/// this does not assume coefficient_k = <H_k, operator>. It solves the normal`
  - L58: `/// equations G alpha = H^T operator with a small ridge and then verifies the`

#### `compile_operator_to_fields`  ·  L60
- **Doc:** Compile a target operator into coefficients over an arbitrary, potentially non-orthogonal SkillField basis. Unlike the original prototype, this does not assume coefficient_k = <H_k, operator>. It solves the normal equations G alpha = H^T operator with a small ridge and then verifies the reconstruction residual fail-closed.
- **Ecuaciones / líneas clave:**
  - L63: `ridge: f64,`
  - L64: `max_relative_residual: f64,`
  - L69: `\|\| !ridge.is_finite()`
  - L70: `\|\| ridge < 0.0`
  - L71: `\|\| !max_relative_residual.is_finite()`
  - L72: `\|\| max_relative_residual < 0.0`
  - L77: `let rank = fields.len();`
  - L78: `let mut gram = Matrix::zeros(rank, rank);`
  - L79: `let mut rhs = vec![0.0; rank];`
  - L80: `for i in 0..rank {`
  - L81: `rhs[i] = dot(&fields[i].direction, operator)?;`
  - L82: `for j in 0..rank {`
  - L83: `gram.set(i, j, dot(&fields[i].direction, &fields[j].direction)?);`
  - L87: `for i in 0..rank {`
  - L88: `gram.set(i, i, gram.get(i, i) + ridge);`
  - L92: `let residual = reconstructed`
  - L97: `let relative_residual = norm(&residual)? / norm(operator)?.max(1e-15);`
  - L98: `if relative_residual > max_relative_residual {`
  - L100: `"parametric_program_operator_outside_skill_span:{relative_residual:.6e}"`
  - L105: `relative_residual,`

#### `apply_parametric_transition`  ·  L113
- **Doc:** Generic linear recurrent state transition. The runtime knows only matrix multiplication and winner selection; the transition semantics live in the composed operator values.
- **Ecuaciones / líneas clave:**
  - L128: `scores[row] += operator[row * state_dim + col] * state[col];`

#### `ties_merge`  ·  L169
- **Ecuaciones / líneas clave:**
  - L186: `.abs()`
  - L187: `.total_cmp(&operator[*a].abs())`

#### `nonorthogonal_fields_compile_both_operators`  ·  L263
- **Ecuaciones / líneas clave:**
  - L267: `assert!(compiled.relative_residual < 1e-8);`
  - L270: `norm(`

#### `generic_parametric_transition_remains_available_without_token_router`  ·  L284
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `test_report`  ·  L291
- **Ecuaciones / líneas clave:**
  - L291: `fn test_report(fields: Vec<SkillField>, rank: usize) -> ReconstructionReport {`
  - L308: `"effective_group_rank": 1.0,`
  - L310: `"numerical_design_rank": 1,`
  - L325: `"field_geometry_numerical_rank": 0,`
  - L326: `"excitation_numerical_rank": 0,`
  - L327: `"resolved_rank": 0,`
  - L341: `"max_edge_residual": 0.0,`
  - L342: `"selected_rank": {rank},`
  - L343: `"effective_rank": {rank}.0,`
  - L346: `"normalized_reconstruction_rms": 0.0,`
  - L365: `"representation_mean_matched_cosine": null,`
  - L379: `"field_count": {rank}.0`

#### `fields_from_report_and_validation`  ·  L388
- **Ecuaciones / líneas clave:**
  - L396: `report.selected_rank = 3;`
  - L401: `report.selected_rank = 0;`

#### `compose_and_compile_error_paths`  ·  L419
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `parametric_transition_error_paths`  ·  L452
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/engine/analysis.rs` (3 fns)

#### `materialize_dense_fields`  ·  L72
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `verify_dense_field_materializations`  ·  L136
- **Doc:** Re-derive dense field references without creating artifacts.  A receipt verifier must never materialize a missing candidate as a side effect: the exact content-addressed dvec must already exist and match the same f64-to-f32 arithmetic used at promotion time.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `analyze_canonical`  ·  L479
- **Ecuaciones / líneas clave:**
  - L487: `let sbas = reconstruct_trajectory(obs, self.config.ridge)?;`
  - L489: `analyze_weight_dynamics(obs, sbas.cycle_rms, sbas.max_edge_residual)?;`
  - L491: `let conf = remove_confounders(obs, self.config.ridge)?;`
  - L514: `.or_default() += 1;`
  - L527: `/ (group_counts[o.independence_group.as_str()] as f64).sqrt(),`
  - L540: `reconstruct_skill_fields(&conf.residuals, &weights, &groups, generation, &self.config)?;`
  - L544: `self.config.ridge,`
  - L549: `let spectral_normalized_rms = spectral.reconstruction_rms / input_rms;`
  - L581: `selected_rank,`
  - L582: `effective_rank,`
  - L585: `normalized_reconstruction_rms,`
  - L593: `persistent.selected_rank,`
  - L594: `persistent.effective_rank,`
  - L597: `persistent.normalized_reconstruction_rms,`
  - L606: `spectral.selected_rank,`
  - L607: `spectral.effective_rank,`
  - L610: `spectral_normalized_rms,`

### `src/engine/runtime.rs` (3 fns)

#### `load_verified_runtime_evidence`  ·  L511
- **Ecuaciones / líneas clave:**
  - L521: `let normalized = normalize_reconstruction_report_wire(&reconstruction)?;`
  - L522: `reconstruction = normalized.0;`
  - L523: `let reconstruction_bytes = normalized.1;`

#### `compose_verified_inputs`  ·  L729
- **Ecuaciones / líneas clave:**
  - L792: `if coefficient.abs() <= f64::EPSILON {`
  - L821: `*output += coefficient * f64::from(value);`
  - L824: `if delta.iter().all(\|value\| value.abs() <= f64::EPSILON) {`

#### `skill_subspace_overlap`  ·  L1528
- **Ecuaciones / líneas clave:**
  - L1538: `let truth_norm = norm(truth_direction)?.max(1e-15);`
  - L1541: `let value = crate::foundation::linalg::dot(truth_direction, &field.direction)?;`
  - L1542: `let squared = value.abs() * value.abs();`
  - L1556: `let ratio = projected.sqrt() / truth_norm;`

### `src/engine/transition.rs` (2 fns)

#### `commit_after_verified_corpus_transition`  ·  L280
- **Ecuaciones / líneas clave:**
  - L302: `let normalized = normalize_reconstruction_report_wire(&report)?;`
  - L303: `report = normalized.0;`
  - L304: `let report_bytes = normalized.1;`
  - L332: `assimilate_bank(&mut shadow_bank, &report.fields, self.config.skill_match_cosine)?;`

#### `sleep_cycle_under_authority`  ·  L1191
- **Ecuaciones / líneas clave:**
  - L1209: `let normalized = normalize_reconstruction_report_wire(&reconstruction)?;`
  - L1210: `reconstruction = normalized.0;`
  - L1211: `let report_bytes = normalized.1;`
  - L1241: `self.config.skill_match_cosine,`
  - L1294: `self.config.skill_match_cosine,`
  - L1303: `self.config.skill_match_cosine,`

### `src/engine/support.rs` (3 fns)

#### `normalize_reconstruction_report_wire`  ·  L15
- **Ecuaciones / líneas clave:**
  - L15: `pub(super) fn normalize_reconstruction_report_wire(`
  - L19: `let normalized: ReconstructionReport = serde_json::from_slice(&first_bytes)?;`
  - L20: `let canonical_bytes = serialize_pretty_line(&normalized)?;`
  - L22: `if stable != normalized {`
  - L27: `Ok((normalized, canonical_bytes))`

#### `verify_learning_finalization_commit_binding`  ·  L870
- **Ecuaciones / líneas clave:**
  - L893: `assimilate_bank(&mut shadow_bank, &report.fields, BrainConfig::default().skill_match_cosine)?;`

#### `validate_brain_config`  ·  L1671
- **Ecuaciones / líneas clave:**
  - L1680: `\|\| config.max_rank == 0`
  - L1681: `\|\| config.max_rank > MAX_ENGINE_SKILL_FIELDS`
  - L1684: `\|\| !config.ridge.is_finite()`
  - L1685: `\|\| config.ridge <= 0.0`
  - L1690: `\|\| !config.skill_match_cosine.is_finite()`
  - L1691: `\|\| !(0.0..=1.0).contains(&config.skill_match_cosine)`
  - L1705: `.max_spectral_normalized_reconstruction_rms`
  - L1707: `\|\| config.max_spectral_normalized_reconstruction_rms < 0.0`

## Materialización shadow

### `src/materialization/low_rank_shadow_materializer.rs` (14 fns)

#### `validate`  ·  L44
- **Ecuaciones / líneas clave:**
  - L45: `if self.schema != "cerebro.tidex.low_rank_shadow_policy/v1"`
  - L46: `\|\| self.maximum_rank == 0`
  - L47: `\|\| self.maximum_rank > MAX_SHADOW_RANK`
  - L56: `\|\| self.maximum_svd_sweeps == 0`
  - L57: `\|\| self.maximum_svd_sweeps > MAX_SVD_SWEEPS`
  - L59: `return Err(BrainError::Invalid("low_rank_shadow_policy_invalid".into()));`

#### `digest`  ·  L64
- **Ecuaciones / líneas clave:**
  - L67: `b"CEREBRO:TIDEX:LOW-RANK-SHADOW-POLICY:v1\0",`
  - L75: `pub struct VerifiedLowRankFactors {`
  - L79: `pub rank: usize,`
  - L89: `impl VerifiedLowRankFactors {`

#### `materialize_dense`  ·  L90
- **Ecuaciones / líneas clave:**
  - L91: `CandidateRepresentation::LowRank {`
  - L94: `rank: self.rank,`

#### `factor_dense_delta_verified`  ·  L102
- **Ecuaciones / líneas clave:**
  - L106: `policy: &LowRankShadowPolicy,`
  - L107: `) -> BrainResult<VerifiedLowRankFactors> {`
  - L111: `.ok_or_else(\|\| BrainError::Invalid("low_rank_dense_shape_overflow".into()))?;`
  - L115: `.ok_or_else(\|\| BrainError::Invalid("low_rank_gram_shape_overflow".into()))?;`
  - L123: `return Err(BrainError::Invalid("low_rank_dense_input_invalid".into()));`
  - L125: `let target_norm = norm(dense)?;`
  - L128: `.map(\|value\| value.abs())`
  - L131: `return Err(BrainError::Numerical("low_rank_zero_delta_has_no_factors".into()));`
  - L135: `.and_then(\|value\| value.checked_mul(policy.maximum_svd_sweeps))`
  - L136: `.ok_or_else(\|\| BrainError::Invalid("low_rank_svd_work_overflow".into()))?;`
  - L137: `if rotations > MAX_SVD_ROTATIONS {`
  - L138: `return Err(BrainError::Invalid("low_rank_svd_work_limit".into()));`
  - L142: `.is_none_or(\|work\| work > MAX_SVD_ROTATIONS)`
  - L144: `return Err(BrainError::Invalid("low_rank_gram_work_limit".into()));`
  - L175: `let maximum_rank = policy.maximum_rank.min(eigen.len());`
  - L176: `for rank in 1..=maximum_rank {`
  - L178: `.checked_mul(rank)`
  - L179: `.and_then(\|value\| value.checked_add(rank.checked_mul(columns)?))`
  - L180: `.ok_or_else(\|\| BrainError::Invalid("low_rank_factor_shape_overflow".into()))?;`
  - L181: `let parameter_reduction_ratio = 1.0 - factor_count as f64 / dense_count as f64;`
  - L185: `let mut left = vec![0.0; rows * rank];`
  - L186: `let mut right = vec![0.0; rank * columns];`
  - L187: `for component in 0..rank {`
  - L189: `let scaled_sigma = eigen[component].0.sqrt();`
  - L190: `let balanced_scale = scale.sqrt() * scaled_sigma.sqrt();`

#### `values_digest`  ·  L286
- **Ecuaciones / líneas clave:**
  - L288: `b"CEREBRO:TIDEX:SHADOW-LOW-RANK-DENSE-VALUES:v1\0",`
  - L293: `impl ShadowLowRankCandidate {`

#### `calculate_digest`  ·  L294
- **Ecuaciones / líneas clave:**
  - L304: `b"CEREBRO:TIDEX:SHADOW-LOW-RANK-CANDIDATE:v1\0",`

#### `validate`  ·  L309
- **Ecuaciones / líneas clave:**
  - L314: `policy: &LowRankShadowPolicy,`
  - L321: `if shadow.materialization_plan.strategy != MaterializationStrategy::LowRank {`
  - L322: `return Err(BrainError::Invalid("shadow_low_rank_strategy_required".into()));`
  - L324: `if self.schema != "cerebro.tidex.shadow_low_rank_candidate/v1"`
  - L331: `return Err(BrainError::Integrity("shadow_low_rank_binding_invalid".into()));`
  - L334: `return Err(BrainError::Integrity("shadow_low_rank_manifest_invalid".into()));`
  - L351: `.map_err(\|_\| BrainError::Invalid("shadow_low_rank_offset_overflow".into()))?;`
  - L354: `.ok_or_else(\|\| BrainError::Invalid("shadow_low_rank_range_overflow".into()))?;`
  - L357: `.ok_or_else(\|\| BrainError::Integrity("shadow_low_rank_range_invalid".into()))?;`
  - L361: `"shadow_low_rank_nonzero_outside_planned_regions".into(),`
  - L368: `.ok_or_else(\|\| BrainError::Integrity("shadow_low_rank_tensor_missing".into()))?;`
  - L372: `.checked_mul(factors.rank)`
  - L373: `.and_then(\|count\| count.checked_add(factors.rank.checked_mul(factors.columns)?))`
  - L374: `.ok_or_else(\|\| BrainError::Invalid("low_rank_factor_shape_overflow".into()))?;`
  - L375: `let expected_reduction = 1.0 - factor_count as f64 / block.count as f64;`
  - L382: `\|\| factors.schema != "cerebro.tidex.verified_low_rank_factors/v1"`
  - L385: `\|\| factors.rank == 0`
  - L386: `\|\| factors.rank > policy.maximum_rank`
  - L387: `\|\| factors.left.len() != factors.rows * factors.rank`
  - L388: `\|\| factors.right.len() != factors.rank * factors.columns`
  - L396: `\|\| (factors.parameter_reduction_ratio - expected_reduction).abs() > 1e-12`
  - L400: `"shadow_low_rank_factor_contract_invalid".into(),`
  - L404: `let residual = dense`
  - L409: `let absolute = norm(&residual)?;`
  - L410: `let target_norm = norm(dense)?;`

#### `build_candidate`  ·  L432
- **Ecuaciones / líneas clave:**
  - L436: `policy: &LowRankShadowPolicy,`
  - L437: `) -> BrainResult<ShadowLowRankCandidate> {`
  - L442: `if shadow.materialization_plan.strategy != MaterializationStrategy::LowRank {`
  - L443: `return Err(BrainError::Invalid("shadow_low_rank_strategy_required".into()));`
  - L449: `.map_err(\|_\| BrainError::Invalid("shadow_low_rank_target_overflow".into()))?`
  - L452: `return Err(BrainError::Integrity("shadow_low_rank_target_invalid".into()));`
  - L469: `.map_err(\|_\| BrainError::Invalid("shadow_low_rank_offset_overflow".into()))?;`
  - L472: `.ok_or_else(\|\| BrainError::Invalid("shadow_low_rank_range_overflow".into()))?;`
  - L475: `.ok_or_else(\|\| BrainError::Integrity("shadow_low_rank_range_invalid".into()))?;`
  - L478: `return Err(BrainError::Invalid("shadow_low_rank_tensor_must_be_matrix".into()));`
  - L480: `tensors.push(ShadowLowRankTensorDelta {`
  - L495: `"shadow_low_rank_nonzero_outside_planned_regions".into(),`
  - L499: `let mut candidate = ShadowLowRankCandidate {`
  - L500: `schema: "cerebro.tidex.shadow_low_rank_candidate/v1".into(),`

#### `materialize_replayed_low_rank_shadow`  ·  L517
- **Ecuaciones / líneas clave:**
  - L517: `pub fn materialize_replayed_low_rank_shadow(`
  - L521: `policy: &LowRankShadowPolicy,`
  - L522: `) -> BrainResult<ShadowLowRankCandidate> {`

#### `persist_low_rank_shadow`  ·  L528
- **Ecuaciones / líneas clave:**
  - L528: `pub fn persist_low_rank_shadow(`
  - L533: `policy: &LowRankShadowPolicy,`
  - L534: `candidate: &ShadowLowRankCandidate,`
  - L539: `.map_err(\|_\| BrainError::Invalid("shadow_low_rank_size_overflow".into()))?`
  - L540: `> MAX_LOW_RANK_SHADOW_BYTES`
  - L542: `return Err(BrainError::Invalid("shadow_low_rank_candidate_too_large".into()));`
  - L546: `.join("low-rank-shadow-candidates")`

#### `load_low_rank_shadow`  ·  L552
- **Ecuaciones / líneas clave:**
  - L552: `pub fn load_low_rank_shadow(`
  - L557: `policy: &LowRankShadowPolicy,`
  - L559: `) -> BrainResult<ShadowLowRankCandidate> {`
  - L560: `let bytes = reference.read_verified_bounded(roots.staging_root(), MAX_LOW_RANK_SHADOW_BYTES)?;`
  - L561: `let candidate: ShadowLowRankCandidate = serde_json::from_slice(&bytes)?;`
  - L564: `.join("low-rank-shadow-candidates")`
  - L567: `return Err(BrainError::Integrity("shadow_low_rank_candidate_path_invalid".into()));`

#### `policy`  ·  L577
- **Ecuaciones / líneas clave:**
  - L577: `fn policy(maximum_rank: usize) -> LowRankShadowPolicy {`
  - L578: `LowRankShadowPolicy {`
  - L579: `schema: "cerebro.tidex.low_rank_shadow_policy/v1".into(),`
  - L580: `maximum_rank,`
  - L584: `maximum_svd_sweeps: 100,`

#### `exact_low_rank_delta_is_factored_and_full_rank_delta_is_rejected`  ·  L589
- **Ecuaciones / líneas clave:**
  - L589: `fn exact_low_rank_delta_is_factored_and_full_rank_delta_is_rejected() {`
  - L597: `assert_eq!(factors.rank, 1);`
  - L604: `.all(\|(expected, actual)\| (expected - actual).abs() < 1.0e-10)`

#### `rectangular_orientations_and_finite_scaling_reconstruct`  ·  L614
- **Ecuaciones / líneas clave:**
  - L620: `.map(\|index\| (index as f64 + 0.5) / scale.sqrt())`
  - L630: `assert_eq!(factors.rank, 1);`

### `src/materialization/sparse_shadow_materializer.rs` (8 fns)

#### `digest`  ·  L63
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `values_digest`  ·  L109
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `sparsify`  ·  L116
- **Ecuaciones / líneas clave:**
  - L127: `let target_norm = norm(values)?;`
  - L128: `let mut ranked = values`
  - L132: `.filter(\|(_, value)\| value.abs() > policy.absolute_zero_threshold)`
  - L134: `ranked.sort_by(\|(ia, a), (ib, b)\| {`
  - L135: `b.abs()`
  - L136: `.partial_cmp(&a.abs())`
  - L140: `ranked.truncate(budget.min(ranked.len()));`
  - L141: `ranked.sort_by_key(\|(index, _)\| *index);`
  - L142: `let coordinates = ranked`
  - L161: `let residual = values`
  - L166: `let absolute = norm(&residual)?;`
  - L167: `let relative = if target_norm == 0.0 {`
  - L170: `absolute / target_norm`
  - L172: `let reconstructed_norm = norm(&reconstructed)?;`
  - L173: `let retained = if target_norm == 0.0 {`
  - L176: `(reconstructed_norm / target_norm).powi(2).clamp(0.0, 1.0)`

#### `calculate_digest`  ·  L187
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `materialize_replayed_sparse_shadow`  ·  L335
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `persist_sparse_shadow`  ·  L346
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `load_sparse_shadow`  ·  L371
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `deterministic_top_magnitude_sparse_encoding_and_rejection`  ·  L409
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/materialization/dense_shadow_materializer.rs`
_Sin funciones algorítmicas filtradas (revisar a mano si es glue)._

### `src/materialization/activation_steering_materializer.rs` (10 fns)

#### `calculate_digest`  ·  L173
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L207
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `normalize`  ·  L241
- **Ecuaciones / líneas clave:**
  - L241: `fn normalize(mut vector: Vec<f64>, normalization: SteeringNormalization) -> BrainResult<Vec<f64>> {`
  - L242: `let magnitude = norm(&vector)?;`
  - L243: `match normalization {`
  - L244: `SteeringNormalization::None => {}`
  - L245: `SteeringNormalization::UnitL2 if magnitude > 0.0 => {`
  - L248: `SteeringNormalization::RootMeanSquare if magnitude > 0.0 => {`
  - L249: `let rms = magnitude / (vector.len() as f64).sqrt();`

#### `target_digest`  ·  L257
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `calculate_digest`  ·  L265
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `build_candidate`  ·  L288
- **Ecuaciones / líneas clave:**
  - L318: `if !affected.contains(&hook.source_tensor_id) \|\| hook.gain.abs() > policy.maximum_gain {`
  - L341: `let vector = normalize(raw, hook.normalization)?`
  - L345: `let vector_l2 = norm(&vector)?;`
  - L350: `.any(\|v\| !v.is_finite() \|\| v.abs() > policy.maximum_absolute_component)`

#### `materialize_replayed_activation_steering_shadow`  ·  L378
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `persist_activation_steering_shadow`  ·  L390
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `load_activation_steering_shadow`  ·  L416
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `normalization_is_exact_and_finite`  ·  L442
- **Ecuaciones / líneas clave:**
  - L442: `fn normalization_is_exact_and_finite() {`
  - L443: `let unit = normalize(vec![3.0, 4.0], SteeringNormalization::UnitL2).unwrap();`
  - L444: `assert!((norm(&unit).unwrap() - 1.0).abs() < 1e-12);`
  - L445: `let rms = normalize(vec![3.0, 4.0], SteeringNormalization::RootMeanSquare).unwrap();`
  - L446: `assert!((norm(&rms).unwrap() - 2.0_f64.sqrt()).abs() < 1e-12);`

### `src/materialization/materialization_selector.rs` (9 fns)

#### `minimum_controls_for_strategy`  ·  L23
- **Ecuaciones / líneas clave:**
  - L28: `MaterializationStrategy::LowRank => &[ComparativeControl::ConventionalLowRank],`
  - L40: `ConventionalLowRank,`
  - L62: `pub normalized_risk: f64,`
  - L76: `pub maximum_normalized_risk: f64,`

#### `rigorous_default`  ·  L91
- **Ecuaciones / líneas clave:**
  - L98: `maximum_normalized_risk: 0.1,`
  - L110: `ComparativeControl::ConventionalLowRank,`

#### `validate`  ·  L125
- **Ecuaciones / líneas clave:**
  - L130: `self.maximum_normalized_risk,`
  - L172: `pub struct RankedBackend {`
  - L185: `pub ranked: Vec<RankedBackend>,`

#### `dominates`  ·  L254
- **Ecuaciones / líneas clave:**
  - L258: `&& a.normalized_risk <= b.normalized_risk`
  - L264: `\|\| a.normalized_risk < b.normalized_risk`

#### `select_materialization_backend`  ·  L270
- **Ecuaciones / líneas clave:**
  - L326: `&& e.normalized_risk <= policy.maximum_normalized_risk`
  - L345: `let mut ranked = admitted`
  - L351: `+ policy.risk_weight * (1.0 - e.normalized_risk)`
  - L353: `* (1.0 - e.latency_micros as f64 / policy.maximum_latency_micros as f64)`
  - L355: `* (1.0 - e.resident_bytes as f64 / policy.maximum_resident_bytes as f64))`
  - L357: `RankedBackend {`
  - L367: `ranked.sort_by(\|a, b\| {`
  - L372: `let mut selected_strategy = ranked[0].strategy;`
  - L373: `let mut selected_candidates = vec![ranked[0].candidate_sha256.clone()];`

#### `rejects_low_rank_without_low_rank_specific_gate`  ·  L499
- **Ecuaciones / líneas clave:**
  - L499: `fn rejects_low_rank_without_low_rank_specific_gate() {`
  - L501: `let mut low_rank = evaluation(b"lowrank", MaterializationStrategy::LowRank, 0.91);`
  - L502: `low_rank`
  - L504: `.remove(&ComparativeControl::ConventionalLowRank);`
  - L506: `select_materialization_backend(&[low_rank], &[], &policy).is_err(),`
  - L507: `"low-rank must satisfy low-rank comparative controls"`

#### `sparse_and_steering_require_strategy_specific_gates`  ·  L512
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `convergence_rejects_confidence_above_score_and_overflowing_weights`  ·  L585
- **Ecuaciones / líneas clave:**
  - L587: `e.functional_score = 0.1;`

#### `convergence_ranking_is_independent_of_input_order`  ·  L603
- **Ecuaciones / líneas clave:**
  - L603: `fn convergence_ranking_is_independent_of_input_order() {`

### `src/materialization/shadow_evaluation.rs` (4 fns)

#### `validate`  ·  L104
- **Ecuaciones / líneas clave:**
  - L111: `self.normalized_risk,`
  - L143: `pub normalized_risk: f64,`

#### `run_shadow_evaluation`  ·  L226
- **Ecuaciones / líneas clave:**
  - L255: `output.normalized_risk,`
  - L281: `normalized_risk: output.normalized_risk,`

#### `strict_metrics`  ·  L325
- **Ecuaciones / líneas clave:**
  - L333: `normalized_risk: 0.01,`

#### `strict_runtime_metrics_are_the_only_metric_authority`  ·  L362
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/materialization/universal_capability_compiler.rs` (7 fns)

#### `replay_universal_capability_shadow_plan`  ·  L280
- **Ecuaciones / líneas clave:**
  - L320: `SteeringNormalization, TokenSelection, load_activation_steering_shadow,`
  - L328: `LowRankShadowPolicy, load_low_rank_shadow, materialize_replayed_low_rank_shadow,`
  - L329: `persist_low_rank_shadow,`

#### `fixture`  ·  L353
- **Ecuaciones / líneas clave:**
  - L381: `CapabilityNodeId::parse("node.normalize").unwrap(),`
  - L382: `PrimitiveId::parse("tensor.normalize").unwrap(),`
  - L386: `TypedPort::tensor_f64(PortId::parse("normalized").unwrap(), vec![2, 1])`
  - L396: `node_id: CapabilityNodeId::parse("node.normalize").unwrap(),`
  - L403: `let pre = 2.0_f64.sqrt();`

#### `frozen_receiver_compiler`  ·  L467
- **Ecuaciones / líneas clave:**
  - L499: `ridge: 1e-10,`
  - L502: `minimum_decoder_loo_cosine: 0.999,`

#### `replayed_plan_materializes_only_the_compiler_target_delta`  ·  L610
- **Ecuaciones / líneas clave:**
  - L711: `.target_delta[0] += 1.0;`
  - L731: `tampered_dense.tensors[0].values[0] += 1.0;`

#### `low_rank_full_chain_replays_persists_reloads_and_detects_tampering`  ·  L771
- **Ecuaciones / líneas clave:**
  - L771: `fn low_rank_full_chain_replays_persists_reloads_and_detects_tampering() {`
  - L781: `let query_norm_squared = query.iter().map(\|v\| v * v).sum::<f64>();`
  - L784: `query.iter().zip(signature).map(\|(a, b)\| a * b).sum::<f64>() / query_norm_squared;`
  - L793: `let tensor_id = TensorId::parse("layers.0.low_rank.weight").unwrap();`
  - L805: `model_id: ModelId::parse("receiver.low-rank.v1").unwrap(),`
  - L814: `supported_strategies: BTreeSet::from([MaterializationStrategy::LowRank]),`
  - L832: `Sha256Digest::digest_bytes(b"low-rank-model"),`
  - L858: `acceptable_strategies: BTreeSet::from([MaterializationStrategy::LowRank]),`
  - L860: `requested_strategy: MaterializationStrategy::LowRank,`
  - L864: `let policy = LowRankShadowPolicy {`
  - L865: `schema: "cerebro.tidex.low_rank_shadow_policy/v1".into(),`
  - L866: `maximum_rank: 1,`
  - L870: `maximum_svd_sweeps: 100,`
  - L873: `materialize_replayed_low_rank_shadow(&request, &receipt, &layout, &policy).unwrap();`
  - L884: `persist_low_rank_shadow(&roots, &request, &receipt, &layout, &policy, &candidate)`
  - L887: `load_low_rank_shadow(&roots, &request, &receipt, &layout, &policy, &reference)`
  - L888: `.unwrap_or_else(\|error\| panic!("low-rank reload failed: {error:?}")),`
  - L894: `load_low_rank_shadow(&roots, &request, &receipt, &layout, &policy, &bad_reference)`

#### `sparse_delta_full_chain_replays_persists_reloads_and_detects_tampering`  ·  L901
- **Ecuaciones / líneas clave:**
  - L911: `let query_norm_squared = query`
  - L918: `query.iter().zip(signature).map(\|(a, b)\| a * b).sum::<f64>() / query_norm_squared;`
  - L1028: `tampered_candidate.tensors[0].coordinates[0].value += 1.0;`

#### `activation_steering_full_chain_replays_persists_reloads_and_detects_tampering`  ·  L1038
- **Ecuaciones / líneas clave:**
  - L1048: `let query_norm_squared = query`
  - L1055: `query.iter().zip(signature).map(\|(a, b)\| a * b).sum::<f64>() / query_norm_squared;`
  - L1146: `normalization: SteeringNormalization::UnitL2,`

### `src/materialization/universality_evidence.rs` (2 fns)

#### `wilson_lower`  ·  L201
- **Ecuaciones / líneas clave:**
  - L208: `((p + z2 / (2.0 * n) - z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt()) / (1.0 + z2 / n))`

#### `minimum_group_wilson`  ·  L212
- **Ecuaciones / líneas clave:**
  - L220: `entry.0 += usize::from(trial.passes(protocol));`
  - L221: `entry.1 += 1;`

### `src/materialization/materialization_pipeline.rs` (1 fns)

#### `strategy`  ·  L83
- **Ecuaciones / líneas clave:**
  - L86: `Self::LowRank { .. } => MaterializationStrategy::LowRank,`
  - L104: `pub struct NamedLowRankFactors {`
  - L106: `pub factors: VerifiedLowRankFactors,`
  - L115: `LowRank {`
  - L116: `tensors: Vec<NamedLowRankFactors>,`

## Receiver / binding / validación

### `src/receiver/receiver_compiler.rs` (53 fns)

#### `validate`  ·  L53
- **Ecuaciones / líneas clave:**
  - L55: `\|\| !self.ridge.is_finite()`
  - L56: `\|\| self.ridge <= 0.0`
  - L61: `\|\| !self.minimum_decoder_loo_cosine.is_finite()`
  - L62: `\|\| !(-1.0..=1.0).contains(&self.minimum_decoder_loo_cosine)`

#### `benchmark_receiver_signature`  ·  L104
- **Ecuaciones / líneas clave:**
  - L133: `pub max_rank: usize,`
  - L134: `pub ridge: f64,`
  - L146: `pub selected_rank: usize,`
  - L157: `pub effective_rank: f64,`

#### `benchmark_receiver_basis`  ·  L167
- **Doc:** Numerical adapter over the existing tomography and identifiability kernels. It creates no observations, model updates, artifacts or promotion authority. Callers authenticate the calibration deltas and exclude held-out/target rows before this call. The source mixtures retain exact row lineage.
- **Ecuaciones / líneas clave:**
  - L175: `\|\| input.max_rank == 0`
  - L176: `\|\| input.max_rank > n.min(p).min(32)`
  - L178: `return Err(BrainError::Invalid("receiver_basis_resource_or_rank_bounds".into()));`
  - L185: `\|\| !input.ridge.is_finite()`
  - L186: `\|\| input.ridge <= 0.0`
  - L220: `max_rank: input.max_rank,`
  - L221: `ridge: input.ridge,`
  - L228: `let rank = tomography.selected_rank;`
  - L229: `if rank == 0`
  - L230: `\|\| rank > input.max_rank`
  - L231: `\|\| tomography.fields.len() != rank`
  - L233: `\|\| tomography.coefficients.column_count() != rank`
  - L234: `\|\| tomography.source_mixtures.len() != rank`
  - L247: `\|\| !tomography.effective_rank.is_finite()`
  - L263: `let tolerance = f64::EPSILON.sqrt() * (n.max(p) as f64).sqrt() * 16.0;`
  - L265: `for (axis, axis_values) in axes.iter().enumerate().take(rank) {`
  - L273: `dot(&tomography.source_mixtures[axis], &column)`
  - L281: `if norm(&difference)? > tolerance * norm(axis_values)?.max(1.0) {`
  - L293: `if (reconstruction_rms - tomography.reconstruction_rms).abs() > tolerance * raw_rms {`
  - L296: `let retained_energy = 1.0 - (reconstruction_rms / raw_rms).powi(2);`
  - L308: `input.ridge,`

#### `validate_rows`  ·  L384
- **Ecuaciones / líneas clave:**
  - L412: `pub relational_source_projection_cosine: Option<f64>,`
  - L414: `pub relational_coefficient_norm: Option<f64>,`
  - L416: `pub relational_min_loo_source_cosine: Option<f64>,`
  - L418: `pub relational_max_loo_coefficient_norm: Option<f64>,`
  - L424: `pub decoder_min_loo_cosine: f64,`
  - L426: `pub encoder_min_loo_cosine: f64,`
  - L428: `pub correct_cosine: f64,`
  - L429: `pub maximum_wrong_cosine: f64,`
  - L433: `pub protection_max_weighted_residual: f64,`

#### `compile_receiver_capability`  ·  L445
- **Doc:** Compile one held-out operational capability into receiver-native parameters. The operational profile retains its historical wire schema and gates.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `finish_operational_compilation`  ·  L461
- **Ecuaciones / líneas clave:**
  - L475: `decoder_min_loo_cosine: numerical.decoder_min_loo_cosine,`
  - L477: `encoder_min_loo_cosine: numerical.encoder_min_loo_cosine,`
  - L479: `correct_cosine: numerical.correct_cosine,`
  - L480: `maximum_wrong_cosine: numerical.maximum_wrong_cosine,`
  - L484: `protection_max_weighted_residual: numerical.protection_max_weighted_residual,`

#### `compile_receiver_readout_capability`  ·  L514
- **Doc:** Execute CapabilityIr before deriving the request to the receiver backend. The learned inverse remains a prediction; materialized receiver execution is a separate authority and is not claimed by this report.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `project_functional_signature`  ·  L550
- **Doc:** Canonical scalar f64 projection shared by acquisition and compilation. Preserve the sequential subtract/multiply/add order; do not fuse it.
- **Ecuaciones / líneas clave:**
  - L571: `sum += product;`

#### `to_map`  ·  L642
- **Ecuaciones / líneas clave:**
  - L674: `mean_loo_cosine: f64,`
  - L675: `min_loo_cosine: f64,`

#### `from_map`  ·  L680
- **Ecuaciones / líneas clave:**
  - L688: `mean_loo_cosine: map.mean_loo_cosine,`
  - L689: `min_loo_cosine: map.min_loo_cosine,`

#### `to_map`  ·  L694
- **Ecuaciones / líneas clave:**
  - L700: `\|\| [self.loo_cv_r2, self.mean_loo_cosine, self.min_loo_cosine]`
  - L713: `mean_loo_cosine: self.mean_loo_cosine,`
  - L714: `min_loo_cosine: self.min_loo_cosine,`
  - L728: `mean_loo_cosine: f64,`
  - L729: `min_loo_cosine: f64,`

#### `from_map`  ·  L734
- **Ecuaciones / líneas clave:**
  - L741: `mean_loo_cosine: map.mean_loo_cosine,`
  - L742: `min_loo_cosine: map.min_loo_cosine,`

#### `to_map`  ·  L747
- **Ecuaciones / líneas clave:**
  - L754: `self.mean_loo_cosine,`
  - L755: `self.min_loo_cosine,`
  - L769: `mean_loo_cosine: self.mean_loo_cosine,`
  - L770: `min_loo_cosine: self.min_loo_cosine,`

#### `to_maps`  ·  L823
- **Ecuaciones / líneas clave:**
  - L845: `\|\| !relational.ridge.is_finite()`
  - L846: `\|\| relational.ridge < 0.0`
  - L848: `relational.min_loo_source_cosine,`
  - L849: `relational.min_loo_target_cosine,`
  - L850: `relational.mean_loo_source_cosine,`
  - L851: `relational.mean_loo_target_cosine,`
  - L852: `relational.max_loo_coefficient_norm,`

#### `fit_receiver_compiler_maps`  ·  L910
- **Ecuaciones / líneas clave:**
  - L925: `let regression = AffineTransportPolicy::CenteredTraceRidge {`
  - L926: `relative_ridge: policy.ridge,`
  - L943: `policy.ridge,`
  - L948: `policy.ridge,`
  - L956: `policy.ridge,`

#### `freeze_receiver_compiler`  ·  L1014
- **Ecuaciones / líneas clave:**
  - L1066: `\|\| maps.decoder.min_loo_cosine < input.policy.minimum_decoder_loo_cosine`
  - L1081: `input.policy.ridge,`

#### `verify`  ·  L1116
- **Doc:** Authentication replays calibration once under the exact compiled source identity. The returned handle owns the replayed maps; target compilation performs no calibration fitting.
- **Ecuaciones / líneas clave:**
  - L1146: `self.input.policy.ridge,`

#### `compile_capability`  ·  L1178
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `compile`  ·  L1191
- **Ecuaciones / líneas clave:**
  - L1214: `\|\| norm(requested)? <= 1e-15`
  - L1226: `input.policy.ridge,`

#### `safe_coordinate_inverse`  ·  L1259
- **Doc:** Fit the requested response through the actual protection operator P: min_z ||M P z - (requested - bias)||^2 + ridge ||z||^2. P is constructed by applying the existing, fixed linear protection map to coordinate unit vectors. Hard protected directions cannot be recovered by this solve. The final proposal must STILL pass protection.allowed, the same quadratic risk budget, support bounds and the resulting response residual. Soft braking is a preconditioner here, not a promise of a fixed shrinkage of
- **Ecuaciones / líneas clave:**
  - L1263: `ridge: f64,`
  - L1279: `weighted_normal_solve(&design, &targets, &vec![1.0; targets.len()], ridge)`
  - L1286: `source_projection_cosine: Option<f64>,`
  - L1287: `coefficient_norm: f64,`
  - L1288: `min_loo_source_cosine: f64,`
  - L1289: `max_loo_coefficient_norm: f64,`

#### `relational_receiver_proposal_from_map`  ·  L1294
- **Doc:** Apply a previously calibrated relational map. The target contributes only its functional signature; no target receiver observations are used here.
- **Ecuaciones / líneas clave:**
  - L1314: `coordinates[index] += coefficient * anchor[index];`
  - L1325: `source_projection_cosine: transplant.source_projection_cosine,`
  - L1326: `coefficient_norm: transplant.coefficient_norm,`
  - L1327: `min_loo_source_cosine: map.min_loo_source_cosine,`
  - L1328: `max_loo_coefficient_norm: map.max_loo_coefficient_norm,`

#### `compile_receiver_signature`  ·  L1338
- **Doc:** Shared decoder, inverse predictor, protection and trust-region kernel.  This entry point does not fabricate an OperationalCapabilityContract for a generative model. A model binding must supply a separately sealed response protocol and enforce calibration/target separation. All output is candidate evidence; the learned encoder is only a prediction of receiver behavior.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `compile_receiver_signature_calibrated_affine`  ·  L1359
- **Doc:** Compile a measured functional IR with scale-aware, fold-local regression. This profile always retains both decoder and inverse validation. It does not turn behavioral measurements into authority to skip numerical gates.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `validate_functional_anchor_identity`  ·  L1381
- **Doc:** Distinct calibration capabilities must be distinguishable in their input IR. The tolerance concerns floating-point resolution, not an invented noise model. A single zero/constant-coordinate vector is valid; coincident rows are not independent evidence. No coordinate-specific variance heuristic is used.
- **Ecuaciones / líneas clave:**
  - L1390: `let scale = norm(&rows[i])?.max(norm(&rows[j])?).max(f64::MIN_POSITIVE);`
  - L1391: `let relative = norm(&difference)? / scale;`

#### `compile_receiver_signature_behaviorally_calibrated_candidate`  ·  L1411
- **Doc:** Candidate-only cross-model affine compilation. This deliberately does NOT interpret decoder coordinate-space LOO R² as the final proposal-quality authority. The caller must first authenticate an independent behavioral leave-one-capability-out calibration over exactly the receiver basis lineage. Encoder verification, functional residual, identity, protection, trust region and support gates remain mandatory.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `compile_receiver_signature_in_safe_coordinates`  ·  L1433
- **Doc:** Response-space inversion that includes protection in the forward design. This is an explicitly selected candidate profile, not an automatic alternate used to turn a rejected legacy compilation into an accepted one. Legacy operational compilation retains DecodeThenProject and its original serialized contract.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `compile_receiver_signature_relational`  ·  L1454
- **Doc:** Compile from relational capability geometry.  The target contributes only its functional signature; barycentric coefficients are inferred against the calibration functional anchors and then applied to receiver coordinates of those same calibration capabilities.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `compile_signature_with_method`  ·  L1471
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `compile_signature_with_method_and_validation`  ·  L1490
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `compile_signature_with_maps`  ·  L1522
- **Ecuaciones / líneas clave:**
  - L1558: `if norm(requested)? <= 1e-15 {`
  - L1585: `norm(signature).is_err() \|\| norm(signature).is_ok_and(\|value\| value <= 1e-15)`
  - L1613: `relational_source_projection_cosine,`
  - L1614: `relational_coefficient_norm,`
  - L1615: `relational_min_loo_source_cosine,`
  - L1616: `relational_max_loo_coefficient_norm,`
  - L1622: `safe_coordinate_inverse(&encoder.map, requested, protected_cortex, policy.ridge)?,`
  - L1638: `relational.source_projection_cosine,`
  - L1639: `Some(relational.coefficient_norm),`
  - L1640: `Some(relational.min_loo_source_cosine),`
  - L1641: `Some(relational.max_loo_coefficient_norm),`

#### `evaluate_portability`  ·  L1780
- **Doc:** Compare one receiver-native compiled result against an untouched receiver, a direct receiver oracle, and an explicit wrong-skill control using one common functional score. This metric is intentionally independent of parameter distance. A positive score supports functional recovery only in the declared evaluation domain; it is not, by itself, a universal portability claim.
- **Ecuaciones / líneas clave:**
  - L1792: `\|\| norm(expected_functional_signature)? <= 1e-15`
  - L1806: `let score = \|observed: &[f64]\| -> BrainResult<f64> {`
  - L1807: `let residual = observed`
  - L1812: `Ok(1.0 - norm(&residual)? / norm(expected_functional_signature)?.max(1e-15))`
  - L1814: `let virgin_score = score(virgin_functional_signature)?;`
  - L1815: `let direct_score = score(direct_functional_signature)?;`
  - L1816: `let transferred_score = score(transferred_functional_signature)?;`
  - L1817: `let wrong_score = score(wrong_functional_signature)?;`
  - L1846: `pub ridge: f64,`
  - L1856: `pub decoder_min_loo_cosine: f64,`
  - L1857: `pub encoder_min_loo_cosine: f64,`

#### `benchmark_receiver_portability_leave_one_out`  ·  L1886
- **Doc:** Leave-one-skill-out functional compilation benchmark. The direct receiver solution of the held-out skill is never passed to either learned map; it is opened only after compilation as an oracle for `RecoveredGain`.  The retained `portability` schema/API name is historical compatibility. The benchmark measures held-out recovery inside its supplied calibration set; it does not establish portability across arbitrary model architectures.
- **Ecuaciones / líneas clave:**
  - L1890: `\|\| !input.ridge.is_finite()`
  - L1891: `\|\| input.ridge <= 0.0`
  - L1916: `\|\| norm(&case.functional_signature)? <= 1e-15`
  - L1939: `learn_functional_transplant(&training_functional, &training_receiver, input.ridge)?;`
  - L1941: `learn_transport_validated(&training_receiver, &training_functional, input.ridge)?;`
  - L1968: `decoder_min_loo_cosine: decoder.min_loo_cosine,`
  - L1969: `encoder_min_loo_cosine: encoder.min_loo_cosine,`

#### `receiver_basis_test_input`  ·  L2012
- **Ecuaciones / líneas clave:**
  - L2028: `max_rank: 2,`
  - L2029: `ridge: 1e-6,`

#### `receiver_basis_adapter_preserves_real_source_mixtures_and_row_lineage`  ·  L2035
- **Ecuaciones / líneas clave:**
  - L2040: `assert_eq!(result.selected_rank, 2);`
  - L2044: `assert!(result.retained_energy > 1.0 - 1e-12);`
  - L2048: `for axis in 0..result.selected_rank {`
  - L2057: `assert!((actual - result.axes[axis][parameter]).abs() < 1e-10);`
  - L2067: `assert!((actual - input.calibration_deltas[row][parameter]).abs() < 1e-10);`

#### `receiver_basis_adapter_reports_unweighted_raw_reconstruction_energy`  ·  L2073
- **Ecuaciones / líneas clave:**
  - L2084: `input.max_rank = 1;`
  - L2086: `assert_eq!(result.selected_rank, 1);`
  - L2092: `sse += (reconstructed - value).powi(2);`
  - L2093: `total_energy += value * value;`
  - L2097: `assert!((result.retained_energy - (1.0 - sse / total_energy)).abs() < 1e-12);`
  - L2098: `assert!((result.reconstruction_rms - (sse / 12.0).sqrt()).abs() < 1e-12);`

#### `receiver_basis_adapter_rejects_ambiguous_lineage_nonfinite_values_and_resource_excess`  ·  L2102
- **Ecuaciones / líneas clave:**
  - L2129: `for max_rank in [0, 5, 33] {`
  - L2131: `invalid.max_rank = max_rank;`
  - L2136: `.contains("resource_or_rank_bounds")`
  - L2149: `.contains("resource_or_rank_bounds")`
  - L2157: `.contains("resource_or_rank_bounds")`

#### `calibrated_ir_rejects_collisions_but_accepts_constant_coordinate_vectors`  ·  L2165
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `centered_compilation_uses_both_maps_and_cannot_skip_inverse_validation`  ·  L2180
- **Ecuaciones / líneas clave:**
  - L2206: `ridge: 1e-9,`
  - L2209: `minimum_decoder_loo_cosine: 0.0,`
  - L2223: `assert!((result.target_delta[0] - 4.2).abs() < 1e-6);`
  - L2224: `assert!((result.target_delta[1] + 1.0).abs() < 1e-6);`

#### `receiver_readout_compilation_uses_executed_ir_values_and_projection`  ·  L2255
- **Ecuaciones / líneas clave:**
  - L2348: `assert!((report.execution.raw_margins[0] - 0.5).abs() < 1e-14);`
  - L2349: `assert!((report.requested_signature[0] - 0.2).abs() < 1e-14);`
  - L2350: `assert!((report.requested_signature[1] - 0.4).abs() < 1e-14);`
  - L2370: `assert!((changed.requested_signature[0] - 0.3).abs() < 1e-14);`

#### `fixture_ir`  ·  L2394
- **Ecuaciones / líneas clave:**
  - L2423: `CapabilityNodeId::parse("node.normalize").unwrap(),`
  - L2424: `PrimitiveId::parse("tensor.normalize").unwrap(),`
  - L2428: `TypedPort::tensor_f64(PortId::parse("normalized").unwrap(), vec![2, 1])`
  - L2438: `node_id: CapabilityNodeId::parse("node.normalize").unwrap(),`

#### `toggle_contract`  ·  L2448
- **Ecuaciones / líneas clave:**
  - L2449: `let pre = 2.0_f64.sqrt();`

#### `coupled_calibration`  ·  L2499
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `response_policy`  ·  L2519
- **Ecuaciones / líneas clave:**
  - L2522: `ridge: 1e-10,`
  - L2525: `minimum_decoder_loo_cosine: 0.99,`

#### `frozen_compiler_roundtrip_uses_stored_maps_without_target_time_fitting`  ·  L2555
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `frozen_compiler_rejects_rehashed_forged_stored_maps`  ·  L2586
- **Ecuaciones / líneas clave:**
  - L2591: `forged.maps.decoder.target_decoder.weights[0][0] += 999.0;`

#### `frozen_compiler_rejects_target_leakage_and_unsupported_extrapolation`  ·  L2607
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `protected_coordinate_fit_preserves_the_requested_response_without_weakening_gates`  ·  L2641
- **Ecuaciones / líneas clave:**
  - L2668: `assert!((fitted.target_delta[0] - 0.2).abs() < 1e-6);`
  - L2669: `assert!((fitted.target_delta[1] + 0.2).abs() < 1e-6);`

#### `protected_coordinate_fit_cannot_restore_a_hard_forbidden_direction`  ·  L2672
- **Ecuaciones / líneas clave:**
  - L2692: `assert!(result.target_delta[0].abs() < 1e-12);`
  - L2694: `assert!(result.protection_max_weighted_residual < 1e-12);`

#### `protected_coordinate_fit_still_rejects_insufficient_risk_budget`  ·  L2697
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `protected_coordinate_fit_does_not_bypass_protection_removal_limit`  ·  L2720
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `calibration`  ·  L2739
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `held_out_receiver_compilation_recovers_capability_without_donor_weights`  ·  L2753
- **Ecuaciones / líneas clave:**
  - L2768: `ridge: 1e-10,`
  - L2771: `minimum_decoder_loo_cosine: 0.999,`

#### `receiver_compiler_fails_promotion_when_protection_destroys_contract`  ·  L2865
- **Ecuaciones / líneas clave:**
  - L2880: `ridge: 1e-10,`
  - L2883: `minimum_decoder_loo_cosine: 0.9,`

### `src/receiver/receiver_weight_binding.rs` (45 fns)

#### `describe_linear_readout`  ·  L123
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `readout_gaps`  ·  L237
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `readout_partial_bundle`  ·  L246
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `verify_observed_linear_readout`  ·  L273
- **Doc:** Standard forward bound for a dot product over exact F32 operands. gamma_d covers rounded products and reduction; the absolute term also covers gradual underflow. The F64 reference uses the existing scaled compensated dot product, conservatively enclosed by gamma_(8d+8).
- **Ecuaciones / líneas clave:**
  - L319: `let positive = dot(&inspection.positive_weights, input)?;`
  - L320: `let negative = dot(&inspection.negative_weights, input)?;`
  - L321: `let positive_bound = crate::receiver::validation::readout_dot_roundoff_bound(`
  - L325: `let negative_bound = crate::receiver::validation::readout_dot_roundoff_bound(`
  - L331: `let positive_error = (positive - observed_positive).abs();`
  - L332: `let negative_error = (negative - observed_negative).abs();`
  - L340: `let unit32 = 2.0_f64.powi(-24);`
  - L341: `let subtraction_bound = unit32 / (1.0 - unit32)`
  - L342: `* (observed_positive.abs() + observed_negative.abs())`
  - L343: `+ 2.0_f64.powi(-150);`
  - L344: `let reference_difference_bound = crate::receiver::validation::readout_dot_roundoff_bound(`
  - L350: `let margin_error = (execution.raw_margins[index] - f32_margin).abs();`

#### `readout_acquisition_path`  ·  L366
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `derive_linear_readout_acquisition`  ·  L371
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `acquire_linear_readout`  ·  L500
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `authenticate_linear_readout_details`  ·  L573
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `authenticate_linear_readout`  ·  L605
- **Doc:** Read-only deep replay. Imported PASS fields never authorize a readout.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `cross_model`  ·  L902
- **Ecuaciones / líneas clave:**
  - L997: `pub maximum_calibration_coordinate_norm: f64,`
  - L998: `pub proposed_coordinate_norm: f64,`

#### `read_cross_model_projection`  ·  L1091
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `exact_f64_vector_digest`  ·  L1157
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `functional_signature_digest`  ·  L1166
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `receiver_coordinates_digest`  ·  L1170
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `validate_cross_model_functional_evidence`  ·  L1174
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `validate_behavioral_calibration_evidence`  ·  L1312
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `prepare_receiver_weight_candidate`  ·  L2060
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `authenticate_receiver_weight_candidate`  ·  L2080
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `materialize_receiver_weight_candidate`  ·  L2106
- **Doc:** Materialize only a recomputed, unblocked candidate. Neither an arbitrary caller-supplied coefficient vector nor an edited `allowed` flag is accepted. The output is confined to this experimental private root, never installed as the active model. Independent execution remains outstanding.
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `digest`  ·  L2174
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `fixture`  ·  L2186
- **Ecuaciones / líneas clave:**
  - L2200: `"model.norm.weight":{"dtype":"F32","shape":[4],"data_offsets":[0,16]},`
  - L2214: `let tensor = TensorId::parse("model.norm.weight").unwrap();`
  - L2326: `ridge: 1e-9,`
  - L2329: `minimum_decoder_loo_cosine: 0.99,`

#### `readout_fixture`  ·  L2521
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `bind_readout_to_cross_model_fixture`  ·  L2598
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `linear_readout_acquisition_authenticates_real_checkpoint_capture_ir_and_replay`  ·  L2652
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `linear_readout_acquisition_rejects_wrong_forward_prompt_and_f32_evidence`  ·  L2693
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `linear_readout_candidate_is_driven_by_executed_ir_not_supplied_signature`  ·  L2718
- **Ecuaciones / líneas clave:**
  - L2741: `f.request.policy.ridge,`

#### `signature_to_dense_to_checkpoint_uses_one_existing_actuator`  ·  L2762
- **Ecuaciones / líneas clave:**
  - L2779: `read_model_tensor_f32(&out, &TensorId::parse("model.norm.weight").unwrap()).unwrap();`
  - L2781: `assert!((*observed - expected).abs() < 1e-6);`

#### `cross_model_evidence_is_semantically_bound_not_only_hash_authenticated`  ·  L2791
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `cross_model_support_uses_functional_leverage_and_blocks_extreme_query`  ·  L2824
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `canonical_projection_preserves_sequential_rounding_without_fma`  ·  L2859
- **Ecuaciones / líneas clave:**
  - L2864: `let step = 2.0_f64.powi(-27);`
  - L2866: `project_functional_signature(&[-1.0, 1.0 + step], &[0.0; 2], &[vec![1.0, 1.0 - step]])`

#### `reselling_modified_raw_values_cannot_reuse_a_projected_signature`  ·  L2877
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `functional_ir_binds_actual_prompt_text_and_raw_probe_order`  ·  L2895
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `authenticated_but_changed_projection_must_recompute_the_response`  ·  L2923
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `projection_lineage_cannot_silently_include_a_target`  ·  L2953
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `protocol_self_test_can_validate_but_never_materialize_weights`  ·  L2976
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `cross_language_vector_digest_contract_matches_v69_python_authority`  ·  L3002
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `python_json_roundtrip_preserves_exact_functional_signature_digest`  ·  L3021
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `json_authority_preserves_signed_zero_subnormals_and_finite_extremes`  ·  L3048
- **Ecuaciones / líneas clave:**
  - L3048: `fn json_authority_preserves_signed_zero_subnormals_and_finite_extremes() {`

#### `forged_candidate_flags_and_coordinates_are_recomputed_not_trusted`  ·  L3077
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `target_may_not_appear_in_calibration_observations`  ·  L3108
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `outside_calibrated_radius_records_blocker_without_delta_or_checkpoint`  ·  L3149
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `resource_and_safety_bounds_fail_before_numerical_work`  ·  L3212
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `distributed_request_blocks_legacy_final_norm_basis`  ·  L3224
- **Ecuaciones / líneas clave:**
  - L3224: `fn distributed_request_blocks_legacy_final_norm_basis() {`

#### `calibration_lora_solution_is_allowed_only_when_bound_to_one_basis_axis`  ·  L3299
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `convergence_measured_candidate_uses_common_dense_and_sparse_checkpoint_pipeline`  ·  L3544
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/receiver/validation.rs` (2 fns)

#### `readout_dot_roundoff_bound`  ·  L12
- **Ecuaciones / líneas clave:**
  - L12: `pub fn readout_dot_roundoff_bound(weights: &[f64], input: &[f64]) -> BrainResult<f64> {`
  - L15: `return Err(BrainError::Invalid("readout_dot_bound_shape".into()));`
  - L17: `let absolute_weights = weights.iter().map(\|value\| value.abs()).collect::<Vec<_>>();`
  - L18: `let absolute_input = input.iter().map(\|value\| value.abs()).collect::<Vec<_>>();`
  - L19: `let absolute_sum = dot(&absolute_weights, &absolute_input)?;`
  - L20: `let unit32 = 2.0_f64.powi(-24);`
  - L21: `let unit64 = 2.0_f64.powi(-53);`
  - L24: `let gamma32 = (count32 * unit32) / (1.0 - count32 * unit32);`
  - L25: `let gamma64 = (count64 * unit64) / (1.0 - count64 * unit64);`
  - L26: `let upper_sum = absolute_sum / (1.0 - gamma64);`
  - L27: `let underflow = (2 * dimension + 1) as f64 * 2.0_f64.powi(-150);`
  - L30: `return Err(BrainError::Invalid("readout_dot_bound_nonfinite".into()));`

#### `receiver_validation_contracts_fail_closed`  ·  L74
- **Ecuaciones / líneas clave:**
  - L77: `assert!(readout_dot_roundoff_bound(&[1.0, 2.0], &[3.0, 4.0]).unwrap() >= 0.0);`
  - L78: `assert!(readout_dot_roundoff_bound(&[], &[]).is_err());`

### `src/receiver/weight_actuator.rs` (17 fns)

#### `visit_map`  ·  L101
- **Ecuaciones / líneas clave:**
  - L132: `pub struct ShardedSafetensorsNormalizationInput {`
  - L147: `pub struct ShardedSafetensorsNormalization {`
  - L153: `pub normalized_checkpoint: PrivateFileReference,`
  - L154: `pub normalized_inventory: ModelParameterInventory,`
  - L162: `pub struct ShardedSafetensorsNormalizationReceipt {`
  - L164: `pub normalization: ShardedSafetensorsNormalization,`
  - L165: `pub normalization_reference: PrivateFileReference,`
  - L211: `pub lora_rank: usize,`
  - L241: `rank_pattern: BTreeMap<String, usize>,`

#### `next`  ·  L282
- **Ecuaciones / líneas clave:**
  - L287: `self.patch += 1;`
  - L294: `for component in 0..patch.rank {`
  - L295: `value += patch.b[row * patch.rank + component]`
  - L298: `self.element += 1;`

#### `prepare_lora_patches`  ·  L918
- **Ecuaciones / líneas clave:**
  - L923: `rank: usize,`
  - L925: `if rank == 0 {`
  - L926: `return Err(invalid("lora_adapter_rank_zero"));`
  - L980: `if a_spec.shape != vec![rank, input_dim] \|\| b_spec.shape != vec![output_dim, rank] {`
  - L986: `rank,`

#### `import_peft_lora_as_dense_axis`  ·  L1004
- **Doc:** Convert a PEFT LoRA into an immutable dense receiver axis. This is the missing data-plane bridge between learned calibration adapters and the V69 receiver compiler. The target capability must never be used to create these adapters; that scientific lineage is validated by receiver_weight_binding.
- **Ecuaciones / líneas clave:**
  - L1029: `\|\| !config.rank_pattern.is_empty()`
  - L1061: `config.lora_alpha / (config.r as f64).sqrt()`
  - L1092: `lora_rank: config.r,`

#### `canonical_shard_from_index`  ·  L1185
- **Ecuaciones / líneas clave:**
  - L1193: `.any(\|component\| !matches!(component, Component::Normal(_)))`
  - L1201: `let Component::Normal(name) = component else {`

#### `normalized_checkpoint_path`  ·  L1226
- **Ecuaciones / líneas clave:**
  - L1226: `fn normalized_checkpoint_path(root: &Path, digest: &Sha256Digest) -> PathBuf {`

#### `sharded_normalization_path`  ·  L1231
- **Ecuaciones / líneas clave:**
  - L1231: `fn sharded_normalization_path(root: &Path, digest: &Sha256Digest) -> PathBuf {`
  - L1232: `root.join("state/model_normalizations/sharded-safetensors/by-sha")`

#### `normalized_sharded_header`  ·  L1236
- **Ecuaciones / líneas clave:**
  - L1236: `fn normalized_sharded_header(locations: &[ShardedTensorLocation]) -> BrainResult<Vec<u8>> {`
  - L1245: `"tidex_normalization":"hf_safetensors_index_to_single_v1"`
  - L1252: `.ok_or_else(\|\| invalid("sharded_safetensors_normalized_size_overflow"))?;`
  - L1267: `return Err(invalid("sharded_safetensors_normalized_header_invalid"));`

#### `validate_normalization_contract`  ·  L1272
- **Ecuaciones / líneas clave:**
  - L1272: `fn validate_normalization_contract(`
  - L1274: `normalization: &ShardedSafetensorsNormalization,`
  - L1276: `if normalization.schema != SHARDED_SAFETENSORS_NORMALIZATION_SCHEMA`
  - L1277: `\|\| normalization.index.byte_len == 0`
  - L1278: `\|\| normalization.index.byte_len > MAX_SHARDED_INDEX_BYTES`
  - L1279: `\|\| !normalization`
  - L1285: `\|\| normalization.shards.is_empty()`
  - L1286: `\|\| normalization.shards.len() > MAX_SHARDED_FILES`
  - L1287: `\|\| normalization.weight_map_entry_count == 0`
  - L1288: `\|\| normalization.weight_map_entry_count > MAX_SHARDED_TENSORS`
  - L1289: `\|\| normalization.weight_map_entry_count != normalization.normalized_inventory.tensor_count`
  - L1290: `\|\| !normalization.tensor_payloads_preserved_exactly`
  - L1291: `\|\| normalization.authorizes_behavioral_equivalence`
  - L1292: `\|\| normalization.authorizes_promotion`
  - L1293: `\|\| normalization.normalized_checkpoint.path`
  - L1294: `!= normalized_checkpoint_path(root, &normalization.normalized_checkpoint.sha256)`
  - L1295: `\|\| normalization.normalized_inventory.model_sha256`
  - L1296: `!= normalization.normalized_checkpoint.sha256`
  - L1298: `return Err(invalid("sharded_safetensors_normalization_contract_invalid"));`
  - L1300: `if normalization`
  - L1304: `\|\| normalization`
  - L1312: `normalization`
  - L1313: `.normalized_inventory`
  - L1321: `if payload_bytes != normalization.source_tensor_byte_count {`
  - L1324: `normalization.normalized_checkpoint.verify(root)?;`

#### `authenticate_sharded_safetensors_normalization`  ·  L1332
- **Ecuaciones / líneas clave:**
  - L1332: `pub fn authenticate_sharded_safetensors_normalization(`
  - L1335: `) -> BrainResult<ShardedSafetensorsNormalization> {`
  - L1337: `if reference.path != sharded_normalization_path(&root, &reference.sha256) {`
  - L1338: `return Err(integrity("sharded_safetensors_normalization_path_not_canonical"));`
  - L1340: `let bytes = reference.read_verified_bounded(&root, MAX_NORMALIZATION_RECORD_BYTES)?;`
  - L1341: `let normalization: ShardedSafetensorsNormalization = serde_json::from_slice(&bytes)?;`
  - L1342: `if serde_json::to_vec(&normalization)? != bytes {`
  - L1343: `return Err(integrity("sharded_safetensors_normalization_noncanonical"));`
  - L1345: `validate_normalization_contract(&root, &normalization)?;`
  - L1346: `Ok(normalization)`

#### `normalize_sharded_safetensors`  ·  L1349
- **Ecuaciones / líneas clave:**
  - L1349: `pub fn normalize_sharded_safetensors(`
  - L1351: `input: &ShardedSafetensorsNormalizationInput,`
  - L1352: `) -> BrainResult<ShardedSafetensorsNormalizationReceipt> {`
  - L1354: `if input.schema != SHARDED_SAFETENSORS_NORMALIZATION_INPUT_SCHEMA {`
  - L1355: `return Err(invalid("sharded_safetensors_normalization_input_invalid"));`
  - L1456: `let header = normalized_sharded_header(&locations)?;`
  - L1457: `let staging_destination = root.join("artifacts/models/normalization-staging/model.safetensors");`
  - L1458: `let (temporary, normalized_sha256) =`
  - L1480: `let normalized_path = normalized_checkpoint_path(&root, &normalized_sha256);`
  - L1481: `if !install_private_immutable_file(&root, &temporary, &normalized_path, &normalized_sha256)? {`
  - L1482: `PrivateFileReference::new(normalized_path.clone(), normalized_sha256.clone())`
  - L1485: `let normalized_checkpoint =`
  - L1486: `PrivateFileReference::new(normalized_path, normalized_sha256.clone());`
  - L1487: `let normalized_inventory = inspect_model_safetensors(&normalized_checkpoint.path)?;`
  - L1492: `if normalized_inventory.model_sha256 != normalized_sha256`
  - L1493: `\|\| normalized_inventory.tensors != expected_specs`
  - L1495: `return Err(integrity("sharded_safetensors_normalized_output_mismatch"));`

#### `resolve_base_model_path`  ·  L1539
- **Ecuaciones / líneas clave:**
  - L1541: `Ok(normalize_sharded_safetensors(`
  - L1543: `&ShardedSafetensorsNormalizationInput {`
  - L1544: `schema: SHARDED_SAFETENSORS_NORMALIZATION_INPUT_SCHEMA.to_string(),`
  - L1548: `.normalization`
  - L1549: `.normalized_checkpoint`

#### `sharded_normalization_is_exact_content_addressed_and_source_independent_after_import`  ·  L2395
- **Ecuaciones / líneas clave:**
  - L2395: `fn sharded_normalization_is_exact_content_addressed_and_source_independent_after_import() {`
  - L2396: `let (root, index) = sharded_base_fixture("sharded-normalization");`
  - L2397: `let receipt = normalize_sharded_safetensors(`
  - L2399: `&ShardedSafetensorsNormalizationInput {`
  - L2400: `schema: SHARDED_SAFETENSORS_NORMALIZATION_INPUT_SCHEMA.to_string(),`
  - L2405: `assert_eq!(receipt.normalization.weight_map_entry_count, 2);`
  - L2406: `assert_eq!(receipt.normalization.source_tensor_byte_count, 32);`
  - L2407: `assert!(receipt.normalization.tensor_payloads_preserved_exactly);`
  - L2408: `assert!(!receipt.normalization.authorizes_behavioral_equivalence);`
  - L2412: `.normalization`
  - L2413: `.normalized_checkpoint`
  - L2420: `read_model_tensor_f32(&receipt.normalization.normalized_checkpoint.path, &q).unwrap(),`
  - L2424: `read_model_tensor_f32(&receipt.normalization.normalized_checkpoint.path, &v).unwrap(),`
  - L2428: `authenticate_sharded_safetensors_normalization(&root, &receipt.normalization_reference)`
  - L2430: `assert_eq!(replay, receipt.normalization);`
  - L2435: `authenticate_sharded_safetensors_normalization(&root, &receipt.normalization_reference)`
  - L2437: `receipt.normalization`

#### `sharded_normalization_rejects_nonbijective_duplicate_and_escaping_indexes`  ·  L2443
- **Ecuaciones / líneas clave:**
  - L2443: `fn sharded_normalization_rejects_nonbijective_duplicate_and_escaping_indexes() {`
  - L2459: `normalize_sharded_safetensors(`
  - L2461: `&ShardedSafetensorsNormalizationInput {`
  - L2462: `schema: SHARDED_SAFETENSORS_NORMALIZATION_INPUT_SCHEMA.to_string(),`
  - L2475: `normalize_sharded_safetensors(`
  - L2477: `&ShardedSafetensorsNormalizationInput {`
  - L2478: `schema: SHARDED_SAFETENSORS_NORMALIZATION_INPUT_SCHEMA.to_string(),`
  - L2497: `normalize_sharded_safetensors(`
  - L2499: `&ShardedSafetensorsNormalizationInput {`
  - L2500: `schema: SHARDED_SAFETENSORS_NORMALIZATION_INPUT_SCHEMA.to_string(),`

#### `dense_materialization_accepts_sharded_base_via_same_normalization_authority`  ·  L2511
- **Ecuaciones / líneas clave:**
  - L2511: `fn dense_materialization_accepts_sharded_base_via_same_normalization_authority() {`
  - L2513: `let normalized = normalize_sharded_safetensors(`
  - L2515: `&ShardedSafetensorsNormalizationInput {`
  - L2516: `schema: SHARDED_SAFETENSORS_NORMALIZATION_INPUT_SCHEMA.to_string(),`
  - L2523: `&normalized.normalization.normalized_inventory,`
  - L2532: `&normalized.normalization.normalized_checkpoint.sha256,`
  - L2541: `&normalized.normalization.normalized_checkpoint.path,`

#### `peft_lora_import_accepts_sharded_base_through_retained_normalization`  ·  L2553
- **Ecuaciones / líneas clave:**
  - L2553: `fn peft_lora_import_accepts_sharded_base_through_retained_normalization() {`

#### `linear_readout_extracts_real_f32_f16_bf16_rows_and_preserves_f64_difference`  ·  L2974
- **Ecuaciones / líneas clave:**
  - L3007: `let small = 2.0_f32.powi(-25);`
  - L3014: `assert_eq!(inspection.difference_weights, vec![1.0 - f64::from(small)]);`

### `src/receiver/model_adaptation.rs` (3 fns)

#### `profile_receiver_model`  ·  L618
- **Ecuaciones / líneas clave:**
  - L632: `normalize_sharded_safetensors(`
  - L634: `&ShardedSafetensorsNormalizationInput {`
  - L635: `schema: SHARDED_SAFETENSORS_NORMALIZATION_INPUT_SCHEMA.to_string(),`
  - L639: `.normalization`
  - L640: `.normalized_checkpoint`

#### `sharded_checkpoint_is_normalized_and_profile_survives_source_removal`  ·  L1032
- **Ecuaciones / líneas clave:**
  - L1032: `fn sharded_checkpoint_is_normalized_and_profile_survives_source_removal() {`

#### `write_shard`  ·  L1038
- **Ecuaciones / líneas clave:**
  - L1069: `("model.layers.0.input_layernorm.weight", vec![1.0, 1.0], vec![2]),`
  - L1087: `"model.layers.0.input_layernorm.weight":"model-00001-of-00002.safetensors",`

### `src/receiver/capability_discovery.rs` (2 fns)

#### `validate`  ·  L63
- **Ecuaciones / líneas clave:**
  - L75: `\|\| norm(&self.functional_signature)? <= 1e-15`
  - L76: `\|\| norm(&self.wrong_control_signature)? <= 1e-15`

#### `analyze_group`  ·  L140
- **Ecuaciones / líneas clave:**
  - L155: `*aggregate += value / group.len() as f64;`
  - L161: `consistency = consistency.min(cosine(`
  - L170: `Ok(cosine(&trial.functional_signature, &mean)?`
  - L171: `- cosine(&trial.wrong_control_signature, &mean)?)`

## Capability / operator

### `src/capability/capability_ir.rs` (22 fns)

#### `verify`  ·  L245
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `calculate_digest`  ·  L266
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `system_envelope_digest`  ·  L384
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `manifest_digest`  ·  L388
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `calculate_digest`  ·  L619
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `execute_linear_readout`  ·  L661
- **Doc:** Execute the scalar-output specialization of the existing authenticated linear-map IR. There is exactly one runtime tensor [d, 1], one resident parameter tensor [1, d], one matmul(parameter, input), and its single [1, 1] public output. Names come from the actual graph; no new IR constructor or donor-supplied executable vocabulary is introduced.  Parameters must have been authenticated by the caller against the acquired descriptor/model. The returned value digests bind exactly what this call consu
- **Ecuaciones / líneas clave:**
  - L753: `execution_arithmetic: "f64_scaled_neumaier_dot/v1".into(),`

#### `validate_against`  ·  L809
- **Ecuaciones / líneas clave:**
  - L880: `.map(\|(left, right)\| (left - right).powi(2))`
  - L882: `.sqrt();`

#### `verify_receiver_signature`  ·  L930
- **Doc:** Verify a receiver-produced functional signature against the same closure and contraction semantics used to seal V63 evidence.
- **Ecuaciones / líneas clave:**
  - L969: `.map(\|(left, right)\| (left - right).powi(2))`
  - L971: `.sqrt();`
  - L975: `.map(\|(left, right)\| (left - right).powi(2))`
  - L977: `.sqrt();`

#### `resolve_reference`  ·  L1028
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `domain_digest`  ·  L1101
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `envelope`  ·  L1117
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `weight_parameter`  ·  L1169
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `weighted_node`  ·  L1174
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `weighted_output`  ·  L1192
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `ir_is_closed_typed_and_bound_to_the_captured_tree`  ·  L1203
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `ir_rejects_self_reference_wrong_type_shape_and_unbounded_shape`  ·  L1436
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `ir_rejects_dead_nodes_output_contract_mismatch_and_tampered_envelope`  ·  L1545
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `linear_readout_fixture`  ·  L1651
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `linear_readout_executes_existing_rectangular_ir_with_real_parameters`  ·  L1702
- **Ecuaciones / líneas clave:**
  - L1714: `assert!((actual - expected).abs() < 1e-12);`

#### `linear_readout_rejects_tampered_ir_parameter_binding_and_envelope`  ·  L1743
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `linear_readout_rejects_authenticated_graphs_outside_scalar_matmul_profile`  ·  L1767
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `linear_readout_bounds_work_and_rejects_nonfinite_inputs_and_results`  ·  L1802
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

### `src/capability/capability_bundle.rs`
_Sin funciones algorítmicas filtradas (revisar a mano si es glue)._

### `src/capability/acquisition_contract.rs` (5 fns)

#### `projection_roots`  ·  L528
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `validate_scope`  ·  L1141
- **Ecuaciones / líneas clave:**
  - L1151: `let normalized = normalize_declared_roots(roots)?;`
  - L1152: `if normalized != *roots {`

#### `normalize_scope`  ·  L1160
- **Ecuaciones / líneas clave:**
  - L1160: `fn normalize_scope(scope: AcquisitionScope) -> BrainResult<AcquisitionScope> {`
  - L1171: `roots: normalize_declared_roots(&roots)?,`

#### `normalize_declared_roots`  ·  L1177
- **Ecuaciones / líneas clave:**
  - L1177: `fn normalize_declared_roots(`
  - L1184: `let mut normalized = Vec::with_capacity(unique.len());`
  - L1186: `if normalized`
  - L1192: `normalized.push(root);`
  - L1194: `Ok(normalized)`

#### `declared_scope_is_normalized_and_never_claims_dependency_closure`  ·  L1917
- **Ecuaciones / líneas clave:**
  - L1917: `fn declared_scope_is_normalized_and_never_claims_dependency_closure() {`
  - L1918: `let normalized = AcquisitionRequest::new(`
  - L1935: `assert_eq!(normalized.scope, declared(vec![PathBuf::from("config"), PathBuf::from("src")]));`

### `src/operator/control_plane.rs` (13 fns)

#### `read_hub_snapshot_file_bounded`  ·  L612
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `resolve_assets`  ·  L1478
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `execute_behavioral_discovery_workflow`  ·  L1793
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `execute_behavioral_discovery_workflow_cancelable`  ·  L1800
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `empty_plasticity_advice`  ·  L2241
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `compute_operator_plasticity_advice`  ·  L2270
- **Ecuaciones / líneas clave:**
  - L2342: `let score = evaluation`

#### `compute_operator_plasticity_advice`  ·  L2726
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `try_acquire`  ·  L3269
- **Ecuaciones / líneas clave:**
  - L3322: `<section class="panel result"><div class="tabs"><button onclick="showTab('result')">Resultado · evidencia</button><button onclick="showTab('history')">Historial · jobs</button><button onclick="showTab('executors')">Ejecu`
  - L3352: `'receiver.normalize_sharded':['Normalizar SafeTensors fragmentados','Normaliza un checkpoint HF fragmentado autenticado en una única autoridad SafeTensors retenida.'],`

#### `operator_living_staircase_composes_plasticity_and_graph_without_production`  ·  L3529
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `plasticity_advice_is_empty_without_evaluation_jobs`  ·  L3702
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `plasticity_advice_does_not_invent_elo_from_an_isolated_evaluation`  ·  L3723
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `plasticity_advice_withholds_ranking_on_measured_tie`  ·  L3737
- **Ecuaciones / líneas clave:**
  - L3737: `fn plasticity_advice_withholds_ranking_on_measured_tie() {`

#### `plasticity_advice_ranks_and_routes_on_measured_score_gap`  ·  L3757
- **Ecuaciones / líneas clave:**
  - L3757: `fn plasticity_advice_ranks_and_routes_on_measured_score_gap() {`

### `src/operator/graph.rs`
_Sin funciones algorítmicas filtradas (revisar a mano si es glue)._

### `src/operator/executor_registry.rs`
_Sin funciones algorítmicas filtradas (revisar a mano si es glue)._

## Gobernanza / knowledge / runtime crate

### `src/governance/adapter_bank.rs` (3 fns)

#### `validate_manifest_contract`  ·  L1100
- **Ecuaciones / líneas clave:**
  - L1164: `rank_truncation_used,`
  - L1168: `\|\| *rank_truncation_used`

#### `resolve_active`  ·  L2828
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `verify_history`  ·  L2888
- **Ecuaciones / líneas clave:**
  - L2920: `count += 1;`

### `src/governance/universal_promotion_gate.rs` (1 fns)

#### `convergence_rejects_selection_metrics_substituted_for_execution_metrics`  ·  L301
- **Ecuaciones / líneas clave:**
  - L304: `evaluation.functional_score = 0.0;`

### `src/governance/residency_decision.rs` (8 fns)

#### `rank`  ·  L115
- **Ecuaciones / líneas clave:**
  - L115: `fn rank(self) -> u8 {`

#### `evaluate_fact_matrix`  ·  L1565
- **Ecuaciones / líneas clave:**
  - L1586: `let ranked_dimensions = [`
  - L1587: `(ResidencyDimension::Requirements, requirement_rank(facts.requirements)),`
  - L1588: `(ResidencyDimension::Effects, effect_rank(facts.effects)),`
  - L1589: `(ResidencyDimension::ExternalState, external_state_rank(facts.external_state)),`
  - L1590: `(ResidencyDimension::Observability, observability_rank(facts.observability)),`
  - L1592: `let minimum_rank = ranked_dimensions`
  - L1594: `.map(\|(_, rank)\| *rank)`
  - L1597: `let minimum_candidate = candidate_for_rank(minimum_rank);`
  - L1598: `let forced_by = ranked_dimensions`
  - L1600: `.filter_map(\|(dimension, rank)\| {`
  - L1601: `(*rank == minimum_rank && minimum_rank > 0).then_some(*dimension)`
  - L1612: `if candidate.rank() < minimum_rank {`

#### `requirement_rank`  ·  L1699
- **Ecuaciones / líneas clave:**
  - L1699: `fn requirement_rank(value: ExecutionRequirements) -> u8 {`

#### `effect_rank`  ·  L1707
- **Ecuaciones / líneas clave:**
  - L1707: `fn effect_rank(value: EffectSemantics) -> u8 {`

#### `external_state_rank`  ·  L1715
- **Ecuaciones / líneas clave:**
  - L1715: `fn external_state_rank(value: ExternalStateSemantics) -> u8 {`

#### `observability_rank`  ·  L1723
- **Ecuaciones / líneas clave:**
  - L1723: `fn observability_rank(value: ObservabilitySemantics) -> u8 {`

#### `candidate_for_rank`  ·  L1731
- **Ecuaciones / líneas clave:**
  - L1731: `fn candidate_for_rank(rank: u8) -> ResidencyCandidate {`
  - L1732: `match rank {`

#### `semantic_parsers_and_ranks_behave_exhaustively`  ·  L2120
- **Ecuaciones / líneas clave:**
  - L2120: `fn semantic_parsers_and_ranks_behave_exhaustively() {`
  - L2181: `assert_eq!(requirement_rank(ExecutionRequirements::ClosedComputation), 0);`
  - L2182: `assert_eq!(requirement_rank(ExecutionRequirements::BoundaryRuntime), 1);`
  - L2183: `assert_eq!(requirement_rank(ExecutionRequirements::SoftwareRuntime), 2);`
  - L2184: `assert_eq!(requirement_rank(ExecutionRequirements::Unknown), 2);`
  - L2186: `assert_eq!(effect_rank(EffectSemantics::Pure), 0);`
  - L2187: `assert_eq!(effect_rank(EffectSemantics::BoundaryEffects), 1);`
  - L2188: `assert_eq!(effect_rank(EffectSemantics::SoftwareEffects), 2);`
  - L2189: `assert_eq!(effect_rank(EffectSemantics::Unknown), 2);`
  - L2191: `assert_eq!(external_state_rank(ExternalStateSemantics::None), 0);`
  - L2192: `assert_eq!(external_state_rank(ExternalStateSemantics::BoundaryManaged), 1);`
  - L2193: `assert_eq!(external_state_rank(ExternalStateSemantics::SoftwareAuthoritative), 2);`
  - L2194: `assert_eq!(external_state_rank(ExternalStateSemantics::Unknown), 2);`
  - L2196: `assert_eq!(observability_rank(ObservabilitySemantics::WeightComplete), 0);`
  - L2197: `assert_eq!(observability_rank(ObservabilitySemantics::BoundaryComplete), 1);`
  - L2198: `assert_eq!(observability_rank(ObservabilitySemantics::SoftwareComplete), 2);`
  - L2199: `assert_eq!(observability_rank(ObservabilitySemantics::Unknown), 2);`
  - L2201: `assert_eq!(candidate_for_rank(0), ResidencyCandidate::Weights);`
  - L2202: `assert_eq!(candidate_for_rank(1), ResidencyCandidate::Hybrid);`
  - L2203: `assert_eq!(candidate_for_rank(2), ResidencyCandidate::Software);`
  - L2204: `assert_eq!(candidate_for_rank(99), ResidencyCandidate::Software);`

### `src/knowledge/knowledge_engine.rs` (8 fns)

#### `hypothesis_families_resolved`  ·  L5416
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `living_staircase_projects_open_obligations_and_next_plan`  ·  L6131
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `living_staircase_projects_satisfied_after_real_advance`  ·  L6184
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `living_staircase_projects_blocked_after_executor_block`  ·  L6210
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `living_staircase_projects_authenticated_dependency_depth_after_expansion`  ·  L6236
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `living_staircase_projects_persisted_revision_from_canonical_head`  ·  L6310
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `living_staircase_projects_open_after_bounded_no_result`  ·  L6342
- _(algoritmo de control/validación sin aritmética densa en las primeras líneas)_

#### `trace_verifier_derives_events_and_rejects_tamper_wrong_kind_and_cross_authority`  ·  L6852
- **Ecuaciones / líneas clave:**
  - L6934: `tampered_artifact.max_events += 1;`

### `src/runtime/isolated_execution.rs` (1 fns)

#### `request_digest_commits_arguments_limits_and_backend_contract`  ·  L1231
- **Ecuaciones / líneas clave:**
  - L1240: `relabeled_limit.limits.cpu_seconds += 1;`

### `src/runtime/staging_isolation.rs`
_Sin funciones algorítmicas filtradas (revisar a mano si es glue)._

### `src/runtime/pure_capability_e2e.rs` (1 fns)

#### `normal_relative_path`  ·  L2130
- **Ecuaciones / líneas clave:**
  - L2130: `fn normal_relative_path(path: &Path) -> BrainResult<PathBuf> {`
  - L2136: `.any(\|component\| !matches!(component, Component::Normal(_)))`

## Otros módulos con señales algorítmicas

### `src/capability/content_vault.rs` (1)
- `projection_path` L572

### `src/cross_model/co_evolution/consensus_builder.rs` (1)
- `get_statistics` L310 — L321: `ConsensusState::Approved => stats.approved += 1,`; L322: `ConsensusState::Rejected => stats.rejected += 1,`; L323: `ConsensusState::Pending => stats.pending += 1,`; L324: `ConsensusState::Expired => stats.expired += 1,`

### `src/cross_model/integration/capability_discovery_bridge.rs` (5)
- `default` L17 — L24: `pub struct CapabilityDiscoveryBridge {`; L25: `config: CapabilityDiscoveryBridgeConfig,`; L29: `impl CapabilityDiscoveryBridge {`
- `new` L30 — L30: `pub fn new(config: CapabilityDiscoveryBridgeConfig) -> Result<Self, String> {`; L32: `return Err("discovery_bridge_cache_invalid".into());`
- `bridge_capability` L40 — L40: `pub fn bridge_capability(`; L56: `return Err("discovery_bridge_evidence_invalid".into());`; L68: `.map_err(|error| format!("discovery_bridge_serialize:{error}"))?,`; L84: `return Err("discovery_bridge_cache_limit".into());`
- `batch_bridge` L91 — L91: `pub fn batch_bridge(`; L103: `.map(|evidence| self.bridge_capability(report, evidence, model, domain))`
- `default` L122 — L123: `Self::new(CapabilityDiscoveryBridgeConfig::default())`; L124: `.expect("static discovery bridge config")`

### `src/cross_model/integration/causal_credit_bridge.rs` (1)
- `allocate_credit` L19

### `src/cross_model/models/traits.rs` (6)
- `l2_norm` L92 — L92: `pub fn l2_norm(&self) -> f64 {`; L97: `.sqrt()`
- `normalize` L100 — L100: `pub fn normalize(&self) -> Result<Self, String> {`; L102: `let norm = self.l2_norm();`; L103: `if norm <= f64::EPSILON {`; L104: `return Err("tensor_zero_norm".into());`; L107: `data: self.data.iter().map(|value| value / norm).collect(),`
- `dot` L115 — L115: `pub fn dot(&self, other: &Self) -> Option<f64> {`
- `cosine_similarity` L122 — L122: `pub fn cosine_similarity(&self, other: &Self) -> Option<f64> {`; L123: `let dot = self.dot(other)?;`; L124: `let a = self.l2_norm();`; L125: `let b = other.l2_norm();`; L129: `Some((dot / (a * b)).clamp(-1.0, 1.0))`
- `from_runtime_architecture` L339 — L340: `let normalized = value.trim().to_ascii_lowercase();`; L341: `if normalized.starts_with("llama") {`; L343: `} else if normalized.starts_with("mistral") || normalized.starts_with("mixtral") {`; L345: `} else if normalized.starts_with("qwen") {`; L347: `} else if normalized.starts_with("gpt") {`; L349: `} else if normalized.starts_with("claude") {`; L352: `Self::Other(normalized)`
- `new` L449 — L476: `CalibratedLinearRidge,`; L486: `pub normalized_residual: f64,`

### `src/engine/store.rs` (2)
- `validate_bank_scalar` L417 — L421: `if value.abs() > f64::MAX.sqrt() {`
- `validate_skill_bank_semantics` L427 — L520: `|| geometry.max_local_rank == 0`; L521: `|| !geometry.mean_effective_rank.is_finite()`; L522: `|| geometry.mean_effective_rank <= 0.0`; L535: `|| block.selected_rank != block.axes.len()`; L536: `|| block.selected_rank > block.count`; L537: `|| !block.effective_rank.is_finite()`; L538: `|| block.effective_rank < 0.0`; L543: `|| !block.normalized_block_energy.is_finite()`

### `src/foundation/artifact.rs` (1)
- `stream_linear_combination_bytes` L894 — Stream exactly the f32 payload bytes of a linear combination.  Both materialization and verification use this one arithm — L910: `sum += sources[index].1 * f64::from(value);`; L912: `if !sum.is_finite() || sum.abs() > f32::MAX as f64 {`

### `src/foundation/authority.rs` (1)
- `optional_resolver_rejects_a_symlinked_missing_prefix` L2038

### `src/foundation/contracts.rs` (2)
- `as_str` L55 — L129: `pub selected_rank: usize,`; L130: `pub effective_rank: f64,`; L133: `pub normalized_block_energy: f64,`; L143: `pub max_local_rank: usize,`; L144: `pub mean_effective_rank: f64,`
- `default` L250 — L255: `max_rank: 32,`; L256: `ridge: 1e-6,`; L259: `skill_match_cosine: 0.82,`; L266: `max_spectral_normalized_reconstruction_rms: 0.45,`

### `src/foundation/identity.rs` (4)
- `validate_ascii_id` L8 — L10: `allow_dot: bool,`; L11: `forbid_leading_dot: bool,`; L16: `|| (forbid_leading_dot && value.starts_with('.'))`; L21: `|| (allow_dot && byte == b'.')`
- `parse` L127 — L129: `validate_ascii_id(value, $allow_dot, true, $label)?;`; L130: `if $allow_dot && value.split('.').any(str::is_empty) {`
- `parse` L171 — L173: `validate_ascii_id(value, $allow_dot, true, $label)?;`; L174: `if $allow_dot && value.split('.').any(str::is_empty) {`
- `observation_id_allows_dot_but_not_hidden_or_parent_paths` L628 — L628: `fn observation_id_allows_dot_but_not_hidden_or_parent_paths() {`

### `src/learning/sleep_diagnostics.rs` (1)
- `diagnose_consolidation` L38 — L47: `let similarity = cosine(&old.direction, &new_field.direction)?.abs();`; L58: `let similarity = cosine(&old.direction, &new_field.direction)?.abs();`; L74: `born += 1;`; L82: `merged += 1;`; L97: `retained += 1;`; L109: `faded += 1;`; L117: `split += 1;`

### `src/operator/artifact.rs` (3)
- `parse` L225 — L254: `"backend_ranking" => Ok(Self::BackendRanking),`; L323: `"low_rank_policy" => Ok(Self::LowRankPolicy),`; L324: `"low_rank_shadow_artifact" => Ok(Self::LowRankShadowArtifact),`
- `as_str` L432 — L461: `Self::BackendRanking => "backend_ranking",`; L530: `Self::LowRankPolicy => "low_rank_policy",`; L531: `Self::LowRankShadowArtifact => "low_rank_shadow_artifact",`
- `role` L638 — L667: `Self::BackendRanking => ArtifactRole::Terminal,`; L736: `Self::LowRankPolicy => ArtifactRole::ExternalInput,`; L737: `Self::LowRankShadowArtifact => ArtifactRole::Terminal,`

### `src/receiver/architecture_families.rs` (1)
- `classify_module` L77 — L94: `} else if name.contains("norm") {`; L95: `ModuleFamily::Normalizer`

### `src/receiver/receiver_profile.rs` (1)
- `compatible_receiver_only_produces_shadow_plan` L333 — L342: `acceptable_strategies: BTreeSet::from([MaterializationStrategy::LowRank]),`; L351: `MaterializationStrategy::LowRank,`

## Políticas numéricas (`config/`) — parámetros de fórmulas

### `config/evaluations/binary-logic.json`
```
{
  "schema": "cerebro.cross_model.behavioral_benchmark/v1",
  "benchmark_id": "binary_logic_v1",
  "domain": "reasoning",
  "probes": [
    {
      "probe_id": "l01",
      "prompt": "Return only YES or NO: If every zorp is a mip and every mip is a tav, is every zorp a tav?",
      "verifier": {
        "kind": "exact_text",
        "expected": "YES",
        "trim": true,
        "case_sensitive": false
      },
      "weight": 1.0
    },
    {
      "probe_id": "l02",
      "prompt": "Return only YES or NO: If no red object is blue and X is red, can X be blue?",
      "verifier": {
        "kind": "exact_text",
        "expected": "NO",
        "trim": true,
        "case_sensitive": false
      },
      "weight": 1.0
    },
    {
      "probe_id": "l03",
      "prompt": "Return only YES or NO: If A > B and B > C, must A > C?",
      "verifier": {
        "kind": "exact_text",
        "expected": "YES",
        "trim": true,
        "case_sensitive": false
      },
      "weight": 1.0
    },
    {
      "probe_id": "l04",
      "prompt": "Return only YES or NO: If some cats are black, does it follow that all cats are black?",
      "verifier": {
        "kind": "exact_text",
        "expected": "NO",
        "trim": true,
        "case_sensitive": false
      },
      "weight": 1.0
    },
    {
      "probe_id": "l05",
      "prompt": "Return only YES or NO: If P implies Q and P is true, must Q be true?",
      "verifier": {
        "kind": "exact_text",
        "expected": "YES",
        "trim": true,
        "case_sensitive": false
      },
      "weight": 1.0
    },
    {
      "probe_id": "l06",
      "prompt": "Return only YES or NO: If P implies Q and Q is false, can P be true?",
      "verifier": {
        "kind": "exact_text",
        "expected": "NO",
        "trim": true,
        "case_sensitive": false
      },
      "weight": 1.0
    },
    {
      "probe_id": "l07",
      "prompt": "Return only YES or NO: If all squares are rectangles, is every square a rectangle?",
      "verifier": {
        "kind": "exact_text",
        "expected": "YES",
        "trim": true,
        "case_sensitive": false
      },
      "weight": 1.0
    },
    {
      "probe_id": "l08",
      "prompt": "Return only YES or NO: If some rectangles are squares, are all rectangles squares?",
      "verifier": {
        "kind": "exact_text",
        "expected": "NO",
        "trim": true,
        "case_sensitive": false
      },
      "weight": 1.0
    }
  ],
  "minimum_mean_gap": 0.1,
  "significance_alpha": 0.05
}
```

### `config/evaluations/integer-arithmetic.json`
```
{
  "schema": "cerebro.cross_model.behavioral_benchmark/v1",
  "benchmark_id": "integer_arithmetic_v1",
  "domain": "mathematics",
  "probes": [
    {
      "probe_id": "m01",
      "prompt": "Return only the decimal integer for 127 * 89.",
      "verifier": {
        "kind": "numeric",
        "expected": 11303,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m02",
      "prompt": "Return only the decimal integer for 997 - 463.",
      "verifier": {
        "kind": "numeric",
        "expected": 534,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m03",
      "prompt": "Return only the decimal integer for 48 * 37.",
      "verifier": {
        "kind": "numeric",
        "expected": 1776,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m04",
      "prompt": "Return only the decimal integer for 4096 / 16.",
      "verifier": {
        "kind": "numeric",
        "expected": 256,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m05",
      "prompt": "Return only the decimal integer for 73 + 819.",
      "verifier": {
        "kind": "numeric",
        "expected": 892,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m06",
      "prompt": "Return only the decimal integer for 144 * 12.",
      "verifier": {
        "kind": "numeric",
        "expected": 1728,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m07",
      "prompt": "Return only the decimal integer for 9999 - 1234.",
      "verifier": {
        "kind": "numeric",
        "expected": 8765,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m08",
      "prompt": "Return only the decimal integer for 25 * 25.",
      "verifier": {
        "kind": "numeric",
        "expected": 625,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m09",
      "prompt": "Return only the decimal integer for 1024 + 2048.",
      "verifier": {
        "kind": "numeric",
        "expected": 3072,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m10",
      "prompt": "Return only the decimal integer for 81 / 9.",
      "verifier": {
        "kind": "numeric",
        "expected": 9,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m11",
      "prompt": "Return only the decimal integer for 17 * 19.",
      "verifier": {
        "kind": "numeric",
        "expected": 323,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m12",
      "prompt": "Return only the decimal integer for 123 + 456 + 789.",
      "verifier": {
        "kind": "numeric",
        "expected": 1368,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m13",
      "prompt": "Return only the decimal integer for 2^10.",
      "verifier": {
        "kind": "numeric",
        "expected": 1024,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "m14",
      "prompt": "Return only the decimal integer for 15^2.",
      "verifier": {
        "kind": "numeric",
        "expected": 225,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "pr
```

### `config/evaluations/python-semantics.json`
```
{
  "schema": "cerebro.cross_model.behavioral_benchmark/v1",
  "benchmark_id": "python_semantics_v1",
  "domain": "programming",
  "probes": [
    {
      "probe_id": "c01",
      "prompt": "Python expression: len([1,2,3,4]). Return only the integer result.",
      "verifier": {
        "kind": "numeric",
        "expected": 4,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "c02",
      "prompt": "Python expression: sum([2,4,6]). Return only the integer result.",
      "verifier": {
        "kind": "numeric",
        "expected": 12,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "c03",
      "prompt": "Python expression: list(range(5))[-1]. Return only the integer result.",
      "verifier": {
        "kind": "numeric",
        "expected": 4,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "c04",
      "prompt": "Python expression: 7 // 2. Return only the integer result.",
      "verifier": {
        "kind": "numeric",
        "expected": 3,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "c05",
      "prompt": "Python expression: 2 ** 8. Return only the integer result.",
      "verifier": {
        "kind": "numeric",
        "expected": 256,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "c06",
      "prompt": "Python expression: len({1,1,2,3}). Return only the integer result.",
      "verifier": {
        "kind": "numeric",
        "expected": 3,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "c07",
      "prompt": "Python expression: int(True) + int(False). Return only the integer result.",
      "verifier": {
        "kind": "numeric",
        "expected": 1,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    },
    {
      "probe_id": "c08",
      "prompt": "Python expression: min([9,3,7]). Return only the integer result.",
      "verifier": {
        "kind": "numeric",
        "expected": 3,
        "absolute_tolerance": 0.0
      },
      "weight": 1.0
    }
  ],
  "minimum_mean_gap": 0.1,
  "significance_alpha": 0.05
}
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

### `config/models.toml`
```
schema = "cerebro.cross_model.models/v2"

[[models]]
name = "smollm2:135m"
backend = "ollama"
endpoint = "http://127.0.0.1:11434"
enabled = true
access = ["behavioral_inference"]

[[models]]
name = "qwen2.5:0.5b"
backend = "ollama"
endpoint = "http://127.0.0.1:11434"
enabled = true
access = ["behavioral_inference"]

[[models]]
name = "llama3.2:1b"
backend = "ollama"
endpoint = "http://127.0.0.1:11434"
enabled = true
access = ["behavioral_inference"]

[[models]]
name = "smollm2-135m-candle"
backend = "candle_llama"
enabled = false
checkpoint_path = "/home/yo/Future/runtime/llms/huggingface/hub/models--HuggingFaceTB--SmolLM2-135M/snapshots/93efa2f097d58c2a74874c7e644dbc9b0cee75a2/model.safetensors"
config_path = "/home/yo/Future/runtime/llms/huggingface/hub/models--HuggingFaceTB--SmolLM2-135M/snapshots/93efa2f097d58c2a74874c7e644dbc9b0cee75a2/config.json"
tokenizer_path = "/home/yo/Future/runtime/llms/huggingface/hub/models--HuggingFaceTB--SmolLM2-135M/snapshots/93efa2f097d58c2a74874c7e644dbc9b0cee75a2/tokenizer.json"
access = ["behavioral_inference"]

[[models]]
name = "smollm2-135m-hf"
backend = "hf_transformers"
enabled = true
python_executable = "/home/yo/Future/runtime/python/tidex-mechinterp/bin/python"
require_nnsight = true
require_sae_lens = false
model_dir = "/home/yo/Future/runtime/llms/huggingface/hub/models--HuggingFaceTB--SmolLM2-135M/snapshots/93efa2f097d58c2a74874c7e644dbc9b0cee75a2"
checkpoint_path = "/home/yo/Future/runtime/llms/huggingface/hub/models--HuggingFaceTB--SmolLM2-135M/snapshots/93efa2f097d58c2a74874c7e644dbc9b0cee75a2/model.safetensors"
config_path = "/home/yo/Future/runtime/llms/huggingface/hub/models--HuggingFaceTB--SmolLM2-135M/snapshots/93efa2f097d58c2a74874c7e644dbc9b0cee75a2/config.json"
tokenizer_path = "/home/yo/Future/runtime/llms/huggingface/hub/models--HuggingFaceTB--SmolLM2-135M/snapshots/93efa2f097d58c2a74874c7e644dbc9b0cee75a2/tokenizer.json"
threads = 8
access = ["behavioral_inference", "internal_activations", "activation_intervention"]

[[models]]
name = "smollm2-135m-instruct-hf"
backend = "hf_transformers"
enabled = true
python_executable = "/home/yo/Future/runtime/python/tidex-mechinterp/bin/python"
require_nnsight = true
require_sae_lens = false
model_dir = "/home/yo/Future/runtime/llms/huggingface/hub/models--HuggingFaceTB--SmolLM2-135M-Instruct/snapshots/12fd25f77366fa6b3b4b768ec3050bf629380bac"
checkpoint_path = "/home/yo/Future/runtime/llms/huggingface/hub/models--HuggingFaceTB--SmolLM2-135M-Instruct/snapshots/12fd25f77366fa6b3b4b768ec3050bf629380bac/model.safetensors"
config_path = "/home/yo/Future/runtime/llms/huggingface/hub/models--HuggingFaceTB--SmolLM2-135M-Instruct/snapshots/12fd25f77366fa6b3b4b768ec3050bf629380bac/config.json"
tokenizer_path = "/home/yo/Future/runtime/llms/huggingface/hub/models--HuggingFaceTB--SmolLM2-135M-Instruct/snapshots/12fd25f77366fa6b3b4b768ec3050bf629380bac/tokenizer.json"
threads = 8
access = ["behavioral_inference", "internal_activations", "activation_intervention"]
```

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

## Apéndice: cuerpos de funciones críticas (plasticidad + linalg + low-rank)

### `src/cross_model/plasticity/bcm_metaplasticity.rs`
#### `validate`
```rust
    fn validate(&self) -> Result<(), String> {
        if !self.initial_theta.is_finite()
            || !(0.0..=1.0).contains(&self.initial_theta)
            || self.window_size == 0
            || self.window_size > 1_000_000
            || !self.learning_rate.is_finite()
            || self.learning_rate <= 0.0
            || self.learning_rate > 1.0
            || !self.theta_decay.is_finite()
            || !(0.0..1.0).contains(&self.theta_decay)
        {
            return Err("bcm_config_invalid".into());
        }
        Ok(())
    }
```

#### `initialize_state`
```rust
    pub fn initialize_state(&mut self, capability_name: &str) -> Result<(), String> {
        if capability_name.trim().is_empty() {
            return Err("bcm_capability_name_invalid".into());
        }
        if self.states.contains_key(capability_name) {
            return Err("bcm_state_already_initialized".into());
        }
        self.states.insert(
            capability_name.into(),
            BCMState {
                theta_m: self.config.initial_theta,
                sliding_window: Vec::with_capacity(self.config.window_size),
                learning_rate: self.config.learning_rate,
            },
        );
        Ok(())
    }
```

#### `update_threshold`
```rust
    pub fn update_threshold(
        &mut self,
        capability_name: &str,
        activation: f64,
    ) -> Result<f64, String> {
        if !activation.is_finite() || !(0.0..=1.0).contains(&activation) {
            return Err("bcm_activation_invalid".into());
        }
        let state = self
            .states
            .get_mut(capability_name)
            .ok_or("bcm_state_missing")?;
        state.sliding_window.push(activation);
        if state.sliding_window.len() > self.config.window_size {
            state.sliding_window.remove(0);
        }
        let mean_squared = state
            .sliding_window
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            / state.sliding_window.len() as f64;
        state.theta_m =
            (state.theta_m + state.learning_rate * (mean_squared - state.theta_m)).clamp(0.0, 1.0);
        Ok(state.theta_m)
    }
```

#### `calculate_weight_change`
```rust
    pub fn calculate_weight_change(
        &self,
        capability_name: &str,
        pre_synaptic: f64,
        post_synaptic: f64,
    ) -> Result<f64, String> {
        if !pre_synaptic.is_finite() || !post_synaptic.is_finite() {
            return Err("bcm_signal_invalid".into());
        }
        let state = self
            .states
            .get(capability_name)
            .ok_or("bcm_state_missing")?;
        Ok(state.learning_rate * pre_synaptic * post_synaptic * (post_synaptic - state.theta_m))
    }
```

#### `get_threshold`
```rust
    pub fn get_threshold(&self, capability_name: &str) -> Option<f64> {
        self.states.get(capability_name).map(|state| state.theta_m)
    }
```

#### `reset_state`
```rust
    pub fn reset_state(&mut self, capability_name: &str) -> Result<(), String> {
        let state = self
            .states
            .get_mut(capability_name)
            .ok_or("bcm_state_missing")?;
        state.theta_m = self.config.initial_theta;
        state.sliding_window.clear();
        Ok(())
    }
```

#### `apply_decay`
```rust
    pub fn apply_decay(&mut self) {
        for state in self.states.values_mut() {
            state.theta_m = (state.theta_m * (1.0 - self.config.theta_decay)).clamp(0.0, 1.0);
        }
    }
```

### `src/cross_model/plasticity/eligibility_traces.rs`
#### `validate`
```rust
    fn validate(&self) -> Result<(), String> {
        if !self.initial_trace.is_finite()
            || self.initial_trace < 0.0
            || !self.decay_factor.is_finite()
            || !(0.0..=1.0).contains(&self.decay_factor)
            || !self.trace_update_rate.is_finite()
            || self.trace_update_rate < 0.0
            || !self.max_trace_value.is_finite()
            || self.max_trace_value <= 0.0
            || self.initial_trace > self.max_trace_value
        {
            return Err("eligibility_trace_config_invalid".into());
        }
        Ok(())
    }
```

#### `update_trace`
```rust
    pub fn update_trace(&mut self, capability_name: &str, activation: f64) -> Result<f64, String> {
        if !activation.is_finite() || !(0.0..=1.0).contains(&activation) {
            return Err("eligibility_activation_invalid".into());
        }
        let trace = self
            .traces
            .get_mut(capability_name)
            .ok_or("eligibility_trace_missing")?;
        trace.trace_value = (trace.trace_value * self.config.decay_factor
            + self.config.trace_update_rate * activation)
            .clamp(0.0, self.config.max_trace_value);
        trace.last_update = chrono::Utc::now().to_rfc3339();
        Ok(trace.trace_value)
    }
```

#### `accumulate_credit`
```rust
    pub fn accumulate_credit(&mut self, capability_name: &str, credit: f64) -> Result<f64, String> {
        if !credit.is_finite() {
            return Err("eligibility_credit_invalid".into());
        }
        let trace = self
            .traces
            .get_mut(capability_name)
            .ok_or("eligibility_trace_missing")?;
        trace.credit_accumulated += credit * trace.trace_value;
        if !trace.credit_accumulated.is_finite() {
            return Err("eligibility_credit_overflow".into());
        }
        Ok(trace.credit_accumulated)
    }
```

#### `decay_all`
```rust
    pub fn decay_all(&mut self) {
        for trace in self.traces.values_mut() {
            trace.trace_value *= self.config.decay_factor;
        }
    }
```

### `src/cross_model/plasticity/neuromodulation.rs`
#### `calculate_plasticity_modulation`
```rust
    pub fn calculate_plasticity_modulation(&self) -> Result<f64, String> {
        self.config.validate()?;
        let mut modulation = 0.0;
        for (modulator, weight) in &self.config.weights {
            modulation += *weight * self.get_level(*modulator)?;
        }
        if !modulation.is_finite() {
            return Err("neuromodulation_output_invalid".into());
        }
        Ok(modulation.clamp(0.0, 1.0))
    }
```

#### `modulate_learning_rate`
```rust
    pub fn modulate_learning_rate(&self, base_rate: f64) -> Result<f64, String> {
        if !base_rate.is_finite() || base_rate < 0.0 {
            return Err("base_learning_rate_invalid".into());
        }
        Ok(base_rate * self.calculate_plasticity_modulation()?)
    }
```

#### `apply_decay`
```rust
    pub fn apply_decay(&mut self) {
        for level in self.current_levels.values_mut() {
            *level *= self.config.decay_factor;
        }
    }
```

### `src/cross_model/plasticity/elo_system.rs`
#### `validate`
```rust
    fn validate(&self) -> Result<(), String> {
        if !self.initial_rating.is_finite()
            || !self.k_factor.is_finite()
            || self.k_factor <= 0.0
            || !self.rating_floor.is_finite()
            || !self.rating_ceiling.is_finite()
            || self.rating_floor >= self.rating_ceiling
            || !(self.rating_floor..=self.rating_ceiling).contains(&self.initial_rating)
            || !self.logistic_scale.is_finite()
            || self.logistic_scale <= 0.0
        {
            return Err("elo_config_invalid".into());
        }
        Ok(())
    }
```

#### `initialize_rating`
```rust
    pub fn initialize_rating(&mut self, entity_name: &str) -> Result<(), String> {
        if entity_name.trim().is_empty() || self.ratings.contains_key(entity_name) {
            return Err("elo_initialization_invalid".into());
        }
        self.ratings.insert(
            entity_name.into(),
            ELOState {
                rating: self.config.initial_rating,
                comparisons: 0,
                last_evidence_sha256: None,
                last_update: chrono::Utc::now().to_rfc3339(),
            },
        );
        Ok(())
    }
```

#### `update_observed`
```rust
    pub fn update_observed(
        &mut self,
        first: &str,
        second: &str,
        first_observed_score: f64,
        evidence_sha256: &str,
    ) -> Result<(f64, f64), String> {
        if first == second
            || !first_observed_score.is_finite()
            || !(0.0..=1.0).contains(&first_observed_score)
            || evidence_sha256.len() != 64
            || !evidence_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("elo_observation_invalid".into());
        }
        let first_rating = self
            .ratings
            .get(first)
            .ok_or("elo_first_entity_missing")?
            .rating;
        let second_rating = self
            .ratings
            .get(second)
            .ok_or("elo_second_entity_missing")?
            .rating;
        let expected_first = 1.0
            / (1.0 + 10.0_f64.powf((second_rating - first_rating) / self.config.logistic_scale));
        let expected_second = 1.0 - expected_first;
        let observed_second = 1.0 - first_observed_score;
        let first_new = (first_rating
            + self.config.k_factor * (first_observed_score - expected_first))
            .clamp(self.config.rating_floor, self.config.rating_ceiling);
        let second_new = (second_rating
            + self.config.k_factor * (observed_second - expected_second))
            .clamp(self.config.rating_floor, self.config.rating_ceiling);
        let now = chrono::Utc::now().to_rfc3339();
        {
            let state = self
                .ratings
                .get_mut(first)
                .ok_or("elo_first_entity_missing")?;
            state.rating = first_new;
            state.comparisons = state
                .comparisons
                .checked_add(1)
                .ok_or("elo_comparison_overflow")?;
            state.last_evidence_sha256 = Some(evidence_sha256.into());
            state.last_update = now.clone();
        }
        {
            let state = self
                .ratings
                .get_mut(second)
                .ok_or("elo_second_entity_missing")?;
            state.rating = second_new;
            state.comparisons = state
                .comparisons
                .checked_add(1)
                .ok_or("elo_comparison_overflow")?;
            state.last_evidence_sha256 = Some(evidence_sha256.into());
            state.last_update = now;
        }
        Ok((first_new, second_new))
    }
```

#### `get_rating`
```rust
    pub fn get_rating(&self, entity_name: &str) -> Option<f64> {
        self.ratings.get(entity_name).map(|state| state.rating)
    }
```

#### `get_state`
```rust
    pub fn get_state(&self, entity_name: &str) -> Option<&ELOState> {
        self.ratings.get(entity_name)
    }
```

#### `get_all_ratings`
```rust
    pub fn get_all_ratings(&self) -> HashMap<String, f64> {
        self.ratings
            .iter()
            .map(|(name, state)| (name.clone(), state.rating))
            .collect()
    }
```

#### `get_leaderboard`
```rust
    pub fn get_leaderboard(&self) -> Vec<(String, f64)> {
        let mut ratings = self.get_all_ratings().into_iter().collect::<Vec<_>>();
        ratings.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        ratings
    }
```

#### `reset_rating`
```rust
    pub fn reset_rating(&mut self, entity_name: &str) -> Result<(), String> {
        let state = self
            .ratings
            .get_mut(entity_name)
            .ok_or("elo_entity_missing")?;
        state.rating = self.config.initial_rating;
        state.comparisons = 0;
        state.last_evidence_sha256 = None;
        state.last_update = chrono::Utc::now().to_rfc3339();
        Ok(())
    }
```

#### `clear_all`
```rust
    pub fn clear_all(&mut self) {
        self.ratings.clear();
    }
```

### `src/cross_model/plasticity/pi_controller.rs`
#### `validate`
```rust
    fn validate(&self) -> Result<(), String> {
        if !self.proportional_gain.is_finite()
            || !self.integral_gain.is_finite()
            || !self.output_min.is_finite()
            || !self.output_max.is_finite()
            || self.output_min >= self.output_max
            || !self.integral_windup_limit.is_finite()
            || self.integral_windup_limit <= 0.0
        {
            return Err("pi_controller_config_invalid".into());
        }
        Ok(())
    }
```

#### `update`
```rust
    pub fn update(&mut self, setpoint: f64, measurement: f64, dt: f64) -> Result<f64, String> {
        if !setpoint.is_finite() || !measurement.is_finite() || !dt.is_finite() || dt <= 0.0 {
            return Err("pi_controller_input_invalid".into());
        }
        let error = setpoint - measurement;
        let candidate_integral = (self.state.integral + error * dt)
            .clamp(-self.config.integral_windup_limit, self.config.integral_windup_limit);
        let raw =
            self.config.proportional_gain * error + self.config.integral_gain * candidate_integral;
        let output = raw.clamp(self.config.output_min, self.config.output_max);
        // Conditional integration anti-windup: do not accumulate further into saturation.
        let saturated_high = raw > self.config.output_max && error > 0.0;
        let saturated_low = raw < self.config.output_min && error < 0.0;
        if !saturated_high && !saturated_low {
            self.state.integral = candidate_integral;
        }
        self.state.last_error = error;
        self.state.last_output = output;
        self.state.updates = self
            .state
            .updates
            .checked_add(1)
            .ok_or("pi_update_overflow")?;
        Ok(output)
    }
```

#### `reset`
```rust
    pub fn reset(&mut self) {
        self.state = PIControllerState {
            integral: 0.0,
            last_error: 0.0,
            last_output: 0.0,
            updates: 0,
        };
    }
```

#### `set_gains`
```rust
    pub fn set_gains(&mut self, proportional_gain: f64, integral_gain: f64) -> Result<(), String> {
        let mut candidate = self.config.clone();
        candidate.proportional_gain = proportional_gain;
        candidate.integral_gain = integral_gain;
        candidate.validate()?;
        self.config = candidate;
        Ok(())
    }
```

#### `get_gains`
```rust
    pub fn get_gains(&self) -> (f64, f64) {
        (self.config.proportional_gain, self.config.integral_gain)
    }
```

### `src/foundation/linalg.rs`
#### `zeros`
```rust
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }
```

#### `from_rows`
```rust
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
```

#### `validate`
```rust
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
```

#### `get`
```rust
    pub fn get(&self, r: usize, c: usize) -> f64 {
        self.data[r * self.cols + c]
    }
```

#### `set`
```rust
    pub(crate) fn set(&mut self, r: usize, c: usize, v: f64) {
        self.data[r * self.cols + c] = v;
    }
```

#### `row`
```rust
    pub fn row(&self, r: usize) -> &[f64] {
        &self.data[r * self.cols..(r + 1) * self.cols]
    }
```

#### `matmul`
```rust
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
```

#### `matvec`
```rust
    pub fn matvec(&self, v: &[f64]) -> BrainResult<Vec<f64>> {
        self.validate("matvec_matrix")?;
        if self.cols != v.len() || v.iter().any(|value| !value.is_finite()) {
            return Err(BrainError::Invalid("matvec_shape".into()));
        }
        (0..self.rows)
            .map(|row| dot(self.row(row), v))
            .collect::<BrainResult<Vec<_>>>()
    }
```

#### `dot`
```rust
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
```

#### `compensated_sum`
```rust
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
```

#### `stable_rms`
```rust
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
```

#### `norm`
```rust
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
```

#### `normalize`
```rust
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
```

#### `cosine`
```rust
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
```

#### `sub`
```rust
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
```

#### `add_scaled`
```rust
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
```

#### `solve`
```rust
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
```

#### `inverse_with_ridge`
```rust
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
```

#### `weighted_normal_solve`
```rust
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
```

#### `symmetric_top_eigen`
```rust
pub fn symmetric_top_eigen(
    a: &Matrix,
    k: usize,
    iterations: usize,
) -> BrainResult<Vec<(f64, Vec<f64>)>> {
    if a.rows != a.cols {
        return Err(BrainError::Invalid("eigen_shape".into()));
    }
    let n = a.rows;
    let mut basis: Vec<Vec<f64>> = Vec::new();
    let mut out = Vec::new();
    for comp in 0..k.min(n) {
        let mut v = (0..n)
            .map(|i| (((i + 1) * (comp + 3)) as f64 * 0.731).sin() + 0.17)
            .collect::<Vec<_>>();
        v = normalize(&v)?;
        for _ in 0..iterations {
            let mut w = a.matvec(&v)?;
            for q in &basis {
                let p = dot(&w, q)?;
                add_scaled(&mut w, q, -p)?;
            }
            let wn = norm(&w)?;
            if wn < 1e-12 {
                break;
            }
            for x in &mut w {
                *x /= wn;
            }
            v = w;
        }
        let av = a.matvec(&v)?;
        let lambda = dot(&v, &av)?.max(0.0);
        if lambda < 1e-12 {
            break;
        }
        basis.push(v.clone());
        out.push((lambda, v));
    }
    out.sort_by(|a, b| b.0.total_cmp(&a.0));
    Ok(out)
}
```

#### `weighted_row_gram`
```rust
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
```

#### `median`
```rust
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
```

#### `symmetric_eigen_jacobi_raw`
```rust
fn symmetric_eigen_jacobi_raw(
    a: &Matrix,
    tolerance: f64,
    max_rotations: usize,
) -> BrainResult<Vec<(f64, Vec<f64>)>> {
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
    let mut d = a.clone();
    let mut v = Matrix::identity(n);
    let default_rotations = n.saturating_mul(n).saturating_mul(8);
    let mut converged = n == 1;
    for _ in 0..max_rotations.max(default_rotations) {
        let mut p = 0usize;
        let mut q = 0usize;
        let mut max_off = 0.0f64;
        for i in 0..n {
            for j in i + 1..n {
                let x = d.get(i, j).abs();
                if x > max_off {
                    max_off = x;
                    p = i;
                    q = j;
                }
            }
        }
        if max_off <= tolerance {
            converged = true;
            break;
        }
        let app = d.get(p, p);
        let aqq = d.get(q, q);
        let apq = d.get(p, q);
        let phi = 0.5 * (2.0 * apq).atan2(aqq - app);
        let c = phi.cos();
        let s = phi.sin();
        for k in 0..n {
            if k == p || k == q {
                continue;
            }
            let dkp = d.get(k, p);
            let dkq = d.get(k, q);
            let np = c * dkp - s * dkq;
            let nq = s * dkp + c * dkq;
            d.set(k, p, np);
            d.set(p, k, np);
            d.set(k, q, nq);
            d.set(q, k, nq);
        }
        let new_pp = c * c * app - 2.0 * s * c * apq + s * s * aqq;
        let new_qq = s * s * app + 2.0 * s * c * apq + c * c * aqq;
        d.set(p, p, new_pp);
        d.set(q, q, new_qq);
        d.set(p, q, 0.0);
        d.set(q, p, 0.0);
        for k in 0..n {
            let vkp = v.get(k, p);
            let vkq = v.get(k, q);
            v.set(k, p, c * vkp - s * vkq);
            v.set(k, q, s * vkp + c * vkq);
        }
// ... truncated ...
```

#### `symmetric_eigen_jacobi`
```rust
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
```

#### `signed_eigensolver_preserves_negative_eigenvalues`
```rust
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
```

#### `direction_operations_reject_zero_norm_instead_of_fabricating_geometry`
```rust
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
```

#### `stable_reductions_survive_large_finite_inputs_and_cancellation`
```rust
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
```

#### `weighted_algebra_rejects_invalid_weights_and_regularization`
```rust
    fn weighted_algebra_rejects_invalid_weights_and_regularization() {
        let design = Matrix::from_rows(&[vec![1.0], vec![2.0]]).unwrap();
        assert!(weighted_normal_solve(&design, &[1.0, 2.0], &[1.0, -0.1], 1e-6).is_err());
        assert!(weighted_normal_solve(&design, &[1.0, 2.0], &[1.0, f64::NAN], 1e-6).is_err());
        assert!(weighted_normal_solve(&design, &[1.0, 2.0], &[1.0, 1.0], -1.0).is_err());
        assert!(weighted_row_gram(&design, &[1.0, -0.1]).is_err());
        assert!(inverse_with_ridge(&Matrix::identity(1), f64::NAN).is_err());
    }
```

### `src/foundation/low_rank_math.rs`
#### `solve_minimum_norm_rank_one`
```rust
pub fn solve_minimum_norm_rank_one(
    input_activation: &[f32],
    desired_output_shift: &[f32],
    damping: f64,
) -> BrainResult<MinimumNormRankOneSolution> {
    if input_activation.is_empty()
        || desired_output_shift.is_empty()
        || !damping.is_finite()
        || damping < 0.0
        || input_activation
            .iter()
            .chain(desired_output_shift)
            .any(|value| !value.is_finite())
    {
        return Err(BrainError::Invalid("minimum_norm_rank_one_input_invalid".into()));
    }
    let input_squared_norm = input_activation
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>();
    if !input_squared_norm.is_finite() || input_squared_norm <= f64::EPSILON {
        return Err(BrainError::Numerical("minimum_norm_rank_one_activation_degenerate".into()));
    }
    let denominator = input_squared_norm + damping;
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(BrainError::Numerical("minimum_norm_rank_one_denominator_invalid".into()));
    }
    let right = input_activation
        .iter()
        .map(|value| (f64::from(*value) / denominator) as f32)
        .collect::<Vec<_>>();
    if right.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Numerical("minimum_norm_rank_one_factor_non_finite".into()));
    }
    let response_scale = input_activation
        .iter()
        .zip(&right)
        .map(|(input, factor)| f64::from(*input) * f64::from(*factor))
        .sum::<f64>();
    let predicted_shift = desired_output_shift
        .iter()
        .map(|value| (f64::from(*value) * response_scale) as f32)
        .collect::<Vec<_>>();
    let residual_norm = predicted_shift
        .iter()
        .zip(desired_output_shift)
        .map(|(predicted, desired)| f64::from(*predicted - *desired).powi(2))
        .sum::<f64>()
        .sqrt();
    let left_squared_norm = desired_output_shift
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>();
    let right_squared_norm = right
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>();
    let frobenius_norm = (left_squared_norm * right_squared_norm).sqrt();
    if !residual_norm.is_finite() || !frobenius_norm.is_finite() || frobenius_norm == 0.0 {
        return Err(BrainError::Numerical("minimum_norm_rank_one_solution_invalid".into()));
    }
    Ok(MinimumNormRankOneSolution {
        left: desired_output_shift.to_vec(),
        right,
        predicted_shift,
        residual_norm,
        frobenius_norm,
    })
}
```

#### `solve_regularized_multi_case_low_rank`
```rust
pub fn solve_regularized_multi_case_low_rank(
    input_activations: &[Vec<f32>],
    desired_output_shifts: &[Vec<f32>],
    damping: f64,
) -> BrainResult<MultiCaseLowRankSolution> {
    let cases = input_activations.len();
    if cases < 2
        || cases != desired_output_shifts.len()
        || cases as u64 > MAX_LOW_RANK
        || !damping.is_finite()
        || damping <= 0.0
    {
        return Err(BrainError::Invalid("multi_case_low_rank_input_invalid".into()));
    }
    let input_dimension = input_activations[0].len();
    let output_dimension = desired_output_shifts[0].len();
    if input_dimension == 0
        || output_dimension == 0
        || input_activations
            .iter()
            .any(|row| row.len() != input_dimension || row.iter().any(|value| !value.is_finite()))
        || desired_output_shifts
            .iter()
            .any(|row| row.len() != output_dimension || row.iter().any(|value| !value.is_finite()))
        || input_activations
            .iter()
            .enumerate()
            .any(|(index, row)| input_activations[..index].contains(row))
    {
        return Err(BrainError::Invalid("multi_case_low_rank_examples_invalid".into()));
    }

    let mut gram = vec![0.0_f64; cases * cases];
    for row in 0..cases {
        for column in 0..cases {
            gram[row * cases + column] = input_activations[row]
                .iter()
                .zip(&input_activations[column])
                .map(|(left, right)| f64::from(*left) * f64::from(*right))
                .sum::<f64>();
        }
        gram[row * cases + row] += damping;
    }
    let cholesky = cholesky_spd(&gram, cases)?;

    let mut coefficients_by_input = vec![0.0_f32; input_dimension * cases];
    for input in 0..input_dimension {
        let rhs = input_activations
            .iter()
            .map(|row| f64::from(row[input]))
            .collect::<Vec<_>>();
        let coefficients = solve_cholesky(&cholesky, &rhs, cases)?;
        for case_index in 0..cases {
            coefficients_by_input[input * cases + case_index] = coefficients[case_index] as f32;
        }
    }
    let mut right = vec![0.0_f32; cases * input_dimension];
    for component in 0..cases {
        for input in 0..input_dimension {
            right[component * input_dimension + input] =
                coefficients_by_input[input * cases + component];
        }
    }
    let mut left = vec![0.0_f32; output_dimension * cases];
    for output in 0..output_dimension {
        for case_index in 0..cases {
            left[output * cases + case_index] = desired_output_shifts[case_index][output];
        }
    }

    let mut residual_squared = 0.0_f64;
    for case_index in 0..cases {
        for output in 0..output_dimension {
            let predicted = (0..cases)
                .map(|component| {
                    f64::from(left[output * cases + component])
                        * input_activations[case_index]
                            .iter()
                            .enumerate()
                            .map(|(input, value)| {
// ... truncated ...
```

#### `dense_multi_case_relative_residual`
```rust
pub fn dense_multi_case_relative_residual(
    dense: &[f32],
    rows: usize,
    columns: usize,
    inputs: &[Vec<f32>],
    exact_shifts: &[Vec<f64>],
) -> BrainResult<f64> {
    if inputs.len() < 2
        || inputs.len() != exact_shifts.len()
        || dense.len() != rows.saturating_mul(columns)
        || inputs
            .iter()
            .any(|input| input.len() != columns || input.iter().any(|value| !value.is_finite()))
        || exact_shifts
            .iter()
            .any(|shift| shift.len() != rows || shift.iter().any(|value| !value.is_finite()))
    {
        return Err(BrainError::Integrity("multi_case_low_rank_residual_inputs_invalid".into()));
    }
    let mut residual_squared = 0.0_f64;
    let mut target_squared = 0.0_f64;
    for (input, shift) in inputs.iter().zip(exact_shifts) {
        for row in 0..rows {
            let predicted = input
                .iter()
                .enumerate()
                .map(|(column, input)| f64::from(dense[row * columns + column]) * f64::from(*input))
                .sum::<f64>();
            residual_squared += (predicted - shift[row]).powi(2);
            target_squared += shift[row].powi(2);
        }
    }
    if target_squared <= f64::EPSILON {
        return Err(BrainError::Numerical("multi_case_low_rank_relative_residual_invalid".into()));
    }
    let relative = residual_squared.sqrt() / target_squared.sqrt();
    if !relative.is_finite() {
        return Err(BrainError::Numerical("multi_case_low_rank_relative_residual_invalid".into()));
    }
    Ok(relative)
}
```

#### `cholesky_spd`
```rust
fn cholesky_spd(matrix: &[f64], dimension: usize) -> BrainResult<Vec<f64>> {
    let mut lower = vec![0.0; dimension * dimension];
    for row in 0..dimension {
        for column in 0..=row {
            let sum = matrix[row * dimension + column]
                - (0..column)
                    .map(|k| lower[row * dimension + k] * lower[column * dimension + k])
                    .sum::<f64>();
            if row == column {
                if !sum.is_finite() || sum <= f64::EPSILON {
                    return Err(BrainError::Numerical(
                        "multi_case_low_rank_gram_indefinite".into(),
                    ));
                }
                lower[row * dimension + column] = sum.sqrt();
            } else {
                lower[row * dimension + column] = sum / lower[column * dimension + column];
            }
        }
    }
    Ok(lower)
}
```

#### `solve_cholesky`
```rust
fn solve_cholesky(lower: &[f64], rhs: &[f64], dimension: usize) -> BrainResult<Vec<f64>> {
    let mut forward = vec![0.0; dimension];
    for row in 0..dimension {
        forward[row] = (rhs[row]
            - (0..row)
                .map(|k| lower[row * dimension + k] * forward[k])
                .sum::<f64>())
            / lower[row * dimension + row];
    }
    let mut result = vec![0.0; dimension];
    for row in (0..dimension).rev() {
        result[row] = (forward[row]
            - ((row + 1)..dimension)
                .map(|k| lower[k * dimension + row] * result[k])
                .sum::<f64>())
            / lower[row * dimension + row];
    }
    if result.iter().any(|value| !value.is_finite()) {
        return Err(BrainError::Numerical("multi_case_low_rank_linear_solve_invalid".into()));
    }
    Ok(result)
}
```

#### `rank_one_solution_is_exact_without_damping`
```rust
    fn rank_one_solution_is_exact_without_damping() {
        let solution = solve_minimum_norm_rank_one(&[3.0, 4.0], &[2.0, -1.0], 0.0).unwrap();
        assert!(solution.residual_norm <= 1e-6);
        assert_eq!(solution.left, vec![2.0, -1.0]);
        assert_eq!(solution.right, vec![0.12, 0.16]);
    }
```

#### `degenerate_and_invalid_rank_one_inputs_fail_closed`
```rust
    fn degenerate_and_invalid_rank_one_inputs_fail_closed() {
        assert!(solve_minimum_norm_rank_one(&[0.0, 0.0], &[1.0], 0.0).is_err());
        assert!(solve_minimum_norm_rank_one(&[1.0], &[1.0], -1.0).is_err());
        assert!(solve_minimum_norm_rank_one(&[f32::NAN], &[1.0], 0.0).is_err());
    }
```
