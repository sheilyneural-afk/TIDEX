# Auditoría completa TIDE-X — INFORME MAESTRO

**Actualizado:** 2026-09-12T01:23:26
**Checkout:** `/home/yo/Future`  
**Método:** pipeline serie; HTTP en vivo; análisis de basura git; UI por HTTP. Capas bloqueadas por otro `cargo test --lib --all-features` concurrente se marcan explícitas.

## 0. Veredicto ejecutivo

El **núcleo de ingeniería está sano** en corrida serial: fmt/check/lib/integración registrada/contracts/fuzz/gate0/API verdes.

Hay **tres problemas estructurales de alto impacto**:

1. **`.git/objects/pack/tmp_pack_*` ≈ 16,57 GiB** (13 packs temporales abortados) — basura recuperable, no historia útil.
2. **`tests/brain.rs` eliminado** — pérdida de cobertura de análisis/phantom/sleep.
3. **Identidades de modelos del operator no estables** entre rescans — un `behavioral_evaluation` falló con `operator_model_not_cataloged` al reutilizar IDs viejos; el catálogo actual ya no contiene `17b383c9…` del probe histórico.

Naming `cerebro.*` sigue omnipresente (~**770** matches en src/config/docs clave).

**Clippy / cargo-deny / gates 1–4 / release build:** no cerrados en esta pasada (lock de cargo ajeno + coste/env de gate1).

## 1. Tamaño y layout

| Path | Tamaño | Notas |
|------|--------|-------|
| `.` | ~25 G | |
| `.git` | ~17 G | pack real ~1,3 M; **tmp_pack_\*** 16,57 G; loose grande ~206 M en `objects/f6/…` |
| `runtime/` | ~8,1 G | gitignored |
| `src/` | 5,0 M | 12 dominios |
| `docs/` | ~616 K | fórmulas + auditorías |
| `quality/` | ~544 K | gates/experiments |

### Basura git (causa raíz)
13 archivos `tmp_pack_*` en `.git/objects/pack/` (3,67 G + 2,44 G + …). Son restos de `git fetch/gc/repack` interrumpidos.  
**Remedio (manual, no ejecutado):** borrar `tmp_pack_*` huérfanos y `git prune` / `git gc` — liberaría ~16,5 G.

## 2. Git / working tree

- `main` @ `dccfd3c`; remote `sheilyneural-afk/TIDEX`
- ~254 porcelain
- `.env.development/.production/.staging` como **Deleted** (bien fuera del índice; revisar history aparte)

## 3. Pipeline de verificación (serie)

