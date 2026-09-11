# TIDE-X

TIDE-X es un proyecto Rust orientado a la ejecución real, la evidencia verificable y la autoridad explícita. El sistema no busca simular inteligencia ni delegar la verdad a modelos externos. Su objetivo es mantener un ciclo gobernado en el que cada paso sólo avanza si se demuestra que es necesario, útil, autentificado y verificable.

## 0. Estado honesto y límites de evidencia

## Calidad profesional del repositorio

TIDE-X mantiene una base de ingeniería real y reproducible:

- `Makefile` con el pipeline mínimo de validación: `fmt`, `check`, `test`, `integration`, `ci`.
- `rustfmt.toml` y `clippy.toml` para un estilo y una calidad de código coherentes.
- `.github/workflows/ci.yml` para ejecutar compilación y pruebas en cada cambio.
- `docs/QUALITY_AND_TESTING.md` como referencia del modelo de pruebas y la política de evidencia.

Esto evita que el proyecto dependa de una UI bonita, nombres atractivos o una narración de producto sin verificación real.

La arquitectura formal del sistema y la separación de dominios están documentadas en [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), con el propósito de dejar explícito qué es autoridad, qué es workflow y qué es evidencia verificable.

La documentación de TIDE-X distingue tres capas con reglas distintas:

- capa de producción o autoridad: módulos en Rust con validación, receipts y fail-closed;
- capa de ejecución acotada: workers externos, backends reales y modelos que ejecutan operaciones concretas pero no deciden autoridad;
- capa experimental o evaluativa: benchmarks, transfer experiments, synthetic data y rutas de diagnóstico que no equivalen a producción ni a autoridad final.

Esto no es un detalle burocrático: es la diferencia entre “lo que está realmente probado” y “lo que es un experimento útil o un punto de investigación”.

Lo que el repositorio sí sostiene en código (no es un recuento de `cargo test` re-ejecutado en esta nota):

- la autoridad central está en Rust, con receipts, hashes, identidad y fail-closed;
- la ejecución de modelos externos queda acotada bajo validación explícita y no decide promoción;
- Makefile y GitHub Actions definen el pipeline `fmt` / `check` / `test --lib` / integración / contratos de configuración productiva / compilación del crate de fuzzing.

Lo que todavía no está demostrado como una afirmación universal del proyecto:

- que cualquier backend externo produzca transferencia útil de capacidad sin límites;
- que la activación o steering automático implique mejora funcional real;
- que todas las rutas cross-model sean equivalentes en calidad y seguridad;
- que las métricas experimentales sean automáticamente “promocionables” a producción.

La política del proyecto es clara: no se promociona una hipótesis como hechos reales sin origen verificable, hash, contrato y evidencia.

El código fuente está organizado por dominio bajo `src/` (`foundation`, `knowledge`, `governance`, `operator`, `runtime`, `engine`, `learning`, `receiver`, `materialization`, `analysis`, `capability` y, con feature, `cross_model`). No hay un módulo `src/adaptive_staircase.rs`.

## 1. La idea central: no hay una segunda autoridad

TIDE-X no se organiza como un sistema paralelo que decide por sí mismo. El patrón que el código sí sostiene hoy es:

- KnowledgeEngine planifica, valida obligaciones y emite receipts;
- AdapterBank autentica y muta el ciclo de vida de adaptadores;
- la interfaz y los backends ejecutan trabajo acotado y no conceden autoridad productiva.

La “escalera viva” no es un coordinador separado. En código es `KnowledgeEngine::living_staircase`: una proyección de solo lectura sobre las obligaciones ya autenticadas. No detecta necesidades, no resuelve executors y no ejecuta el ciclo.

## 2. Autoridad real del sistema

La separación de roles implementada es esta:

```text
KnowledgeEngine  (src/knowledge/knowledge_engine.rs)
  -> autoriza el plan
  -> revisa obligaciones, hipótesis y evidencia
  -> decide si una acción puede avanzar
  -> emite receipts y transiciones autorizadas
  -> deriva LivingStaircaseProjection (vista, no orquestador)

AdapterBank  (src/governance/adapter_bank.rs)
  -> resuelve un backend o adapter real
  -> autentica la resolución
  -> activa, revoca y rollback cuando la evidencia lo exige
  -> único descriptor con production_authority=true (adapter.bank)

Cross-model runtime  (src/cross_model/, feature cross-model-plasticity)
  -> ejecuta backend real
  -> observa activaciones, métricas y firma del runtime
  -> devuelve resultados con hashing y validación de contrato
  -> no decide promoción ni activación productiva
```

`KnowledgeEngine::open` falla cerrado con `authority_instance_required`. El arranque real es `open_with_authority_instance`. No existe un segundo poder de decisión.

## 3. Qué es la escalera viva en el código

