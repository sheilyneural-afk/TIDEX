# CEREBRO3

CEREBRO3 es un proyecto Rust orientado a la ejecución real, la evidencia verificable y la autoridad explícita. El sistema no busca simular inteligencia ni delegar la verdad a modelos externos. Su objetivo es mantener un ciclo gobernado en el que cada paso sólo avanza si se demuestra que es necesario, útil, autentificado y verificable.

## 0. Estado honesto y límites de evidencia

## Calidad profesional del repositorio

CEREBRO3 mantiene una base de ingeniería real y reproducible:

- `Makefile` con el pipeline mínimo de validación: `fmt`, `check`, `test`, `integration`, `ci`.
- `rustfmt.toml` y `clippy.toml` para un estilo y una calidad de código coherentes.
- `.github/workflows/ci.yml` para ejecutar compilación y pruebas en cada cambio.
- `docs/QUALITY_AND_TESTING.md` como referencia del modelo de pruebas y la política de evidencia.

Esto evita que el proyecto dependa de una UI bonita, nombres atractivos o una narración de laboratorio sin verificación real.

La arquitectura formal del sistema y la separación de dominios están documentadas en [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), con el propósito de dejar explícito qué es autoridad, qué es workflow y qué es evidencia verificable.

La documentación de CEREBRO3 distingue tres capas con reglas distintas:

- capa de producción o autoridad: módulos en Rust con validación, receipts y fail-closed;
- capa de ejecución acotada: workers externos, backends reales y modelos que ejecutan operaciones concretas pero no deciden autoridad;
- capa experimental o evaluativa: benchmarks, transfer experiments, synthetic data y rutas de diagnóstico que no equivalen a producción ni a autoridad final.

Esto no es un detalle burocrático: es la diferencia entre “lo que está realmente probado” y “lo que es un experimento útil o un punto de investigación”.

Lo que sí está verificado con evidencia real en el repositorio:

- la base Rust del sistema compila y pasa la suite de tests de librería;
- la autoridad central sigue siendo Rust;
- la ejecución de modelos externos queda acotada bajo validación explícita;
- el sistema exige receipts, hashes, identidad y fail-closed cuando la evidencia no encaja.

Lo que todavía no está demostrado como una afirmación universal del proyecto:

- que cualquier backend externo produzca transferencia útil de capacidad sin límites;
- que la activación o steering automático implique mejora funcional real;
- que todas las rutas cross-model sean equivalentes en calidad y seguridad;
- que las métricas experimentales sean automáticamente “promocionables” a producción.

La política del proyecto es clara: no se promociona una hipótesis como hechos reales sin origen verificable, hash, contrato y evidencia.

## 1. La idea central: no hay “segundo cerebro”

CEREBRO3 no se organiza como un sistema paralelo que decide por sí mismo. Su patrón operativo es:

- una capacidad detecta una necesidad;
- la autoridad del sistema decide si esa necesidad es real;
- un executor concreto resuelve la acción correcta;
- la evidencia resultante se valida;
- el sistema emite un receipt y sólo entonces avanza o bloquea.

La escalera viva no reemplaza la autoridad. La escalera vive encima de la autoridad y la utiliza.

## 2. Autoridad real del sistema

La separación de roles del sistema es clara:

```text
KnowledgeEngine
  -> autoriza el plan
  -> revisa obligaciones, hipótesis y evidencia
  -> decide si una acción puede avanzar
  -> emite receipts y transiciones autorizadas

AdapterBank
  -> resuelve un backend o adapter real
  -> autentica la resolución
  -> activa, revoca y rollback cuando la evidencia lo exige

Living Staircase
  -> coordina necesidades, pasos y contención
  -> no sustituye la autoridad
  -> no inventa evidencia ni promueve capacidades sin respaldo

Cross-model runtime
  -> ejecuta backend real
  -> observa activaciones, métricas y firma del runtime
  -> devuelve resultados con hashing y contract validation
```

No existe un segundo poder de decisión. La autoridad final sigue siendo Rust y la capa de gobernanza legítima del sistema.

## 3. Qué es la escalera viva en CEREBRO3

La escalera viva es la estructura recurrente que hace que todas las capacidades entren en un mismo ciclo:

