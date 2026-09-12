# `src/operator`

Control plane HTTP, registry, grafo y workspace.

**Ubicación:** `src/operator/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

Superficie operativa local. Nunca shell arbitrario; nunca autoridad productiva.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `artifact.rs` | 861 | Closed connection-artifact vocabulary for Operator executors. These are production identities, not slogans. A require/produce string that does not parse is a catalog defect. |
| `control_plane.rs` | 4155 | TIDE-X production operator control plane. The control plane is an operator surface over existing TIDE-X authorities. It never evaluates arbitrary shell commands and it never tur… |
| `executor_registry.rs` | 1738 | Canonical TIDE-X executor capability registry. This registry is descriptive, not authoritative by itself: it declares which existing implementation owns each executable capabili… |
| `graph.rs` | 320 | Production Operator graph. Composes `executor_catalog` and `recipe_catalog` into a closed topology of artifacts, reachability and authority. This module is part of the runtime, … |
| `workspace.rs` | 318 | API pública: const WORKSPACE_SCHEMA, const MODEL_SCHEMA, const MAX_RECORD_BYTES, struct WorkspaceManifest |

## Árbol (archivos)

- `artifact.rs`
- `control_plane.rs`
- `executor_registry.rs`
- `graph.rs`
- `mod.rs`
- `workspace.rs`

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
