# `src/knowledge`

KnowledgeEngine: autoridad epistémica del ciclo gobernado.

**Ubicación:** `src/knowledge/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

Un solo archivo grande concentra el motor de conocimiento. `living_staircase` es vista, no boss.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `knowledge_engine.rs` | 7512 | Autonomous, evidence-reducing knowledge engine for one governed capability. This module is the sole executable, revisioned epistemic authority for a governed capability. No call… |

## Árbol (archivos)

- `knowledge_engine.rs`
- `mod.rs`

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
