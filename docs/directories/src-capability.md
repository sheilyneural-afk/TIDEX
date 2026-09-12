# `src/capability`

IR y bundles de capacidad + vault de contenido capturado.

**Ubicación:** `src/capability/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

Define cómo se representa una capacidad (`capability_ir`, `capability_bundle`), contratos de adquisición y el content vault de captures. Es vocabulario/contrato, no el motor de learning completo.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `acquisition_contract.rs` | 2523 | Universal, donor-agnostic admission and source-envelope primitives. This module deliberately does **not** know about any donor, provider, language toolchain, model format, or pr… |
| `capability_bundle.rs` | 656 | Sealed capability intake bundles. A bundle is the immutable subject that knowledge and residency authorities consume. It binds a capability identity and its current representati… |
| `capability_ir.rs` | 1859 | A closed, typed intermediate representation for acquired capabilities. Source code is evidence, not a safe runtime representation. This IR is a deliberately small bridge between… |
| `content_vault.rs` | 806 | Immutable, private retention of bytes admitted by an acquisition envelope. A [`SystemEnvelope`](crate::capability::acquisition_contract::SystemEnvelope) is a descriptor-bound ob… |

## Árbol (archivos)

- `acquisition_contract.rs`
- `capability_bundle.rs`
- `capability_ir.rs`
- `content_vault.rs`
- `mod.rs`

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
