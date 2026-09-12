# `src/governance`

AdapterBank, residencia y gate de promoción.

**Ubicación:** `src/governance/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

Autoridad de lifecycle y readiness. Ver sistema authority-and-receipts.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `adapter_bank.rs` | 4051 | Modular, content-addressed receiver-adapter bank. Registered manifests are immutable candidates. Dynamic capability/model indexes are regenerated from the primary table and auth… |
| `residency_decision.rs` | 2316 | Universal, fail-closed residency decisions. Residency is an authority decision, not a caller preference. A caller may precommit a target and authenticated input references, but … |
| `universal_promotion_gate.rs` | 405 | Final evidence gate. A passing result is readiness for a separate promotion authority; this module has no activation or model-writing capability. Self-consistent input hashes ar… |

## Árbol (archivos)

- `adapter_bank.rs`
- `mod.rs`
- `residency_decision.rs`
- `universal_promotion_gate.rs`

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