| Paso | Resultado | Detalle |
|------|-----------|---------|
| Toolchain 1.96.0 | OK | |
| `cargo fmt --check` | **PASS** | |
| `cargo check --all-targets --locked` | **PASS** | |
| `cargo test --lib` | **PASS** | **610** (~14,7s) |
| `production_surface` | **PASS** | **16** |
| `quality_properties` | **PASS** | **3** |
| `convergence_pipeline` | **PASS** | **1** (serial) |
| `configuration_contracts` | **PASS** | **2** |
| fuzz check | **PASS** | |
| gate0-empty-state | **PASS** | dir 0700 + `CARGO_TARGET_DIR` |
| GET ×11 | **PASS** | 200 |
| UI `/` + `/operator` | **PASS** | 200; nav overview/pipeline/plasticity/modules/evidence; operator con recipes/workflows |
| HF probe histórico `6c9a1b86…` | **PASS (disco)** | evidence `17e04221…`, auth false — **IDs de modelo de ese job ya no están en catálogo actual** |
| HF `behavioral_evaluation` (intento 1) | **FAIL** | `invalid:operator_model_not_cataloged` (ID stale `232587d3…`) |
| HF `behavioral_evaluation` (reintento #2) | **PASS** | job `72422153…` completed; weighted_score **0.0** en integer_arithmetic_v1; evidence `eff2afc6…` / stdout ev `df9aeec…`; auth false |

## 4. Regresiones vs auditorías previas

| Antes | Ahora |
|-------|-------|
| fmt FAIL (8 diffs) | **PASS** |
| 12 benches sin `[[bin]]` | **declarados** |
| brain 13/13 | **archivo borrado** |
| transport/universality tests huérfanos | **ausentes** |
| plasticidad v1 | **v2** con BCM/eligibility/neuromod/PI |
| convergence FAIL (carrera) | **PASS** serial |
| model ids 17b383c9 / 232587d3 | **rotados** a b855ea85 / 90e65e6f tras rescan |

## 5. Seguridad

- `unsafe_code = forbid`
- Sin `.env` vivos superficiales; deletes de `.env.*` en working tree
- Schemas/digests: prefijo **`cerebro` / `CEREBRO:TIDEX`** (~770 hits)
- Clippy `-D warnings`: **bloqueado** por `cargo test --lib --all-features` ajeno (PID observado)
- cargo-deny: **no ejecutado** (mismo lock)
- Secret scan profundo / history de `.env.*`: **no hecho**

## 6. Operator / HF / plasticidad

Serve `:8793` loopback.

**Plasticidad v2 (snapshot):**
- `available: false`
- `source_jobs: 2`
- BCM `integer_arithmetic_v1` theta_m≈0.49005, observations=2
- eligibility trace 0 / credit 0
- neuromodulation novelty=1, plasticity_modulation=0.3
- pi_controller setpoint=1, measurement=0, output=0.6

**UI:**
- `web-console`: TIDE-X Interfaz; paneles overview, pipeline, plasticity, modules, evidence
- `/operator`: recipes + plasticity + workflows API cableados (~65 KB HTML)

**HF:**
- Probe histórico OK en CAS
- Behav #1 FAIL por catálogo rotado (hallazgo de **estabilidad de identidad**)
- Behav #2 **PASS** job `724221530aae0294…`: SmolLM2 `b855ea85…` + integer-arithmetic; `weighted_score=0.0` (modelo no resuelve el bench; fail-closed coherente con plasticidad/ELO); `authorizes_production=false`

## 7. Fórmulas

`docs/FORMULAS_CANONICAS.md` / `INDEX` / `ALGORITMICAS` (849 / 151) — inventario previo; no regenerado.

## 8. Gates 1–4 (estado de auditoría, no ejecución completa)

| Gate | Estado auditoría |
|------|------------------|
| gate0 | **PASS** ejecutado |
| gate1 | **NO EJECUTADO** — exige Nightly pin `0ed41eb…`, fuzz hasta 1e5, floors cobertura 74/70/75, sanitizers; coste alto |
| gate2–4 | **NO EJECUTADOS** — dependen de tooling/tiempo; scripts presentes |

## 9. Prioridades

1. **P0 disco:** eliminar `tmp_pack_*` + prune (recuperar ~16,5 G)
2. **P0 cobertura:** restaurar o sustituir `tests/brain.rs`
3. **P1 operator:** anclar model_id a content-address estable o re-probe tras cada scan antes de workflows
4. **P1 CI:** correr clippy `-D warnings` + deny en árbol quieto
5. **P2:** rename `cerebro`→`tidex` en schemas/digests
6. **P2:** gate1 en máquina dedicada / nightly

## 10. Capas aún abiertas

- [ ] Clippy full (lock ajeno)
- [ ] cargo-deny
- [ ] gate1–gate4 ejecución
- [ ] Build release reproducible / gate0-release
- [ ] History scan de `.env.*`
- [x] Resultado final behav #2 — PASS score 0.0
- [ ] Walk UI click-path (esta pasada solo HTTP estático + operator embed)

---
*Contraste con `CONTRA_AUDITORIA_FUNCIONAL.md` y `AUDITORIA_FUNCIONAL.md` (snapshots previos).*
