# Índice de fórmulas algorítmicas TIDE-X

Total: **849** funciones/algoritmos catalogados.

| Módulo | Función | L | Doc / idea | Línea clave |
|--------|---------|---|------------|-------------|
| `src/foundation/linalg.rs` | `dot` | 115 |  | `L124: let normalized = compensated_sum(` |
| `src/foundation/linalg.rs` | `compensated_sum` | 138 | Neumaier compensated summation for finite values. This is the shared numerical reduction primitive for persisted metrics and decisions. | `L146: if sum.abs() >= value.abs() {` |
| `src/foundation/linalg.rs` | `stable_rms` | 161 | Overflow-resistant RMS using a scaled sum-of-squares recurrence. | `L191: scale * (sum_squares / count as f64).sqrt()` |
| `src/foundation/linalg.rs` | `norm` | 198 |  | `L198: pub fn norm(a: &[f64]) -> BrainResult<f64> {` |
| `src/foundation/linalg.rs` | `normalize` | 213 |  | `L213: pub fn normalize(a: &[f64]) -> BrainResult<Vec<f64>> {` |
| `src/foundation/linalg.rs` | `cosine` | 224 |  | `L228: let left_norm = norm(a)?;` |
| `src/foundation/linalg.rs` | `solve` | 265 |  | `L278: .map(\|value\| value.abs())` |
| `src/foundation/linalg.rs` | `inverse_with_ridge` | 331 |  | `L331: pub fn inverse_with_ridge(a: &Matrix, ridge: f64) -> BrainResult<Matrix> {` |
| `src/foundation/linalg.rs` | `weighted_normal_solve` | 353 |  | `L353: pub fn weighted_normal_solve(` |
| `src/foundation/linalg.rs` | `symmetric_top_eigen` | 391 |  | `L406: v = normalize(&v)?;` |
| `src/foundation/linalg.rs` | `weighted_row_gram` | 434 |  | `L446: let v = weights[i].sqrt() * weights[j].sqrt() * dot(d.row(i), d.row(j))?;` |
| `src/foundation/linalg.rs` | `symmetric_eigen_jacobi_raw` | 467 |  | `L486: let symmetry_tolerance = f64::EPSILON.sqrt() * scale * n as f64;` |
| `src/foundation/linalg.rs` | `symmetric_eigen_jacobi_signed` | 565 | Signed eigen-decomposition for symmetric matrices. Unlike the historical energy helper, this preserves negative and near-zero eigenvalues so callers can validat | `` |
| `src/foundation/linalg.rs` | `symmetric_eigen_jacobi` | 575 | Energy-oriented symmetric eigendecomposition. Negative eigenvalues are intentionally discarded because Gram/energy callers require a PSD spectrum. | `L586: let negative_tolerance = f64::EPSILON.sqrt() * scale * a.rows.max(1) as f64;` |
| `src/foundation/linalg.rs` | `signed_eigensolver_preserves_negative_eigenvalues` | 604 |  | `` |
| `src/foundation/linalg.rs` | `direction_operations_reject_zero_norm_instead_of_fabricating_geometry` | 617 |  | `L617: fn direction_operations_reject_zero_norm_instead_of_fabricating_geometry() {` |
| `src/foundation/linalg.rs` | `stable_reductions_survive_large_finite_inputs_and_cancellation` | 641 |  | `L642: let scale = f64::MAX.sqrt();` |
| `src/foundation/linalg.rs` | `weighted_algebra_rejects_invalid_weights_and_regularization` | 655 |  | `L657: assert!(weighted_normal_solve(&design, &[1.0, 2.0], &[1.0, -0.1], 1e-6).is_err());` |
| `src/foundation/low_rank_math.rs` | `solve_minimum_norm_rank_one` | 40 | Solve the damped minimum-norm rank-one update for one observation.  For an input `x` and requested local output shift `y`, this returns `delta-W = y x^T / (x^T  | `L40: pub fn solve_minimum_norm_rank_one(` |
| `src/foundation/low_rank_math.rs` | `solve_regularized_multi_case_low_rank` | 115 | Fit the minimum-norm ridge update `Y (X X^T + lambda I)^-1 X`.  The returned factorization has one component per independently supplied case. A singleton or dup | `L217: let residual_norm = residual_squared.sqrt();` |
| `src/foundation/low_rank_math.rs` | `dense_multi_case_relative_residual` | 234 | Recompute the relative residual of a dense linear update against an exact multi-case contract. Integrations use this independent recomputation rather than trust | `L269: let relative = residual_squared.sqrt() / target_squared.sqrt();` |
| `src/foundation/low_rank_math.rs` | `cholesky_spd` | 276 |  | `L290: lower[row * dimension + column] = sum.sqrt();` |
| `src/foundation/low_rank_math.rs` | `solve_cholesky` | 299 |  | `L317: return Err(BrainError::Numerical("multi_case_low_rank_linear_solve_invalid".into()));` |
| `src/foundation/low_rank_math.rs` | `rank_one_solution_is_exact_without_damping` | 327 |  | `L328: let solution = solve_minimum_norm_rank_one(&[3.0, 4.0], &[2.0, -1.0], 0.0).unwrap();` |
| `src/foundation/low_rank_math.rs` | `degenerate_and_invalid_rank_one_inputs_fail_closed` | 335 |  | `L336: assert!(solve_minimum_norm_rank_one(&[0.0, 0.0], &[1.0], 0.0).is_err());` |
| `src/foundation/validation.rs` | `validate_identifier` | 19 | Canonical identifier validator shared by all persistent manifests. This keeps stable naming rules in one place, preventing divergent checks in different subsyst | `` |
| `src/foundation/validation.rs` | `validate_http_endpoint` | 34 | Validate a network endpoint used by external model providers. | `L38: let normalized = value` |
| `src/foundation/validation.rs` | `source_support_indices` | 69 | Indices whose coefficient is numerically material in a source mixture. The tolerance is scale-relative and shared by memory, dense materialization, dual-space r | `L80: let tolerance = max_abs * f64::EPSILON.sqrt();` |
| `src/foundation/validation.rs` | `regression_r2` | 137 |  | `L162: Ok(if sst <= 1e-18 { 0.0 } else { 1.0 - sse / sst })` |
| `src/foundation/validation.rs` | `validate_symmetric_psd` | 165 |  | `L176: let tolerance = f64::EPSILON.sqrt() * scale * matrix.rows as f64;` |
| `src/foundation/validation.rs` | `symmetric_psd_condition` | 191 |  | `L200: let tolerance = largest * f64::EPSILON.sqrt() * matrix.rows.max(1) as f64;` |
| `src/foundation/validation.rs` | `effective_rank_from_spectrum` | 213 |  | `L213: pub fn effective_rank_from_spectrum(eigenvalues: &[f64]) -> BrainResult<f64> {` |
| `src/foundation/validation.rs` | `choose_energy_rank` | 231 |  | `L231: pub fn choose_energy_rank(` |
| `src/foundation/validation.rs` | `identifier_validator_matches_managed_name_policy` | 305 |  | `` |
| `src/foundation/digest.rs` | `digest_bytes` | 31 |  | `` |
| `src/foundation/digest.rs` | `digest_domain` | 36 | Hash a domain-separated payload in exactly one SHA-256 round. | `` |
| `src/foundation/digest.rs` | `as_digest` | 182 |  | `` |
| `src/foundation/digest.rs` | `digest_requires_canonical_lowercase_without_changing_wire_shape` | 459 |  | `` |
| `src/foundation/digest.rs` | `digest_rejects_missing_or_malformed_identity` | 468 |  | `` |
| `src/foundation/digest.rs` | `sha256_digest_traits_file_hash_and_conversions_are_canonical` | 486 |  | `` |
| `src/foundation/digest.rs` | `every_compatibility_semantic_digest_exercises_its_full_trait_surface` | 545 |  | `` |
| `src/foundation/digest.rs` | `sealed_semantic_digests_round_trip_without_raw_public_conversions` | 597 |  | `` |
| `src/foundation/digest.rs` | `semantic_digest_domains_are_distinct_but_wire_compatible` | 617 |  | `` |
| `src/foundation/finite.rs` | `rejects_nonfinite_and_noncanonical_wire_values` | 74 |  | `` |
| `src/analysis/transport.rs` | `validate` | 23 |  | `L25: Self::FixedRidge { ridge } => *ridge,` |
| `src/analysis/transport.rs` | `fit_affine` | 143 |  | `L160: let beta = weighted_normal_solve(&design, &y, &row_weights, ridge)?;` |
| `src/analysis/transport.rs` | `fit_affine_with_policy` | 182 |  | `L240: let unit_scale = stable_rms(centered.iter().flatten().copied())? * count.sqrt();` |
| `src/analysis/transport.rs` | `learn_transport` | 316 | Backwards-compatible generation transport, now affine rather than forced through the origin. Use `learn_transport_validated` before promotion. | `L319: ridge: f64,` |
| `src/analysis/transport.rs` | `global_r2` | 340 |  | `L361: Ok(if sst <= 1e-18 { 0.0 } else { 1.0 - sse / sst })` |
| `src/analysis/transport.rs` | `leave_one_out_predictions` | 364 |  | `L367: ridge: f64,` |
| `src/analysis/transport.rs` | `learn_transport_validated` | 392 |  | `L395: ridge: f64,` |
| `src/analysis/transport.rs` | `compute_sinkhorn_optimal_transport` | 417 | Computes formal Entropic Regularized Sinkhorn Optimal Transport between two representation point clouds: \min_{P \in U(a,b)} \langle P, C \rangle + \epsilon \Om | `L450: kernel[i][j] = (-cost_matrix[i][j] / reg).exp();` |
| `src/analysis/transport.rs` | `validated_from_predictions` | 516 |  | `L532: let loo_cv_rms = (squared_error / (target.len() * target[0].len()) as f64).sqrt();` |
| `src/analysis/transport.rs` | `functional_leverage` | 560 |  | `L563: ridge: f64,` |
| `src/analysis/transport.rs` | `functional_support_envelope` | 603 | Maximum leave-one-capability-out leverage is fixed from calibration alone. | `L606: ridge: f64,` |
| `src/analysis/transport.rs` | `validate_transport_with_topology` | 626 | Validates a transport map by checking both quantitative leave-one-out metrics (R^2, RMS, Cosine) and qualitative manifold topology (Betti numbers, homotopy scor | `L647: (target_topo.topological_homotopy_score - pred_topo.topological_homotopy_score).abs();` |
| `src/analysis/transport.rs` | `learn_transport_validated_with_policy` | 696 | Explicit policy entry point. Existing callers of learn_transport_validated retain the original fixed-ridge implementation. Numerical stabilization is not eviden | `` |
| `src/analysis/transport.rs` | `learn_functional_transplant_with_policy` | 719 | Functional-signature to receiver-coordinate compilation using an explicitly selected affine policy. No target update or target training data is accepted. | `L742: mean_loo_cosine: validated.mean_loo_cosine,` |
| `src/analysis/transport.rs` | `learn_functional_transplant` | 754 | Functional transplantation does not require source and target parameter dimensions to match. Common functional signatures are the bridge. The map learns functio | `L757: ridge: f64,` |
| `src/analysis/transport.rs` | `transplant` | 780 |  | `L809: pub loo_coefficient_norms: Vec<f64>,` |
| `src/analysis/transport.rs` | `normalize_anchor_rows` | 833 |  | `L833: fn normalize_anchor_rows(rows: &[Vec<f64>], label: &str) -> BrainResult<Vec<Vec<f64>>> {` |
| `src/analysis/transport.rs` | `relational_coefficients` | 845 |  | `L848: ridge: f64,` |
| `src/analysis/transport.rs` | `learn_relational_transport` | 901 | Learn a transport from *relations between matched capabilities*, not from arbitrary target basis IDs. Each source holdout anchor is reconstructed from the remai | `L912: let source = normalize_anchor_rows(source_anchors, "relational_source")?;` |
| `src/analysis/transport.rs` | `transplant` | 985 |  | `L988: \|\| norm(source_signature)? <= 1e-15` |
| `src/analysis/transport.rs` | `centered_trace_ridge_preserves_source_units_and_affine_origins` | 1025 |  | `L1085: assert!((diagnostics.full_fit.effective_ridge / expected_ridge - 1.0).abs() < 1e-12);` |
| `src/analysis/transport.rs` | `trace_ridge_loo_refits_mean_and_scale_without_holdout_values` | 1091 |  | `L1101: // slope=(2*5)/(5+0.5*5), bias=6-slope*1.5.` |
| `src/analysis/transport.rs` | `explicit_fixed_policy_preserves_legacy_maps_and_diagnostics_are_honest` | 1121 |  | `L1124: let policy = AffineTransportPolicy::FixedRidge { ridge: 0.75 };` |
| `src/analysis/transport.rs` | `centered_trace_ridge_rejects_unidentified_design_in_full_fit_or_any_fold` | 1143 |  | `L1143: fn centered_trace_ridge_rejects_unidentified_design_in_full_fit_or_any_fold() {` |
| `src/analysis/transport.rs` | `centered_trace_ridge_does_not_penalize_the_intercept_or_claim_constant_targets_resolved` | 1171 |  | `L1171: fn centered_trace_ridge_does_not_penalize_the_intercept_or_claim_constant_targets_resolved() {` |
| `src/analysis/transport.rs` | `relational_transport_validates_geometry_and_rejects_ood_query` | 1188 |  | `L1203: assert!(map.min_loo_target_cosine + 1e-10 >= map.min_loo_source_cosine);` |
| `src/analysis/transport.rs` | `validated_affine_transport_generalizes_across_generation_anchors` | 1214 |  | `L1235: assert!(map.min_loo_cosine > 0.999999);` |
| `src/analysis/transport.rs` | `functional_transplant_crosses_incompatible_parameter_dimensions` | 1240 |  | `` |
| `src/analysis/transport.rs` | `topological_transport_validation_preserves_manifold_homology` | 1271 |  | `` |
| `src/analysis/tomography.rs` | `reconstruct` | 28 |  | `L45: Ok((coeff, rec, (err / (d.rows * d.cols).max(1) as f64).sqrt()))` |
| `src/analysis/tomography.rs` | `reconstruct_skill_fields` | 48 |  | `L87: for (lambda, u) in eigs.iter().take(rank) {` |
| `src/analysis/tomography.rs` | `global_field_alignment` | 218 |  | `L246: let similarity = cosine(&old[old_index].direction, &incoming[incoming_index].direction)?;` |
| `src/analysis/tomography.rs` | `align_incoming_identities` | 295 |  | `` |
| `src/analysis/tomography.rs` | `assimilate_bank` | 322 |  | `L386: merged = normalize(&merged)?;` |
| `src/analysis/tomography.rs` | `reconcile_full_corpus` | 438 | Reconcile a complete-corpus reconstruction against durable capabilities with one global Hungarian assignment. This makes identity independent of incoming field  | `L471: prior.persistence *= 0.85;` |
| `src/analysis/tomography.rs` | `tomography_sources_exactly_recreate_normalized_fields_and_report_used_weights` | 517 |  | `L517: fn tomography_sources_exactly_recreate_normalized_fields_and_report_used_weights() {` |
| `src/analysis/tomography.rs` | `align_incoming_identities_preserves_prior_and_aligns_sign` | 578 |  | `L589: assert!((aligned[0].direction[0] - 1.0).abs() < 1e-10);` |
| `src/analysis/tomography.rs` | `assimilate_bank_replaces_on_exact_evidence_match` | 594 |  | `` |
| `src/analysis/tomography.rs` | `assimilate_bank_rejects_partial_evidence_overlap` | 609 |  | `` |
| `src/analysis/tomography.rs` | `assimilate_bank_merges_disjoint_evidence_and_adds_unmatched` | 623 |  | `` |
| `src/analysis/tomography.rs` | `reconcile_full_corpus_decays_or_drops_unassigned_priors` | 656 |  | `L688: // f1 is matched, f2 decayed (0.5 * 0.85 = 0.425 >= 0.15), f3 dropped (0.15 * 0.85 = 0.1275 < 0.15)` |
| `src/analysis/weight_tomography.rs` | `clamp01` | 115 |  | `` |
| `src/analysis/weight_tomography.rs` | `weighted_nonnegative_mean` | 126 |  | `L143: let normalized = compensated_sum(` |
| `src/analysis/weight_tomography.rs` | `detrend_scale_normalized` | 169 |  | `L169: fn detrend_scale_normalized(values: &[f64]) -> BrainResult<(Vec<f64>, f64)> {` |
| `src/analysis/weight_tomography.rs` | `neumaier_add` | 226 |  | `L231: if sum.abs() >= value.abs() {` |
| `src/analysis/weight_tomography.rs` | `positive_dft` | 245 | Bounded direct DFT.  History is capped at 128 samples, so avoiding another numerical dependency keeps the production surface small while bounding work. | `L262: re += re_correction;` |
| `src/analysis/weight_tomography.rs` | `normalized_power` | 272 |  | `L272: fn normalized_power(values: &[f64]) -> BrainResult<(Vec<f64>, f64)> {` |
| `src/analysis/weight_tomography.rs` | `normalized_spectral_entropy` | 288 |  | `L288: fn normalized_spectral_entropy(power: &[f64]) -> BrainResult<f64> {` |
| `src/analysis/weight_tomography.rs` | `positive_autocorrelation` | 305 |  | `L316: let normalized = values.iter().map(\|value\| value / scale).collect::<Vec<_>>();` |
| `src/analysis/weight_tomography.rs` | `scaled_cosine` | 354 |  | `L371: let left_energy = compensated_sum(left.iter().map(\|value\| (value / left_scale).powi(2)))?;` |
| `src/analysis/weight_tomography.rs` | `directional_consistency` | 383 |  | `L386: if let Some(cosine) = scaled_cosine(&pair[0], &pair[1])? {` |
| `src/analysis/weight_tomography.rs` | `normalized_window` | 397 |  | `L397: fn normalized_window(values: &[f64]) -> BrainResult<Vec<f64>> {` |
| `src/analysis/weight_tomography.rs` | `subaperture_metrics` | 402 |  | `L421: let master = normalized_window(&channel[..window])?;` |
| `src/analysis/weight_tomography.rs` | `analyze_weight_dynamics` | 475 |  | `L612: let mut normalized_spectral_energy_sum = 0.0_f64;` |
| `src/analysis/weight_tomography.rs` | `coordinate_sampling_is_bounded_and_uses_the_full_span` | 986 |  | `` |
| `src/analysis/protected_map.rs` | `persist_protected_map` | 73 |  | `L107: \|\| norm(&direction.direction)? == 0.0` |
| `src/analysis/protected_map.rs` | `load_protected_cortex` | 138 |  | `L167: \|\| norm(&direction)? == 0.0` |
| `src/analysis/protected_map.rs` | `validate_evidence` | 187 |  | `L197: let sensitivity_norm = norm(&probe.sensitivity)?;` |
| `src/analysis/protected_map.rs` | `pearson` | 215 |  | `L229: let denom = (ld * rd).sqrt();` |
| `src/analysis/protected_map.rs` | `metric_dual_direction` | 238 | Represent a sensitivity covector g in the metric used by protected.rs. Its projector enforces u^T D delta = 0, so u must be proportional to D^+ g, not g. Normal | `L258: whitened.push(gradient / importance.sqrt());` |
| `src/analysis/protected_map.rs` | `build_protected_cortex_map` | 293 |  | `L327: eigenvalues.iter().take(selected_rank).sum::<f64>() / total_energy;` |
| `src/analysis/protected_map.rs` | `persistent_scatterer_cortex_protection_identifies_invariants` | 471 |  | `` |
| `src/analysis/protected_map.rs` | `protected_map_learns_load_bearing_subspace` | 503 |  | `L530: assert!(norm(&damaging.projected).unwrap() < norm(&orthogonal.projected).unwrap());` |
| `src/analysis/protected_map.rs` | `protected_map_rejects_zero_reliability_instead_of_fabricating_a_weight` | 534 |  | `` |
| `src/analysis/protected_map.rs` | `protected_map_preserves_probe_responses_in_anisotropic_metric` | 577 |  | `L595: assert_eq!(map.selected_rank, 1);` |
| `src/analysis/protected_map.rs` | `protected_map_preserves_all_independent_retained_covectors` | 624 |  | `L640: assert_eq!(map.selected_rank, 2);` |
| `src/analysis/protected_map.rs` | `protected_map_rejects_metric_underflow_that_loses_sensitivity` | 655 |  | `` |
| `src/analysis/protected_map.rs` | `protected_map_roundtrip_keeps_covector_contract_and_rejects_legacy_schema` | 682 |  | `L718: dot(&evidence[0].sensitivity, &result.projected)` |
| `src/analysis/protected.rs` | `wdot` | 16 |  | `L16: fn wdot(a: &[f64], b: &[f64], weights: &[f64]) -> BrainResult<f64> {` |
| `src/analysis/protected.rs` | `project_to_safe_subspace` | 39 |  | `L67: .map(\|(value, importance)\| value / (1.0 + importance))` |
| `src/analysis/persistent.rs` | `union` | 64 |  | `L70: if self.rank[ra] < self.rank[rb] {` |
| `src/analysis/persistent.rs` | `normalized_rows` | 80 |  | `L80: fn normalized_rows(d: &Matrix) -> BrainResult<Vec<Vec<f64>>> {` |
| `src/analysis/persistent.rs` | `components_at_threshold` | 96 |  | `` |
| `src/analysis/persistent.rs` | `clustering_silhouette` | 119 |  | `L139: .map(\|other\| 1.0 - scores.get(row, other))` |
| `src/analysis/persistent.rs` | `recurrence_metrics` | 160 |  | `L178: weighted_persistence += component.len() as f64 * seen.len() as f64 / group_count as f64;` |
| `src/analysis/persistent.rs` | `better_cluster_candidate` | 189 |  | `L198: if (candidate.0 - current.0).abs() > EPS {` |
| `src/analysis/persistent.rs` | `centroid_for_members` | 310 |  | `L340: let pre_norm = norm(&pre)?;` |
| `src/analysis/persistent.rs` | `functional_dimension` | 347 |  | `` |
| `src/analysis/persistent.rs` | `weighted_functional_signature` | 366 |  | `L380: let sign = if cosine(reference, &rows[index])? >= 0.0 {` |
| `src/analysis/persistent.rs` | `assignment_min_margin` | 410 |  | `L423: let matched = cosine(&reference[reference_index], &candidate[matched_candidate])?.abs();` |
| `src/analysis/persistent.rs` | `cross_aperture_functional_cv` | 440 |  | `L528: let cv_r2 = if sst <= 1e-18 { 0.0 } else { 1.0 - sse / sst };` |
| `src/analysis/persistent.rs` | `effective_rank_from_energies` | 578 |  | `L578: fn effective_rank_from_energies(energies: &[f64]) -> f64 {` |
| `src/analysis/persistent.rs` | `reconstruct_persistent_skill_fields` | 590 |  | `L614: let rows = normalized_rows(d)?;` |
| `src/analysis/persistent.rs` | `structural_threshold_selection_avoids_largest_gap_fragmentation` | 836 |  | `L837: let g = 0.3_f64.sqrt();` |
| `src/analysis/persistent.rs` | `persistent_inverse_discovers_recurrent_skills_without_task_labels` | 867 |  | `L868: let skill_a = normalize(&[1.0, 0.2, 0.0, 0.0, 0.0, 0.0]).unwrap();` |
| `src/analysis/persistent.rs` | `persistent_inverse_solves_nonorthogonal_field_coefficients_jointly` | 938 |  | `L940: let second = vec![0.5, 0.75_f64.sqrt()];` |
| `src/analysis/pythagoras_topology.rs` | `evaluate_and_correct` | 58 | Correct a discrete step update `delta_w` to ensure trust region calculations reflect true geodesic manifold distance rather than Manhattan step inflation. | `L68: let l2_norm = norm(delta_w)?;` |
| `src/analysis/pythagoras_topology.rs` | `project_to_geodesic` | 109 | Rescale a discrete weight shift to match the true geodesic constraint. | `` |
| `src/analysis/pythagoras_topology.rs` | `analyze_topology` | 146 | Analyze the persistent topology of a set of skill representation vectors. | `L192: let dist = sum_sq.sqrt();` |
| `src/analysis/pythagoras_topology.rs` | `find` | 202 |  | `L252: let homotopy_score = (1.0 / (betti_0 as f64)).clamp(0.0, 1.0);` |
| `src/analysis/pythagoras_topology.rs` | `process_range_doppler` | 370 | Decompose a weight matrix into Doppler subapertures to focus multi-layer drift. This upgraded version uses true sub-pixel phase correlation (from temporal_track | `L427: let shift = corr.estimated_shift.iter().sum::<f64>() / (cols as f64);` |
| `src/analysis/pythagoras_topology.rs` | `separate_carrier_and_detail` | 467 | Decomposes a weight matrix into its low-rank universal carrier component (capturing the dominant spectral structure transferable across architectures) and the h | `L518: // Choose rank k: smallest k such that sum(σ²_1..k) >= 0.80 * retained energy` |
| `src/analysis/pythagoras_topology.rs` | `pythagoras_staircase_corrects_high_dim_step_inflation` | 565 |  | `L577: let corrected_l2 = norm(&corrected).unwrap();` |
| `src/analysis/pythagoras_topology.rs` | `sar_doppler_subapertures_decomposes_weight_matrix` | 597 |  | `` |
| `src/analysis/pythagoras_topology.rs` | `sar_carrier_and_detail_reconstructs_original` | 613 |  | `L630: (sum - mat.get(r, c)).abs() < 1e-12,` |
| `src/analysis/trust_region.rs` | `validate_quadratic_metric` | 54 |  | `` |
| `src/analysis/trust_region.rs` | `quadratic_cost` | 59 |  | `L69: if cost < -f64::EPSILON.sqrt() {` |
| `src/analysis/trust_region.rs` | `apply_quadratic_trust_region` | 75 |  | `L89: (max_quadratic_cost / proposed).sqrt().clamp(0.0, 1.0)` |
| `src/analysis/trust_region.rs` | `apply_pythagoras_geodesic_trust_region` | 112 | Applies a trust region constraint that eliminates the Pythagoras Staircase metric inflation in discrete high-dimensional parameter updates. | `L141: (max_quadratic_cost / proposed).sqrt().clamp(0.0, 1.0)` |
| `src/analysis/trust_region.rs` | `signed_magnitude_metric` | 182 |  | `` |
| `src/analysis/trust_region.rs` | `causal_priority_candidate` | 209 |  | `L215: let tolerance = f64::EPSILON.sqrt()` |
| `src/analysis/trust_region.rs` | `apply_causal_priority_trust_region` | 247 | Contract a proposed composition to a verified quadratic budget while preserving the fields with the largest *lower-confidence* causal benefit. The routine is in | `L266: let tolerance = f64::EPSILON.sqrt() * (1.0 + proposed.abs() + max_quadratic_cost.abs());` |
| `src/analysis/trust_region.rs` | `trust_region_rejects_indefinite_metric` | 370 |  | `` |
| `src/analysis/trust_region.rs` | `trust_region_scales_exactly_to_quadratic_budget` | 378 |  | `L384: assert!((result.proposed_quadratic_cost - 5.0).abs() < 1e-12);` |
| `src/analysis/trust_region.rs` | `causal_priority_preserves_high_value_field_before_uniform_scaling` | 396 |  | `L407: assert!((result.accepted_coefficients[0] - 1.0).abs() < 1e-10);` |
| `src/analysis/trust_region.rs` | `pythagoras_geodesic_trust_region_corrects_step_inflation` | 423 |  | `L437: assert!((result.accepted_quadratic_cost - 1.0).abs() < 1e-10);` |
| `src/analysis/sbas.rs` | `reconstruct_trajectory` | 16 |  | `L129: let mut edge_residual_norms = Vec::new();` |
| `src/analysis/sbas.rs` | `digest` | 159 |  | `` |
| `src/analysis/sbas.rs` | `parallel_base_to_variant_edges_have_near_zero_cycle_residual` | 196 |  | `L196: fn parallel_base_to_variant_edges_have_near_zero_cycle_residual() {` |
| `src/analysis/temporal_tracking.rs` | `phase_correlation` | 56 | Compute normalised phase correlation between two weight vectors.  Analogous to InSAR interferogram formation:  ```text Z_k = a_k · conj(b_k) / \|a_k · conj(b_k)\| | `L99: let peak_magnitude = (peak_real * peak_real + peak_imag * peak_imag).sqrt();` |
| `src/analysis/temporal_tracking.rs` | `sbas_inversion` | 155 | Reconstruct a time-series of parameter drift from pairwise differentials.  This is the SBAS (Small BAseline Subset) algorithm adapted from InSAR:  ```text For e | `L247: // Velocity: linear fit  u(t) = v·t + c  →  v = (n·Σ(t·u) - Σt·Σu) / (n·Σt² - (Σt)²)` |
| `src/analysis/temporal_tracking.rs` | `cholesky_solve` | 287 | Cholesky LLT solve for SPD system. | `L299: l[j * n + j] = diag.sqrt();` |
| `src/analysis/temporal_tracking.rs` | `identify_persistent_scatterers` | 389 | Identify persistent scatterers — parameters that remain structurally invariant across multiple learning epochs.  Adapted from PS-InSAR: for each parameter, we c | `L427: let variance = values.iter().map(\|v\| (v - mean).powi(2)).sum::<f64>() / nf;` |
| `src/analysis/temporal_tracking.rs` | `sbas_recovers_linear_drift` | 532 |  | `L572: (result.cumulative[i] - (i as f64 * 2.0)).abs() < 0.01,` |
| `src/analysis/temporal_tracking.rs` | `sbas_rejects_insufficient_epochs` | 584 |  | `` |
| `src/analysis/temporal_tracking.rs` | `persistent_scatterers_identify_stable_parameters` | 589 |  | `` |
| `src/analysis/identifiability.rs` | `gram_of_fields` | 37 |  | `L43: let norm_tolerance = f64::EPSILON.sqrt() * (dim.max(1) as f64).sqrt() * 16.0;` |
| `src/analysis/identifiability.rs` | `spectrum_and_rank` | 93 |  | `L97: .map(\|(eigenvalue, _)\| eigenvalue.max(0.0).sqrt())` |
| `src/analysis/identifiability.rs` | `principal_angle_min` | 120 |  | `L127: let c = cosine(&fields[i].direction, &fields[j].direction)?` |
| `src/analysis/identifiability.rs` | `resolution_map` | 136 |  | `L177: let coefficient_rms = (weighted_squared_coefficient / total_observation_weight).sqrt();` |
| `src/analysis/identifiability.rs` | `resolution_map_accepts_independent_excited_fields` | 259 |  | `L269: assert_eq!(map.resolved_rank, 2);` |
| `src/analysis/identifiability.rs` | `resolution_map_marks_collinear_fields_unresolved` | 275 |  | `L285: assert!(map.resolved_rank < 2);` |
| `src/analysis/identifiability.rs` | `resolution_map_rejects_zero_weight_and_noncanonical_field_geometry` | 291 |  | `` |
| `src/analysis/aperture_independence.rs` | `confounder_names` | 32 |  | `` |
| `src/analysis/aperture_independence.rs` | `group_design_profiles` | 41 |  | `L79: present += 1;` |
| `src/analysis/aperture_independence.rs` | `standardized_profiles` | 117 |  | `L141: stds[col] = variance.sqrt();` |
| `src/analysis/aperture_independence.rs` | `design_numerical_rank` | 163 |  | `L175: let tolerance = largest * f64::EPSILON.sqrt() * gram.rows.max(1) as f64;` |
| `src/analysis/aperture_independence.rs` | `valid_digest` | 179 |  | `` |
| `src/analysis/aperture_independence.rs` | `estimate_aperture_independence` | 241 |  | `L252: let numerical_design_rank = design_numerical_rank(&profiles)?;` |
| `src/analysis/aperture_independence.rs` | `many_well_separated_designs_exceed_three_effective_apertures` | 397 |  | `` |
| `src/analysis/block_tomography.rs` | `parameter_layout_digest` | 204 | Hash the canonical semantic projection of a validated layout. Serde field order is fixed by the Rust structs, while offsets and totals are required to be contig | `` |
| `src/analysis/block_tomography.rs` | `reconstruct_block` | 361 |  | `L405: normalized_block_energy: 0.0,` |
| `src/analysis/block_tomography.rs` | `reconstruct_structured_geometry` | 484 |  | `L511: block.normalized_block_energy = block.block_energy / total_block_energy;` |
| `src/analysis/block_tomography.rs` | `block_geometry_preserves_local_rank_and_source_coefficients` | 658 |  | `L658: fn block_geometry_preserves_local_rank_and_source_coefficients() {` |
| `src/analysis/block_tomography.rs` | `layout_complexity_limits_fail_closed_before_geometry_work` | 896 |  | `L901: let mut excessive_rank = minimal_layout();` |
| `src/analysis/dual_space.rs` | `fit_output` | 156 |  | `L179: betas.push(weighted_normal_solve(&x, &target, &weights, ridge)?);` |
| `src/analysis/dual_space.rs` | `fit_representation_map` | 191 |  | `L195: ridge: f64,` |
| `src/analysis/dual_space.rs` | `representation_centroid` | 244 |  | `L278: normalize(&centroid).map_err(\|error\| match error {` |
| `src/analysis/dual_space.rs` | `expected_field_for_observation` | 286 |  | `L303: let ambiguity_tolerance = magnitude * f64::EPSILON.sqrt();` |
| `src/analysis/dual_space.rs` | `representation_recurrence` | 315 |  | `L345: let normalized = normalize(&aligned).map_err(\|error\| match error {` |
| `src/analysis/dual_space.rs` | `analyze_dual_space` | 390 |  | `L438: let tolerance = max_abs * f64::EPSILON.sqrt();` |
| `src/analysis/dual_space.rs` | `representation_map_recovers_cross_aperture_linear_signatures` | 538 |  | `L558: assert!((fit.field_signatures[0][0] - 2.0).abs() < 1e-6);` |
| `src/analysis/dual_space.rs` | `directional_matching_cannot_hide_failed_cross_aperture_generalization` | 568 |  | `L609: ridge: 1e-9,` |
| `src/analysis/functional.rs` | `fit_functional_map` | 26 |  | `L83: let beta = weighted_normal_solve(&x, &y, &weights, ridge)?;` |
| `src/analysis/gauge.rs` | `align_bases` | 14 |  | `L26: cost[i][j] = 1.0 - cosine(&reference[i], &candidate[j])?.abs().clamp(0.0, 1.0);` |
| `src/analysis/confounders.rs` | `raw_design` | 18 |  | `L69: let std = variance.sqrt();` |
| `src/analysis/confounders.rs` | `remove_confounders` | 83 |  | `L130: let mut normal = Matrix::zeros(q, q);` |
| `src/analysis/active.rs` | `trace` | 70 |  | `` |
| `src/analysis/active.rs` | `choose_active_aperture` | 74 |  | `L96: let snr = dot(&candidate.sensing_vector, &q)?.max(0.0) / candidate.noise_variance;` |
| `src/analysis/active.rs` | `assimilate_aperture_result` | 170 | Assimilate the realized scalar result y = a^T x + ε after an aperture has actually run. This is the exact Gaussian linear update of both posterior mean and cova | `L187: let predictive_variance = dot(&candidate.sensing_vector, &projected)?.max(0.0);` |
| `src/analysis/active.rs` | `plan_active_apertures` | 212 | Sequential experiment design. Candidates are used at most once. After each selected aperture the posterior covariance is updated, so the next choice reflects in | `L255: total_information_gain += best.information_gain;` |
| `src/analysis/active.rs` | `aperture_candidate_wire_rejects_paths_and_digests` | 290 |  | `` |
| `src/analysis/active.rs` | `active_aperture_rejects_indefinite_covariance` | 310 |  | `` |
| `src/analysis/active.rs` | `active_aperture_rejects_zero_sensing_and_breaks_ties_canonically` | 325 |  | `` |
| `src/analysis/active.rs` | `active_aperture_prefers_information_when_costs_match` | 352 |  | `` |
| `src/analysis/active.rs` | `realized_aperture_updates_mean_and_covariance` | 390 |  | `L404: assert!(updated.mean[1].abs() < 1e-12);` |
| `src/analysis/interaction.rs` | `mvdr_weights` | 45 |  | `L45: pub fn mvdr_weights(covariance: &Matrix, desired: &[f64], ridge: f64) -> BrainResult<Vec<f64>> {` |
| `src/cross_model/plasticity/bcm_metaplasticity.rs` | `default` | 26 |  | `L28: initial_theta: 0.5,` |
| `src/cross_model/plasticity/bcm_metaplasticity.rs` | `validate` | 37 |  | `L38: if !self.initial_theta.is_finite()` |
| `src/cross_model/plasticity/bcm_metaplasticity.rs` | `update_threshold` | 86 |  | `L108: state.theta_m =` |
| `src/cross_model/plasticity/bcm_metaplasticity.rs` | `calculate_weight_change` | 113 |  | `L126: Ok(state.learning_rate * pre_synaptic * post_synaptic * (post_synaptic - state.theta_m))` |
| `src/cross_model/plasticity/bcm_metaplasticity.rs` | `get_threshold` | 129 |  | `L130: self.states.get(capability_name).map(\|state\| state.theta_m)` |
| `src/cross_model/plasticity/bcm_metaplasticity.rs` | `apply_decay` | 150 |  | `L152: state.theta_m = (state.theta_m * (1.0 - self.config.theta_decay)).clamp(0.0, 1.0);` |
| `src/cross_model/plasticity/eligibility_traces.rs` | `validate` | 35 |  | `L38: \|\| !self.decay_factor.is_finite()` |
| `src/cross_model/plasticity/eligibility_traces.rs` | `initialize_trace` | 66 |  | `` |
| `src/cross_model/plasticity/eligibility_traces.rs` | `update_trace` | 81 |  | `L89: trace.trace_value = (trace.trace_value * self.config.decay_factor` |
| `src/cross_model/plasticity/eligibility_traces.rs` | `accumulate_credit` | 96 |  | `L104: trace.credit_accumulated += credit * trace.trace_value;` |
| `src/cross_model/plasticity/eligibility_traces.rs` | `get_trace` | 111 |  | `` |
| `src/cross_model/plasticity/eligibility_traces.rs` | `get_accumulated_credit` | 116 |  | `` |
| `src/cross_model/plasticity/eligibility_traces.rs` | `reset_trace` | 122 |  | `` |
| `src/cross_model/plasticity/eligibility_traces.rs` | `decay_all` | 133 |  | `L133: pub fn decay_all(&mut self) {` |
| `src/cross_model/plasticity/eligibility_traces.rs` | `get_all_traces` | 141 |  | `` |
| `src/cross_model/plasticity/neuromodulation.rs` | `validate` | 65 |  | `L78: \|\| (self.weights.values().sum::<f64>() - 1.0).abs() > 1e-12` |
| `src/cross_model/plasticity/neuromodulation.rs` | `calculate_plasticity_modulation` | 118 |  | `L122: modulation += *weight * self.get_level(*modulator)?;` |
| `src/cross_model/plasticity/neuromodulation.rs` | `modulate_learning_rate` | 130 |  | `` |
| `src/cross_model/plasticity/neuromodulation.rs` | `apply_decay` | 137 |  | `L137: pub fn apply_decay(&mut self) {` |
| `src/cross_model/plasticity/content_plasticity.rs` | `default` | 34 |  | `L42: eligibility_lambda_content: 0.95,` |
| `src/cross_model/plasticity/content_plasticity.rs` | `update_source_trust` | 84 |  | `` |
| `src/cross_model/plasticity/content_plasticity.rs` | `consolidate_fact` | 124 |  | `L140: confidence = (confidence * (1.0 - decay) + evidence_strength * self.consolidation_rate)` |
| `src/cross_model/plasticity/content_plasticity.rs` | `update_similarity` | 224 |  | `L241: let evidence_strength = (1.0 - measured_similarity.abs()).clamp(0.0, 1.0);` |
| `src/cross_model/plasticity/content_plasticity.rs` | `advanced_content_plasticity_tracks_fact_confidence_and_source_trust` | 304 |  | `` |
| `src/cross_model/plasticity/routing_plasticity.rs` | `update_weight` | 62 |  | `L73: let delta = self.learning_rate * correlation - self.decay_rate;` |
| `src/cross_model/plasticity/routing_plasticity.rs` | `stability_adjusted_score` | 91 |  | `` |
| `src/cross_model/plasticity/routing_plasticity.rs` | `decay_tick` | 113 |  | `L117: let factor = 1.0 - self.decay_rate;` |
| `src/cross_model/plasticity/routing_plasticity.rs` | `route_capability` | 184 |  | `L214: * ((total_samples.max(1.0).ln() / row.sample_size as f64).max(0.0)).sqrt();` |
| `src/cross_model/plasticity/routing_plasticity.rs` | `routing_capability_prefers_stability_weighted_model_when_scores_are_equal` | 353 |  | `` |
| `src/cross_model/plasticity/elo_system.rs` | `update_observed` | 88 |  | `L114: / (1.0 + 10.0_f64.powf((second_rating - first_rating) / self.config.logistic_scale));` |
| `src/cross_model/extraction/cross_model_aligner.rs` | `validate` | 38 |  | `L43: \|\| !self.ridge_lambda.is_finite()` |
| `src/cross_model/extraction/cross_model_aligner.rs` | `default` | 90 |  | `L92: ridge_lambda: 1e-4,` |
| `src/cross_model/extraction/cross_model_aligner.rs` | `validate` | 100 |  | `L101: if !self.ridge_lambda.is_finite()` |
| `src/cross_model/extraction/cross_model_aligner.rs` | `calibrate_from_prompts` | 134 |  | `L180: ridge_lambda: self.config.ridge_lambda,` |
| `src/cross_model/extraction/cross_model_aligner.rs` | `align` | 198 |  | `L218: alignment_score: (1.0 - fitted.validation_residual).clamp(0.0, 1.0),` |
| `src/cross_model/extraction/cross_model_aligner.rs` | `fit` | 244 |  | `L266: calibration.ridge_lambda` |
| `src/cross_model/extraction/cross_model_aligner.rs` | `predict` | 303 |  | `L303: fn predict(map: &FittedRidgeMap, source: &Tensor) -> Result<Tensor, String> {` |
| `src/cross_model/extraction/cross_model_aligner.rs` | `validation_residual` | 333 |  | `L349: Ok((error_sq / target_sq).sqrt())` |
| `src/cross_model/extraction/cross_model_aligner.rs` | `dot` | 352 |  | `L352: fn dot(left: &[f64], right: &[f64]) -> f64 {` |
| `src/cross_model/extraction/cross_model_aligner.rs` | `calibration_digest` | 356 |  | `` |
| `src/cross_model/extraction/hierarchical_steering_extractor.rs` | `validate` | 72 |  | `L80: \|\| !self.minimum_mean_direction_cosine.is_finite()` |
| `src/cross_model/extraction/hierarchical_steering_extractor.rs` | `extract` | 99 |  | `L168: + ((best.quality.mean_direction_cosine + 1.0) / 2.0))` |
| `src/cross_model/extraction/hierarchical_steering_extractor.rs` | `extract_layer` | 220 |  | `L262: *target += *value / pair_differences.len() as f64;` |
| `src/cross_model/extraction/hierarchical_steering_extractor.rs` | `quality_score` | 314 |  | `L315: metrics.direction_persistence * ((metrics.mean_direction_cosine + 1.0) / 2.0)` |
| `src/cross_model/extraction/lora_synthesizer.rs` | `validate` | 46 |  | `L50: \|\| self.rank == 0` |
| `src/cross_model/extraction/lora_synthesizer.rs` | `materialize_dense` | 68 |  | `L77: let scale = self.alpha / self.rank as f64;` |
| `src/cross_model/extraction/lora_synthesizer.rs` | `build_from_verified_factors` | 164 |  | `L176: // Core factors are dense = left @ right. PEFT uses B @ A * alpha/rank.` |
| `src/cross_model/extraction/lora_synthesizer.rs` | `synthesis_digest` | 240 |  | `` |
| `src/cross_model/extraction/counterfactual_analyzer.rs` | `validate` | 73 |  | `L86: \|\| !row.cosine_similarity.is_finite()` |
| `src/cross_model/extraction/counterfactual_analyzer.rs` | `analyze_one` | 156 |  | `L165: let original_score = scenario.verifier.score(&original.text)?;` |
| `src/cross_model/extraction/counterfactual_analyzer.rs` | `counterfactual_digest` | 261 |  | `` |
| `src/cross_model/discovery/gap_detector.rs` | `detect_gaps` | 102 |  | `` |
| `src/cross_model/discovery/gap_detector.rs` | `detect_from_evaluations` | 119 |  | `L174: let radius = ((1.0 / benchmark.significance_alpha).ln() * sum_weight_sq` |
| `src/cross_model/discovery/gap_detector.rs` | `gap_digest` | 203 |  | `` |
| `src/cross_model/discovery/gap_detector.rs` | `conservative_gap_requires_real_margin` | 284 |  | `` |
| `src/cross_model/discovery/prioritizer.rs` | `score_gap` | 106 |  | `L110: let receiver_deficit = (1.0 - gap.target_score).clamp(0.0, 1.0);` |
| `src/cross_model/discovery/prioritizer.rs` | `priority_digest` | 152 |  | `` |
| `src/cross_model/promotion/promotion_gates.rs` | `run_all_gates` | 79 |  | `L124: let preservation_score = minimum_score_for(validation, EvidenceType::Preservation);` |
| `src/cross_model/promotion/promotion_gates.rs` | `minimum_score_for` | 204 |  | `` |
| `src/cross_model/promotion/domain_fitness.rs` | `evaluate_fitness` | 60 |  | `` |
| `src/cross_model/promotion/evidence_validator.rs` | `validate` | 142 |  | `L164: let minimum_score = items` |
| `src/learning/solver_portfolio.rs` | `as_digest` | 40 |  | `` |
| `src/learning/solver_portfolio.rs` | `as_digest` | 62 |  | `` |
| `src/learning/solver_portfolio.rs` | `as_digest` | 77 |  | `` |
| `src/learning/solver_portfolio.rs` | `as_digest` | 96 |  | `` |
| `src/learning/solver_portfolio.rs` | `digest` | 159 |  | `` |
| `src/learning/solver_portfolio.rs` | `max_numerical_work_units` | 286 |  | `L298: relative_rank_tolerance: f64,` |
| `src/learning/solver_portfolio.rs` | `default` | 314 |  | `L318: relative_rank_tolerance: 1.0e-10,` |
| `src/learning/solver_portfolio.rs` | `validate` | 333 |  | `L338: \|\| !self.relative_rank_tolerance.is_finite()` |
| `src/learning/solver_portfolio.rs` | `with_rank_policy` | 374 |  | `L374: pub fn with_rank_policy(` |
| `src/learning/solver_portfolio.rs` | `with_residual_tolerances` | 389 |  | `L389: pub fn with_residual_tolerances(mut self, relative: f64, absolute: f64) -> BrainResult<Self> {` |
| `src/learning/solver_portfolio.rs` | `with_svd_convergence` | 396 |  | `L396: pub fn with_svd_convergence(` |
| `src/learning/solver_portfolio.rs` | `relative_rank_tolerance` | 427 |  | `L427: pub fn relative_rank_tolerance(&self) -> f64 {` |
| `src/learning/solver_portfolio.rs` | `minimum_rank` | 435 |  | `L435: pub fn minimum_rank(&self) -> usize {` |
| `src/learning/solver_portfolio.rs` | `maximum_rank` | 439 |  | `L439: pub fn maximum_rank(&self) -> usize {` |
| `src/learning/solver_portfolio.rs` | `relative_residual_tolerance` | 443 |  | `L443: pub fn relative_residual_tolerance(&self) -> f64 {` |
| `src/learning/solver_portfolio.rs` | `absolute_residual_tolerance` | 447 |  | `L447: pub fn absolute_residual_tolerance(&self) -> f64 {` |
| `src/learning/solver_portfolio.rs` | `svd_orthogonality_tolerance` | 451 |  | `L451: pub fn svd_orthogonality_tolerance(&self) -> f64 {` |
| `src/learning/solver_portfolio.rs` | `max_svd_sweeps` | 455 |  | `L455: pub fn max_svd_sweeps(&self) -> usize {` |
| `src/learning/solver_portfolio.rs` | `digest` | 467 |  | `L478: self.relative_rank_tolerance,` |
| `src/learning/solver_portfolio.rs` | `values` | 648 |  | `L662: LowRank {` |
| `src/learning/solver_portfolio.rs` | `rank` | 708 |  | `L708: pub fn rank(&self) -> Option<usize> {` |
| `src/learning/solver_portfolio.rs` | `stored_parameter_count` | 715 |  | `L717: Self::LowRank {` |
| `src/learning/solver_portfolio.rs` | `materialize_dense` | 733 |  | `L755: Self::LowRank {` |
| `src/learning/solver_portfolio.rs` | `exact_digest` | 863 |  | `L879: Self::LowRank {` |
| `src/learning/solver_portfolio.rs` | `append_f64_slice` | 926 |  | `L952: target_norm: f64,` |
| `src/learning/solver_portfolio.rs` | `absolute_residual` | 958 |  | `L958: pub fn absolute_residual(&self) -> f64 {` |
| `src/learning/solver_portfolio.rs` | `root_mean_square_residual` | 962 |  | `L962: pub fn root_mean_square_residual(&self) -> f64 {` |
| `src/learning/solver_portfolio.rs` | `maximum_absolute_residual` | 966 |  | `L966: pub fn maximum_absolute_residual(&self) -> f64 {` |
| `src/learning/solver_portfolio.rs` | `relative_residual` | 970 |  | `L970: pub fn relative_residual(&self) -> Option<f64> {` |
| `src/learning/solver_portfolio.rs` | `target_norm` | 974 |  | `L974: pub fn target_norm(&self) -> f64 {` |
| `src/learning/solver_portfolio.rs` | `frobenius_norm` | 978 |  | `L978: pub fn frobenius_norm(&self) -> f64 {` |
| `src/learning/solver_portfolio.rs` | `stored_parameter_count` | 982 |  | `L991: RankDeficient,` |
| `src/learning/solver_portfolio.rs` | `gram_spectrum` | 1019 |  | `` |
| `src/learning/solver_portfolio.rs` | `numerical_rank` | 1027 |  | `L1027: pub fn numerical_rank(&self) -> usize {` |
| `src/learning/solver_portfolio.rs` | `effective_rank` | 1031 |  | `L1031: pub fn effective_rank(&self) -> f64 {` |
| `src/learning/solver_portfolio.rs` | `energy_rank` | 1035 |  | `L1035: pub fn energy_rank(&self) -> usize {` |
| `src/learning/solver_portfolio.rs` | `finite_singular_condition_number` | 1047 |  | `` |
| `src/learning/solver_portfolio.rs` | `finite_gram_condition_number` | 1051 |  | `` |
| `src/learning/solver_portfolio.rs` | `direct_svd_sweeps` | 1055 |  | `L1055: pub fn direct_svd_sweeps(&self) -> usize {` |
| `src/learning/solver_portfolio.rs` | `direct_svd_reconstruction_relative_error` | 1063 |  | `L1063: pub fn direct_svd_reconstruction_relative_error(&self) -> f64 {` |
| `src/learning/solver_portfolio.rs` | `direct_svd_vector_orthogonality_error` | 1067 |  | `L1067: pub fn direct_svd_vector_orthogonality_error(&self) -> f64 {` |
| `src/learning/solver_portfolio.rs` | `constructed_rank` | 1125 |  | `L1125: pub fn constructed_rank(&self) -> Option<usize> {` |
| `src/learning/solver_portfolio.rs` | `metrics` | 1133 |  | `L1159: target_norm_bits: u64,` |
| `src/learning/solver_portfolio.rs` | `from` | 1165 |  | `L1171: target_norm_bits: metrics.target_norm.to_bits(),` |
| `src/learning/solver_portfolio.rs` | `from` | 1197 |  | `L1210: numerical_rank: diagnostics.numerical_rank,` |
| `src/learning/solver_portfolio.rs` | `problem_digest` | 1283 |  | `` |
| `src/learning/solver_portfolio.rs` | `policy_digest` | 1287 |  | `` |
| `src/learning/solver_portfolio.rs` | `exact_digest` | 1293 | Return the exact run digest only after recomputing and authenticating the complete canonical projection. | `` |
| `src/learning/solver_portfolio.rs` | `projection` | 1375 |  | `` |
| `src/learning/solver_portfolio.rs` | `seal_portfolio_report` | 1438 |  | `` |
| `src/learning/solver_portfolio.rs` | `bounded_unknown` | 1505 |  | `` |
| `src/learning/solver_portfolio.rs` | `solve_with_portfolio` | 1536 | Diagnose, generate, verify, and deterministically select bounded solver candidates. This function never promotes or persists its selected proposal. | `L1585: EvaluationReason::ResourceLimit("solver_max_svd_work_units"),` |
| `src/learning/solver_portfolio.rs` | `seal_with_unavailable_direct_solver` | 1687 |  | `L1695: NumericalBackendRole::BuiltInDirectJacobiSvd,` |
| `src/learning/solver_portfolio.rs` | `problem_limit` | 1764 |  | `L1795: let safe_gram_magnitude = (f64::MAX / energy_terms).sqrt();` |
| `src/learning/solver_portfolio.rs` | `direct_solver_work_upper_bound` | 1839 | Conservative work bound for the complete built-in rank-revealing path: Jacobi sweeps (including convergence checks), decomposition verification, Gram diagnostic | `L1848: let sweeps = u128::try_from(policy.max_svd_sweeps).ok()?;` |
| `src/learning/solver_portfolio.rs` | `single_candidate_evaluation_work_upper_bound` | 1917 |  | `` |
| `src/learning/solver_portfolio.rs` | `row_gram` | 1929 |  | `L1935: let value = scaled_compensated_dot(inputs.row(row), inputs.row(column))?;` |
| `src/learning/solver_portfolio.rs` | `compensated_sum` | 1945 |  | `L1953: if sum.abs() >= value.abs() {` |
| `src/learning/solver_portfolio.rs` | `scaled_compensated_dot` | 1971 | Scale-normalized Neumaier dot product used at the independent numerical boundary. The common linalg dot remains the general fast primitive; this path additional | `L1980: let normalized = compensated_sum(` |
| `src/learning/solver_portfolio.rs` | `direct_one_sided_jacobi_svd` | 2013 | Thin one-sided Jacobi SVD of X^T. It orthogonalizes the case columns directly instead of diagonalizing X X^T, so rank revelation does not first square the condi | `L2092: let correlation = gamma.abs() / (alpha.sqrt() * beta.sqrt());` |
| `src/learning/solver_portfolio.rs` | `verify_direct_svd` | 2203 |  | `L2242: let input_norm = norm(problem.inputs.as_slice())?;` |
| `src/learning/solver_portfolio.rs` | `rotate_columns` | 2260 |  | `L2260: fn rotate_columns(columns: &mut [Vec<f64>], first: usize, second: usize, cosine: f64, sine: f64) {` |
| `src/learning/solver_portfolio.rs` | `maximum_column_correlation` | 2272 |  | `L2282: maximum = maximum.max(gamma.abs() / (alpha.sqrt() * beta.sqrt()));` |
| `src/learning/solver_portfolio.rs` | `diagnose_problem` | 2291 |  | `L2322: let normalized_spectrum = if largest == 0.0 {` |
| `src/learning/solver_portfolio.rs` | `cholesky_eligibility` | 2392 |  | `L2400: if problem.case_count() > MAX_LOW_RANK as usize {` |
| `src/learning/solver_portfolio.rs` | `run_cholesky` | 2428 |  | `L2430: NumericalBackendRole::BuiltInCholeskyRidge,` |
| `src/learning/solver_portfolio.rs` | `run_direct_svd` | 2476 |  | `L2476: fn run_direct_svd(` |
| `src/learning/solver_portfolio.rs` | `direct_svd_factors` | 2531 |  | `L2531: fn direct_svd_factors(` |
| `src/learning/solver_portfolio.rs` | `compact_representation` | 2577 |  | `L2580: rank: usize,` |
| `src/learning/solver_portfolio.rs` | `run_external` | 2607 |  | `L2624: constructed_rank: candidate.rank(),` |
| `src/learning/solver_portfolio.rs` | `evaluate_candidate` | 2674 |  | `L2708: let relative_allowance = policy.relative_residual_tolerance * metrics.target_norm;` |
| `src/learning/solver_portfolio.rs` | `measure_candidate` | 2749 |  | `L2775: let absolute_residual = norm(&residuals)?;` |
| `src/learning/solver_portfolio.rs` | `choose_accepted` | 2849 |  | `L2886: .frobenius_norm` |
| `src/learning/solver_portfolio.rs` | `exact_policy` | 2900 |  | `L2902: .with_rank_policy(1.0e-10, 0.999999, 1, 64)` |
| `src/learning/solver_portfolio.rs` | `rejects_wrong_shape_and_nonfinite_problem` | 2909 |  | `` |
| `src/learning/solver_portfolio.rs` | `exact_problem_digest_commits_to_shape_order_and_ieee_bits` | 2919 |  | `` |
| `src/learning/solver_portfolio.rs` | `exact_candidate_and_policy_digests_commit_to_semantics` | 2946 |  | `L2957: let factored_zero = CandidateRepresentation::LowRank {` |
| `src/learning/solver_portfolio.rs` | `rank_deficiency_forbids_cholesky_and_uses_minimum_norm_spectral_path` | 2972 |  | `L2972: fn rank_deficiency_forbids_cholesky_and_uses_minimum_norm_spectral_path() {` |
| `src/learning/solver_portfolio.rs` | `direct_svd_uses_tall_orientation_for_overdetermined_contracts` | 2988 |  | `L2988: fn direct_svd_uses_tall_orientation_for_overdetermined_contracts() {` |
| `src/learning/solver_portfolio.rs` | `ill_conditioned_full_rank_problem_selects_preplanned_spectral_backend` | 3016 |  | `L3016: fn ill_conditioned_full_rank_problem_selects_preplanned_spectral_backend() {` |
| `src/learning/solver_portfolio.rs` | `spectral_rank_expands_until_the_contract_residual_is_met` | 3033 |  | `L3033: fn spectral_rank_expands_until_the_contract_residual_is_met() {` |
| `src/learning/solver_portfolio.rs` | `repeated_solve_is_deterministic` | 3056 |  | `` |
| `src/learning/solver_portfolio.rs` | `zero_target_has_a_verified_zero_minimum_norm_solution` | 3092 |  | `L3092: fn zero_target_has_a_verified_zero_minimum_norm_solution() {` |
| `src/learning/solver_portfolio.rs` | `degenerate_zero_design_still_returns_the_verified_zero_solution` | 3117 |  | `L3126: assert_eq!(report.diagnostics().unwrap().numerical_rank(), 0);` |
| `src/learning/solver_portfolio.rs` | `scale_free_rank_diagnostics_do_not_turn_tiny_data_into_rank_zero` | 3142 |  | `L3142: fn scale_free_rank_diagnostics_do_not_turn_tiny_data_into_rank_zero() {` |
| `src/learning/solver_portfolio.rs` | `overflowing_tolerance_is_unknown_and_never_silently_accepts` | 3159 |  | `L3162: .with_residual_tolerances(f64::MAX, f64::MAX)` |
| `src/learning/solver_portfolio.rs` | `direct_svd_and_verifier_handle_large_finite_scaling` | 3171 |  | `L3171: fn direct_svd_and_verifier_handle_large_finite_scaling() {` |
| `src/learning/solver_portfolio.rs` | `compensated_verification_preserves_cancellation_residual` | 3199 |  | `L3199: fn compensated_verification_preserves_cancellation_residual() {` |
| `src/learning/solver_portfolio.rs` | `exhausted_direct_svd_sweeps_are_bounded_unknown` | 3205 |  | `L3205: fn exhausted_direct_svd_sweeps_are_bounded_unknown() {` |
| `src/learning/solver_portfolio.rs` | `external_proposal_cannot_self_report_or_bypass_residual_verification` | 3230 |  | `L3230: fn external_proposal_cannot_self_report_or_bypass_residual_verification() {` |
| `src/learning/solver_portfolio.rs` | `gram_overflow_risk_is_a_bounded_unknown` | 3366 |  | `` |
| `src/learning/solver_portfolio.rs` | `direct_svd_work_is_bounded_before_execution` | 3375 |  | `L3375: fn direct_svd_work_is_bounded_before_execution() {` |
| `src/learning/solver_portfolio.rs` | `bounded_builtin_work_does_not_hide_an_exact_external_candidate` | 3393 |  | `` |
| `src/learning/solver_portfolio.rs` | `aggregate_external_candidate_storage_is_bounded_before_cloning` | 3449 |  | `` |
| `src/learning/solver_portfolio.rs` | `direct_svd_work_limit_has_an_absolute_ceiling` | 3477 |  | `L3477: fn direct_svd_work_limit_has_an_absolute_ceiling() {` |
| `src/learning/solver_portfolio.rs` | `public_candidate_materialization_enforces_the_absolute_dense_limit` | 3486 |  | `` |
| `src/learning/solver_portfolio.rs` | `sparse_and_block_claims_are_verified_from_canonical_structure` | 3500 |  | `L3539: assert_eq!(external.metrics.as_ref().unwrap().absolute_residual, 0.0);` |
| `src/learning/solver_portfolio.rs` | `compact_solution_retains_low_rank_factors_when_they_are_smaller` | 3575 |  | `L3575: fn compact_solution_retains_low_rank_factors_when_they_are_smaller() {` |
| `src/learning/solver_portfolio.rs` | `rank_cap_reports_bounded_unknown_instead_of_false_impossibility` | 3592 |  | `L3592: fn rank_cap_reports_bounded_unknown_instead_of_false_impossibility() {` |
| `src/learning/causal_credit.rs` | `lower_confidence_bound` | 51 | Pessimistic 95% interaction effect used by governed routing. Positive synergy is admitted only when it survives its uncertainty margin; possible negative interf | `` |
| `src/learning/causal_credit.rs` | `certified_causal_priority_weights` | 84 | Return the conservative, evidence-backed causal utility weight for every runtime field in the caller's canonical order.  The lower confidence bound, rather than | `` |
| `src/learning/causal_credit.rs` | `mean_and_se` | 166 |  | `L178: / (values.len() - 1) as f64;` |
| `src/learning/causal_credit.rs` | `estimate_causal_credit` | 199 |  | `L262: raw_pair_count += 1;` |
| `src/learning/causal_credit.rs` | `factorial` | 368 |  | `` |
| `src/learning/causal_credit.rs` | `compute_shapley_values` | 374 | Computes formal N-player Shapley Values for each skill field across contexts: \phi_i = \sum_{S \subseteq N \setminus \{i\}} \frac{\|S\|!(\|N\|-\|S\|-1)!}{\|N\|!} ( v(S  | `L434: total_shapley += weight * avg_diff;` |
| `src/learning/causal_credit.rs` | `causal_credit_recovers_main_and_interaction_effects` | 455 |  | `L490: assert!((a.mean_marginal_effect - 2.25).abs() < 1e-12);` |
| `src/learning/causal_credit.rs` | `shapley_values_computed_correctly` | 523 |  | `L552: // \phi_b = 0.5 * (1.0 - 0.0) + 0.5 * (3.5 - 2.0) = 0.5 + 0.75 = 1.25` |
| `src/learning/sleep_evidence.rs` | `verify_file` | 279 |  | `` |
| `src/learning/sleep_evidence.rs` | `finite_scalar_match` | 300 |  | `L303: && (left - right).abs() <= 64.0 * f64::EPSILON * (1.0 + left.abs().max(right.abs()))` |
| `src/learning/sleep_evidence.rs` | `finite_vector_match` | 306 |  | `` |
| `src/learning/sleep_evidence.rs` | `exact_metric_schema` | 324 |  | `` |
| `src/learning/sleep_evidence.rs` | `mean_metrics` | 330 |  | `` |
| `src/learning/sleep_evidence.rs` | `verify_full_coalition_evaluations` | 498 |  | `` |
| `src/learning/sleep_evidence.rs` | `recompute_functional_replay` | 557 |  | `L648: / (count - 1) as f64;` |
| `src/learning/sleep_evidence.rs` | `verify_protection_artifacts` | 735 |  | `` |
| `src/learning/sleep_evidence.rs` | `verify_causal_credit_artifacts` | 743 |  | `` |
| `src/learning/sleep_evidence.rs` | `verified_causal_credit_with_weights` | 1044 | Load causal credit only after its source replay and wrapper have both been recomputed and matched. The returned priority vector is the authoritative lower-confi | `` |
| `src/learning/sleep_evidence.rs` | `dense_fields_from_observations` | 1071 |  | `L1096: if coefficient.abs() <= f64::EPSILON {` |
| `src/learning/sleep_evidence.rs` | `verify_sleep_evidence` | 1136 |  | `L1204: let tolerance = f64::EPSILON.sqrt() * map.parameter_dimension.max(1) as f64 * 8.0;` |
| `src/learning/sleep_evidence.rs` | `load_sleep_evidence` | 1551 |  | `` |
| `src/learning/sleep_evidence.rs` | `load_sleep_evidence_rejects_symlink_or_directory_current_pointer` | 1612 |  | `` |
| `src/learning/sleep_evidence.rs` | `sensitivity_loader_rejects_artifact_under_symlinked_parent` | 1629 |  | `L1647: "causal_damage_per_parameter_norm":1.0,` |
| `src/learning/sleep_evidence.rs` | `sleep_evidence_recomputes_replay_and_rejects_summary_forgery` | 1694 |  | `L1784: {"probe_id":"p1","artifact":gradient_a,"causal_damage_per_parameter_norm":1.0,"reliability":1.0},` |
| `src/learning/procedural_memory.rs` | `is_zero_digest` | 77 |  | `` |
| `src/learning/procedural_memory.rs` | `canonical_finite` | 81 |  | `` |
| `src/learning/procedural_memory.rs` | `is_canonical_finite` | 85 |  | `` |
| `src/learning/procedural_memory.rs` | `bind_exact_digest` | 103 | Bind an already authenticated byte identity into this semantic domain.  Equal raw hashes in different domains remain distinct Rust types and receive different s | `` |
| `src/learning/procedural_memory.rs` | `deserialize` | 302 |  | `L334: LowRankObserved,` |
| `src/learning/procedural_memory.rs` | `new` | 363 |  | `L366: estimated_effective_rank: Option<u64>,` |
| `src/learning/procedural_memory.rs` | `validate_axes` | 379 |  | `L386: .estimated_effective_rank` |
| `src/learning/procedural_memory.rs` | `digest` | 535 |  | `` |
| `src/learning/procedural_memory.rs` | `estimated_effective_rank` | 548 |  | `L548: pub fn estimated_effective_rank(&self) -> Option<u64> {` |
| `src/learning/procedural_memory.rs` | `unit_interval` | 557 |  | `L564: CholeskyRidgeLowRank,` |
| `src/learning/procedural_memory.rs` | `validate` | 662 |  | `L676: (SolverFamily::CholeskyRidgeLowRank, SolverParameters::LowRank { rank }) => *rank > 0,` |
| `src/learning/procedural_memory.rs` | `validate_for` | 771 |  | `L779: SolverParameters::LowRank { rank } => u64::from(*rank) <= minimum_dimension,` |
| `src/learning/procedural_memory.rs` | `rank` | 813 |  | `L813: pub fn rank(&self) -> Option<u32> {` |
| `src/learning/procedural_memory.rs` | `digest` | 821 |  | `` |
| `src/learning/procedural_memory.rs` | `problem_digest` | 926 |  | `` |
| `src/learning/procedural_memory.rs` | `candidate_digest` | 930 |  | `` |
| `src/learning/procedural_memory.rs` | `receipt_digest` | 1053 |  | `` |
| `src/learning/procedural_memory.rs` | `research_evaluation_design` | 1062 |  | `L1092: RankInsufficient,` |
| `src/learning/procedural_memory.rs` | `kind` | 1150 |  | `L1158: IncreaseRank,` |
| `src/learning/procedural_memory.rs` | `new` | 1213 | Test/internal constructor. Production code derives these fields from a selected solver evaluation and its step-local independent gate. | `L1214: solver_relative_residual: Option<f64>,` |
| `src/learning/procedural_memory.rs` | `positive_utility` | 1298 |  | `L1300: (Some(true), Some(residual)) => Some(1.0 / (1.0 + residual.get())),` |
| `src/learning/procedural_memory.rs` | `projection` | 1581 |  | `` |
| `src/learning/procedural_memory.rs` | `calculate_digest` | 1596 |  | `` |
| `src/learning/procedural_memory.rs` | `digest` | 1637 |  | `` |
| `src/learning/procedural_memory.rs` | `configuration_matches_backend` | 1664 |  | `L1669: NumericalBackendRole::BuiltInCholeskyRidge => {` |
| `src/learning/procedural_memory.rs` | `failure_from_solver_rejection` | 1691 |  | `L1693: EvaluationReason::ResidualExceedsTolerance => {` |
| `src/learning/procedural_memory.rs` | `projection` | 1843 |  | `` |
| `src/learning/procedural_memory.rs` | `calculate_digest` | 1856 |  | `` |
| `src/learning/procedural_memory.rs` | `digest` | 1883 |  | `` |
| `src/learning/procedural_memory.rs` | `projection` | 2080 |  | `` |
| `src/learning/procedural_memory.rs` | `calculate_digest` | 2095 |  | `` |
| `src/learning/procedural_memory.rs` | `digest` | 2118 |  | `` |
| `src/learning/procedural_memory.rs` | `solver_run_receipt` | 2127 |  | `` |
| `src/learning/procedural_memory.rs` | `solver_run_failure_outcome` | 2131 |  | `L2139: CholeskyGate::RankDeficient \| CholeskyGate::Degenerate,` |
| `src/learning/procedural_memory.rs` | `classify_solver_resource` | 2187 |  | `L2197: \| "solver_max_svd_work_units"` |
| `src/learning/procedural_memory.rs` | `feature_bounded` | 2255 |  | `` |
| `src/learning/procedural_memory.rs` | `digest` | 2331 |  | `` |
| `src/learning/procedural_memory.rs` | `priority_score` | 2449 |  | `` |
| `src/learning/procedural_memory.rs` | `advice` | 2479 |  | `` |
| `src/learning/procedural_memory.rs` | `solver_run_cautions` | 2494 | Exact-policy cautions from runs that produced no candidate. These are separate from configuration advice because attributing a portfolio preflight failure to on | `` |
| `src/learning/procedural_memory.rs` | `solver_run_failure_count` | 2593 |  | `` |
| `src/learning/procedural_memory.rs` | `derive_functional_change` | 2784 | Derive only non-causal functional drift from reports already reduced into this memory. Revisions and invalidation boundaries come from the registered report-to- | `` |
| `src/learning/procedural_memory.rs` | `record_solver_run_failure` | 2876 | Preserve a candidate-free solver failure without allowing it to become candidate evidence or promotion authority. | `` |
| `src/learning/procedural_memory.rs` | `rebuild_with_solver_failures` | 2919 | Canonical replay including failures that occurred before candidate materialization. | `` |
| `src/learning/procedural_memory.rs` | `retrieve` | 2952 |  | `L3046: 1.0 - (-(evaluation.independent_group_count as f64)` |
| `src/learning/procedural_memory.rs` | `solver_run_receipt_semantics_equal` | 3284 |  | `` |
| `src/learning/procedural_memory.rs` | `correction_is_realized` | 3299 |  | `L3305: CorrectionKind::IncreaseRank => matches!(` |
| `src/learning/procedural_memory.rs` | `applicability_similarity` | 3370 |  | `L3380: let row_score = logarithmic_ratio(left.dimensions.rows, right.dimensions.rows);` |
| `src/learning/procedural_memory.rs` | `add_optional_unit_similarity` | 3454 |  | `L3461: *weighted_score += weight * (1.0 - (left.get() - right.get()).abs());` |
| `src/learning/procedural_memory.rs` | `ratio_similarity` | 3469 |  | `L3477: (1.0 - (left - right).abs()).clamp(0.0, 1.0)` |
| `src/learning/procedural_memory.rs` | `raw_digest` | 3492 |  | `` |
| `src/learning/procedural_memory.rs` | `sealed_digest` | 3496 |  | `` |
| `src/learning/procedural_memory.rs` | `config` | 3551 |  | `L3553: SolverFamily::CholeskyRidgeLowRank => {` |
| `src/learning/procedural_memory.rs` | `tamper_and_semantic_relabel_are_rejected` | 3763 |  | `L3768: config(SolverFamily::DivideConquerSvd),` |
| `src/learning/procedural_memory.rs` | `exact_scope_prevents_cross_project_and_cross_capability_advice` | 3841 |  | `` |
| `src/learning/procedural_memory.rs` | `functional_drift_uses_registered_scope_and_revision_and_makes_advice_stale` | 3873 |  | `` |
| `src/learning/procedural_memory.rs` | `unregistered_drift_reports_are_bounded_unknown_not_free_invalidation` | 3918 |  | `` |
| `src/learning/procedural_memory.rs` | `failed_attempt_is_negative_memory_but_never_a_blocking_authority` | 3939 |  | `L3945: config(SolverFamily::DivideConquerSvd),` |
| `src/learning/procedural_memory.rs` | `declared_external_backend_identity_never_becomes_transferable_advice` | 3979 |  | `` |
| `src/learning/procedural_memory.rs` | `ranking_and_rebuild_are_invariant_to_record_order` | 3997 |  | `L3997: fn ranking_and_rebuild_are_invariant_to_record_order() {` |
| `src/learning/procedural_memory.rs` | `no_independent_evidence_means_no_advice` | 4029 |  | `` |
| `src/learning/procedural_memory.rs` | `lineage_cannot_cross_scope_or_apply_an_unproposed_correction` | 4130 |  | `L4147: config(SolverFamily::DivideConquerSvd),` |
| `src/learning/procedural_memory.rs` | `lineage_cannot_claim_a_correction_that_configuration_did_not_realize` | 4182 |  | `L4188: config(SolverFamily::CholeskyRidgeLowRank),` |
| `src/learning/procedural_memory.rs` | `reducer_deduplicates_exact_replay_and_enforces_resource_bounds` | 4240 |  | `` |
| `src/learning/procedural_memory.rs` | `rank_unknown_is_distinct_from_observed_zero_and_missing_work_is_allowed` | 4287 |  | `L4287: fn rank_unknown_is_distinct_from_observed_zero_and_missing_work_is_allowed() {` |
| `src/learning/procedural_memory.rs` | `candidate_free_solver_failure_is_sealed_and_replay_safe` | 4336 |  | `` |
| `src/learning/procedural_memory.rs` | `materialized_solver_rejection_is_a_typed_negative_attempt_not_a_run_failure` | 4412 |  | `L4419: .with_residual_tolerances(1.0e-12, 1.0e-12)` |
| `src/learning/procedural_memory.rs` | `solver_run_receipt_replay_is_not_new_evidence_and_relabel_is_rejected` | 4499 |  | `` |
| `src/learning/portfolio_governance.rs` | `from_projection` | 118 |  | `` |
| `src/learning/portfolio_governance.rs` | `metric_id` | 272 |  | `` |
| `src/learning/portfolio_governance.rs` | `metric_catalog_digest` | 304 |  | `` |
| `src/learning/portfolio_governance.rs` | `digest` | 441 |  | `` |
| `src/learning/portfolio_governance.rs` | `digest` | 550 |  | `` |
| `src/learning/portfolio_governance.rs` | `metric_catalog_digest` | 554 |  | `` |
| `src/learning/portfolio_governance.rs` | `evaluation_policy_digest` | 558 |  | `` |
| `src/learning/portfolio_governance.rs` | `independence_design_digest` | 578 | Identity of the declared independent-group set, without metric values, candidate identity, repetitions, or observation time. This proves stable binding and dete | `` |
| `src/learning/portfolio_governance.rs` | `projection` | 582 |  | `` |
| `src/learning/portfolio_governance.rs` | `stable_mean` | 863 |  | `L872: .map(\|value\| value.abs())` |
| `src/learning/portfolio_governance.rs` | `digest` | 1003 |  | `` |
| `src/learning/portfolio_governance.rs` | `digest` | 1078 |  | `` |
| `src/learning/portfolio_governance.rs` | `report_digest` | 1082 |  | `` |
| `src/learning/portfolio_governance.rs` | `metric_catalog_digest` | 1086 |  | `` |
| `src/learning/portfolio_governance.rs` | `evaluation_policy_digest` | 1090 |  | `` |
| `src/learning/portfolio_governance.rs` | `independence_design_digest` | 1094 |  | `` |
| `src/learning/portfolio_governance.rs` | `projection` | 1110 |  | `` |
| `src/learning/portfolio_governance.rs` | `decide_candidate` | 1209 |  | `L1342: normalization_scale: FiniteF64,` |
| `src/learning/portfolio_governance.rs` | `new` | 1347 |  | `L1349: normalization_scale: f64,` |
| `src/learning/portfolio_governance.rs` | `digest` | 1538 |  | `` |
| `src/learning/portfolio_governance.rs` | `quality_metric_id` | 1542 |  | `` |
| `src/learning/portfolio_governance.rs` | `quality_normalization_scale` | 1546 |  | `L1546: pub fn quality_normalization_scale(&self) -> BrainResult<f64> {` |
| `src/learning/portfolio_governance.rs` | `digest` | 1796 |  | `` |
| `src/learning/portfolio_governance.rs` | `endpoint_distance_lower` | 1998 |  | `` |
| `src/learning/portfolio_governance.rs` | `digest` | 2026 |  | `` |
| `src/learning/portfolio_governance.rs` | `interval_distance_bounds` | 2096 |  | `L2126: let scale = policy.normalization_scale.get();` |
| `src/learning/portfolio_governance.rs` | `finite_option` | 2133 |  | `` |
| `src/learning/portfolio_governance.rs` | `digest` | 2631 |  | `` |
| `src/learning/portfolio_governance.rs` | `digest` | 2887 |  | `` |
| `src/learning/portfolio_governance.rs` | `digest` | 2948 |  | `` |
| `src/learning/portfolio_governance.rs` | `adaptive_score` | 2973 |  | `L2991: let exploration = policy.exploration_strength.get() * (numerator.ln() / denominator).sqrt();` |
| `src/learning/portfolio_governance.rs` | `allocate_adaptive_budget` | 3000 |  | `L3148: let score =` |
| `src/learning/portfolio_governance.rs` | `digest` | 3301 |  | `` |
| `src/learning/portfolio_governance.rs` | `digest` | 3402 |  | `` |
| `src/learning/portfolio_governance.rs` | `metric` | 4044 |  | `` |
| `src/learning/portfolio_governance.rs` | `eligible_candidate` | 4152 |  | `` |
| `src/learning/portfolio_governance.rs` | `absolute_level_and_paired_effect_have_separate_empirical_envelopes` | 4230 |  | `` |
| `src/learning/portfolio_governance.rs` | `petfc_direct_path_has_unit_tortuosity_and_zero_waste` | 4346 |  | `L4371: assert!((assessment.path_length_upper().unwrap() - 0.4).abs() < 1.0e-12);` |
| `src/learning/portfolio_governance.rs` | `petfc_reproduces_pythagorean_staircase_and_enforces_time_chain` | 4379 |  | `L4431: let expected = 2.0_f64.sqrt();` |
| `src/learning/portfolio_governance.rs` | `petfc_geometry_uses_incumbent_anchored_effects_and_rejects_duplicate_checkpoint` | 4437 |  | `L4483: let expected = 2.0_f64.sqrt();` |
| `src/learning/portfolio_governance.rs` | `petfc_invariant_overlap_is_bounded_unknown_not_rollback` | 4490 |  | `` |
| `src/learning/portfolio_governance.rs` | `adaptive_budget_fails_closed_when_mandatory_coverage_does_not_fit` | 4657 |  | `` |
| `src/learning/portfolio_governance.rs` | `adaptive_budget_rejects_unbounded_history_and_work_before_planning` | 4693 |  | `` |
| `src/learning/portfolio_governance.rs` | `adapter_promotion_witnesses_accept_one_fully_bound_chain` | 4820 |  | `` |
| `src/learning/numerical_evolution.rs` | `fit_metric_id` | 57 |  | `` |
| `src/learning/numerical_evolution.rs` | `worst_error_metric_id` | 61 |  | `` |
| `src/learning/numerical_evolution.rs` | `numerical_metric_specs` | 66 | Exact metric catalog implemented by this numerical evaluator. | `L67: minimum_normalized_fit: f64,` |
| `src/learning/numerical_evolution.rs` | `solver` | 229 |  | `` |
| `src/learning/numerical_evolution.rs` | `exact_digest` | 260 |  | `` |
| `src/learning/numerical_evolution.rs` | `solver_run_failure` | 387 |  | `` |
| `src/learning/numerical_evolution.rs` | `solver_report` | 391 |  | `` |
| `src/learning/numerical_evolution.rs` | `derive_solver_run_failure` | 715 |  | `` |
| `src/learning/numerical_evolution.rs` | `observe_functional_change` | 740 | Record only a non-causal functional change proven by two reports that this engine actually reduced. Detection and invalidation revisions are recovered from proc | `` |
| `src/learning/numerical_evolution.rs` | `derive_observations` | 777 |  | `L804: let target_rms = baseline_metrics.target_norm() / (scalar_count as f64).sqrt();` |
| `src/learning/numerical_evolution.rs` | `variant_from_digest` | 1034 |  | `` |
| `src/learning/numerical_evolution.rs` | `research_trial_digest` | 1067 |  | `` |
| `src/learning/numerical_evolution.rs` | `experimental_input_digests` | 1102 | Conservative structural identity for an experimental input. Targets are deliberately excluded: changing a label by one bit must not make a reused input appear i | `` |
| `src/learning/numerical_evolution.rs` | `applicability_from_report` | 1129 |  | `L1153: u64::try_from(diagnostics.numerical_rank())` |
| `src/learning/numerical_evolution.rs` | `configuration_from_selected` | 1175 |  | `L1183: let tolerance = if policy.relative_residual_tolerance() > 0.0 {` |
| `src/learning/numerical_evolution.rs` | `evaluator_policy_digest` | 1236 |  | `` |
| `src/learning/numerical_evolution.rs` | `exact_sparse_proposal` | 1401 |  | `` |
| `src/learning/numerical_evolution.rs` | `terminal_solver_result_consumes_its_revision` | 1608 |  | `` |
| `src/learning/numerical_evolution.rs` | `pre_materialization_solver_limit_uses_candidate_free_record` | 1637 |  | `` |
| `src/learning/numerical_evolution.rs` | `deterministic_repeat_is_not_fresh_evidence_or_functional_drift` | 1831 |  | `` |
| `src/learning/learning_orchestrator.rs` | `default_cost_weight` | 58 |  | `` |
| `src/learning/learning_orchestrator.rs` | `default_risk_weight` | 61 |  | `L83: pub design_rank: usize,` |
| `src/learning/learning_orchestrator.rs` | `normalized` | 254 |  | `L254: fn normalized(mut values: Vec<f64>) -> BrainResult<Vec<f64>> {` |
| `src/learning/learning_orchestrator.rs` | `rank` | 375 |  | `L394: let tolerance = max * f64::EPSILON.sqrt() * (gram.rows.max(1) as f64);` |
| `src/learning/learning_orchestrator.rs` | `plan_autonomous_learning` | 398 |  | `L424: if weight.abs() > 1e-12 {` |
| `src/learning/learning_orchestrator.rs` | `target_digest` | 461 |  | `` |
| `src/learning/learning_orchestrator.rs` | `policy_digest` | 467 |  | `` |
| `src/learning/learning_orchestrator.rs` | `same_f64` | 530 |  | `L533: && (left - right).abs() <= f64::EPSILON.sqrt() * 64.0 * (1.0 + left.abs().max(right.abs()))` |
| `src/learning/learning_orchestrator.rs` | `next_learning_aperture` | 625 |  | `L656: let score = choose_active_aperture(` |
| `src/learning/learning_orchestrator.rs` | `assimilate_learning_result` | 712 |  | `` |
| `src/learning/learning_orchestrator.rs` | `validate_experiment_evidence` | 881 |  | `L953: let tolerance = f64::EPSILON.sqrt()` |
| `src/learning/learning_orchestrator.rs` | `verify_receipt_ledger_binding` | 1077 |  | `` |
| `src/learning/learning_orchestrator.rs` | `issue_next_persistent_learning_aperture_under_root` | 1429 |  | `` |
| `src/learning/learning_orchestrator.rs` | `issue_next_persistent_learning_aperture` | 1464 | Atomically issue exactly one canonical next aperture. A second issue is rejected until an actual evidence envelope has been assimilated. | `` |
| `src/learning/learning_orchestrator.rs` | `assimilate_persistent_learning_evidence_under_root` | 1502 |  | `` |
| `src/learning/learning_orchestrator.rs` | `assimilate_persistent_learning_evidence` | 1572 | Assimilate a real, content-addressed experiment evidence envelope. Missing, changed or out-of-root evidence is a hard error; no outcome is synthesized. | `` |
| `src/learning/learning_orchestrator.rs` | `learning_plan_is_full_rank_and_covers_every_target` | 1728 |  | `L1728: fn learning_plan_is_full_rank_and_covers_every_target() {` |
| `src/learning/learning_orchestrator.rs` | `adaptive_learning_assimilates_realized_result_before_next_choice` | 1767 |  | `L1795: .any(\|value\| value.abs() > 1e-12)` |
| `src/learning/learning_orchestrator.rs` | `realized_outcome_changes_the_next_canonical_aperture` | 1800 |  | `` |
| `src/learning/learning_orchestrator.rs` | `persistent_cycle_rejects_unbound_outcomes_and_tampered_receipts` | 1814 |  | `L1848: malformed.observed_value += 1.0;` |
| `src/engine/learned_controller.rs` | `fit_weights` | 284 |  | `L300: weights.push(weighted_normal_solve(&design, &target, &reliability, ridge)?);` |
| `src/engine/learned_controller.rs` | `grouped_cv_r2` | 320 |  | `L323: ridge: f64,` |
| `src/engine/learned_controller.rs` | `train_learned_controller` | 358 |  | `L406: training_rms: (squared_error / values.max(1) as f64).sqrt(),` |
| `src/engine/learned_controller.rs` | `train_runtime_learned_controller` | 412 |  | `L415: ridge: f64,` |
| `src/engine/learned_controller.rs` | `controller_policy_digest` | 448 |  | `` |
| `src/engine/learned_controller.rs` | `validate_persisted_controller_policy` | 462 |  | `L464: \|\| !policy.ridge.is_finite()` |
| `src/engine/learned_controller.rs` | `verify_reconstruction_report_binding` | 617 |  | `` |
| `src/engine/learned_controller.rs` | `verify_finalization_binding_under_root` | 635 | The engine-owned finalization receipt is the sole authority that can bridge the immutable raw experiment artifact to a semantic observation digest in the promot | `` |
| `src/engine/learned_controller.rs` | `same_supervision_coefficients` | 785 |  | `L788: (left - right).abs() <= f64::EPSILON.sqrt() * 64.0 * (1.0 + left.abs().max(right.abs()))` |
| `src/engine/learned_controller.rs` | `same_controller_float` | 792 |  | `L795: && (left - right).abs() <= f64::EPSILON.sqrt() * 64.0 * (1.0 + left.abs().max(right.abs()))` |
| `src/engine/learned_controller.rs` | `verify_controller_ledger_binding` | 988 |  | `` |
| `src/engine/learned_controller.rs` | `learned_controller_decide_and_training_validation_errors` | 1877 |  | `L1911: let neg_ridge_ex = vec![ControllerExample {` |
| `src/engine/cognitive_field.rs` | `validate_config` | 111 |  | `L129: \|\| !config.decay.is_finite()` |
| `src/engine/cognitive_field.rs` | `curvature_similarity` | 159 |  | `L160: let denom = (curvature.get(left, left).max(0.0) * curvature.get(right, right).max(0.0)).sqrt();` |
| `src/engine/cognitive_field.rs` | `functional_similarity` | 186 |  | `L195: cosine(&left.functional_signature, &right.functional_signature)` |
| `src/engine/cognitive_field.rs` | `attractor_digest` | 200 |  | `` |
| `src/engine/cognitive_field.rs` | `build` | 213 |  | `L308: .map(\|effect\| (effect / pair_scale).tanh())` |
| `src/engine/cognitive_field.rs` | `evolve` | 379 |  | `L411: .map(\|value\| (self.config.beta * value).tanh())` |
| `src/engine/cognitive_field.rs` | `cognitive_field_rejects_indefinite_curvature` | 744 |  | `` |
| `src/engine/parametric_program.rs` | `compose_skill_fields` | 41 | Compose one parameter-space operator from SkillFields. | `L57: /// this does not assume coefficient_k = <H_k, operator>. It solves the normal` |
| `src/engine/parametric_program.rs` | `compile_operator_to_fields` | 60 | Compile a target operator into coefficients over an arbitrary, potentially non-orthogonal SkillField basis. Unlike the original prototype, this does not assume  | `L97: let relative_residual = norm(&residual)? / norm(operator)?.max(1e-15);` |
| `src/engine/parametric_program.rs` | `apply_parametric_transition` | 113 | Generic linear recurrent state transition. The runtime knows only matrix multiplication and winner selection; the transition semantics live in the composed oper | `L128: scores[row] += operator[row * state_dim + col] * state[col];` |
| `src/engine/parametric_program.rs` | `ties_merge` | 169 |  | `L186: .abs()` |
| `src/engine/parametric_program.rs` | `nonorthogonal_fields_compile_both_operators` | 263 |  | `L270: norm(` |
| `src/engine/parametric_program.rs` | `generic_parametric_transition_remains_available_without_token_router` | 284 |  | `` |
| `src/engine/parametric_program.rs` | `test_report` | 291 |  | `L346: "normalized_reconstruction_rms": 0.0,` |
| `src/engine/parametric_program.rs` | `fields_from_report_and_validation` | 388 |  | `L396: report.selected_rank = 3;` |
| `src/engine/parametric_program.rs` | `compose_and_compile_error_paths` | 419 |  | `` |
| `src/engine/parametric_program.rs` | `parametric_transition_error_paths` | 452 |  | `` |
| `src/engine/analysis.rs` | `materialize_dense_fields` | 72 |  | `` |
| `src/engine/analysis.rs` | `verify_dense_field_materializations` | 136 | Re-derive dense field references without creating artifacts.  A receipt verifier must never materialize a missing candidate as a side effect: the exact content- | `` |
| `src/engine/analysis.rs` | `analyze_canonical` | 479 |  | `L527: / (group_counts[o.independence_group.as_str()] as f64).sqrt(),` |
| `src/materialization/low_rank_shadow_materializer.rs` | `validate` | 44 |  | `L45: if self.schema != "cerebro.tidex.low_rank_shadow_policy/v1"` |
| `src/materialization/low_rank_shadow_materializer.rs` | `digest` | 64 |  | `L67: b"CEREBRO:TIDEX:LOW-RANK-SHADOW-POLICY:v1\0",` |
| `src/materialization/low_rank_shadow_materializer.rs` | `materialize_dense` | 90 |  | `L91: CandidateRepresentation::LowRank {` |
| `src/materialization/low_rank_shadow_materializer.rs` | `factor_dense_delta_verified` | 102 |  | `L125: let target_norm = norm(dense)?;` |
| `src/materialization/low_rank_shadow_materializer.rs` | `values_digest` | 286 |  | `L288: b"CEREBRO:TIDEX:SHADOW-LOW-RANK-DENSE-VALUES:v1\0",` |
| `src/materialization/low_rank_shadow_materializer.rs` | `calculate_digest` | 294 |  | `L304: b"CEREBRO:TIDEX:SHADOW-LOW-RANK-CANDIDATE:v1\0",` |
| `src/materialization/low_rank_shadow_materializer.rs` | `validate` | 309 |  | `L324: if self.schema != "cerebro.tidex.shadow_low_rank_candidate/v1"` |
| `src/materialization/low_rank_shadow_materializer.rs` | `build_candidate` | 432 |  | `L500: schema: "cerebro.tidex.shadow_low_rank_candidate/v1".into(),` |
| `src/materialization/low_rank_shadow_materializer.rs` | `materialize_replayed_low_rank_shadow` | 517 |  | `L517: pub fn materialize_replayed_low_rank_shadow(` |
| `src/materialization/low_rank_shadow_materializer.rs` | `persist_low_rank_shadow` | 528 |  | `L528: pub fn persist_low_rank_shadow(` |
| `src/materialization/low_rank_shadow_materializer.rs` | `load_low_rank_shadow` | 552 |  | `L552: pub fn load_low_rank_shadow(` |
| `src/materialization/low_rank_shadow_materializer.rs` | `policy` | 577 |  | `L579: schema: "cerebro.tidex.low_rank_shadow_policy/v1".into(),` |
| `src/materialization/low_rank_shadow_materializer.rs` | `exact_low_rank_delta_is_factored_and_full_rank_delta_is_rejected` | 589 |  | `L589: fn exact_low_rank_delta_is_factored_and_full_rank_delta_is_rejected() {` |
| `src/materialization/low_rank_shadow_materializer.rs` | `rectangular_orientations_and_finite_scaling_reconstruct` | 614 |  | `L620: .map(\|index\| (index as f64 + 0.5) / scale.sqrt())` |
| `src/materialization/sparse_shadow_materializer.rs` | `digest` | 63 |  | `` |
| `src/materialization/sparse_shadow_materializer.rs` | `values_digest` | 109 |  | `` |
| `src/materialization/sparse_shadow_materializer.rs` | `sparsify` | 116 |  | `L127: let target_norm = norm(values)?;` |
| `src/materialization/sparse_shadow_materializer.rs` | `calculate_digest` | 187 |  | `` |
| `src/materialization/sparse_shadow_materializer.rs` | `materialize_replayed_sparse_shadow` | 335 |  | `` |
| `src/materialization/sparse_shadow_materializer.rs` | `persist_sparse_shadow` | 346 |  | `` |
| `src/materialization/sparse_shadow_materializer.rs` | `load_sparse_shadow` | 371 |  | `` |
| `src/materialization/sparse_shadow_materializer.rs` | `deterministic_top_magnitude_sparse_encoding_and_rejection` | 409 |  | `` |
| `src/materialization/activation_steering_materializer.rs` | `calculate_digest` | 173 |  | `` |
| `src/materialization/activation_steering_materializer.rs` | `digest` | 207 |  | `` |
| `src/materialization/activation_steering_materializer.rs` | `normalize` | 241 |  | `L241: fn normalize(mut vector: Vec<f64>, normalization: SteeringNormalization) -> BrainResult<Vec<f64>> {` |
| `src/materialization/activation_steering_materializer.rs` | `target_digest` | 257 |  | `` |
| `src/materialization/activation_steering_materializer.rs` | `calculate_digest` | 265 |  | `` |
| `src/materialization/activation_steering_materializer.rs` | `build_candidate` | 288 |  | `L341: let vector = normalize(raw, hook.normalization)?` |
| `src/materialization/activation_steering_materializer.rs` | `materialize_replayed_activation_steering_shadow` | 378 |  | `` |
| `src/materialization/activation_steering_materializer.rs` | `persist_activation_steering_shadow` | 390 |  | `` |
| `src/materialization/activation_steering_materializer.rs` | `load_activation_steering_shadow` | 416 |  | `` |
| `src/materialization/activation_steering_materializer.rs` | `normalization_is_exact_and_finite` | 442 |  | `L442: fn normalization_is_exact_and_finite() {` |
| `src/materialization/materialization_selector.rs` | `minimum_controls_for_strategy` | 23 |  | `L62: pub normalized_risk: f64,` |
| `src/materialization/materialization_selector.rs` | `rigorous_default` | 91 |  | `L98: maximum_normalized_risk: 0.1,` |
| `src/materialization/materialization_selector.rs` | `validate` | 125 |  | `L130: self.maximum_normalized_risk,` |
| `src/materialization/materialization_selector.rs` | `dominates` | 254 |  | `L258: && a.normalized_risk <= b.normalized_risk` |
| `src/materialization/materialization_selector.rs` | `select_materialization_backend` | 270 |  | `L326: && e.normalized_risk <= policy.maximum_normalized_risk` |
| `src/materialization/materialization_selector.rs` | `rejects_low_rank_without_low_rank_specific_gate` | 499 |  | `L499: fn rejects_low_rank_without_low_rank_specific_gate() {` |
| `src/materialization/materialization_selector.rs` | `sparse_and_steering_require_strategy_specific_gates` | 512 |  | `` |
| `src/materialization/materialization_selector.rs` | `convergence_rejects_confidence_above_score_and_overflowing_weights` | 585 |  | `L587: e.functional_score = 0.1;` |
| `src/materialization/materialization_selector.rs` | `convergence_ranking_is_independent_of_input_order` | 603 |  | `L603: fn convergence_ranking_is_independent_of_input_order() {` |
| `src/materialization/shadow_evaluation.rs` | `validate` | 104 |  | `L111: self.normalized_risk,` |
| `src/materialization/shadow_evaluation.rs` | `run_shadow_evaluation` | 226 |  | `L255: output.normalized_risk,` |
| `src/materialization/shadow_evaluation.rs` | `strict_metrics` | 325 |  | `L333: normalized_risk: 0.01,` |
| `src/materialization/shadow_evaluation.rs` | `strict_runtime_metrics_are_the_only_metric_authority` | 362 |  | `` |
| `src/receiver/receiver_compiler.rs` | `validate` | 53 |  | `L55: \|\| !self.ridge.is_finite()` |
| `src/receiver/receiver_compiler.rs` | `benchmark_receiver_signature` | 104 |  | `L133: pub max_rank: usize,` |
| `src/receiver/receiver_compiler.rs` | `benchmark_receiver_basis` | 167 | Numerical adapter over the existing tomography and identifiability kernels. It creates no observations, model updates, artifacts or promotion authority. Callers | `L263: let tolerance = f64::EPSILON.sqrt() * (n.max(p) as f64).sqrt() * 16.0;` |
| `src/receiver/receiver_compiler.rs` | `validate_rows` | 384 |  | `L414: pub relational_coefficient_norm: Option<f64>,` |
| `src/receiver/receiver_compiler.rs` | `compile_receiver_capability` | 445 | Compile one held-out operational capability into receiver-native parameters. The operational profile retains its historical wire schema and gates. | `` |
| `src/receiver/receiver_compiler.rs` | `finish_operational_compilation` | 461 |  | `L475: decoder_min_loo_cosine: numerical.decoder_min_loo_cosine,` |
| `src/receiver/receiver_compiler.rs` | `compile_receiver_readout_capability` | 514 | Execute CapabilityIr before deriving the request to the receiver backend. The learned inverse remains a prediction; materialized receiver execution is a separat | `` |
| `src/receiver/receiver_compiler.rs` | `project_functional_signature` | 550 | Canonical scalar f64 projection shared by acquisition and compilation. Preserve the sequential subtract/multiply/add order; do not fuse it. | `L571: sum += product;` |
| `src/receiver/receiver_compiler.rs` | `to_map` | 642 |  | `L674: mean_loo_cosine: f64,` |
| `src/receiver/receiver_compiler.rs` | `from_map` | 680 |  | `L688: mean_loo_cosine: map.mean_loo_cosine,` |
| `src/receiver/receiver_compiler.rs` | `to_map` | 694 |  | `L700: \|\| [self.loo_cv_r2, self.mean_loo_cosine, self.min_loo_cosine]` |
| `src/receiver/receiver_compiler.rs` | `from_map` | 734 |  | `L741: mean_loo_cosine: map.mean_loo_cosine,` |
| `src/receiver/receiver_compiler.rs` | `to_map` | 747 |  | `L754: self.mean_loo_cosine,` |
| `src/receiver/receiver_compiler.rs` | `to_maps` | 823 |  | `L852: relational.max_loo_coefficient_norm,` |
| `src/receiver/receiver_compiler.rs` | `fit_receiver_compiler_maps` | 910 |  | `L925: let regression = AffineTransportPolicy::CenteredTraceRidge {` |
| `src/receiver/receiver_compiler.rs` | `freeze_receiver_compiler` | 1014 |  | `L1066: \|\| maps.decoder.min_loo_cosine < input.policy.minimum_decoder_loo_cosine` |
| `src/receiver/receiver_compiler.rs` | `verify` | 1116 | Authentication replays calibration once under the exact compiled source identity. The returned handle owns the replayed maps; target compilation performs no cal | `L1146: self.input.policy.ridge,` |
| `src/receiver/receiver_compiler.rs` | `compile_capability` | 1178 |  | `` |
| `src/receiver/receiver_compiler.rs` | `compile` | 1191 |  | `L1214: \|\| norm(requested)? <= 1e-15` |
| `src/receiver/receiver_compiler.rs` | `safe_coordinate_inverse` | 1259 | Fit the requested response through the actual protection operator P: min_z \|\|M P z - (requested - bias)\|\|^2 + ridge \|\|z\|\|^2. P is constructed by applying the ex | `L1279: weighted_normal_solve(&design, &targets, &vec![1.0; targets.len()], ridge)` |
| `src/receiver/receiver_compiler.rs` | `relational_receiver_proposal_from_map` | 1294 | Apply a previously calibrated relational map. The target contributes only its functional signature; no target receiver observations are used here. | `L1326: coefficient_norm: transplant.coefficient_norm,` |
| `src/receiver/receiver_compiler.rs` | `compile_receiver_signature` | 1338 | Shared decoder, inverse predictor, protection and trust-region kernel.  This entry point does not fabricate an OperationalCapabilityContract for a generative mo | `` |
| `src/receiver/receiver_compiler.rs` | `compile_receiver_signature_calibrated_affine` | 1359 | Compile a measured functional IR with scale-aware, fold-local regression. This profile always retains both decoder and inverse validation. It does not turn beha | `` |
| `src/receiver/receiver_compiler.rs` | `validate_functional_anchor_identity` | 1381 | Distinct calibration capabilities must be distinguishable in their input IR. The tolerance concerns floating-point resolution, not an invented noise model. A si | `L1390: let scale = norm(&rows[i])?.max(norm(&rows[j])?).max(f64::MIN_POSITIVE);` |
| `src/receiver/receiver_compiler.rs` | `compile_receiver_signature_behaviorally_calibrated_candidate` | 1411 | Candidate-only cross-model affine compilation. This deliberately does NOT interpret decoder coordinate-space LOO R² as the final proposal-quality authority. The | `` |
| `src/receiver/receiver_compiler.rs` | `compile_receiver_signature_in_safe_coordinates` | 1433 | Response-space inversion that includes protection in the forward design. This is an explicitly selected candidate profile, not an automatic alternate used to tu | `` |
| `src/receiver/receiver_compiler.rs` | `compile_receiver_signature_relational` | 1454 | Compile from relational capability geometry.  The target contributes only its functional signature; barycentric coefficients are inferred against the calibratio | `` |
| `src/receiver/receiver_compiler.rs` | `compile_signature_with_method` | 1471 |  | `` |
| `src/receiver/receiver_compiler.rs` | `compile_signature_with_method_and_validation` | 1490 |  | `` |
| `src/receiver/receiver_compiler.rs` | `compile_signature_with_maps` | 1522 |  | `L1558: if norm(requested)? <= 1e-15 {` |
| `src/receiver/receiver_compiler.rs` | `evaluate_portability` | 1780 | Compare one receiver-native compiled result against an untouched receiver, a direct receiver oracle, and an explicit wrong-skill control using one common functi | `L1792: \|\| norm(expected_functional_signature)? <= 1e-15` |
| `src/receiver/receiver_compiler.rs` | `benchmark_receiver_portability_leave_one_out` | 1886 | Leave-one-skill-out functional compilation benchmark. The direct receiver solution of the held-out skill is never passed to either learned map; it is opened onl | `L1916: \|\| norm(&case.functional_signature)? <= 1e-15` |
| `src/receiver/receiver_compiler.rs` | `receiver_basis_test_input` | 2012 |  | `L2028: max_rank: 2,` |
| `src/receiver/receiver_compiler.rs` | `receiver_basis_adapter_preserves_real_source_mixtures_and_row_lineage` | 2035 |  | `L2044: assert!(result.retained_energy > 1.0 - 1e-12);` |
| `src/receiver/receiver_compiler.rs` | `receiver_basis_adapter_reports_unweighted_raw_reconstruction_energy` | 2073 |  | `L2097: assert!((result.retained_energy - (1.0 - sse / total_energy)).abs() < 1e-12);` |
| `src/receiver/receiver_compiler.rs` | `receiver_basis_adapter_rejects_ambiguous_lineage_nonfinite_values_and_resource_excess` | 2102 |  | `L2129: for max_rank in [0, 5, 33] {` |
| `src/receiver/receiver_compiler.rs` | `calibrated_ir_rejects_collisions_but_accepts_constant_coordinate_vectors` | 2165 |  | `` |
| `src/receiver/receiver_compiler.rs` | `centered_compilation_uses_both_maps_and_cannot_skip_inverse_validation` | 2180 |  | `L2206: ridge: 1e-9,` |
| `src/receiver/receiver_compiler.rs` | `receiver_readout_compilation_uses_executed_ir_values_and_projection` | 2255 |  | `L2348: assert!((report.execution.raw_margins[0] - 0.5).abs() < 1e-14);` |
| `src/receiver/receiver_compiler.rs` | `fixture_ir` | 2394 |  | `L2423: CapabilityNodeId::parse("node.normalize").unwrap(),` |
| `src/receiver/receiver_compiler.rs` | `toggle_contract` | 2448 |  | `L2449: let pre = 2.0_f64.sqrt();` |
| `src/receiver/receiver_compiler.rs` | `coupled_calibration` | 2499 |  | `` |
| `src/receiver/receiver_compiler.rs` | `response_policy` | 2519 |  | `L2522: ridge: 1e-10,` |
| `src/receiver/receiver_compiler.rs` | `frozen_compiler_roundtrip_uses_stored_maps_without_target_time_fitting` | 2555 |  | `` |
| `src/receiver/receiver_compiler.rs` | `frozen_compiler_rejects_rehashed_forged_stored_maps` | 2586 |  | `L2591: forged.maps.decoder.target_decoder.weights[0][0] += 999.0;` |
| `src/receiver/receiver_compiler.rs` | `frozen_compiler_rejects_target_leakage_and_unsupported_extrapolation` | 2607 |  | `` |
| `src/receiver/receiver_compiler.rs` | `protected_coordinate_fit_preserves_the_requested_response_without_weakening_gates` | 2641 |  | `L2668: assert!((fitted.target_delta[0] - 0.2).abs() < 1e-6);` |
| `src/receiver/receiver_compiler.rs` | `protected_coordinate_fit_cannot_restore_a_hard_forbidden_direction` | 2672 |  | `L2692: assert!(result.target_delta[0].abs() < 1e-12);` |
| `src/receiver/receiver_compiler.rs` | `protected_coordinate_fit_still_rejects_insufficient_risk_budget` | 2697 |  | `` |
| `src/receiver/receiver_compiler.rs` | `protected_coordinate_fit_does_not_bypass_protection_removal_limit` | 2720 |  | `` |
| `src/receiver/receiver_compiler.rs` | `calibration` | 2739 |  | `` |
| `src/receiver/receiver_compiler.rs` | `held_out_receiver_compilation_recovers_capability_without_donor_weights` | 2753 |  | `L2768: ridge: 1e-10,` |
| `src/receiver/receiver_compiler.rs` | `receiver_compiler_fails_promotion_when_protection_destroys_contract` | 2865 |  | `L2880: ridge: 1e-10,` |
| `src/receiver/receiver_weight_binding.rs` | `describe_linear_readout` | 123 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `readout_gaps` | 237 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `readout_partial_bundle` | 246 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `verify_observed_linear_readout` | 273 | Standard forward bound for a dot product over exact F32 operands. gamma_d covers rounded products and reduction; the absolute term also covers gradual underflow | `L341: let subtraction_bound = unit32 / (1.0 - unit32)` |
| `src/receiver/receiver_weight_binding.rs` | `readout_acquisition_path` | 366 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `derive_linear_readout_acquisition` | 371 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `acquire_linear_readout` | 500 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `authenticate_linear_readout_details` | 573 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `authenticate_linear_readout` | 605 | Read-only deep replay. Imported PASS fields never authorize a readout. | `` |
| `src/receiver/receiver_weight_binding.rs` | `cross_model` | 902 |  | `L997: pub maximum_calibration_coordinate_norm: f64,` |
| `src/receiver/receiver_weight_binding.rs` | `read_cross_model_projection` | 1091 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `exact_f64_vector_digest` | 1157 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `functional_signature_digest` | 1166 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `receiver_coordinates_digest` | 1170 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `validate_cross_model_functional_evidence` | 1174 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `validate_behavioral_calibration_evidence` | 1312 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `prepare_receiver_weight_candidate` | 2060 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `authenticate_receiver_weight_candidate` | 2080 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `materialize_receiver_weight_candidate` | 2106 | Materialize only a recomputed, unblocked candidate. Neither an arbitrary caller-supplied coefficient vector nor an edited `allowed` flag is accepted. The output | `` |
| `src/receiver/receiver_weight_binding.rs` | `digest` | 2174 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `fixture` | 2186 |  | `L2200: "model.norm.weight":{"dtype":"F32","shape":[4],"data_offsets":[0,16]},` |
| `src/receiver/receiver_weight_binding.rs` | `readout_fixture` | 2521 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `bind_readout_to_cross_model_fixture` | 2598 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `linear_readout_acquisition_authenticates_real_checkpoint_capture_ir_and_replay` | 2652 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `linear_readout_acquisition_rejects_wrong_forward_prompt_and_f32_evidence` | 2693 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `linear_readout_candidate_is_driven_by_executed_ir_not_supplied_signature` | 2718 |  | `L2741: f.request.policy.ridge,` |
| `src/receiver/receiver_weight_binding.rs` | `signature_to_dense_to_checkpoint_uses_one_existing_actuator` | 2762 |  | `L2779: read_model_tensor_f32(&out, &TensorId::parse("model.norm.weight").unwrap()).unwrap();` |
| `src/receiver/receiver_weight_binding.rs` | `cross_model_evidence_is_semantically_bound_not_only_hash_authenticated` | 2791 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `cross_model_support_uses_functional_leverage_and_blocks_extreme_query` | 2824 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `canonical_projection_preserves_sequential_rounding_without_fma` | 2859 |  | `L2866: project_functional_signature(&[-1.0, 1.0 + step], &[0.0; 2], &[vec![1.0, 1.0 - step]])` |
| `src/receiver/receiver_weight_binding.rs` | `reselling_modified_raw_values_cannot_reuse_a_projected_signature` | 2877 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `functional_ir_binds_actual_prompt_text_and_raw_probe_order` | 2895 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `authenticated_but_changed_projection_must_recompute_the_response` | 2923 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `projection_lineage_cannot_silently_include_a_target` | 2953 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `protocol_self_test_can_validate_but_never_materialize_weights` | 2976 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `cross_language_vector_digest_contract_matches_v69_python_authority` | 3002 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `python_json_roundtrip_preserves_exact_functional_signature_digest` | 3021 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `json_authority_preserves_signed_zero_subnormals_and_finite_extremes` | 3048 |  | `L3048: fn json_authority_preserves_signed_zero_subnormals_and_finite_extremes() {` |
| `src/receiver/receiver_weight_binding.rs` | `forged_candidate_flags_and_coordinates_are_recomputed_not_trusted` | 3077 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `target_may_not_appear_in_calibration_observations` | 3108 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `outside_calibrated_radius_records_blocker_without_delta_or_checkpoint` | 3149 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `resource_and_safety_bounds_fail_before_numerical_work` | 3212 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `distributed_request_blocks_legacy_final_norm_basis` | 3224 |  | `L3224: fn distributed_request_blocks_legacy_final_norm_basis() {` |
| `src/receiver/receiver_weight_binding.rs` | `calibration_lora_solution_is_allowed_only_when_bound_to_one_basis_axis` | 3299 |  | `` |
| `src/receiver/receiver_weight_binding.rs` | `convergence_measured_candidate_uses_common_dense_and_sparse_checkpoint_pipeline` | 3544 |  | `` |
| `src/receiver/validation.rs` | `readout_dot_roundoff_bound` | 12 |  | `L24: let gamma32 = (count32 * unit32) / (1.0 - count32 * unit32);` |
| `src/receiver/validation.rs` | `receiver_validation_contracts_fail_closed` | 74 |  | `L77: assert!(readout_dot_roundoff_bound(&[1.0, 2.0], &[3.0, 4.0]).unwrap() >= 0.0);` |
| `src/capability/capability_ir.rs` | `verify` | 245 |  | `` |
| `src/capability/capability_ir.rs` | `calculate_digest` | 266 |  | `` |
| `src/capability/capability_ir.rs` | `system_envelope_digest` | 384 |  | `` |
| `src/capability/capability_ir.rs` | `manifest_digest` | 388 |  | `` |
| `src/capability/capability_ir.rs` | `calculate_digest` | 619 |  | `` |
| `src/capability/capability_ir.rs` | `execute_linear_readout` | 661 | Execute the scalar-output specialization of the existing authenticated linear-map IR. There is exactly one runtime tensor [d, 1], one resident parameter tensor  | `L753: execution_arithmetic: "f64_scaled_neumaier_dot/v1".into(),` |
| `src/capability/capability_ir.rs` | `validate_against` | 809 |  | `L882: .sqrt();` |
| `src/capability/capability_ir.rs` | `verify_receiver_signature` | 930 | Verify a receiver-produced functional signature against the same closure and contraction semantics used to seal V63 evidence. | `L971: .sqrt();` |
| `src/capability/capability_ir.rs` | `resolve_reference` | 1028 |  | `` |
| `src/capability/capability_ir.rs` | `domain_digest` | 1101 |  | `` |
| `src/capability/capability_ir.rs` | `envelope` | 1117 |  | `` |
| `src/capability/capability_ir.rs` | `weight_parameter` | 1169 |  | `` |
| `src/capability/capability_ir.rs` | `weighted_node` | 1174 |  | `` |
| `src/capability/capability_ir.rs` | `weighted_output` | 1192 |  | `` |
| `src/capability/capability_ir.rs` | `ir_is_closed_typed_and_bound_to_the_captured_tree` | 1203 |  | `` |
| `src/capability/capability_ir.rs` | `ir_rejects_self_reference_wrong_type_shape_and_unbounded_shape` | 1436 |  | `` |
| `src/capability/capability_ir.rs` | `ir_rejects_dead_nodes_output_contract_mismatch_and_tampered_envelope` | 1545 |  | `` |
| `src/capability/capability_ir.rs` | `linear_readout_fixture` | 1651 |  | `` |
| `src/capability/capability_ir.rs` | `linear_readout_executes_existing_rectangular_ir_with_real_parameters` | 1702 |  | `L1714: assert!((actual - expected).abs() < 1e-12);` |
| `src/capability/capability_ir.rs` | `linear_readout_rejects_tampered_ir_parameter_binding_and_envelope` | 1743 |  | `` |
| `src/capability/capability_ir.rs` | `linear_readout_rejects_authenticated_graphs_outside_scalar_matmul_profile` | 1767 |  | `` |
| `src/capability/capability_ir.rs` | `linear_readout_bounds_work_and_rejects_nonfinite_inputs_and_results` | 1802 |  | `` |
| `src/operator/control_plane.rs` | `read_hub_snapshot_file_bounded` | 612 |  | `` |
| `src/operator/control_plane.rs` | `resolve_assets` | 1478 |  | `` |
| `src/operator/control_plane.rs` | `execute_behavioral_discovery_workflow` | 1793 |  | `` |
| `src/operator/control_plane.rs` | `execute_behavioral_discovery_workflow_cancelable` | 1800 |  | `` |
| `src/operator/control_plane.rs` | `empty_plasticity_advice` | 2241 |  | `` |
| `src/operator/control_plane.rs` | `compute_operator_plasticity_advice` | 2270 |  | `L2342: let score = evaluation` |
| `src/operator/control_plane.rs` | `compute_operator_plasticity_advice` | 2726 |  | `` |
| `src/operator/control_plane.rs` | `try_acquire` | 3269 |  | `L3322: <section class="panel result"><div class="tabs"><button onclick="showTab('result')">Resultado · evidencia</button><button onclick="showTab('history')">Historial · jobs</button><button onclick="` |
| `src/operator/control_plane.rs` | `operator_living_staircase_composes_plasticity_and_graph_without_production` | 3529 |  | `` |
| `src/operator/control_plane.rs` | `plasticity_advice_is_empty_without_evaluation_jobs` | 3702 |  | `` |
| `src/operator/control_plane.rs` | `plasticity_advice_does_not_invent_elo_from_an_isolated_evaluation` | 3723 |  | `` |
| `src/operator/control_plane.rs` | `plasticity_advice_withholds_ranking_on_measured_tie` | 3737 |  | `L3737: fn plasticity_advice_withholds_ranking_on_measured_tie() {` |
| `src/operator/control_plane.rs` | `plasticity_advice_ranks_and_routes_on_measured_score_gap` | 3757 |  | `L3757: fn plasticity_advice_ranks_and_routes_on_measured_score_gap() {` |
| `src/capability/acquisition_contract.rs` | `projection_roots` | 528 |  | `` |
| `src/capability/acquisition_contract.rs` | `validate_scope` | 1141 |  | `L1151: let normalized = normalize_declared_roots(roots)?;` |
| `src/capability/acquisition_contract.rs` | `normalize_scope` | 1160 |  | `L1160: fn normalize_scope(scope: AcquisitionScope) -> BrainResult<AcquisitionScope> {` |
| `src/capability/acquisition_contract.rs` | `normalize_declared_roots` | 1177 |  | `L1177: fn normalize_declared_roots(` |
| `src/capability/acquisition_contract.rs` | `declared_scope_is_normalized_and_never_claims_dependency_closure` | 1917 |  | `L1917: fn declared_scope_is_normalized_and_never_claims_dependency_closure() {` |
| `src/capability/content_vault.rs` | `projection_path` | 572 |  | `` |
| `src/cross_model/co_evolution/consensus_builder.rs` | `get_statistics` | 310 |  | `L321: ConsensusState::Approved => stats.approved += 1,` |
| `src/cross_model/discovery/domain_analyzer.rs` | `from_label` | 22 |  | `L23: let normalized = label.trim().to_ascii_lowercase();` |
| `src/cross_model/integration/capability_discovery_bridge.rs` | `default` | 17 |  | `L24: pub struct CapabilityDiscoveryBridge {` |
| `src/cross_model/integration/capability_discovery_bridge.rs` | `new` | 30 |  | `L30: pub fn new(config: CapabilityDiscoveryBridgeConfig) -> Result<Self, String> {` |
| `src/cross_model/integration/capability_discovery_bridge.rs` | `bridge_capability` | 40 |  | `L40: pub fn bridge_capability(` |
| `src/cross_model/integration/capability_discovery_bridge.rs` | `batch_bridge` | 91 |  | `L91: pub fn batch_bridge(` |
| `src/cross_model/integration/capability_discovery_bridge.rs` | `default` | 122 |  | `L123: Self::new(CapabilityDiscoveryBridgeConfig::default())` |
| `src/cross_model/integration/causal_credit_bridge.rs` | `allocate_credit` | 19 |  | `` |
| `src/cross_model/models/traits.rs` | `l2_norm` | 92 |  | `L92: pub fn l2_norm(&self) -> f64 {` |
| `src/cross_model/models/traits.rs` | `normalize` | 100 |  | `L100: pub fn normalize(&self) -> Result<Self, String> {` |
| `src/cross_model/models/traits.rs` | `dot` | 115 |  | `L115: pub fn dot(&self, other: &Self) -> Option<f64> {` |
| `src/cross_model/models/traits.rs` | `cosine_similarity` | 122 |  | `L124: let a = self.l2_norm();` |
| `src/cross_model/models/traits.rs` | `from_runtime_architecture` | 339 |  | `L340: let normalized = value.trim().to_ascii_lowercase();` |
| `src/cross_model/models/traits.rs` | `new` | 449 |  | `L486: pub normalized_residual: f64,` |
| `src/engine/runtime.rs` | `load_verified_runtime_evidence` | 511 |  | `L521: let normalized = normalize_reconstruction_report_wire(&reconstruction)?;` |
| `src/engine/runtime.rs` | `compose_verified_inputs` | 729 |  | `L792: if coefficient.abs() <= f64::EPSILON {` |
| `src/engine/runtime.rs` | `skill_subspace_overlap` | 1528 |  | `L1538: let truth_norm = norm(truth_direction)?.max(1e-15);` |
| `src/engine/store.rs` | `validate_bank_scalar` | 417 |  | `L421: if value.abs() > f64::MAX.sqrt() {` |
| `src/engine/store.rs` | `validate_skill_bank_semantics` | 427 |  | `L543: \|\| !block.normalized_block_energy.is_finite()` |
| `src/engine/support.rs` | `normalize_reconstruction_report_wire` | 15 |  | `L15: pub(super) fn normalize_reconstruction_report_wire(` |
| `src/engine/support.rs` | `verify_learning_finalization_commit_binding` | 870 |  | `L893: assimilate_bank(&mut shadow_bank, &report.fields, BrainConfig::default().skill_match_cosine)?;` |
| `src/engine/support.rs` | `validate_brain_config` | 1671 |  | `L1705: .max_spectral_normalized_reconstruction_rms` |
| `src/engine/transition.rs` | `commit_after_verified_corpus_transition` | 280 |  | `L302: let normalized = normalize_reconstruction_report_wire(&report)?;` |
| `src/engine/transition.rs` | `sleep_cycle_under_authority` | 1191 |  | `L1209: let normalized = normalize_reconstruction_report_wire(&reconstruction)?;` |
| `src/foundation/artifact.rs` | `stream_linear_combination_bytes` | 894 | Stream exactly the f32 payload bytes of a linear combination.  Both materialization and verification use this one arithmetic path so a verifier cannot silently  | `L910: sum += sources[index].1 * f64::from(value);` |
| `src/foundation/authority.rs` | `optional_resolver_rejects_a_symlinked_missing_prefix` | 2038 |  | `` |
| `src/foundation/contracts.rs` | `as_str` | 55 |  | `L133: pub normalized_block_energy: f64,` |
| `src/foundation/contracts.rs` | `default` | 250 |  | `L266: max_spectral_normalized_reconstruction_rms: 0.45,` |
| `src/foundation/identity.rs` | `validate_ascii_id` | 8 |  | `L10: allow_dot: bool,` |
| `src/foundation/identity.rs` | `parse` | 127 |  | `L129: validate_ascii_id(value, $allow_dot, true, $label)?;` |
| `src/foundation/identity.rs` | `parse` | 171 |  | `L173: validate_ascii_id(value, $allow_dot, true, $label)?;` |
| `src/foundation/identity.rs` | `observation_id_allows_dot_but_not_hidden_or_parent_paths` | 628 |  | `L628: fn observation_id_allows_dot_but_not_hidden_or_parent_paths() {` |
| `src/governance/adapter_bank.rs` | `validate_manifest_contract` | 1100 |  | `L1164: rank_truncation_used,` |
| `src/governance/adapter_bank.rs` | `resolve_active` | 2828 |  | `` |
| `src/governance/adapter_bank.rs` | `verify_history` | 2888 |  | `L2920: count += 1;` |
| `src/governance/residency_decision.rs` | `rank` | 115 |  | `L115: fn rank(self) -> u8 {` |
| `src/governance/residency_decision.rs` | `evaluate_fact_matrix` | 1565 |  | `L1586: let ranked_dimensions = [` |
| `src/governance/residency_decision.rs` | `requirement_rank` | 1699 |  | `L1699: fn requirement_rank(value: ExecutionRequirements) -> u8 {` |
| `src/governance/residency_decision.rs` | `effect_rank` | 1707 |  | `L1707: fn effect_rank(value: EffectSemantics) -> u8 {` |
| `src/governance/residency_decision.rs` | `external_state_rank` | 1715 |  | `L1715: fn external_state_rank(value: ExternalStateSemantics) -> u8 {` |
| `src/governance/residency_decision.rs` | `observability_rank` | 1723 |  | `L1723: fn observability_rank(value: ObservabilitySemantics) -> u8 {` |
| `src/governance/residency_decision.rs` | `candidate_for_rank` | 1731 |  | `L1731: fn candidate_for_rank(rank: u8) -> ResidencyCandidate {` |
| `src/governance/residency_decision.rs` | `semantic_parsers_and_ranks_behave_exhaustively` | 2120 |  | `L2120: fn semantic_parsers_and_ranks_behave_exhaustively() {` |
| `src/governance/universal_promotion_gate.rs` | `convergence_rejects_selection_metrics_substituted_for_execution_metrics` | 301 |  | `L304: evaluation.functional_score = 0.0;` |
| `src/knowledge/knowledge_engine.rs` | `hypothesis_families_resolved` | 5416 |  | `` |
| `src/knowledge/knowledge_engine.rs` | `living_staircase_projects_open_obligations_and_next_plan` | 6131 |  | `` |
| `src/knowledge/knowledge_engine.rs` | `living_staircase_projects_satisfied_after_real_advance` | 6184 |  | `` |
| `src/knowledge/knowledge_engine.rs` | `living_staircase_projects_blocked_after_executor_block` | 6210 |  | `` |
| `src/knowledge/knowledge_engine.rs` | `living_staircase_projects_authenticated_dependency_depth_after_expansion` | 6236 |  | `` |
| `src/knowledge/knowledge_engine.rs` | `living_staircase_projects_persisted_revision_from_canonical_head` | 6310 |  | `` |
| `src/knowledge/knowledge_engine.rs` | `living_staircase_projects_open_after_bounded_no_result` | 6342 |  | `` |
| `src/knowledge/knowledge_engine.rs` | `trace_verifier_derives_events_and_rejects_tamper_wrong_kind_and_cross_authority` | 6852 |  | `L6934: tampered_artifact.max_events += 1;` |
| `src/learning/learning_finalization.rs` | `promoted_semantic_digest_is_not_the_staged_file_digest` | 399 |  | `L401: observation_id: ObservationId::parse("obs-bridge").unwrap(),` |
| `src/learning/memory.rs` | `build_memory_snapshot` | 93 |  | `L163: total_weight += weight;` |
| `src/learning/sleep_diagnostics.rs` | `diagnose_consolidation` | 38 |  | `L47: let similarity = cosine(&old.direction, &new_field.direction)?.abs();` |
| `src/materialization/materialization_pipeline.rs` | `strategy` | 83 |  | `L86: Self::LowRank { .. } => MaterializationStrategy::LowRank,` |
| `src/materialization/universal_capability_compiler.rs` | `replay_universal_capability_shadow_plan` | 280 |  | `L320: SteeringNormalization, TokenSelection, load_activation_steering_shadow,` |
| `src/materialization/universal_capability_compiler.rs` | `fixture` | 353 |  | `L381: CapabilityNodeId::parse("node.normalize").unwrap(),` |
| `src/materialization/universal_capability_compiler.rs` | `frozen_receiver_compiler` | 467 |  | `L499: ridge: 1e-10,` |
| `src/materialization/universal_capability_compiler.rs` | `replayed_plan_materializes_only_the_compiler_target_delta` | 610 |  | `L711: .target_delta[0] += 1.0;` |
| `src/materialization/universal_capability_compiler.rs` | `low_rank_full_chain_replays_persists_reloads_and_detects_tampering` | 771 |  | `L781: let query_norm_squared = query.iter().map(\|v\| v * v).sum::<f64>();` |
| `src/materialization/universal_capability_compiler.rs` | `sparse_delta_full_chain_replays_persists_reloads_and_detects_tampering` | 901 |  | `L911: let query_norm_squared = query` |
| `src/materialization/universal_capability_compiler.rs` | `activation_steering_full_chain_replays_persists_reloads_and_detects_tampering` | 1038 |  | `L1048: let query_norm_squared = query` |
| `src/materialization/universality_evidence.rs` | `wilson_lower` | 201 |  | `L208: ((p + z2 / (2.0 * n) - z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt()) / (1.0 + z2 / n))` |
| `src/materialization/universality_evidence.rs` | `minimum_group_wilson` | 212 |  | `L220: entry.0 += usize::from(trial.passes(protocol));` |
| `src/operator/artifact.rs` | `parse` | 225 |  | `L254: "backend_ranking" => Ok(Self::BackendRanking),` |
| `src/operator/artifact.rs` | `as_str` | 432 |  | `L461: Self::BackendRanking => "backend_ranking",` |
| `src/operator/artifact.rs` | `role` | 638 |  | `L667: Self::BackendRanking => ArtifactRole::Terminal,` |
| `src/receiver/architecture_families.rs` | `classify_module` | 77 |  | `L94: } else if name.contains("norm") {` |
| `src/receiver/capability_discovery.rs` | `validate` | 63 |  | `L75: \|\| norm(&self.functional_signature)? <= 1e-15` |
| `src/receiver/capability_discovery.rs` | `analyze_group` | 140 |  | `L155: *aggregate += value / group.len() as f64;` |
| `src/receiver/model_adaptation.rs` | `profile_receiver_model` | 618 |  | `L632: normalize_sharded_safetensors(` |
| `src/receiver/model_adaptation.rs` | `sharded_checkpoint_is_normalized_and_profile_survives_source_removal` | 1032 |  | `L1032: fn sharded_checkpoint_is_normalized_and_profile_survives_source_removal() {` |
| `src/receiver/model_adaptation.rs` | `write_shard` | 1038 |  | `L1069: ("model.layers.0.input_layernorm.weight", vec![1.0, 1.0], vec![2]),` |
| `src/receiver/receiver_profile.rs` | `compatible_receiver_only_produces_shadow_plan` | 333 |  | `L342: acceptable_strategies: BTreeSet::from([MaterializationStrategy::LowRank]),` |
| `src/receiver/weight_actuator.rs` | `visit_map` | 101 |  | `L132: pub struct ShardedSafetensorsNormalizationInput {` |
| `src/receiver/weight_actuator.rs` | `next` | 282 |  | `L287: self.patch += 1;` |
| `src/receiver/weight_actuator.rs` | `prepare_lora_patches` | 918 |  | `L923: rank: usize,` |
| `src/receiver/weight_actuator.rs` | `import_peft_lora_as_dense_axis` | 1004 | Convert a PEFT LoRA into an immutable dense receiver axis. This is the missing data-plane bridge between learned calibration adapters and the V69 receiver compi | `L1061: config.lora_alpha / (config.r as f64).sqrt()` |
| `src/receiver/weight_actuator.rs` | `canonical_shard_from_index` | 1185 |  | `L1193: .any(\|component\| !matches!(component, Component::Normal(_)))` |
| `src/receiver/weight_actuator.rs` | `normalized_checkpoint_path` | 1226 |  | `L1226: fn normalized_checkpoint_path(root: &Path, digest: &Sha256Digest) -> PathBuf {` |
| `src/receiver/weight_actuator.rs` | `sharded_normalization_path` | 1231 |  | `L1231: fn sharded_normalization_path(root: &Path, digest: &Sha256Digest) -> PathBuf {` |
| `src/receiver/weight_actuator.rs` | `normalized_sharded_header` | 1236 |  | `L1236: fn normalized_sharded_header(locations: &[ShardedTensorLocation]) -> BrainResult<Vec<u8>> {` |
| `src/receiver/weight_actuator.rs` | `validate_normalization_contract` | 1272 |  | `L1272: fn validate_normalization_contract(` |
| `src/receiver/weight_actuator.rs` | `authenticate_sharded_safetensors_normalization` | 1332 |  | `L1332: pub fn authenticate_sharded_safetensors_normalization(` |
| `src/receiver/weight_actuator.rs` | `normalize_sharded_safetensors` | 1349 |  | `L1349: pub fn normalize_sharded_safetensors(` |
| `src/receiver/weight_actuator.rs` | `resolve_base_model_path` | 1539 |  | `L1541: Ok(normalize_sharded_safetensors(` |
| `src/receiver/weight_actuator.rs` | `sharded_normalization_is_exact_content_addressed_and_source_independent_after_import` | 2395 |  | `L2395: fn sharded_normalization_is_exact_content_addressed_and_source_independent_after_import() {` |
| `src/receiver/weight_actuator.rs` | `sharded_normalization_rejects_nonbijective_duplicate_and_escaping_indexes` | 2443 |  | `L2443: fn sharded_normalization_rejects_nonbijective_duplicate_and_escaping_indexes() {` |
| `src/receiver/weight_actuator.rs` | `dense_materialization_accepts_sharded_base_via_same_normalization_authority` | 2511 |  | `L2511: fn dense_materialization_accepts_sharded_base_via_same_normalization_authority() {` |
| `src/receiver/weight_actuator.rs` | `peft_lora_import_accepts_sharded_base_through_retained_normalization` | 2553 |  | `L2553: fn peft_lora_import_accepts_sharded_base_through_retained_normalization() {` |
| `src/receiver/weight_actuator.rs` | `linear_readout_extracts_real_f32_f16_bf16_rows_and_preserves_f64_difference` | 2974 |  | `L3014: assert_eq!(inspection.difference_weights, vec![1.0 - f64::from(small)]);` |
| `src/runtime/isolated_execution.rs` | `request_digest_commits_arguments_limits_and_backend_contract` | 1231 |  | `L1240: relabeled_limit.limits.cpu_seconds += 1;` |
| `src/runtime/pure_capability_e2e.rs` | `normal_relative_path` | 2130 |  | `L2130: fn normal_relative_path(path: &Path) -> BrainResult<PathBuf> {` |