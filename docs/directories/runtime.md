# `runtime/` (disco, gitignored)

Datos locales grandes: HF hub, venv Python, store del operator. **No es** `src/runtime/`.

**Ubicación:** `runtime/` en el checkout  
**Relacionado:** [evidence-cas-runtime-store](../systems/evidence-cas-runtime-store.md) · [INDEX](../INDEX.md)

## Layout sano

`llms/`, `python/`, `tidex/`, `private/` — sin `cargo-target/`.

## Variables

`TIDEX_RUNTIME_ROOT`, `TIDEX_HOME`, `TIDEX_HF_PYTHON` pueden override paths; por defecto cuelgan de `CARGO_MANIFEST_DIR/runtime/...`.
