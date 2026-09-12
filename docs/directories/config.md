# `config/`

Políticas numéricas y contratos de evaluación/materialización en TOML/JSON.

**Ubicación:** `config/`  
**Relacionado:** [tests](tests.md) (configuration_contracts) · [INDEX](../INDEX.md)

## Para qué

Separar **números y políticas** del código Rust: governance, models, plasticity, tidex system, evaluaciones y policies de materialization.

## Contenidos

| Path | Rol |
|------|-----|
| `tidex.toml` | Sistema cross-model / schema `tidex.cross_model.system/*` |
| `governance.toml` | Política de governance |
| `models.toml` | Catálogo declarado de modelos (paths relativos a `runtime/` del crate) |
| `plasticity.toml` | Control plane de plasticidad |
| `evaluations/*.json` | Benchmarks (integer-arithmetic, binary-logic, python-semantics, …) |
| `materialization/*.json` | backend-selection, low-rank/sparse/steering policies |
| `env.example` | Variables de entorno documentadas (no secretos) |

## Invariantes

- Los tests `configuration_contracts` amarran que estos ficheros siguen el contrato.
- Schemas aún usan prefijo `cerebro.*` (deuda de rename).
- `models.toml` usa prefijos `runtime/...` relativos al crate (no `/home/yo/...`).
