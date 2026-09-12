# Sistema: cross-model runtime

**Ubicación:** `src/cross_model/` (feature `cross-model-plasticity`)  
**Relacionado:** [README_CROSS_MODEL](../README_CROSS_MODEL.md) · [src-cross_model](../directories/src-cross_model.md) · [INDEX](../INDEX.md)

## Qué es

Runtime real de modelos externos (HF Transformers / NNsight, Ollama para conductual) bajo feature explícita. Ejecuta y observa; **no** decide promoción.

## Estado actual del checkout (contexto 2026-09)

- Backend Candle local retirado del árbol de modelos.
- Ruta certificada de runtime local: HF Transformers; Ollama sigue para inferencia conductual.
- `/api/info` puede reportar `nnsight_available` y `bound_sparse_dictionary`.

## Subárbol

`discovery/`, `extraction/`, `integration/`, `models/`, `plasticity/`, `promotion/`, `runtime/`, más `plasticity_daemon.rs` / `plasticity_engine.rs`.

Bins gated: `plasticity-daemon`, `tidex-operator-runner`.