```text
Need
  -> Detectar brecha / insuficiencia / evidencia pendiente
  -> Resolver un executor válido
  -> Ejecutar acción real con evidencia
  -> Validar provenance, identidad, contrato y hash
  -> Decidir: commit / refine / prune / block
  -> Persistir receipt y avanzar o cerrar
```

Esto es la arquitectura que unifica lo que ya existe en el árbol: autoridad, plasticidad, numerics, residency, receiver compiler, materialization, promotion gate y adapter lifecycle.

## 4. Qué debe estar conectado al mismo ciclo

La escalera viva debe coordinar módulos ya existentes, no reemplazarlos. Su papel es canalizar cada capacidad en una dinámica común.

### 4.1 KnowledgeEngine

Es la autoridad epistemológica central. Debe seguir siendo la fuente de:

- planificación,
- obligaciones,
- hipótesis,
- claims,
- evidence,
- receipts,
- decisiones terminales.

### 4.2 ResidencyDecision

Este motor debe derivar residencia, no aceptar un valor impreso por el caller. La decisión debe ser:

- Software,
- Weights,
- Hybrid,
- Unknown.

Si vuelve Unknown, la escalera crea automáticamente un nuevo micro-escalón de evidencia.

### 4.3 BrainEngine

Debe seguir funcionando como ejecutor neuronal y de consolidación, no como coordinador global. Su función final es:

- analizar,
- consolidar,
- recuperar transiciones incompletas,
- preparar memoria/learning sessions,
- sostener la capa de ejecución y consolidación.

### 4.4 NumericalEvolutionEngine

Debe entrar como executor especializado de un need de profundidad numérica. No es un sistema paralelo; es una vía de resolución para problemas donde la numerics requiere validación o evolución.

### 4.5 Plasticity

La plasticidad no debe gobernar el sistema. Debe ejecutar cambios acotados dentro del ciclo de la escalera:

- BCM,
- eligibility traces,
- neuromodulation,
- routing plasticity,
- content plasticity,
- PI controller,
- ELO.

Todos estos mecanismos son herramientas de adaptación, no autoridad.

### 4.6 UniversalPromotionGate

Debe mantenerse como gate canónico de promoción, pero no como sistema autónomo. La promoción es la etapa final del ciclo, no una decisión aislada.

### 4.7 AdapterBank

Debe seguir siendo la autoridad del lifecycle real de activación, revocación y rollback.

## 5. La relación correcta entre módulos

La relación final que más encaja con el árbol real es esta:

```text
Living Staircase
   -> Need detectors
   -> Executor registry
   -> KnowledgeEngine authority
   -> ResidencyDecision
   -> BrainEngine / Plasticity / NumericalEvolution
   -> Materialization / ReceiverCompiler / ShadowEvaluation
   -> UniversalPromotionGate
   -> AdapterBank lifecycle
   -> Evidence + Receipt + decision
```

No se trata de reescribir CEREBRO3 en una sola máquina. Se trata de hacer que todo el árbol entre en una misma dinámica de necesidad → ejecución → evidencia → contención → avance.

## 6. Reglas de diseño que no se deben romper

- ninguna capa ejecuta sin evidencia real;
- ninguna capa inventa autoridad;
- ninguna pieza puede promocionar un cambio sin validación y receipt;
- la escalera sólo orquesta; no sustituye a KnowledgeEngine ni a AdapterBank;
- la plasticidad modifica dentro de límites; no controla el sistema global;
- la memoria y la procedimental memory alimentan decisiones, pero no son autoridad;
- los bridges triviales deben reducirse porque añaden superficie sin aportar verdad.

## 7. TIDE-X Advanced Laboratory

El repositorio incluye una superficie de laboratorio real sobre las autoridades existentes. No reimplementa los algoritmos: ejecuta los mismos backends y contratos del proyecto.

Arranque recomendado desde la raíz del repositorio:

```bash
./tidex lab serve
```

La interfaz queda disponible únicamente en loopback:

```text
http://127.0.0.1:8793
```