`LivingStaircaseProjection` proyecta el estado de conocimiento ya gobernado: obligaciones, profundidades autenticadas, estado open/satisfied/blocked y la siguiente `PlanningDecision`. No persiste un grafo propio y nadie fuera de `KnowledgeEngine` la invoca como orquestador.

El ciclo operativo deseado (necesidad → executor → evidencia → commit/refine/prune/block) es la dirección de integración descrita en [docs/design/adaptive_staircase.md](docs/design/adaptive_staircase.md). No es un runtime que ya unifique plasticidad, numerics, residency, receiver compiler, materialization y promotion gate.

## 4. Módulos que existen y cómo se relacionan

Estos módulos existen. No están cableados por una escalera coordinadora.

### 4.1 KnowledgeEngine

Autoridad epistemológica central: planificación, obligaciones, hipótesis, claims, evidence, receipts y decisiones terminales.

### 4.2 ResidencyDecision

`src/governance/residency_decision.rs` deriva residencia (Software, Weights, Hybrid, Unknown) y no acepta un valor impreso por el caller. Si el resultado es Unknown, el código no crea automáticamente un micro-escalón vía la proyección de la escalera.

### 4.3 BrainEngine

`src/engine/` es ejecutor de consolidación, no coordinador global.

### 4.4 NumericalEvolutionEngine

`src/learning/numerical_evolution.rs` es un motor especializado. El catálogo lo registra; no lo dispara una escalera viva.

### 4.5 Plasticity

Los mecanismos (BCM, eligibility traces, neuromodulation, routing/content plasticity, PI, ELO) viven en la librería. Varios descriptores del catálogo están en `ArchitectureOnly`: existen como diseño o código de análisis, no como autoridad operativa. No gobiernan el sistema.

### 4.6 UniversalPromotionGate

`src/governance/universal_promotion_gate.rs` es el gate canónico de promoción. No es autónomo y no se dispara desde la proyección de la escalera.

### 4.7 AdapterBank

Autoridad del lifecycle real de activación, revocación y rollback. Un receipt de la interfaz con `authorizes_production: false` no sustituye esa autoridad.

## 5. Relación real entre módulos

Hoy la relación implementada es esta:

```text
KnowledgeEngine
   -> obligaciones, planes, receipts
   -> living_staircase()  (proyección de esas obligaciones)

AdapterBank
   -> lifecycle productivo de adaptadores

ExecutorRegistry
   -> catálogo declarado (./tidex executors)
   -> no ejecuta; describe estado, madurez y superficies

Lab / backends / BrainEngine / Plasticity / NumericalEvolution
   -> ejecución acotada o análisis
   -> no conceden autoridad productiva
```

La integración bajo una sola dinámica necesidad → ejecución → evidencia → contención → avance sigue siendo trabajo pendiente, no un hecho del árbol.

## 6. Reglas de diseño que no se deben romper

- ninguna capa ejecuta sin evidencia real;
- ninguna capa inventa autoridad;
- ninguna pieza puede promocionar un cambio sin validación y receipt;
- la proyección de la escalera no sustituye a KnowledgeEngine ni a AdapterBank, ni orquesta el árbol;
- la plasticidad modifica dentro de límites; no controla el sistema global;
- la memoria y la procedural memory alimentan decisiones, pero no son autoridad;
- los bridges triviales deben reducirse porque añaden superficie sin aportar verdad.

## 7. Interfaz de TIDE-X

TIDE-X no tiene un laboratorio aparte. La UI y el CLI son la interfaz del cerebro: ejecutan los mismos backends y contratos del proyecto.

Arranque desde la raíz del repositorio:

```bash
./quality/bootstrap-runtime.sh   # venv HF + SmolLM2-135M y Instruct en el hub
./tidex serve
```

La interfaz queda disponible únicamente en loopback:

```text
http://127.0.0.1:8793
```

El launcher `./tidex` es intencionado. `.cargo/config.toml` fija `build.target-dir` a `../.cache/tidex/cargo-target`, fuera del checkout y separado del runtime operativo. No se debe asumir que exista `./target/debug/tidex`. El runtime persistente queda en `$ROOT/runtime` salvo que se defina `TIDEX_RUNTIME_ROOT`.

- hogar del cerebro: `$TIDEX_HOME` → `runtime/tidex/`
- pesos HF: `runtime/llms/huggingface/hub/`
- catálogo, jobs y receipts: `$TIDEX_HOME/operator/`

Funciones de la interfaz:

- catálogo y selección de modelos HF locales;
- importación y catalogación de datasets con SHA-256 y provenance;
- evaluación conductual real de un LLM;
- descubrimiento comparativo multi-LLM;
- extracción de representaciones internas;
- instrumentación profunda con NNsight cuando el intérprete configurado la soporte;
- análisis SAE sobre un diccionario local ligado (`sae.safetensors` + `config.json`); no entrena y no descarga releases del Hub;
- análisis contrafactual;
- calibración de alineamiento A→B;
- experimento de transferencia por activation steering con baseline, intervención, restauración y evaluación separadas;
- acceso a receiver compiler, materialización, universality, promotion gate y AdapterBank.

