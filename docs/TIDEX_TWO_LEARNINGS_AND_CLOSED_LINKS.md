# TIDE-X: dos aprendizajes y los dos eslabones que faltan

**Fecha:** 2026-09-12 (Europe/Madrid)  
**Checkout:** `/home/yo/Future` @ `6b7d912`+ (`feat/durable-plasticity-controllers`) — Paso 1 CLOSED; Paso 2A in progress on tip  
**Contexto de código:** [PR #1](https://github.com/sheilyneural-afk/TIDEX/pull/1) — controladores durables + coevolución causal + `plan_next_tick`. Aún no es el organismo cerrado.  
**Naturaleza de este doc:** dos partes explícitas. **Parte I** = mapa del problema (qué falta y por qué; los dos eslabones siguen siendo el mapa correcto). **Parte II** = orden de implementación (camino crítico de 6 pasos; **no** es el mismo orden que el mapa). No es código. No pide algoritmos nuevos de plasticidad.

**Ver también:** [TIDEX_PLASTICITY_MODULES_STUDY.md](TIDEX_PLASTICITY_MODULES_STUDY.md) · [TIDEX_SHEI_ARCHAEOLOGICAL_MATRIX.md](TIDEX_SHEI_ARCHAEOLOGICAL_MATRIX.md) · [SHEI_TO_TIDEX_SOTA_MAP.md](SHEI_TO_TIDEX_SOTA_MAP.md) · [systems/learning-and-plasticity.md](systems/learning-and-plasticity.md) · [ARCHITECTURE.md](ARCHITECTURE.md)

---

## Valoración (visión vs estado)

| Juicio | Estado |
|--------|--------|
| Visión de producto / dos aprendizajes / dos eslabones / B antes que A / no más BCM / residencia antes que IR / coordinador = workflow | **CORRECTO** |
| Paso 1 (plasticidad durable) | **CLOSED / CERTIFIED** — 18+ tests verdes; docs + push; no más plasticidad |
| Paso 2 (ProceduralMemory útil) | **2A implemented** (canonical replay + tests); **2B** advisory retrieve helper; **2C** pending (fold into NextAction) |
| Paso 3 (cerrar `NextAction` → executor) | **Hito central** del organismo |
| Pasos 4–5 (adquisición + residencia/IR) | Después de cerrar B |
| Paso 6 (demo real) | Criterio de aceptación bueno |

---

## 0. Lectura en una frase

TIDE-X ya sabe **materializar** una `CapabilityIR` en un receptor y **decidir residencia** (Software | Hybrid | Weights | Unknown). El eslabón **A** (software externo → capacidad autenticada → `CapabilityIR`) está **casi ausente** (❌). El eslabón **B** (estado → decisión → executor → evidencia → siguiente decisión) **ya empezó** (🟡: plasticidad durable certificable, *newer-valid*, `plan_next_tick`, `CoEvolutionDirective`) pero **no está cerrado**: la directiva no elige un executor real ni arranca el job. El coordinador vive en la capa de *workflow*, no dentro de BrainEngine / KnowledgeEngine / AdapterBank. No hacen falta más BCM/ELO hasta cerrar los seis pasos de la Parte II.

**Congelación de aceptación:** congelar desarrollo lateral hasta poder demostrar: TIDE-X recibió evidencia nueva, recordó experiencia previa, eligió una acción distinta *por* esa experiencia, ejecutó un executor real, y re-decidió tras el resultado — sin que un humano pulse el siguiente botón.

**Invariante:** `evidencia → ResidencyDecision → CapabilityIR` (nunca `código → CapabilityIR`). La residencia Software es inteligencia válida.

---

# Parte I — Mapa del problema

Los **dos eslabones siguen siendo el mapa correcto**. A = ❌ (casi ausente). B = 🟡 (empezado, no cerrado). Esta parte **no** es el orden en que hay que implementar: eso es la Parte II.

---

## 1. Dos aprendizajes distintos (no mezclar)

Hay **dos plasticidades**. Confundirlas produce módulos de más y el ciclo de menos.

### 1.1 Plasticidad de objeto — “trasplantar una capacidad a un receptor”

Es la capacidad **externa** (un sistema, un procedimiento, un modelo donante) convertida en algo que un LLM receptor (SmolLM2, Qwen, …) puede **alojar** de forma gobernada.

```text
capacidad externa (SHEI / GPEM / otro)
        │
        ▼
aislar capacidad funcional autenticada
        │
        ▼
   CapabilityIR          ← contrato formal, no “el código tal cual”
        │
        ▼
 ResidencyDecision       ← ¿dónde debe vivir?
        │
        ▼
materializar en el receptor
  low-rank / sparse / steering / controller / software binding…
        │
        ▼
medir + gates (UPG / AdapterBank)  →  promover o abandonar
```

Esto **ya tiene tramo desarrollado** *a partir de* `CapabilityIR` (cola ya existente):
`src/capability/capability_ir.rs` → `ResidencyDecision` → receiver / materialization.

El tramo **roto** está *antes*: software externo observado ≠ capacidad funcional formal ≠ `CapabilityIR`.

Ese diagrama de cola **no** es el método de adquisición. Para software externo el orden correcto es el de §5.1: evidencia observada → `ResidencyDecision` → **solo entonces** `CapabilityIR` si la evidencia lo permite. Inventar el IR leyendo código GPEM es el anti-patrón.

### 1.2 Metaplasticidad de sistema — “aprender CÓMO trasplantar”

TIDE-X debe aprender, con evidencia, **cómo** transplantar para la tupla:

```text
(capacidad X  +  receptor Y  +  arquitectura Z  +  evidencia E)
```

Es decir: qué ruta, qué calibración, cuándo abandonar. Órganos que ya existen para esto:

| Órgano | Rol en la metaplasticidad | Estado honesto post-PR#1 |
|--------|---------------------------|--------------------------|
| `ProceduralMemory` | recuerda qué procedimientos / intentos funcionaron | **reducer acotado no persistente** (derivado); `retrieve` es `pub`; `record_attempt`/`rebuild` son `pub(crate)`. Paths productivos **aún no** lo consumen en el decisor global — **la fuga más grande** de “cuanto más trabaja, mejor sabe qué hacer” |
| `RoutingPlasticity` | adapta pesos de ruta con evidencia medida | durable en `controller_state.json`; **sigue advisory** |
| `ELOSystem` | ranking relativo de entidades | durable; advisory |
| `BCMMetaplasticity` | umbral / sensibilidad | durable; advisory |
| `EligibilityTraces` | crédito temporal | durable; advisory |
| `KnowledgeEngine` | autoridad fail-closed; **no** es el coordinador del ciclo | órganos sí; no encadena observe→decide→execute |
| `BidirectionalLoop.plan_next_tick` → `CoEvolutionDirective` | propone discovery / transfer / hold y sesga routing/PI | **existe y es causal**; **sigue advisory**; no elige executor ni arranca job |

La metaplasticidad **mejora decisiones futuras**. No sustituye re-medir ni pasar gates. Un rating ELO alto no es promoción. Un `CoEvolutionDirective` no es un job ejecutado.

```mermaid
flowchart LR
  subgraph OBJ["1. Plasticidad de objeto"]
    EXT[capacidad externa] --> CAP[capacidad funcional]
    CAP --> IR[CapabilityIR]
    IR --> RD[ResidencyDecision]
    RD --> MAT[materializar en receptor]
    MAT --> MEAS[medir + gates]
  end

  subgraph META["2. Metaplasticidad de sistema"]
    E[evidencia E] --> PM[ProceduralMemory]
    E --> RP[RoutingPlasticity / ELO / BCM / Eligibility]
    PM --> DEC["¿qué ruta / calibración / abandono?"]
    RP --> DEC
    DEC --> NEXT[siguiente intento sobre X+Y+Z]
  end

  MEAS --> E
  NEXT --> MAT
```

---

## 2. `ResidencyDecision` — no siempre son pesos

Tipo vivo: `src/governance/residency_decision.rs` (`Software` | `Weights` | `Hybrid` | `Blocked` | `BoundedUnknown`).

La visión de producto (mapeo informal → enum):

| Visión | Enum | Cuándo |
|--------|------|--------|
| Software | `Software` | La capacidad **debe** quedarse como procedimiento autenticado. Ejemplo: GPEM / provenance durable — el valor es el ledger, no un peso. |
| Weights | `Weights` | La capacidad es una función que el receptor puede **alojar** (low-rank, sparse, steering, controller). |
| Hybrid | `Hybrid` | El LLM decide *qué* buscar / *cuándo* invocar; el software hace el retrieve exacto autenticado. |
| Unknown → investigar | `BoundedUnknown` (+ obligaciones) | No hay evidencia bastante. **No se inventa residencia.** Se investiga. |
| Bloqueado | `Blocked` | Evidencia dice que no se puede alojar (política / integridad). |

Ejemplos de la visión, no recetas de código:

1. **GPEM, provenance durable** → casi siempre `Software`. Meterlo en pesos pierde exactamente lo que lo hace GPEM (append-only, autenticado, reversible).
2. **“¿Qué procedimiento pasado es relevante para este caso?”** → puede ser `Weights` (el receptor aprende a *reconocer analogía*) o `Hybrid` (el receptor propone candidatos; `ProceduralMemory.retrieve` autentica).
3. **Retrieve histórico exacto y autenticado** → `Hybrid`: el LLM no es la fuente de verdad del byte; el software sí.

`ResidencyDecision` **ya existe y decide**. El hueco no es el enum. El hueco es quién **alimenta** esa decisión desde una capacidad externa *observada* (conducta + contratos + evidencia causal — no semántica inventada al leer el árbol), y quién **ejecuta** la residencia elegida como siguiente acción real del organismo.

**Invariante (repetido a propósito):** `evidencia → ResidencyDecision → CapabilityIR`. Nunca `código → CapabilityIR`. La residencia Software es inteligencia válida.

---

## 3. Ciclo completo (el organismo)

```mermaid
flowchart TD
  SRC["SHEI / GPEM / software externo"]
  OBS["observar: acquire_system<br/>bytes autenticados + receipt"]
  CAP["capacidad funcional aislada<br/>❌ AÚN NO EXISTE"]
  IR["CapabilityIR<br/>❌ automático desde software externo"]
  ANA["TIDE-X analiza"]
  WHERE["¿dónde vive?<br/>ResidencyDecision ✅"]
  HOW["¿cómo se transfiere?<br/>ruta / calibración / abandono"]
  REC["receptor<br/>SmolLM2 / Qwen / …"]
  MAT["materializar candidato ✅"]
  MEAS["medir + UPG / AdapterBank ✅"]
  LEARN["TIDE-X aprende CÓMO<br/>🟡 durable + advisory"]
  NEXT["UNA siguiente acción<br/>🟡 B empezado, no cierra"]
  EXEC["executor real"]

  SRC --> OBS --> CAP --> IR --> ANA
  ANA --> WHERE
  ANA --> HOW
  WHERE --> REC
  HOW --> REC
  REC --> MAT --> MEAS --> LEARN --> NEXT --> EXEC
  EXEC --> OBS
  MEAS -.->|"re-medir; los gates no se saltan"| ANA
```

ASCII equivalente:

```text
SHEI / GPEM
    │  acquire_system = bytes autenticados
    │  (NO “entendido”, NO “pasado a pesos”)
    ▼
capacidad funcional observada          ← eslabón A, ❌ casi ausente
    │
    ▼
CapabilityIR                           ← eslabón A, ❌ (auto)
    │
    ▼
TIDE-X analiza
    ├─ ¿dónde vive?   ResidencyDecision     ✅
    └─ ¿cómo se transfiere? ruta / cal / stop
    │
    ▼
receptor (SmolLM2 / Qwen / …)
    │
    ▼
materializar → medir → gates           ✅
    │
    ▼
TIDE-X aprende (metaplasticidad)       🟡 durable, aún advisory
    │
    ▼
UNA siguiente acción + executor real   ← eslabón B, 🟡 empezado, no cerrado
    │                                    (directive ≠ job; retrieve no gobierna)
    └──► loop
```

---

## 4. Estado honesto — 2026-09-12, después del trabajo de plasticidad de PR#1

PR: https://github.com/sheilyneural-afk/TIDEX/pull/1  
Rama local alineada: `feat/durable-plasticity-controllers` @ `456d1eb`  
(*durable controllers + causal coevolution + `plan_next_tick`; el PR seguía OPEN en esta fecha.*)

**Certificación Paso 1 (CLOSED) — 2026-09-12 ~05:10 CEST:**

```text
checkout: feat/durable-plasticity-controllers @ 456d1eb (+ docs certify commit)
cargo test --features cross-model-plasticity plasticity
→ 18 passed / 0 failed
cargo test --features cross-model-plasticity --lib 'operator::control_plane::tests'
→ 22 passed / 0 failed  (incluye advice / persist / causal / loop)
cargo check --features cross-model-plasticity  → ok
cargo check --no-default-features             → ok
cargo check (default)                         → ok
```

Cubre: persistencia entre llamadas, *newer-valid*, intervenciones causales, `BidirectionalLoop` durable (sin doble-registro), `plan_next_tick` cambia el consejo siguiente, sellado `evidence_sha256`, carga fail-closed de `config/plasticity.toml`. **Sin huecos de código Paso-1** (no se abrió hardening de algoritmos). Sigue **AdvisoryOnly**.

| Pregunta de organismo | Estado | Nota de disco |
|-----------------------|--------|----------------|
| TIDE-X mejora *cómo trabaja* a medida que acumula evidencia | 🟡 parcial / avanzado | `controller_state.json` durable; merge *newer-valid*; `plan_next_tick` emite `CoEvolutionDirective` sobre routing/PI; filtro causal de intervención; updates durables de routing/PI. **Sigue advisory.** Tests de plasticidad: 18/0. |
| Decide solo qué ruta tomar | 🟡 parcial | Routing + directive existen; no fuerzan el siguiente job / `target_model` |
| Recuerda qué estrategias funcionaron | 🟡 memoria derivada, no gobierna | `ProceduralMemory` = reducer acotado no persistente; `retrieve` existe; **ningún path productivo lo consume en el decisor global**; aún no hay replay canónico desde receipts |
| Observa software externo tipo GPEM | 🟡 captura, no comprensión | `acquire_system` sella bytes + envelope + receipt; el binario **declara** que no entiende ni transfiere a pesos |
| GPEM → capacidad funcional formal | ❌ | no hay aislamiento de “qué hace” autenticado (conducta observada + contratos + evidencia causal) |
| Capacidad → `CapabilityIR` automático | ❌ | el IR se compila cuando ya existe el contrato; no se induce desde evidencia; **no** se inventa leyendo el árbol |
| `CapabilityIR` → receptor | ✅ desarrollado | receiver compiler / binding / materializers |
| Elegir Software / Hybrid / Weights | ✅ | `ResidencyDecision` (+ `Blocked` / `BoundedUnknown`) |
| Materializar candidato | ✅ | low-rank / sparse / steering / shadow / compiler universal |
| Validar / gobernar / promover | ✅ | UPG + AdapterBank; materializar ≠ activar |
| Transición final B: directive + routing + retrieve + KE → executor → `start_operator_job(...)` | ❌ | **el corte que deja B en 🟡** |
| Bucle autónomo cerrado sin cableado humano | ❌ **hueco de cierre** | órganos sí; coordinador de workflow no; A ausente |

Leyenda: ✅ existe y se usa · 🟡 existe a medias / advisory / no gobierna / empezado no cerrado · ❌ no existe el tramo.

### Qué cambió con PR#1 (y qué no)

El estudio [TIDEX_PLASTICITY_MODULES_STUDY.md](TIDEX_PLASTICITY_MODULES_STUDY.md) describe el mundo *antes* de la persistencia (controllers efímeros por request). PR#1 cierra **una** de aquellas recomendaciones: snapshot durable + coevolución causal + `plan_next_tick`. Eso **abre** el eslabón B; **no** lo cierra.

**No** cierra el organismo:

- la directiva no ejecuta (`CoEvolutionDirective` ≠ `start_operator_job`);
- `ProceduralMemory.retrieve` no gobierna el decisor global (y la memoria aún no se reconstruye por replay canónico desde receipts autenticados);
- `acquire_system` no produce capacidad ni IR;
- no hay un solo decisor de *workflow* que emita `NextAction` y la mande a un executor registrado;
- la frontera de composición Operator ↔ KnowledgeEngine / ProceduralMemory aún no está decidida explícitamente (§8, antes del Paso 3).

---

## 5. Los dos eslabones (el trabajo de verdad)

No son veinte módulos. Son **dos uniones**. A = ❌. B = 🟡.

### 5.1 Eslabón A — de software externo a `CapabilityIR` (❌ casi ausente)

```text
SOFTWARE EXTERNO
    →  capacidad funcional autenticada
    →  CapabilityIR
```

Hoy el corte es:

| Tramo | Existe |
|-------|--------|
| Capturar árbol / envelope / receipt (`acquire_system`) | sí — bytes, no semántica |
| Ejecutar al donante para *observar conducta* | no en este camino (el binario lo prohíbe) |
| Aislar “esta es la capacidad X, con contrato Y” | no |
| Compilar eso a `CapabilityIR` sin un humano que ya traiga el IR | no |
| A partir del IR: residency + receptor + materialize | sí (cola desarrollada) |

Hasta que A exista, “observar GPEM” es **archivo en la caja fuerte**, no **órgano transplantable**.

#### Método de A (anti-patrón vs camino)

**NO:** leer código GPEM → adivinar qué hace → inventar `CapabilityIR`. Eso es frágil: semántica inventada, no capacidad autenticada.

**SÍ:**

```text
conducta real observada
  + contratos funcionales
  + evidencia causal
        │
        ▼
 ResidencyDecision
   (Software | Hybrid | Weights | BoundedUnknown)
        │
        ▼
 CapabilityIR          ← solo si la evidencia lo permite
        │
        ▼
 receiver compiler     ← solo si la residencia y el IR lo autorizan
```

El código capturado es **provenance / análisis**, no semántica inventada. Fail-closed: si no se puede aislar la capacidad, `BoundedUnknown` con obligaciones — nunca un peso ni un IR de ficción.

### 5.2 Eslabón B — de estado a la siguiente acción real (🟡 empezado, no cerrado)

```text
estado + historia + plasticidad + knowledge
    →  UNA decisión de siguiente acción
    →  executor real
    →  evidencia nueva
    →  ↺
```

**Ya existe (por eso B no es ❌):**

- plasticidad durable (`controller_state.json`);
- merge *newer-valid*;
- `plan_next_tick`;
- `CoEvolutionDirective`;
- filtro causal de intervención;
- updates durables de routing / PI;
- carga fail-closed de `config/plasticity.toml` en `compute_operator_plasticity_advice()` (la configuración **gobierna los controladores advisory**; no se convierte en autoridad de ejecución/promoción).

**Falta la transición final (por eso B no está cerrado):**

```text
CoEvolutionDirective
  + RoutingPlasticity
  + ProceduralMemory.retrieve   ← tras replay canónico (Paso 2)
  + KnowledgeEngine
        │
        ▼
 elegir un executor real existente
        │
        ▼
 start_operator_job(...)
        │
        ▼
 evidencia nueva → siguiente decisión
```

Hoy el corte:

| Pieza | Existe | ¿Gobierna el siguiente tick? |
|-------|--------|------------------------------|
| Estado plástico durable | sí, PR#1 (18 tests) | no (advisory) |
| Historia de coevolución | sí, PR#1 | no (directive no es job) |
| `plan_next_tick` / `CoEvolutionDirective` | sí | no |
| Filtro causal de intervención | sí | no (filtra; no ejecuta) |
| Updates durables routing / PI | sí | sesgan consejo; no arrancan job |
| `config/plasticity.toml` | sí — cargado fail-closed en `compute_operator_plasticity_advice()` | gobierna controladores **advisory**; no es autoridad de ejecución/promoción |
| `ProceduralMemory` | sí (reducer derivado; `retrieve` pub) | no — falta replay canónico + consumo en el decisor |
| `KnowledgeEngine` | sí | autoridad, no coordinador de ciclo |
| Un decisor que elige **una** acción y la manda a un executor | no | — |
| Evidencia nueva que vuelve a alimentar A y B | parcial (jobs / receipts) | no cierra solo |

Hasta que B **cierre**, TIDE-X **proyecta** qué haría un organismo. No **es** el organismo. La fuga más cara: `ProceduralMemory` ya sabe rankear procedimientos; el decisor global no lo pregunta — y aún no se reconstruye desde receipts autenticados.

### 5.3 Dónde vive el coordinador (colocación, no un cerebro nuevo)

**No** meter el coordinador dentro de BrainEngine, KnowledgeEngine ni AdapterBank. Esas son autoridades **distintas y legítimas**. Meterle ahí mezcla “qué es verdad / qué se promueve / qué se activa” con “qué conviene correr ahora”.

El coordinador vive en la capa de **workflow / composición** ([ARCHITECTURE.md](ARCHITECTURE.md): workflow, no una quinta autoridad). Produce algo conceptualmente así:

```text
NextAction {
  executor_id,                 ← entre executors EXISTENTES
  authenticated_inputs,
  rationale_evidence,
  expected_information_gain,
  cost,
  risk,
  stop_condition
}
```

**Agudeza de `NextAction`:** no es un planner genérico. Solo responde: *¿cuál de las acciones admisibles ahora produce más evidencia/progreso esperada, dado lo que sé y lo que funcionó?* Debe elegir entre **executors ya existentes**. No inventa capacidades nuevas ni promociona.

Ejemplo concreto de trasplante (agudeza, no teatro):

```text
KE: falta causalidad suficiente
ProceduralMemory: steering falló 4/5; low-rank acertó 6/7
RoutingPlasticity: prefiere ruta B
CoEvolutionDirective: transfer
calibración: insuficiente
        │
        ▼
NextAction = cross_model.align
  reason: calibration_required_before_transfer
        │
        ▼  (más tarde, con evidencia)
transfer_steering  |  abandon_current_strategy
```

**No** decide verdad, promoción ni activación a producción. Solo: *dado lo que sabemos, esta es la siguiente acción admisible que vale la pena correr*. Las autoridades existentes verifican el resultado.

---

## 6. Lo que explícitamente NO hace falta

No hacen falta más BCM, más ELO, más controladores, más memorias ni más planners de plasticidad.

BrainEngine se partió por autoridades de forma correcta: los órganos existen (`KnowledgeEngine`, `AdapterBank`, `UniversalPromotionGate`, `ResidencyDecision`, receiver, materialization, `plasticity/*`, `ProceduralMemory`, `BidirectionalLoop`).

Falta el **coordinador de workflow** que encadene:

```text
observar → entender → decidir → ejecutar → medir → aprender → decidir otra vez
```

**Regla explícita:** no añadir más algoritmos de plasticidad (BCM / controladores / memorias / planners) hasta cerrar los **seis pasos** de la Parte II. Órganos sin coordinador pierden la visión original: TIDE-X sabiendo qué hacer después **sin que un humano cablee cada paso**.

Cualquier módulo nuevo que no sea un tramo de esos seis pasos es trabajo lateral. **Congelar** ese lateral hasta la demostración de aceptación (§0).

---

## 7. PR#1 en su sitio (citar, no mitificar)

- URL: https://github.com/sheilyneural-afk/TIDEX/pull/1  
- Título: *feat: durable operator plasticity controllers*
- Checkout local: `feat/durable-plasticity-controllers` @ `456d1eb`
- Qué entrega: persistencia sellada en `{tidex_home}/operator/plasticity/controller_state.json`; `BidirectionalLoop` durable (`coevolution_history` + `applied_coevolution_keys`); `plan_next_tick` → `CoEvolutionDirective` (discovery / transfer / hold + sesgo routing/PI); *newer-valid*; filtro causal de intervención; fail-closed; feature `cross-model-plasticity`; **sigue AdvisoryOnly**.
- Config: PR#1 **sí** carga `config/plasticity.toml` fail-closed en `compute_operator_plasticity_advice()`. **La configuración gobierna los controladores advisory; no se convierte en autoridad de ejecución/promoción.** El loader **no** está “pendiente de entregar”.
- Certificación de tests (usuario): `cargo test --features cross-model-plasticity plasticity` → **18 passed / 0 failed** (persistencia entre llamadas, *newer-valid*, intervenciones causales, loop durable, `plan_next_tick` cambia consejo).
- Qué no entrega: el ciclo cerrado de la §3, el eslabón A, el **cierre** del eslabón B (directive → executor → job), autoridad de promoción, ni un segundo store de ProceduralMemory.

PR#1 es **metaplasticidad incipiente** y el **arranque de B**. No es el organismo. No mitificarlo como “B cerrado”. Paso 1 **CLOSED / CERTIFIED** 2026-09-12 (18 plasticity + 22 control_plane tests; check ± feature).

---

# Parte II — Orden de implementación

El mapa de la Parte I (A, luego B en el dibujo del organismo) **no** es el orden de trabajo. El camino crítico es **B primero** (cerrar lo ya empezado), **después A**, **después el demo acotado**.

---

## 8. Camino crítico (este orden, no otro)

Seis pasos. Este es el roadmap. No invertir, no saltar, no sustituir por otro BCM.

```text
1. CERTIFY 456d1eb
   (durable plasticity, causal coevolution, plan_next_tick,
    idempotence, replay, gates)
        │
        ▼
2. MAKE PROCEDURAL MEMORY USEFUL
   (receipts → canonical replay → ProceduralMemory → retrieve(context))
        │
        ▼
3. CLOSE WORKFLOW DECIDER
   (KE + CoEvolutionDirective + RoutingPlasticity + ProceduralMemory.retrieve
    + cost/risk/state → ONE NextAction → existing executor → real job → receipt → ↺)
        │
        ▼
4. FUNCTIONAL SOFTWARE ACQUISITION
   (real GPEM execution → observations / interventions /
    counterfactuals / contracts → authenticated capacity)
        │
        ▼
5. RESIDENCY / REPRESENTATION
   (evidence → ResidencyDecision Software|Hybrid|Weights|BoundedUnknown
    → CapabilityIR only when warranted)
        │
        ▼
6. REAL DEMO
   (GPEM → autonomous TIDE-X → small LLM → measure → learn how → better second attempt)
```

### Paso 1 — CERTIFY `456d1eb` (**CLOSED** 2026-09-12 @ tip `6b7d912`)

**Estado:** **CLOSED / CERTIFIED**. Sustancia en `456d1eb`; tip de rama de certificación / formato `6b7d912`; certificación re-ejecutada verde; roadmap + INDEX see-also empujados a la rama / PR#1. **No reabrir plasticidad.**

**Checklist certificación:**

| Ítem | Resultado |
|------|-----------|
| Persist / reload durable (`controller_state.json`) | ✅ tests |
| Idempotencia (mismo discovery no doble-registra) | ✅ `plasticity_advice_persists_bidirectional_loop_across_calls` |
| Replay de estado sellado + `evidence_sha256` | ✅ load/verify/persist + tests |
| *newer-valid* | ✅ |
| Gates / fail-closed (`plasticity.toml`, schema) | ✅ |
| Causal seal de intervenciones | ✅ |
| `plan_next_tick` / directive durable | ✅ |
| `cargo check` ± feature | ✅ |
| Nuevos algoritmos BCM/ELO/planners | ❌ no abiertos (correcto) |

**Anti-patrón (sigue vigente):** abrir otro controlador BCM/ELO/memoria. Congelado. Siguiente trabajo = Paso 2 (replay canónico → retrieve), no más Paso 1.

### Paso 2 — MAKE PROCEDURAL MEMORY USEFUL (replay canónico primero)

**Realidad de código (no negociar):** `ProceduralMemory` es un *“Bounded, non-persistent reducer”*. `record_attempt` / `rebuild` son `pub(crate)`; `retrieve` es `pub`. Es **buena arquitectura**: no debe convertirse en otra DB ni en otra autoridad.

**Anti-patrón explícito:** **NO** `operator/procedural_memory.json` ni segundo store de persistencia. La memoria se **deriva**.

Datos ya cerca:

- `NumericalEvolutionCycle` expone `procedural_attempt()`;
- `tidex numerical.evolve` lo incluye;
- las corridas Operator dejan stdout + hashes / receipts.

Paso 2 se parte en tres:

| Subpaso | Qué |
|---------|-----|
| **2A** | Reconstruir `ProceduralMemory` desde receipts históricos autenticados vía **replay canónico** |
| **2B** | `retrieve(query)` para contexto del siguiente tick |
| **2C** | Plegar ese consejo en la decisión (`NextAction`) |

Sin 2A, “conectar retrieve al decisor” sería teatro sobre memoria vacía o inventada. Sin 2B/2C, la memoria sigue siendo órgano muerto para el workflow.

**Inventario 2A (docs-only, post–Paso 1):**

| Fuente | Path / nota |
|--------|-------------|
| `ProceduralMemory` | `src/learning/procedural_memory.rs` — reducer; `rebuild`/`record_attempt` `pub(crate)`; `retrieve` `pub` |
| Emisión de intento | `NumericalEvolutionCycle::procedural_attempt()` en `src/learning/numerical_evolution.rs` |
| CLI stdout | `src/bin/tidex.rs` incluye `procedural_attempt` / `procedural_attempt_count` en ciclo numérico |
| Receipts Operator | `OperatorRunReceipt` + stdout bajo `{tidex_home}/operator/runs/by-sha/...` |
| Colección local | `collected_receipts/*.json` (muestras; no son aún el feed canónico de replay) |
| Frontera | `build.rs`: engine↛operator cruzado silencioso — replay debe componerse en workflow (`tidex.rs` / capa composición), no importar KE/PM dentro de Operator |

**Estado 2A (2026-09-12):** **IMPLEMENTED** on branch tip after `6b7d912`.

| Pieza | Path |
|-------|------|
| Replay canónico | `src/learning/procedural_replay.rs` — `rebuild_from_authenticated_stdout` / `rebuild_from_numerical_evolution_stdout` → `ProceduralMemory::rebuild_with_solver_failures` |
| 2B helper | `retrieve_procedural_advice(memory, query)` (advisory-only; Paso 3 hook documented in-module) |
| Composición Operator | `src/bin/tidex.rs` — `rebuild_procedural_memory_from_operator_run` + CLI `tidex procedural replay-from-run-receipt` |
| Tests | `procedural_replay::tests` — fixture replay → ranked retrieve; tamper / digest / schema / production / count fail-closed |

**No** `procedural_memory.json`. **No** import de PM/KE en `operator/control_plane.rs`. Siguiente: **2C** / Paso 3 = plegar `retrieve` en `NextAction` → executor existente.

### Antes del Paso 3 — frontera de composición (decidir explícitamente)

`build.rs` separa dominios:

```text
engine   → analysis, foundation, learning
operator → cross_model, foundation
```

Por tanto `operator/control_plane.rs` **no** puede importar a la ligera `KnowledgeEngine` / `ProceduralMemory` / `ResidencyDecision` sin romper deliberadamente la arquitectura.

La raíz de composición que hoy ve casi todo: `src/bin/tidex.rs`.

**Antes de escribir `NextAction`:** documentar y decidir la frontera de composición de forma explícita. **Sin dependencias cruzadas silenciosas hacia Operator.** El coordinador sigue siendo workflow; no tiene por qué vivir *para siempre* dentro de `tidex.rs`, pero el límite debe ser consciente.

### Paso 3 — CLOSE WORKFLOW DECIDER (hito central)

La transición final de B:

```text
KnowledgeEngine
  + CoEvolutionDirective
  + RoutingPlasticity
  + ProceduralMemory.retrieve
  + cost / risk / state
        │
        ▼
  ONE NextAction          ← entre executors EXISTENTES
        │
        ▼
  existing executor
        │
        ▼
  real job → receipt
        │
        ▼
  ↺ (re-decidir)
```

Criterio de cierre de B (= congelación de aceptación §0): el sistema emite y **ejecuta** una sola siguiente acción, re-mide, actualiza estado plástico / procedural (vía replay), y la siguiente decisión **cambia por evidencia** — sin que un humano elija el job a mano. Los gates siguen siendo gates. El coordinador no promociona.

Ver agudeza de `NextAction` y el ejemplo de trasplante en §5.3.

### Paso 4 — FUNCTIONAL SOFTWARE ACQUISITION

No más “archivo en la caja fuerte”. Ejecución real de GPEM → observaciones / intervenciones / contrafácticos / contratos → capacidad autenticada. El árbol capturado queda como provenance. Este paso **aún no** inventa `CapabilityIR`.

### Paso 5 — RESIDENCY / REPRESENTATION

Método de §5.1, no el anti-patrón:

```text
evidence
  → ResidencyDecision (Software | Hybrid | Weights | BoundedUnknown)
  → CapabilityIR only when warranted
```

Fail-closed a `BoundedUnknown` + obligaciones. Nunca un peso inventado. Nunca `código → CapabilityIR`.

Criterio de cierre de A: un GPEM (u otro software) capturado produce, sin IR humano previo, o bien `ResidencyDecision::Software` justificada, o bien una `CapabilityIR` autenticada, o bien `BoundedUnknown` con obligaciones.

### Paso 6 — REAL DEMO

```text
GPEM → TIDE-X autónomo → LLM pequeño → medir → aprender CÓMO → mejor segundo intento
```

Capacidad ejemplo (acotada, no un organismo entero):

> Dado un contexto + varios procedimientos históricos + resultados previos, elegir el procedimiento más adecuado **o** decidir explorar.

Sin cableado humano de cada tramo:

```text
capturar
  → descubrir capacidad
  → obligaciones de evidencia
  → experimentos
  → residencia
  → IR si procede
  → elegir receptor
  → estrategia desde la historia
  → materializar
  → evaluar
  → rechazar / mejorar / promover
  → recordar (replay → ProceduralMemory)
  → reutilizar
```

Si este demo no corre solo, A y B no están cerrados — da igual cuántos órganos haya.

### Fuera de este camino

No abrir Ola RALF / Minimum Space / otros BCM como sustituto de estos seis pasos. Esos mapas ([SHEI_TO_TIDEX_SOTA_MAP.md](SHEI_TO_TIDEX_SOTA_MAP.md), [TIDEX_SHEI_ARCHAEOLOGICAL_MATRIX.md](TIDEX_SHEI_ARCHAEOLOGICAL_MATRIX.md)) siguen válidos como arqueología; no son el camino crítico del organismo.

---

## 9. Relación con el resto del mapa

| Doc | Relación |
|-----|----------|
| [TIDEX_PLASTICITY_MODULES_STUDY.md](TIDEX_PLASTICITY_MODULES_STUDY.md) | Anatomía de controllers vs `PlasticityEngine`. Pre-PR#1 en persistencia; este doc actualiza el *significado* de ese estudio y el orden de trabajo. |
| [SHEI_TO_TIDEX_SOTA_MAP.md](SHEI_TO_TIDEX_SOTA_MAP.md) | Qué portar de SHEI. Este doc dice *por qué* GPEM no es “pásalo a pesos” y *cómo* no inventar IR leyendo código. |
| [TIDEX_SHEI_ARCHAEOLOGICAL_MATRIX.md](TIDEX_SHEI_ARCHAEOLOGICAL_MATRIX.md) | HAVE/PARTIAL/MISS. Este doc recorta el crítico a dos uniones y a seis pasos. |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Capas autoridad / workflow / evidencia. El coordinador **vive en workflow**; no entra en BrainEngine, KnowledgeEngine ni AdapterBank; no crea una quinta autoridad. Frontera `build.rs` engine↔operator debe respetarse al componer `NextAction`. |
| [systems/learning-and-plasticity.md](systems/learning-and-plasticity.md) | Piezas de learning. Este doc es la visión de ciclo + el orden, no el inventario de archivos. |

---

*Doc de mapa (Parte I) + orden de implementación (Parte II). No implementa. No pide módulos nuevos de plasticidad hasta cerrar los seis pasos. Paso 1 CLOSED @ 6b7d912. Paso 2A replay canónico DONE; 2B retrieve helper DONE; 2C/Paso 3 = decisión. Paso 3 = hito central. Sin `procedural_memory.json`. Sin dependencias cruzadas silenciosas Operator←KE/PM. evidencia → ResidencyDecision → CapabilityIR.*
