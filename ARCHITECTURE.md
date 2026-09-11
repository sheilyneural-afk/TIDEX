# Arquitectura de CEREBRO3

## Visión general

CEREBRO3 está estructurado como un sistema de autoridad explícita. La capa operativa principal está en Rust, mientras que los backends de modelos o workers externos solo pueden ejecutar operaciones acotadas. El sistema se diseña para no inventar capacidad, ni decidir promociones sin evidencia observada.

## Límites de evidencia y responsabilidades

La arquitectura del proyecto debe leerse en capas verticales, no como un único nivel de autoridad homogéneo:

- autoridad y gobernanza: Rust, con KnowledgeEngine, AdapterBank y la escalera viva como coordinación real;
- ejecución acotada: runtime cross-model, workers externos y modelos reales que responden bajo protocolo y validación;
- experimentación y evaluación: scripts de benchmark, transfer experiments, synthetic data y rutas diagnósticas que sirven a investigación y análisis, pero no sustituyen la autoridad de producción.

El punto importante es que la arquitectura no pretende ocultar estas diferencias. Las pruebas experimentales aportan señales, pero la promoción real del sistema sigue atada a la capa de autoridad, hashes, receipts y validación de identidad.

## Capa de autoridad

La autoridad del sistema reposa en tres piezas fundamentales:

- `KnowledgeEngine`: decide planes, valida evidencia y emite receipts.
- `AdapterBank`: resuelve, autentica, activa y revoca adaptadores.
- `AdaptiveStaircase`: coordina pasos reales del ciclo de aprendizaje/ejecución.

Estas capas no son equivalentes entre sí. La autoridad no es distribuida ni delegada a Python ni a backends de inferencia.

```text
Cliente / operador
        |
        v
KnowledgeEngine
        |
        +--> valida identidad y evidencia
        +--> autentica resolución
        +--> emite receipts y autorizaciones
        |
        v
AdapterBank
        |
        +--> resuelve adapter
        +--> activa / revoca / rollback
        |
        v
AdaptiveStaircase
        |
        +--> ejecuta paso real
        +--> verifica proof / digest / provenance
        +--> persiste resultado
```

## Modelo temporal y de ejecución

El flujo estándar es:

1. `KnowledgeEngine` produce un plan.
2. El sistema resuelve un backend o adapter válido.
3. Se autentica la resolución y se valida su identidad.
4. Se ejecuta la operación real, con observabilidad y hashes.
5. Se verifica la salida y la evidencia.
6. Se emite un receipt autenticado.
7. El paso queda registrado como evidencia real o se descarta fail-closed.

Si falta cualquiera de estos pasos, el flujo se corta y la decisión no puede avanzar.

## Cross-model runtime

La carpeta `src/cross_model` no es un "segundo cerebro". Es una capa de evidencia y ejecución real sobre los backends existentes.

### Backends soportados

- Ollama: inferencia conductual real.
- Candle: carga real de pesos y ejecución física del modelo.
- HF Transformers: inferencia y extracción de activaciones reales con worker local persistente.

Cada backend se clasifica por capacidades reales:

- `BehavioralInference`
- `InternalActivations`
- `ActivationIntervention`

No se admite un backend que declare más capacidades de las que puede sostener físicamente.

## Python worker

El worker de Python en `src/cross_model/runtime/hf_worker.py` es un ejecutor acotado. Su papel es:

- cargar el modelo local;
- producir texto según política de generación;
- devolver activaciones de un layer concreto;
- instalar y limpiar steering real;
- responder mediante un protocolo JSON-line con hashes y bindings.

No puede emitir autorización, ni decidir promoción, ni actuar como fuente de verdad del sistema.

## Evidencia y receipts

Toda operación que modifique estado, realice intervención o altere la capa de adaptación debe ser verificable con:

- identidad del modelo o adapter;
- hash del payload o snapshot;
- semántica de operación;
- provenance de la ejecución;
- emission de receipt autenticado desde `KnowledgeEngine` o la autoridad equivalente.

Esto evita admitir métricas o activaciones que no tienen origen real.

## Límites de la arquitectura

CEREBRO3 no afirma:

- que un steering o un cambio interno implique mejora funcional útil;
- que un backend externo pueda autorizar producción por sí solo;
- que la activación automática del sistema sea segura o válida sin gate de promoción;
- que un ajuste de activación sustituye la evidencia real del comportamiento.

La arquitectura es de control y evidencia, no de promesas sin prueba.

## Información de referencia

- [README.md](README.md)
- [README_CROSS_MODEL.md](README_CROSS_MODEL.md)
- [SYSTEM_STATUS.md](SYSTEM_STATUS.md)
- [IMPLEMENTATION.md](IMPLEMENTATION.md)
