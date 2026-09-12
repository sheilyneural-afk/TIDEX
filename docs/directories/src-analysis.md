# `src/analysis`

Análisis geométrico/tomográfico y tracking temporal de representaciones.

**Ubicación:** `src/analysis/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

Dominio de diagnóstico estructural: tomografía de pesos/bloques, espacios duales, transporte relacional, trust regions, SBAS, confounds, identifiability, gauge, aperture independence, persistent homology helpers, protected maps.

No decide promoción. Produce evidencia/métricas que otras capas pueden consumir bajo contratos.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `active.rs` | 441 | Exact rank-one Gaussian covariance update for a linear sensing aperture: Σ' = Σ - Σ a aᵀ Σ / (σ² + aᵀ Σ a). No outcome value is needed because expected posterior covariance depe… |
| `aperture_independence.rs` | 420 | API pública: struct ApertureIndependenceReport, fn confounder_names, fn group_design_profiles, fn standardized_profiles |
| `block_tomography.rs` | 898 | Version of the durable parameter-layout envelope. The layout itself keeps its own schema because it is also used as an in-memory reconstruction contract; this outer schema versi… |
| `confounders.rs` | 171 | API pública: struct ConfounderRemoval, fn raw_design, fn remove_confounders |
| `dual_space.rs` | 623 | Precommitted numerical and evidence gates for one dual-space analysis. Keeping them together prevents call sites from accidentally swapping several adjacent `f64` thresholds. |
| `functional.rs` | 129 | API pública: struct FunctionalFit, fn design, fn fit_functional_map, fn attach_signatures |
| `gauge.rs` | 101 | API pública: struct BasisAlignment, fn align_bases |
| `identifiability.rs` | 299 | API pública: struct FieldResolution, struct ResolutionMap, fn gram_of_fields, fn excitation_information |
| `interaction.rs` | 62 | API pública: fn second_order_interactions, fn mvdr_weights |
| `persistent.rs` | 970 | API pública: struct PersistentTomographyResult, struct ClusterSolution, struct UnionFind, fn normalized_rows |
| `protected.rs` | 205 | API pública: struct ProtectionResult, fn wdot, fn project_to_safe_subspace |
| `protected_map.rs` | 729 | Autonomous cortex protection derived from multi-epoch Persistent Scatterer (PS-InSAR) analysis. Parameters exhibiting low amplitude dispersion ($D_A = \sigma/\mu < 0.25$) and hi… |
| `pythagoras_topology.rs` | 661 | Topology, Pythagoras Staircase Metric Correction, and Advanced SAR Algorithms adapted specifically for Neural Network Weight Space Surgery. This module addresses three critical … |
| `sbas.rs` | 206 | API pública: struct SbasResult, fn reconstruct_trajectory |
| `temporal_tracking.rs` | 618 | SAR-inspired temporal tracking for weight drift detection. This module adapts three techniques from Synthetic Aperture Radar (SAR) interferometry to track how model parameters e… |
| `tomography.rs` | 701 | Reconcile a complete-corpus reconstruction against durable capabilities with one global Hungarian assignment. This makes identity independent of incoming field order and prevent… |
| `transport.rs` | 1292 | Backwards-compatible generation transport, now affine rather than forced through the origin. Use `learn_transport_validated` before promotion. |
| `trust_region.rs` | 440 | Applies a trust region constraint that eliminates the Pythagoras Staircase metric inflation in discrete high-dimensional parameter updates. |
| `weight_tomography.rs` | 1019 | Bounded temporal tomography over the *measured* TIDE-X weight-update corpus. This module deliberately does not invent fast/slow weights, RPE, eligibility, or other channels that… |

## Árbol (archivos)

- `active.rs`
- `aperture_independence.rs`
- `block_tomography.rs`
- `confounders.rs`
- `dual_space.rs`
- `functional.rs`
- `gauge.rs`
- `identifiability.rs`
- `interaction.rs`
- `mod.rs`
- `persistent.rs`
- `protected.rs`
- `protected_map.rs`
- `pythagoras_topology.rs`
- `sbas.rs`
- `temporal_tracking.rs`
- `tomography.rs`
- `transport.rs`
- `trust_region.rs`
- `weight_tomography.rs`

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
