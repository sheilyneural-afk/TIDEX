# `src/engine`

Motor de campo cognitivo / programas paramétricos / transición de estado.

**Ubicación:** `src/engine/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

Capa de runtime de engine (heads, store, support, learned controller, cognitive field). Orquesta ejecución interna distinta del operator HTTP.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `analysis.rs` | 1034 | Sin module-doc; ver símbolos públicos en el fuente. |
| `cognitive_field.rs` | 792 | API pública: struct CognitiveFieldConfig, struct CognitiveFieldDrive, struct CognitiveCoalition, struct CognitiveFieldState |
| `engine_head.rs` | 249 | Canonical engine head, transition journal and recovery outcomes. Live pointers remain for compatibility with existing receipts. This module adds the missing atomic snapshot: one… |
| `learned_controller.rs` | 1916 | A training record is intentionally not an in-memory observation/label pair. Its learned observation comes from an authenticated aperture observation and its state/label pair mus… |
| `parametric_program.rs` | 502 | Compose one parameter-space operator from SkillFields. |
| `runtime.rs` | 1581 | Sin module-doc; ver símbolos públicos en el fuente. |
| `store.rs` | 698 | API pública: fn canonical_head_compare_and_swap_matches |
| `support.rs` | 1720 | Shared engine support: confined IO, digests, keys and receipt loaders. |
| `transition.rs` | 1653 | API pública: enum UnsealedCorpusRecoveryDecision, fn classify_unsealed_corpus_recovery |
| `types.rs` | 374 | Wire and transaction types for the TIDE-X engine authorities. |

## Árbol (archivos)

- `analysis.rs`
- `cognitive_field.rs`
- `engine_head.rs`
- `learned_controller.rs`
- `mod.rs`
- `parametric_program.rs`
- `runtime.rs`
- `store.rs`
- `support.rs`
- `transition.rs`
- `types.rs`

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
