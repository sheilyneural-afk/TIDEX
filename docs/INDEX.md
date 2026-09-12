# Índice de documentación TIDE-X

Punto de entrada único. Si un doc de `directories/` o `systems/` parece corto o vago, es un defecto: debe explicar propósito, flujo e invariantes.

**Ubicación:** `docs/INDEX.md`  
**Relacionado:** [ARCHITECTURE](ARCHITECTURE.md) · [SYSTEM_STATUS](SYSTEM_STATUS.md)

## Cómo usar este índice

1. Visión → `ARCHITECTURE.md` + `SYSTEM_STATUS.md`
2. “¿Dónde está X en disco?” → `directories/`
3. “¿Cómo funciona el flujo Y?” → `systems/`
4. Fórmulas / auditorías → secciones al final

## Visión / maestros

| Doc | Propósito |
|-----|-----------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Capas autoridad / workflow / evidencia / experimental |
| [SYSTEM_STATUS.md](SYSTEM_STATUS.md) | Estado honesto del checkout |
| [IMPLEMENTATION.md](IMPLEMENTATION.md) | Reglas de implementación |
| [QUALITY_AND_TESTING.md](QUALITY_AND_TESTING.md) | Modelo de pruebas y evidencia |
| [README_CROSS_MODEL.md](README_CROSS_MODEL.md) | Feature cross-model |
| [../README.md](../README.md) | README raíz |
| [TIDEX_TWO_LEARNINGS_AND_CLOSED_LINKS.md](TIDEX_TWO_LEARNINGS_AND_CLOSED_LINKS.md) | Dos aprendizajes; A ❌ / B 🟡→🟢 decisor; Paso 1 CLOSED; Paso 2A @ bc531d7; Paso 3 NextAction |

## Sistemas (flujos transversales)

| Doc | Cubre |
|-----|-------|
| [systems/authority-and-receipts.md](systems/authority-and-receipts.md) | KnowledgeEngine, AdapterBank, digests, fail-closed |
| [systems/operator-control-plane.md](systems/operator-control-plane.md) | HTTP :8793, jobs, recipes, graph |
| [systems/model-identity-and-catalog.md](systems/model-identity-and-catalog.md) | `model_id`, scan HF, política cero-aliases |
| [systems/learning-and-plasticity.md](systems/learning-and-plasticity.md) | Learning + plasticidad advisory |
| [systems/receiver-and-materialization.md](systems/receiver-and-materialization.md) | Receiver + materializers |
| [systems/cross-model-runtime.md](systems/cross-model-runtime.md) | HF/NNsight feature |
| [systems/quality-gates-and-ci.md](systems/quality-gates-and-ci.md) | Gates, CI, fuzz |
| [systems/web-console.md](systems/web-console.md) | UI estática |
| [systems/evidence-cas-runtime-store.md](systems/evidence-cas-runtime-store.md) | CAS / `runtime/` |

## Directorios

| Doc | Cubre |
|------|--------|
| [directories/root.md](directories/root.md) | Raíz del crate |
| [directories/src.md](directories/src.md) | Mapa `src/` |
| [directories/src-foundation.md](directories/src-foundation.md) | `src/foundation` |
| [directories/src-knowledge.md](directories/src-knowledge.md) | `src/knowledge` |
| [directories/src-governance.md](directories/src-governance.md) | `src/governance` |
| [directories/src-operator.md](directories/src-operator.md) | `src/operator` |
| [directories/src-learning.md](directories/src-learning.md) | `src/learning` |
| [directories/src-receiver.md](directories/src-receiver.md) | `src/receiver` |
| [directories/src-materialization.md](directories/src-materialization.md) | `src/materialization` |
| [directories/src-capability.md](directories/src-capability.md) | `src/capability` |
| [directories/src-analysis.md](directories/src-analysis.md) | `src/analysis` |
| [directories/src-engine.md](directories/src-engine.md) | `src/engine` |
| [directories/src-runtime.md](directories/src-runtime.md) | `src/runtime` (módulo) |
| [directories/src-cross_model.md](directories/src-cross_model.md) | `src/cross_model` |
| [directories/src-bin.md](directories/src-bin.md) | `src/bin` |
| [directories/config.md](directories/config.md) | `config/` |
| [directories/quality.md](directories/quality.md) | `quality/` |
| [directories/tests.md](directories/tests.md) | `tests/` |
| [directories/fuzz.md](directories/fuzz.md) | `fuzz/` |
| [directories/web-console.md](directories/web-console.md) | `web-console/` |
| [directories/github.md](directories/github.md) | `.github/` |
| [directories/runtime.md](directories/runtime.md) | `runtime/` disco |
| [directories/collected_receipts.md](directories/collected_receipts.md) | `collected_receipts/` |
| [directories/docs.md](directories/docs.md) | Este árbol |

## Fórmulas

| Doc | Contenido |
|-----|-----------|
| [FORMULAS_CANONICAS.md](FORMULAS_CANONICAS.md) | Fórmulas canónicas con anclas |
| [FORMULAS_INDEX.md](FORMULAS_INDEX.md) | Índice de funciones/algoritmos |
| [FORMULAS_ALGORITMICAS.md](FORMULAS_ALGORITMICAS.md) | Catálogo exhaustivo |

## Auditorías

| Doc | Contenido |
|-----|-----------|
| [AUDITORIA_COMPLETA.md](AUDITORIA_COMPLETA.md) | Informe maestro |
| [AUDITORIA_FUNCIONAL.md](AUDITORIA_FUNCIONAL.md) | Auditoría funcional |
| [CONTRA_AUDITORIA_FUNCIONAL.md](CONTRA_AUDITORIA_FUNCIONAL.md) | Contra-auditoría |

## Diseño

| Doc | Contenido |
|-----|-----------|
| [design/adaptive_staircase.md](design/adaptive_staircase.md) | Escalera adaptativa / living staircase |
