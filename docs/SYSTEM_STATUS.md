# Estado del proyecto

Estado honesto del crate TIDEX / checkout Future.

**Ubicación:** `docs/SYSTEM_STATUS.md`  
**Relacionado:** [INDEX](INDEX.md) · [ARCHITECTURE](ARCHITECTURE.md) · [QUALITY_AND_TESTING](QUALITY_AND_TESTING.md)

## Base de verificación de esta nota

- **Remoto público verificado:** `main` @ `8091cc8791c9e67794877eb6805fe48b35cf4533` — *refactor: reorganize TIDEX into domain modules…* (2026-09-11).
- **Máquina local `/home/yo/Future`:** el harness de este agente **no pudo ejecutar** `git`/`ls` con `machineId` (el parámetro no enruta al host). Cifras de working tree local, tamaños y tests “ahora mismo” quedan **por verificar** en la máquina del usuario.
- Auditorías previas en `docs/AUDITORIA_*` son históricas de sesión; no reutilizar conteos de tests como estado actual sin re-ejecutar.

## Hechos confirmados en `8091cc8`

| Hecho | Evidencia |
|-------|-----------|
| Reorg de dominios pushed | Commit `8091cc8`; `src/lib.rs` lista foundation…runtime + feature `cross_model` |
| `publish = false` | `Cargo.toml` |
| `autobins` / `autotests` = false | `Cargo.toml`; bins/tests registrados explícitamente |
| Toolchain CI/local pin | `rust-toolchain.toml` → `1.96.0` |
| `tests/brain.rs` ausente | Solo 4 tests externos: production_surface, quality_properties, convergence_pipeline, configuration_contracts |
| Schemas `cerebro.*` | Presentes en gates, authority temps, operator HTML/API |
| Identidad `model_id` por snapshot | `model_candidate_identity` dominio `…OPERATOR-MODEL-CANDIDATE:v2` en `control_plane.rs` |
| Gates pin nightly viejo | `quality/gate1-tooling.sh` → `QUALITY_EXPECTED_COMMIT=0ed41eb4142dda2df61eb1145a312c1a9d62eb56` |
| `runtime/` gitignored | `.gitignore` |

## Gaps conocidos (abiertos)

1. **Naming `cerebro.*`** en schemas/wire — deuda de renombre.
2. **`tests/brain.rs` eliminado** — pérdida de cobertura de integración brain/phantom/sleep.
3. **Gates (gate1+) pin de nightly** posiblemente desfasado → drift de toolchain.
4. **`publish = false`** — crate no publicable en crates.io (intencional hoy).
5. **Estado local vs remoto:** trabajo adicional de `model_id` en working tree del usuario *por verificar*; no se hizo verificación live de IDs.

## Política de verificación

Este documento **no** afirma un recuento de `cargo test` sin re-ejecución. Para estado vivo:

```bash
cd /home/yo/Future
git rev-parse HEAD && git status -sb
make ci   # o el subconjunto necesario; un solo cargo a la vez
```

## Documentación

Índice: [INDEX.md](INDEX.md).

## Estado de fricciones (verificado 2026-09-12)

| Punto | Estado |
|-------|--------|
| Identidad content-bound / cero aliases | **OK** — dominio `OPERATOR-MODEL-CANDIDATE:v2`; test exige cambio de id si mutan pesos; sin `OperatorModelAlias` |
| `cargo fmt --check` | **OK** |
| `runtime/cargo-target` | **OK** — layout `llms/`, `private/`, `python/`, `tidex/` |
| Docs esqueleto | **OK** — reescritos con sustancia; ver `docs/INDEX.md` |
| Paths en `config/models.toml` | **Corregido** a paths relativos `runtime/...` (antes `/home/yo/Future/...`) |
| Working tree dirty | **Abierto** — cambios locales sin commit |
| Schemas `cerebro.*` | **Abierto** — ~745 hits; rename a `tidex.*` pendiente (wire + persistidos) |
| `publish = false` | **Intencional** — no es crate crates.io; no es bug |

Verificación positiva: check/clippy/deny/audit, lib tests, operator graph, fuzz 100k×3 PASS.
