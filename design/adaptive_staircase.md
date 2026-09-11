# Escalera viva (Adaptive Staircase)

La escalera viva no es un segundo cerebro ni un sistema paralelo. Es la capa de coordinación que hace que todas las capacidades del sistema entren en un mismo ciclo: necesidad → executor → evidencia → validación → receipt → avance o bloqueo.

## 1. Principio rector

La escalera viva no decide la verdad por sí misma. La verdad la produce y la valida la autoridad legítima del sistema. Por eso:

- KnowledgeEngine sigue siendo la autoridad epistemológica.
- AdapterBank sigue siendo la autoridad de lifecycle real.
- la escalera viva sólo orquesta la transición entre estados.

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

## 3. Entidades reales del ciclo

### StepReceipt

Registra:

- identidad del paso;
- dependencia del paso anterior;
- digest del invocation o plan;
- digest del action receipt;
- digest del resolution manifest;
- métricas observadas: gain, risk, novelty, preservation;
- decisión final: commit, refine, prune, block.

### Decision

Es la decisión final de un paso ejecutado. No es un texto libre. Debe derivarse de evidencia verificable y de la autoridad del sistema.

### ExecutorDescriptor

Describe qué executor puede atender qué tipo de necesidad. El registro de executors permite que cada módulo especializado entre por una interfaz común sin crear otra autoridad global.

### AdaptiveContainmentGraph

Es la estructura de contención y dependencias. Mantiene el orden de ejecución, el riesgo de cada paso, las relaciones entre transiciones y la trazabilidad de la evolución del sistema.

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

Si la residencia es Unknown, la escalera genera un micro-escalón de evidencia.

### Plasticity

Es un conjunto de mecanismos de adaptación. No gobierna el sistema. Ejecuta cambios dentro de límites definidos y con evidencia de validez.

### NumericalEvolution

Es un executor especializado. Cuando la necesidad es resolver un problema con mayor profundidad o con más estabilidad numérica, la escalera puede invocarlo.

### UniversalPromotionGate

Es la compuerta final de promoción, no una autoridad separada. La promoción debe producirse sólo al terminar el ciclo de evidencia, validación y lifecycle.

### AdapterBank

Es la última autoridad del lifecycle real: activación, revocación, rollback, activación autorizada.

## 5. Relación final correcta

La arquitectura que mejor encaja con el árbol real es:

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

La escalera no reemplaza al sistema. La escalera hace que todo el sistema entre en el mismo ciclo operativo.

## 6. Principios de contención

La escalera debe cumplir siempre estas reglas:

1. no puede producir decisión sin autoridad;
2. no puede aceptar métricas o activaciones sin provenance;
3. no puede promulgar una capacidad sin validación;
4. no puede bloquear la última autoridad del ciclo;
5. no puede avanzar si el paso no queda registrado con receipt;
6. no puede sustituir a AdapterBank como autoridad final de lifecycle.

## 7. Qué no debe hacerse

- no crear un “segundo cerebro” oculto;
- no convertir Python o cualquier backend en autoridad de promoción;
- no aceptar un vector, activación o nombre de modelo como evidencia completa;
- no dejar pasos sin contención ni receipt;
- no acumular bridges triviales que sólo repiten llamadas sin aportar autoridad;
- no permitir que la plasticidad decidiera por todo el sistema sin contexto.

## 8. Estado actual del mecanismo

El proyecto ya tiene la base conceptual y técnica necesaria para esta arquitectura:

- KnowledgeEngine ya es una autoridad real;
- AdaptiveContainmentGraph ya define la estructura del paso;
- AdapterBank ya es el punto legítimo de activación y rollback;
- Plasticity y NumericalEvolution ya son motores especializados;
- la escalera viva ya es la capa que puede unificar el flujo.

Lo que falta no es inventar más algoritmos, sino dejar que todos esos módulos entren en el mismo ciclo de gobernanza.

## 9. Conclusión

La escalera viva debe ser la forma de operar del sistema, no otra autoridad. CEREBRO3 ya tiene los recursos necesarios. La tarea final es ordenar el árbol bajo una sola dinámica:

```text
necesidad -> ejecución -> evidencia -> contención -> avance
```

y cerrar el ciclo sin mover la autoridad de su sitio.

Referencias:

- [README.md](../README.md)
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [SYSTEM_STATUS.md](../SYSTEM_STATUS.md)
- [IMPLEMENTATION.md](../IMPLEMENTATION.md)
