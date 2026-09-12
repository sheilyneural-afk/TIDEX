# `src/runtime`

Aislamiento de ejecución y staging.

**Ubicación:** `src/runtime/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

Helpers de ejecución aislada / staging isolation / pure capability e2e — no confundir con el directorio disco `runtime/`.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `isolated_execution.rs` | 1310 | Fail-closed execution of an authenticated program without exposing TIDE-X state. This module intentionally accepts bytes, not paths. Before launch it verifies their declared SHA… |
| `pure_capability_e2e.rs` | 2714 | First explicit end-to-end profile for a pure deterministic capability. This module supports exactly `linear_map_f64/v1`. It is not a claim that arbitrary programs can be discove… |
| `staging_isolation.rs` | 95 | Staging-root admission. Candidate state must never share a root with activated production state. |

## Árbol (archivos)

- `isolated_execution.rs`
- `mod.rs`
- `pure_capability_e2e.rs`
- `staging_isolation.rs`

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
