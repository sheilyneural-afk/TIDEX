# Auditoría funcional TIDE-X

**Checkout:** `/home/yo/Future`

Este documento conserva el snapshot original de la sesión del 2026-09-11 y la contra-auditoría que lo rectificó. El **estado actual** (post-corrección) está al final.

---

## Snapshot original — 2026-09-11T23:29:46

**Serve en sesión:** `tidex serve` en `127.0.0.1:8793`

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

**Veredicto del snapshot:** único rojo de integración aparente: **convergence** por mismatch de integridad del receiver compiler. `fmt` rojo para `make ci`.

> ⚠️ Este snapshot mezclaba estado temporal con diagnóstico estructural sin etiquetar caducidad. No usar como estado actual sin re-verificar.

---

## Contra-auditoría — 2026-09-11T23:36:11

**Método:** re-ejecución independiente + verificación en disco + spotcheck HTTP.

### Sesgos detectados en el informe original

1. **Serve presente vs caído** — al verificar, `:8793` no escuchaba; el informe asumía proceso activo.
2. **Convergence bloqueante** — fallo histórico en sesión con `cargo test` concurrente; re-run serializado: **3/3 PASS**. Causa probable: carrera de builds (`TIDEX_SOURCE_TREE_DIGEST` en `env!()` vs artefacto congelado / bin `tidex` desalineados).
3. **Gate0 FAIL** — error de invocación (sin args ni `CARGO_TARGET_DIR`); con args correctos + dir privado 0700 → **PASS**.
4. **Plasticidad v1** — runtime stale; código y bin actual exponen `operator_plasticity_advice/v2`.

### Hallazgos que resisten auditoría dura

- Compilación, superficie CLI/bins, lib 610, brain, production_surface, quality_properties, contracts, fuzz.
- Job HF real (`6c9a1b86…8a97`) con evidence `17e04221…` en CAS del operator.
- `fmt` realmente rojo (8 hunks en sesión: `models/mod.rs`, `control_plane.rs×7`).
- Mecánica del mismatch bien citada (`receiver_compiler.rs` compara `compiler_source_sha256` vs `env!("TIDEX_SOURCE_TREE_DIGEST")`).

### Método del informe original — problemas

- `cargo test --test a --test b …` fail-fast: al fallar convergence no corrió production/quality en el mismo lote.
- Otro `cargo test --all-targets … --test-threads=1` concurrente → riesgo de locks/digest skew.
- Gate0 evaluado mal invocado y reportado como hallazgo del producto.

---

## Estado actual — 2026-09-12 (post-corrección)

**Correcciones aplicadas:** `cargo fmt --all` (1 hunk restante en `control_plane.rs`); documento actualizado.

| Paso | Resultado | Detalle |
|------|-----------|---------|
| Toolchain | **PASS** | rustc/cargo **1.96.0** |
| `cargo fmt --check` | **PASS** | sin diffs |
| `cargo check --all-targets --locked` | **PASS** | |
| Build bins (default + `--all-features`) | **PASS** | 13 bins no-bench + benches |
| Integration `convergence_pipeline` | **PASS** | re-run serializado OK |
| `gate0-empty-state.sh` | **PASS** | con `CARGO_TARGET_DIR` + dir 0700 |
| Operator HTTP GET (11) | **PASS** | todos 200 (serve en `:8793`) |
| Plasticidad API | **v2** | `available: false` (fail-closed, empates 0.000) |
| HF `probe_runtime` E2E | **PASS** | job `6c9a1b86…8a97` completed |

### Tests históricos (no re-corridos completos en esta sesión)

| Suite | Resultado | Evidencia |
|-------|-----------|-----------|
| `cargo test --lib` | **610 PASS** | `/tmp/tidex-test-lib.out` (~179s) |
| Integration `brain` | **13/13 PASS** | log integración 23:25 |
| Integration `production_surface` | **17/17 PASS** | `/tmp/tidex-test-ps.out` |
| Integration `quality_properties` | **3/3 PASS** | `/tmp/tidex-test-qp.out` |
| `configuration_contracts` | **2/2 PASS** | `/tmp/tidex-test-cfg.out` |
| Fuzz `cargo check` | **PASS** | |

### Veredicto actual

**Verde:** check, fmt, bins, convergence (serializado), gate0 (bien invocado), API×11, HF probe, contracts/fuzz (histórico).

**Aviso operativo:** plasticidad `available:false` es comportamiento esperado (sin ELO por empates). Correr integración con un solo `cargo test` a la vez para evitar digest skew.

**Recomendación CI:** `make ci` requiere re-ejecutar lib + integración en árbol limpio antes de release.
