# Sistema: evidencia CAS y store de runtime

**Ubicación:** `runtime/` (gitignored) + APIs de persistencia en operator/foundation  
**Relacionado:** [directories/runtime](../directories/runtime.md) · [operator-control-plane](operator-control-plane.md) · [INDEX](../INDEX.md)

## Qué es

Almacén local content-addressed y roots de runtime que el código espera en disco, **fuera** de git.

## Layout conceptual deseado

```text
runtime/
  llms/huggingface/hub/     # snapshots HF (blobs + snapshots/<rev>)
  python/tidex-mechinterp/  # venv tools
  tidex/operator/           # models/jobs/runs/datasets by-sha
  private/                  # roots privados de autoridad (si se usan)
```

## Anti-patrón

`runtime/cargo-target` **no** pertenece aquí. Aparece si alguien exporta `CARGO_TARGET_DIR=.../runtime/cargo-target` al hacer `tidex serve`. Gate0 lo rechaza.

## Jobs / runs

Evidence receipts ligan request/executor/stdout/stderr hashes. Tamper del receipt → load falla (integridad).

