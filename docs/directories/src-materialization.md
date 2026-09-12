# `src/materialization`

Materializers y evaluación shadow de candidatos.

**Ubicación:** `src/materialization/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

Traduce candidatos a formas evaluables (dense/low-rank/sparse/steering). No activa producción.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `activation_steering_materializer.rs` | 448 | Authenticated activation-steering compilation for shadow execution. Parameter coordinates are never re-labelled as activations. Every hook owns an explicit, bounded linear proje… |
| `dense_shadow_materializer.rs` | 204 | Inert, replay-bound dense-delta materialization for staging receivers. This backend converts the compiler's authenticated flat `target_delta` into exact tensor-shaped blocks. It… |
| `low_rank_shadow_materializer.rs` | 637 | Verified numerical core for the shadow LowRank/LoRA backend. This module factors an already compiled dense tensor delta. It does not define capability identity and has no model-… |
| `materialization_pipeline.rs` | 541 | A single physical materialization boundary for both TIDE-X compiler frontends. Numerical planning is not execution evidence. Alternative encodings may reach the physical actuato… |
| `materialization_selector.rs` | 621 | Deterministic advisory ranking of supplied comparative measurements. This reducer does not attest measurement origins and never authorizes activation. |
| `shadow_evaluation.rs` | 412 | Isolated execution protocol for real shadow backends and evaluators. The runtime receives only one sealed JSON input containing authenticated receiver, candidate, and evaluation… |
| `shadow_materializer.rs` | 292 | Non-actuating candidate construction for receiver-coordinate experiments. This module has no model-runtime, adapter, tensor-write, or activation dependency. It can construct a d… |
| `sparse_shadow_materializer.rs` | 428 | Replay-bound sparse materialization of a compiler-produced receiver delta. Sparse coordinates are selected deterministically by magnitude, reconstructed independently, and admit… |
| `universal_capability_compiler.rs` | 1208 | Experimental orchestration for receiver-native capability compilation. This module intentionally contains no donor-weight, adapter, LoRA, or task vector representation. It binds… |
| `universality_evidence.rs` | 567 | Reproducible measurement of experimental universality N. Universality is admitted only when held-out capability evidence clears the same success contract across receivers, recei… |

## Árbol (archivos)

- `activation_steering_materializer.rs`
- `dense_shadow_materializer.rs`
- `low_rank_shadow_materializer.rs`
- `materialization_pipeline.rs`
- `materialization_selector.rs`
- `mod.rs`
- `shadow_evaluation.rs`
- `shadow_materializer.rs`
- `sparse_shadow_materializer.rs`
- `universal_capability_compiler.rs`
- `universality_evidence.rs`

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
