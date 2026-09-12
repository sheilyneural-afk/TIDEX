# `quality/`

Scripts de gates, smoke, evidence receipts y experimentos de calidad.

**Ubicación:** `quality/`  
**Relacionado:** [quality-gates-and-ci](../systems/quality-gates-and-ci.md) · [QUALITY_AND_TESTING](../QUALITY_AND_TESTING.md) · [INDEX](../INDEX.md)

## Para qué

Más allá de `cargo test`: demostrar release readiness, tooling (ASan/Miri/fuzz), y que el checkout no se ensucia.

## Piezas

| Path | Rol |
|------|-----|
| `gate0-empty-state.sh` / `gate0-release.sh` | P0 layout/release |
| `gate1-tooling.sh` | Cobertura, sanitizers, fuzz |
| `gate2-verification.sh` … `gate4-release-readiness.sh` | P2–P4 |
| `smoke/` | Humo operativo |
| `evidence/` | Receipts de corridas |
| `convergence/` | Auditoría de convergencia |
| `experiments/` | Benches/smoke experimentales (algunos `[[bin]]`) |
| `bootstrap-runtime.sh`, `build-release-bundle.sh`, `sign-release.sh`, … | Release ops |

## Lectura recomendada

Empieza por el sistema [quality-gates-and-ci](../systems/quality-gates-and-ci.md); este directorio es el filesystem de esa política.
