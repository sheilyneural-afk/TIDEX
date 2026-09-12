# `src/foundation`

Primitivas de identidad, digest, autoridad de ficheros privados, ledger y álgebra.

**Ubicación:** `src/foundation/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

Base fail-closed de todo TIDE-X. Si foundation miente, el resto del sistema no puede ser íntegro.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `artifact.rs` | 1467 | Descriptor-bound sequential reader for one already authenticated dense delta artifact. Opening checks the declared content identity. Subsequent reads hash the exact bytes delive… |
| `authority.rs` | 2086 | Reserve a unique private staging file beside an immutable destination. The returned descriptor is opened with `CREAT|EXCL` relative to a verified parent directory fd, so no path… |
| `contracts.rs` | 467 | API pública: fn deserialize_unique_skill_ids, fn deserialize_unique_skill_fields, enum ReconstructionInverseMode, struct ConfounderValue |
| `digest.rs` | 665 | Canonical SHA-256 identity used across TIDE-X authority artifacts. The in-memory invariant is exactly 64 lowercase ASCII hexadecimal bytes. JSON remains a plain string, so stren… |
| `error.rs` | 35 | API pública: enum BrainError, type BrainResult |
| `finite.rs` | 79 | Exact, finite floating-point values for authenticated wire formats. |
| `identity.rs` | 726 | Durable identity of one reconstructed capability field. Skill IDs are compact symbolic identifiers such as `cap-a18f...` or `skill-g3-002`. They are neither filesystem paths nor… |
| `ledger.rs` | 431 | Return one verified V2 ledger snapshot for diagnostics without reopening the ledger after chain verification. Legacy V1 records are deliberately rejected here because `LedgerEve… |
| `linalg.rs` | 663 | Neumaier compensated summation for finite values. This is the shared numerical reduction primitive for persisted metrics and decisions. |
| `low_rank_math.rs` | 340 | Architecture-independent low-rank solvers. This module owns only the numerical problem and its validated results. It has no model identity, tensor-file format, evaluation suite,… |
| `security.rs` | 96 | Resolve the private state root for this installation. The path is configuration, not part of TIDE-X's identity. It must already exist as a private directory: creation belongs to… |
| `validation.rs` | 319 | Compatibility predicate for callers that only need a boolean. `ObservationId` is the single authority for the persisted observation-ID contract. Keeping this wrapper preserves e… |

## Árbol (archivos)

- `artifact.rs`
- `authority.rs`
- `contracts.rs`
- `digest.rs`
- `error.rs`
- `finite.rs`
- `identity.rs`
- `ledger.rs`
- `linalg.rs`
- `low_rank_math.rs`
- `mod.rs`
- `security.rs`
- `validation.rs`

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
