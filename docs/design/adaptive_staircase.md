# Escalera viva (Adaptive Staircase)

Este archivo es una nota de diseño. No describe un módulo que exista.

En el código no hay `src/adaptive_staircase.rs`, ni el tipo `AdaptiveStaircase`, ni `AdaptiveContainmentGraph`, ni un `StepReceipt` de escalera. Lo implementado es `KnowledgeEngine::living_staircase` → `LivingStaircaseProjection`: una vista de solo lectura de las obligaciones ya autenticadas. Nadie la usa como orquestador del árbol.

El ciclo necesidad → executor → evidencia → validación → receipt → avance o bloqueo es la dirección de integración. Presentarlo como si ya unificara el runtime era un error documental.

## 1. Principio rector

La escalera viva no decide la verdad por sí misma. La verdad la produce y la valida la autoridad legítima del sistema. Por eso:

- KnowledgeEngine sigue siendo la autoridad epistemológica.
- AdapterBank sigue siendo la autoridad de lifecycle real.
- si la escalera se implementa como capa de coordinación, sólo orquesta transiciones; no decide verdad. Hoy esa capa no existe: hay una proyección de obligaciones.

Esto evita dos errores graves:

1. inventar capacidades sin evidencia;
2. convertir un backend o un worker en autoridad de promoción.

## 2. El ciclo de la escalera

```text
Need / gap / uncertainty
   |
   v
KnowledgeEngine::plan
   |
   v
ExecutorRegistry -> resolve executor
   |
   v
execute real action
   |
   v
collect provenance + schema + hash + evidence
   |
   v
verify / refine / prune / block
   |
   v
issue receipt and persist decision
```

La escalera no reemplaza la autoridad; la usa como punto de decisión.

## 3. Entidades del ciclo

### Lo que sí existe

- `LivingStaircaseProjection` / `LivingStaircaseStep` en `src/knowledge/knowledge_engine.rs`: proyección de obligaciones (open / satisfied / blocked), profundidad autenticada y `PlanningDecision`.
- receipts de autoridad en KnowledgeEngine (no un `StepReceipt` de escalera).
- `ExecutorDescriptor` en `src/operator/executor_registry.rs`: catálogo declarado. `new` fuerza `implementation_status = Implemented`; hay estados `ArchitectureOnly` y un solo `production_authority` (`adapter.bank`).

### Lo que este diseño pide y el código no tiene

- `StepReceipt` de escalera con gain/risk/novelty/preservation y commit/refine/prune/block.
- `AdaptiveContainmentGraph` como grafo de contención y dependencias de pasos.
- creación automática de un micro-escalón cuando `ResidencyDecision` vuelve Unknown.

## 4. Cómo encajan los módulos actuales

### KnowledgeEngine

Es el centro epistemológico del sistema. Debe seguir siendo la fuente de:

- claims,
- obligations,
- hypotheses,
- plans,
- invocations,
- evidence,
- receipts,
- terminal state decisions.

### BrainEngine

Debe seguir siendo ejecutor neuronal y consolidación. No es el coordinador global.

### ResidencyDecision

Debe derivar la residencia correcta de una capacidad:

- Software,
- Weights,
- Hybrid,
- Unknown.

Si la residencia es Unknown, este diseño pide un micro-escalón de evidencia. El código de `ResidencyDecision` no lo crea a través de `living_staircase`.

### Plasticity

Es un conjunto de mecanismos de adaptación. No gobierna el sistema. Ejecuta cambios dentro de límites definidos y con evidencia de validez.

### NumericalEvolution

Es un executor especializado. Cuando la necesidad es resolver un problema con mayor profundidad o con más estabilidad numérica, la escalera puede invocarlo.

### UniversalPromotionGate

Es la compuerta final de promoción, no una autoridad separada. La promoción debe producirse sólo al terminar el ciclo de evidencia, validación y lifecycle.

### AdapterBank

Es la última autoridad del lifecycle real: activación, revocación, rollback, activación autorizada.

## 5. Relación objetivo (no implementada como orquestador)

La arquitectura a la que apunta esta nota es:

```text
Need
  -> Living Staircase
      -> KnowledgeEngine (authority and plan)
      -> ExecutorRegistry (specialized executors)
      -> ResidencyDecision
      -> BrainEngine / Plasticity / NumericalEvolution
      -> ReceiverCompiler / Materialization / ShadowEvaluation
      -> UniversalPromotionGate
      -> AdapterBank
      -> Evidence + Receipt + Decision
```

Ese diagrama es el objetivo. En el árbol actual KnowledgeEngine y AdapterBank operan por su cuenta; la proyección no dispara executors, plasticidad, numerics ni el promotion gate.

## 6. Principios de contención

La escalera debe cumplir siempre estas reglas:

1. no puede producir decisión sin autoridad;
2. no puede aceptar métricas o activaciones sin provenance;
3. no puede promulgar una capacidad sin validación;
4. no puede bloquear la última autoridad del ciclo;
5. no puede avanzar si el paso no queda registrado con receipt;
6. no puede sustituir a AdapterBank como autoridad final de lifecycle.

## 7. Qué no debe hacerse

- no crear una segunda autoridad oculta;
- no convertir Python o cualquier backend en autoridad de promoción;
- no aceptar un vector, activación o nombre de modelo como evidencia completa;
- no dejar pasos sin contención ni receipt;
- no acumular bridges triviales que sólo repiten llamadas sin aportar autoridad;
- no permitir que la plasticidad decidiera por todo el sistema sin contexto.

## 8. Estado actual del mecanismo

Lo que el código sostiene:

- KnowledgeEngine es una autoridad real y deriva `LivingStaircaseProjection`;
- AdapterBank es el punto legítimo de activación y rollback;
- Plasticity y NumericalEvolution existen como motores; varios entran al catálogo como `ArchitectureOnly` o advisory.

Lo que el código no sostiene:

- AdaptiveContainmentGraph;
- una capa `AdaptiveStaircase` que unifique el flujo;
- que “ya es la capa que puede unificar” el árbol.

Lo que falta no es inventar más nombres. Falta cablear módulos existentes al ciclo de gobernanza sin convertir la proyección en una segunda autoridad.

## 9. Conclusión

La escalera viva, si se implementa, debe ser la forma de operar del sistema, no otra autoridad. TIDE-X ya tiene autoridades y ejecutores. La tarea pendiente es ordenar el árbol bajo una sola dinámica:

```text
necesidad -> ejecución -> evidencia -> contención -> avance
```

y cerrar el ciclo sin mover la autoridad de su sitio.

Referencias:

- [README.md](../README.md)
- [ARCHITECTURE.md](../docs/ARCHITECTURE.md)
- [SYSTEM_STATUS.md](../docs/SYSTEM_STATUS.md)
- [IMPLEMENTATION.md](../docs/IMPLEMENTATION.md)
