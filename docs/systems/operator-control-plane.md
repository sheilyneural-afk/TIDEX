# Sistema: operator control plane

**Ubicación:** `src/operator/control_plane.rs` (+ `executor_registry`, `graph`, `workspace`, `artifact`)  
**Relacionado:** [model-identity-and-catalog](model-identity-and-catalog.md) · [web-console](web-console.md) · [src-operator](../directories/src-operator.md) · [INDEX](../INDEX.md)

## Qué es

Superficie operativa HTTP **loopback** sobre autoridades ya existentes. El propio módulo lo declara:

> never evaluates arbitrary shell commands and it never turns a control plane result into production activation authority.

Cada recipe ejecutable mapea a un comando `tidex` allow-listed; inputs/outputs se persisten con identity.

## Por qué existe

Para operar el lab local (scan HF, jobs, datasets, plasticidad advisory) **sin** convertir la UI en segunda autoridad.

## Bind y límites

- Escucha **127.0.0.1** (tests rechazan host non-loopback / cross-origin).
- Puerto habitual de la UI: **8793**.
- Concurrencia acotada (`MAX_CONCURRENT_OPERATOR_JOBS`, conexiones HTTP limitadas).
- Scan de modelos: profundidad y nº de entradas acotados; root debe caer dentro del hub HF del runtime.

## API (rutas reales en código)

| Método | Path | Función |
|--------|------|---------|
| GET | `/api/info` | Metadatos runtime (engine, nnsight, SAE bound, …) |
| GET | `/api/recipes` | Catálogo de recipes del operator |
| GET | `/api/graph` | Receipt del grafo cerrado (executors↔recipes↔artifacts) |
| GET | `/api/executors` | Descriptores del registry |
| GET | `/api/models` | Modelos catalogados |
| POST | `/api/models/scan` | `catalog_local_models` sobre un root absoluto *dentro* del hub |
| GET | `/api/datasets` / POST import | Datasets content-addressed |
| GET/POST | `/api/jobs`, `/api/jobs/{id}`, cancel | Cola de jobs + estado + evidence receipts |
| POST | `/api/workflows/direct`, `behavioral-discovery` | Workflows directos allow-listed |
| GET | `/api/plasticity` | Consejo de plasticidad v2 (advisory; puede `available:false`) |

## Piezas internas

| Módulo | Rol |
|--------|-----|
| `control_plane.rs` (~4k líneas) | HTTP, jobs, discover/catalog modelos, datasets, workflows |
| `executor_registry.rs` (~1.7k) | Catálogo descriptivo de executors (estado, efecto, autoridad, superficies) |
| `graph.rs` (~320) | Topología cerrada; findings si hay dead-ends / autoridad inválida |
| `artifact.rs` (~860) | Vocabulario cerrado de artifact kinds/roles |
| `workspace.rs` (~320) | Workspace de operator (perfiles de modelo, etc.) |

## Grafo de producción

`compute_operator_graph()` compone `executor_catalog` + `recipe_catalog`. Un receipt sano reporta `passed: true` y `findings: []`. El único `production_authority` esperado es `adapter.bank`.

## Persistencia

Bajo el home del operator (p.ej. `runtime/tidex/operator/`):

- `models/by-sha/{model_id}.json`
- `jobs/by-sha/...`, `runs/by-sha/...`
- datasets `by-sha` + manifests

## Deuda conocida (no maquillar)

- Schemas wire aún `tidex.*`.
- Identidad de modelo: ver [model-identity-and-catalog](model-identity-and-catalog.md) (política cero-aliases / content-bound).

