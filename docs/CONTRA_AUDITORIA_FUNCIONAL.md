# Contra-auditoría del informe funcional

**Objeto auditado:** `docs/AUDITORIA_FUNCIONAL.md` (Generado 2026-09-11T23:29:46)  
**Contra-auditoría:** 2026-09-11T23:36:11  
**Método:** re-ejecución independiente + verificación en disco + spotcheck HTTP (serve se encontró caído y se reinició para la prueba en vivo).

## Veredicto sobre el informe

El informe es **mayormente fiel a lo que ocurrió en esa sesión**, pero **no es un estado actual fiable** y tiene **3 sesgos importantes**:

1. Presenta el serve como hecho presente; al verificar, **:8793 no escuchaba**.
2. Declara `convergence_pipeline` como bloqueante rojo; al re-ejecutar **3/3 PASS** (fallo histórico, no reproducible ahora).
3. Marca `gate0` como fallo; era **error de invocación**. Con `CARGO_TARGET_DIR` + dir privado `0700` → **PASS**.

## Tabla de verificación casilla a casilla

| Claim del informe | Veredicto | Evidencia ahora |
|-------------------|-----------|-----------------|
| Toolchain 1.96.0 | **CONFIRMADO** | `rustc`/`cargo` 1.96.0 |
| fmt --check FAIL | **CONFIRMADO** | exit 1; **8** hunks Diff (`models/mod.rs`, `control_plane.rs`×7) |
| cargo check PASS | **CONFIRMADO** | exit 0 (re-check) |
| Build bins PASS + dead_code warning | **CONFIRMADO** | 13 bins ejecutables presentes; warning en log all-features |
| lib 610 PASS | **CONFIRMADO (histórico)** | `/tmp/tidex-test-lib.out` 23:13 — `610 passed`. No re-corrido completo en esta contra-auditoría (~3–4 min). Atributos `#[test]` en src ≈639 (no igual a tests ejecutados). |
| brain 13/13 PASS | **CONFIRMADO (histórico)** | log integración 23:25 |
| production_surface 17/17 | **CONFIRMADO (histórico)** | `/tmp/tidex-test-ps.out` |
| quality_properties 3/3 | **CONFIRMADO (histórico)** | `/tmp/tidex-test-qp.out` |
| convergence FAIL mismatch | **HISTÓRICO CIERTO / ACTUAL FALSO** | Falló a las ~23:25. Re-run 23:34+ y 3 repeticiones: **PASS**. Causa probable: carrera de builds (`TIDEX_SOURCE_TREE_DIGEST` en `env!` vs frozen artifact / bin `tidex` desalineados mientras corrían varios `cargo test` en paralelo). |
| configuration_contracts 2/2 | **CONFIRMADO** | log audit + cfg |
| fuzz check PASS | **CONFIRMADO** | log audit FUZZ_EXIT=0 |
| gate0 FAIL uso | **PARCIAL / ENGAÑOSO** | Sin args falla. **Con** args correctos + `CARGO_TARGET_DIR=…/runtime/cargo-target` → **exit 0**. |
| GET ×11 PASS | **CONFIRMADO tras reinicio** | Al empezar la contra-auditoría: serve muerto (000). Tras `./tidex serve`: 11×200. |
| HF probe job 6c9a1b86… | **CONFIRMADO** | Disco `status.json` + API: completed, evidence `17e04221…`, auth false, access flags todos true, 134515008 params / 30 layers |
| Plasticidad schema v1 | **CADUCADO** | Con el serve viejo era v1. Tras reinicio del bin actual: **`operator_plasticity_advice/v2`**, `available:false`. El informe acertó el síntoma “serve no reiniciado”; no debería leerse como verdad del código actual. |
| 6 jobs / 2 SmolLM2 | **CONFIRMADO** | 6 dirs job; 2 model json + 2 hubs HF |
| nnsight + py3.12 | **CONFIRMADO** | nnsight 0.7.0; python 3.12.3 |
| Veredicto “único rojo convergence” | **YA NO SOSTIENE** | Convergence verde ahora; fmt sigue rojo; gate0 no es rojo real |

## Qué falló en el método del informe original

- Mezcló **snapshot temporal** con **diagnóstico estructural** sin etiquetar caducidad.
- `cargo test --test a --test b --test c --test d` **fail-fast**: al fallar convergence no corrió production/quality en el mismo lote (luego se corrigió aparte — OK, pero el primer lote estaba incompleto).
- Había **otro** `cargo test --all-targets … --test-threads=1` concurrente → riesgo alto de locks/digest skew (explica el mismatch).
- gate0 se evaluó mal invocado y se reportó como hallazgo del producto.

## Qué sí resiste auditoría dura

- Compilación y superficie CLI/bins.
- Lib 610 en esa corrida.
- brain / production_surface / quality_properties / contracts / fuzz.
- Job HF real con evidencia en CAS del operator.
- fmt realmente rojo (8 diffs).
- Mecánica del mismatch está bien citada (`receiver_compiler.rs` compara `compiler_source_sha256` vs `env!("TIDEX_SOURCE_TREE_DIGEST")`).

## Correcciones al veredicto

**Estado real ahora (post contra-auditoría):**

- Verde: check, bins, lib (histórico), brain, production_surface, quality_properties, contracts, fuzz, convergence (re-run), gate0 (bien invocado), HF probe, API×11 (serve reiniciado).
- Rojo actual: **`cargo fmt --check`**.
- Aviso: plasticidad **v2** tras reinicio; `available:false` se mantiene.
- Serve: **reiniciado** por esta verificación (PID del wrapper shell); conviene que confirmes si quieres dejarlo arriba.

## Recomendación

1. No tratar el FAIL de convergence del informe como deuda abierta sin re-ejecutar en árbol limpio/serializado.
2. Correr integración con **un solo** `cargo test` a la vez.
3. Arreglar fmt (8 hunks) antes de `make ci`.
4. Actualizar `AUDITORIA_FUNCIONAL.md` con esta contra-auditoría o archivarlo como snapshot 23:29.

---

## Post-corrección — 2026-09-12

Acciones aplicadas tras esta contra-auditoría:

| Item | Estado |
|------|--------|
| `cargo fmt --all` | **PASS** — 1 hunk restante en `control_plane.rs` corregido |
| `AUDITORIA_FUNCIONAL.md` | Actualizado con snapshot + contra-auditoría + estado actual |
| `convergence_pipeline` | **PASS** (re-run serializado) |
| `gate0-empty-state.sh` | **PASS** (invocación correcta) |
| `configuration_contracts` | **PASS** (re-verificado) |
| Fuzz `cargo check` | **PASS** (re-verificado) |

**Rojo restante para `make ci`:** ninguno en los checks rápidos re-ejecutados. Lib + brain (~7 min) no re-corridos en esta sesión; ver logs históricos en `/tmp/tidex-test-*.out`.