El launcher `./tidex` es intencionado: el proyecto configura Cargo para construir fuera del checkout, bajo `/home/yo/Future/cerebro3-runtime/cargo-target`, por lo que no se debe asumir que exista `./target/debug/tidex`. El runtime persistente vive fuera del repo pero dentro de `Future`.

Funciones de alto nivel disponibles en el laboratorio:

- catálogo y selección de modelos HF locales;
- importación y catalogación de datasets con SHA-256 y provenance;
- evaluación conductual real de un LLM;
- descubrimiento comparativo multi-LLM;
- extracción de representaciones internas;
- instrumentación profunda con NNsight cuando el intérprete configurado la soporte;
- análisis SAE cuando NNsight + SAE Lens estén disponibles;
- análisis contrafactual;
- calibración de alineamiento A→B;
- experimento de transferencia por activation steering con baseline, intervención, restauración y evaluación separadas;
- acceso avanzado a las operaciones canónicas de receiver compiler, materialización, universality, promotion gate y AdapterBank.

CLI del laboratorio:

```bash
./tidex lab models scan "/home/yo/Future/cerebro3-runtime/llms/huggingface/hub"
./tidex lab models list
./tidex lab datasets list
./tidex lab dataset import <nombre> json <benchmark.json>
./tidex lab evaluate <model-id> <dataset-sha256>
./tidex lab discover <dataset-sha256> <model-id-1> <model-id-2> [model-id...]
```

Para usar un intérprete HF específico —por ejemplo uno que incluya NNsight y SAE Lens— se configura explícitamente:

```bash
export TIDEX_HF_PYTHON=/ruta/absoluta/al/python
./tidex lab serve
```

Los resultados del Lab quedan bajo `/home/yo/Future/cerebro3-runtime/tidex/lab/` por defecto. Un resultado de laboratorio nunca concede por sí mismo autoridad de activación productiva.

## 8. Estado verificado del proyecto

La validación de ingeniería debe ejecutarse con los targets/features que correspondan a la superficie probada. El laboratorio se valida con `--all-features --all-targets`; las pruebas de LLM además requieren ejecución real del backend seleccionado.

## 9. Estructura del repositorio

- src/ — runtime principal, autoridad y contratos.
- src/cross_model/ — backends reales y lógica de plasticidad/cross-model.
- src/adaptive_staircase.rs — coordinación de la escalera viva.
- src/knowledge_engine.rs — autoridad epistemológica del sistema.
- src/residency_decision.rs — derivación de residencia.
- src/numerical_evolution.rs — evolución numérica acotada.
- src/adapter_bank.rs — autenticación y lifecycle de adaptadores.
- src/cross_model/runtime/hf_worker.py — executor acotado, no autoridad.
- design/ — arquitectura y documentos operativos.
- quality/ — evidencia y puertas de validación.

## 10. Política documental

La documentación debe reflejar el estado real del código. Si una capacidad no puede justificarse con evidencia, identidad, hashes, contratos y validación auténtica, no debe presentarse como parte del sistema real.

## 11. Documentación de referencia

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [IMPLEMENTATION.md](IMPLEMENTATION.md)
- [SYSTEM_STATUS.md](SYSTEM_STATUS.md)
- [README_CROSS_MODEL.md](README_CROSS_MODEL.md)
- [design/adaptive_staircase.md](design/adaptive_staircase.md)

La conclusión operativa es más precisa: CEREBRO3 ya dispone de múltiples autoridades, ejecutores y mecanismos de evidencia reales, pero cada afirmación de capacidad debe seguir demostrarse en el alcance concreto donde se usa. La integración debe unificar esas piezas bajo una misma dinámica sin convertir la escalera en una segunda autoridad.


### Executor Registry multi-eje

`./tidex executors` devuelve el catálogo canónico de ejecutores. Todos los descriptores registrados tienen `state=operational` en el sentido de contrato implementado y rastreable, pero la madurez se separa en campos independientes: `runtime_status`, `workflow_status`, `evidence_status`, `maturity`, `production_authority` y `actionable_now`. No se debe usar `state` como sinónimo de autorización productiva.

La única autoridad de ciclo de vida productivo para adaptadores es `adapter.bank`. Los ejecutores candidate/advisory pueden producir evidencia, candidatos o señales, pero no activan producción por sí solos.
