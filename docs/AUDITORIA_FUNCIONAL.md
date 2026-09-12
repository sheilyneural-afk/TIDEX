# Auditoría funcional TIDE-X — INFORME FINAL

**Generado:** 2026-09-11T23:29:46  
**Checkout:** `/home/yo/Future`  
**Serve:** `tidex serve` en `127.0.0.1:8793`

## Resumen

| Paso | Resultado | Detalle |
|------|-----------|---------|
| Toolchain | OK | rustc/cargo **1.96.0** |
| `cargo fmt --check` | **FAIL** | diffs de estilo + warnings nightly rustfmt |
| `cargo check --all-targets --locked` | **PASS** | |
| Build bins (default + `--all-features`) | **PASS** | warning dead_code `empty_plasticity_advice` |
| `cargo test --lib` | **PASS** | **610** passed (~179s) |
| Integration `brain` | **PASS** | **13/13** (~259s) |
| Integration `production_surface` | **PASS** | **17/17** |
| Integration `quality_properties` | **PASS** | **3/3** |
| Integration `convergence_pipeline` | **FAIL** | `frozen_receiver_compiler_source_mismatch` |
| `configuration_contracts` | **PASS** | **2/2** |
| Fuzz `cargo check` | **PASS** | |
| `gate0-empty-state.sh` | **FAIL uso** | requiere arg `<directorio-privado>` |
| Operator HTTP GET (11) | **PASS** | todos 200 |
| HF `probe_runtime` E2E | **PASS** | job `6c9a1b86…8a97` completed |

## Bloqueante

`convergence_pipeline::both_architectures_produce_replayed_physical_checkpoints_through_production_cli`

```
compile universal-plan: integrity:frozen_receiver_compiler_source_mismatch
```

Origen: `src/receiver/receiver_compiler.rs` — el digest congelado del compiler no coincide con el árbol post-reorg.

## HF end-to-end

- `POST /api/workflows/direct` con SmolLM2-135M (`17b383c9…`)
- HTTP 202 → **completed**
- Access: behavioral_inference, internal_activations, activation_intervention, deep_instrumentation, sparse_autoencoder_analysis = **true**
- `authorizes_production: false` (esperado)
- evidence_sha256 `17e04221…`

## Operator / plasticidad

- Schema **v1** (`available: false`); empates 0.000 en integer_arithmetic → sin ELO (fail-closed)
- 6 jobs (incl. probe nuevo); 2 modelos SmolLM2; dataset integer-arithmetic
- nnsight + sparse dictionary bound; HF python 3.12 OK

## Veredicto

Compilación, lib, brain, production_surface, quality_properties, contracts, fuzz y **probe HF real** en verde.  
Único rojo de integración: **convergence** por mismatch de integridad del receiver compiler (deuda de la reorg).  
`fmt` sigue rojo para `make ci`.
