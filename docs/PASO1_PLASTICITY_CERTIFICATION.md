# Paso 1 — Plasticidad durable: certificación

**Fecha:** 2026-09-12 ~05:10 CEST (Europe/Madrid)  
**Rama:** `feat/durable-plasticity-controllers`  
**Base de código:** `456d1eb` (`fix: seal coevolution causally and close advisory BidirectionalLoop`)  
**PR:** https://github.com/sheilyneural-afk/TIDEX/pull/1  
**Roadmap:** [TIDEX_TWO_LEARNINGS_AND_CLOSED_LINKS.md](TIDEX_TWO_LEARNINGS_AND_CLOSED_LINKS.md)

## Resultado

**Paso 1 = CLOSED / CERTIFIED.** Sin commits de hardening de algoritmos: la suite no reveló huecos de idempotencia, replay sellado, gates fail-closed ni sello causal.

## Comandos

```text
cargo test --features cross-model-plasticity plasticity
→ 18 passed / 0 failed

cargo test --features cross-model-plasticity --lib 'operator::control_plane::tests'
→ 22 passed / 0 failed

cargo check --features cross-model-plasticity  → ok
cargo check --no-default-features             → ok
cargo check                                   → ok
```

## Cobertura relevante

- Persistencia `operator/plasticity/controller_state.json` entre llamadas
- Idempotencia de coevolución (mismo discovery no doble-registra)
- *newer-valid*
- Intervenciones causales (`submitted_unix_ns` ≤ sellado del ciclo)
- `BidirectionalLoop` durable + `plan_next_tick` / `CoEvolutionDirective`
- Sellado `evidence_sha256` + verify en load/persist
- Carga fail-closed de `config/plasticity.toml`
- Sigue **AdvisoryOnly** (no ejecución / promoción)

## Fuera de alcance (congelado)

No BCM/ELO/planners nuevos. No cablear `ProceduralMemory.retrieve` al decisor. No `procedural_memory.json`. Paso 2 = replay canónico → retrieve.
