# `src/bin`

Binarios CLI y benches declarados en Cargo.toml.

**Ubicación:** `src/bin/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

`autobins=false`: todo binario vive aquí (o paths explícitos) y debe estar listado en `Cargo.toml`.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `acquire_system.rs` | 311 | Produce the single current source-capture authority: request, exact re-verifiable envelope and retained CAS objects are sealed together in one receipt. This command performs no … |
| `active_aperture_bench.rs` | 64 | API pública: fn main |
| `adaptive_learning_cycle.rs` | 249 | API pública: const MAX_CLI_JSON_BYTES, fn read_confined_invocation, fn controller_compose, fn usage |
| `autonomous_learning_plan.rs` | 65 | API pública: fn main, const MAX_LEARNING_TARGET_BYTES, fn run_from_path, fn run |
| `causal_credit_bench.rs` | 49 | API pública: struct ReplayPayload, fn main |
| `cognitive_field_bench.rs` | 127 | API pública: struct TrustArtifact, struct CausalArtifact, fn main |
| `dual_space_bench.rs` | 79 | API pública: struct RepresentationPayload, fn main |
| `functional_transplant_bench.rs` | 46 | API pública: struct Input, fn main |
| `ledger_diagnose.rs` | 49 | API pública: fn main, fn run_diagnose |
| `protected_map_bench.rs` | 87 | API pública: struct EvidenceRow, struct Payload, fn main |
| `pure_linear_runner.rs` | 20 | API pública: fn main, fn run |
| `record_representation_evidence.rs` | 40 | API pública: fn main, fn parse_arguments, fn run |
| `relational_transport_bench.rs` | 55 | API pública: struct Input, fn main |
| `sbas_diag.rs` | 44 | API pública: fn main |
| `structured_geometry_bench.rs` | 107 | API pública: struct AdapterRow, struct TrainingManifest, fn main |
| `tidex.rs` | 1773 | API pública: const MAX_CLI_JSON_BYTES, const MAX_ANALYSIS_INPUT_BYTES, struct KnowledgePlanRequest, struct ResidencyDecisionRequest |
| `tidex_finalize.rs` | 56 | API pública: fn parse_arguments, fn main, fn run |
| `tidex_operator_runner.rs` | 352 | API pública: const MAX_REQUEST_BYTES, enum OperatorRunnerRequest, struct ActivationTransferReport, fn read_request |
| `trust_region_bench.rs` | 165 | API pública: struct ProtectedWrapper, struct Assignment, struct CausalPlan, struct CausalCreditArtifact |
| `universal_shadow_runner.rs` | 46 | API pública: const RUNNER_INPUT_PATH, const MAX_INPUT_BYTES, fn run, fn main |

## Árbol (archivos)

- `acquire_system.rs`
- `active_aperture_bench.rs`
- `adaptive_learning_cycle.rs`
- `autonomous_learning_plan.rs`
- `causal_credit_bench.rs`
- `cognitive_field_bench.rs`
- `dual_space_bench.rs`
- `functional_transplant_bench.rs`
- `ledger_diagnose.rs`
- `protected_map_bench.rs`
- `pure_linear_runner.rs`
- `record_representation_evidence.rs`
- `relational_transport_bench.rs`
- `sbas_diag.rs`
- `structured_geometry_bench.rs`
- `tidex.rs`
- `tidex_finalize.rs`
- `tidex_operator_runner.rs`
- `trust_region_bench.rs`
- `universal_shadow_runner.rs`

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