CLI:

```bash
./tidex models scan "$PWD/runtime/llms/huggingface/hub"
./tidex models list
./tidex datasets list
./tidex dataset import <nombre> json <benchmark.json>
./tidex evaluate <model-id> <dataset-sha256>
./tidex discover <dataset-sha256> <model-id-1> <model-id-2> [model-id...]
```

Para usar un intérprete HF específico —el certificado en `src/cross_model/runtime/hf-runtime.lock.txt`— se configura explícitamente:

```bash
export TIDEX_HF_PYTHON=/ruta/absoluta/al/python
./tidex serve
```

Un receipt de esta interfaz marca `authorizes_production: false`. La activación productiva sigue en AdapterBank (`tidex adapter-bank …`).

## 8. Estado verificado del proyecto

La validación de ingeniería debe ejecutarse con los targets y features de la superficie que se afirma. El Makefile y GitHub Actions corren `fmt --check`, `cargo check --all-targets --locked`, `cargo test --lib --locked`, las integraciones registradas, `configuration_contracts` con `--all-features` y `cargo check --locked --manifest-path fuzz/Cargo.toml --all-targets`. Clippy sigue perteneciendo a las puertas de `quality/`. El toolchain local (`rust-toolchain.toml`) es 1.96.0; CI fija 1.85.0. Las pruebas de LLM que requieren ejecución de modelo siguen dependiendo de un backend real disponible. Esta nota no inventa resultados de campañas no ejecutadas.

## 9. Estructura del repositorio

- `src/` — runtime principal por dominio (`foundation`, `knowledge`, `governance`, `operator`, `engine`, …).
- `src/knowledge/knowledge_engine.rs` — autoridad epistemológica; incluye `living_staircase`.
- `src/governance/adapter_bank.rs` — autenticación y lifecycle de adaptadores.
- `src/governance/residency_decision.rs` — derivación de residencia.
- `src/learning/numerical_evolution.rs` — evolución numérica acotada.
- `src/operator/executor_registry.rs` — catálogo declarado de ejecutores.
- `src/operator/control_plane.rs` — HTTP/CLI de la interfaz TIDE-X.
- `web-console/` — UI de la interfaz.
- `src/cross_model/` — backends reales y plasticidad (feature `cross-model-plasticity`).
- `src/cross_model/runtime/hf_worker.py` — worker embebido con `include_str!`; executor acotado, no autoridad.
- `src/cross_model/runtime/hf-runtime.lock.txt` — artefactos exactos con SHA-256 del venv HF certificado (torch CPU, transformers, nnsight). SAE no es un paquete pip: es un diccionario local ligado.
- `docs/` — arquitectura, implementación y estado.
- `config/evaluations/` — definiciones versionadas de evaluación conductual consumibles por los runtimes reales de evaluación.
- `config/materialization/` — políticas operativas versionadas para selección y materialización de backends.
- `quality/` — gates, evidencias y `quality/smoke/`.
- `docs/design/` — propuestas de diseño explícitamente no implementadas; no forman parte de la autoridad de runtime.
- `./tidex` — launcher del binario.

## 10. Política documental

La documentación debe reflejar el estado real del código. Si una capacidad no puede justificarse con evidencia, identidad, hashes, contratos y validación auténtica, no debe presentarse como parte del sistema real.

## 11. Documentación de referencia

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [docs/IMPLEMENTATION.md](docs/IMPLEMENTATION.md)
- [docs/SYSTEM_STATUS.md](docs/SYSTEM_STATUS.md)
- [docs/README_CROSS_MODEL.md](docs/README_CROSS_MODEL.md)
- [docs/design/adaptive_staircase.md](docs/design/adaptive_staircase.md)

La conclusión operativa: TIDE-X dispone de autoridades, ejecutores y evidencia reales, pero cada afirmación de capacidad debe demostrarse en el alcance concreto donde se usa. Unificar esas piezas bajo una sola dinámica no debe convertir la proyección de la escalera en una segunda autoridad.

### Executor Registry multi-eje

`./tidex executors` devuelve el catálogo declarado de ejecutores. El campo `state` no es uniforme: hay `Operational`, `OperationalNeedsWorkflow`, `ExperimentalCandidateOnly` y `ArchitectureOnly`. `ExecutorDescriptor::new` fuerza `implementation_status = Implemented` en todos los descriptores; eso no significa que el módulo esté cableado a producción. La madurez se separa en `runtime_status`, `workflow_status`, `evidence_status`, `maturity`, `production_authority` y `actionable_now`. No se debe usar `state` como sinónimo de autorización productiva.

La única autoridad de ciclo de vida productivo para adaptadores es `adapter.bank`. Los ejecutores candidate/advisory pueden producir evidencia, candidatos o señales, pero no activan producción por sí solos.
