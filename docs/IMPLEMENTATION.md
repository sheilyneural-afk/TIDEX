# Guía de implementación de TIDE-X

## 1. Reglas de implementación

La implementación actual de TIDE-X debe seguir estas reglas:

- la autoridad va en Rust;
- los workers externos no deciden ni promueven;
- la evidencia debe ser observada y hashada;
- los receipts son obligatorios en cualquier ruta de ejecución relevante;
- los fallos de validación se cierran en fail-closed.
- la evidencia de tests y producción debe proceder de artefactos reales o de contratos matemáticos directos; no se aceptan mocks, stubs, fallbacks, simulaciones ni datos sintéticos presentados como evidencia.

No se aceptan "stubs" para la lógica de autoridad ni para la validación de ejecución. Tampoco se aceptan rutas que conviertan una prueba experimental en una afirmación de autoridad sin cadena de evidencia.

## 1.1 Política de honestidad de evidencia

Antes de cerrar una entrega o de documentar una mejora, conviene responder estas preguntas:

1. ¿La afirmación pertenece a la capa de producción o a la capa experimental?
2. ¿Hay un origen real, verificable y ligado a un hash o a un receipt?
3. ¿La salida está siendo interpretada como evidencia de autoridad sin que la autoridad la haya validado?
4. ¿La ruta es un bench, un script de diagnóstico o una prueba de transfer y no una decisión operativa?

Si la respuesta a cualquiera de estas preguntas es ambigua, la documentación debe decirlo claramente y no promocionar la evidencia como si fuese definitiva.

## 2. Build y comprobación rápida

El pipeline local alineado con GitHub Actions es `make ci`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --offline --locked -- -D warnings
cargo check --all-targets --all-features --locked
cargo test --lib --all-features --locked
cargo test --all-features --locked --test production_surface --test quality_properties --test convergence_pipeline
cargo test --locked --all-features --test configuration_contracts
cargo check --locked --manifest-path fuzz/Cargo.toml --all-targets
```

También puede usarse una comprobación más amplia del proyecto:

```bash
cargo check --all-targets --all-features --locked --offline
cargo test --locked --offline --all-targets --all-features
```

Notas vigentes:

- `rust-toolchain.toml` y CI usan 1.96.0; `Cargo.toml` conserva `rust-version = "1.85"` como MSRV declarada, no como toolchain de CI.
- GitHub Actions ejecuta Clippy con `-D warnings`.
- `Cargo.toml` mantiene `autobins = false` y `autotests = false`; todos los `src/bin/*.rs` y todos los tests externos conservados están registrados explícitamente.

## 3. Flujo de desarrollo recomendado

1. Reproduce o comprueba el comportamiento real en Rust.
2. Añade o ajusta la prueba que cubra el caso.
3. Implementa el cambio mínimo con contratos explícitos.
4. Validar identidad, hashes, schemas y provenance.
5. Asegura que el resultado no se presente como evidencia sin origen real.

## 4. Backend HF / cross-model

El backend HF está diseñado como ejecución real y observación de activaciones, no como autoridad del sistema.

### Operaciones permitidas

- `generate`: texto real con política explícita.
- `activation`: vector de activación del layer solicitado.
- `set_steering`: instalación acotada de steering.
- `clear_steering`: limpieza explícita.
- `shutdown`: cierre ordenado del worker.

Cada una de estas operaciones responde con un payload firmado por el protocolo, y el Rust valida la respuesta antes de aceptarla.

### Requisitos de identidad

La runtime config debe comprobar:

- Python absoluto y existente;
- path absoluto del modelo y sus artifacts;
- binding exacto `model.safetensors`, `config.json`, `tokenizer.json`;
- hashes SHA-256 válidos;
- metadata del runtime consistente con la identidad real del worker.

## 5. Estructura lógica del proyecto

`src/` está partido por dominio. No existe `src/adaptive_staircase.rs`.

### Autoridad

- `src/knowledge/knowledge_engine.rs` — incluye `LivingStaircaseProjection` (vista de obligaciones, no orquestador)
- `src/governance/adapter_bank.rs`
- `src/governance/universal_promotion_gate.rs`
- `src/governance/residency_decision.rs`

### Interfaz y operación

- `src/operator/executor_registry.rs`
- `src/operator/control_plane.rs`
- `src/engine/` — BrainEngine
- `src/runtime/` — aislamiento de ejecución

### Cross-model (feature `cross-model-plasticity`)

- `src/cross_model/models/`
- `src/cross_model/runtime/` — `hf_worker.py` se embebe con `include_str!`; `hf-runtime.lock.txt` fija artefactos Python con `--require-hashes`
- `src/cross_model/plasticity*`
- módulos de descubrimiento, alineación y validación bajo `src/cross_model/`

## 6. Qué no hacer

- no convertir métricas importadas en evidencia interna sin validación;
- no dar autoridad al worker de Python;
- no prometer mejora funcional sin prueba held-out o evidencias observadas;
- no aceptar un output modelado sin que la entrada y la generación estén ligadas a un digest verificable;
- no añadir un segundo flujo de ejecución paralelo a la autorización principal.

## 7. Verificación de entrega

Antes de cerrar cambios importantes, la verificación mínima debe incluir:

```bash
cargo test --locked --offline --lib --quiet
```

Si se introduce integración con un backend nuevo, conviene validar además:

- identidad del modelo;
- hashes y binding;
- validación del request/response;
- cierre de fallos en rutas no soportadas;
- ausencia de autoridad delegada.

## 8. Referencias

- [README.md](../README.md)
- [ARCHITECTURE.md](ARCHITECTURE.md)
- [README_CROSS_MODEL.md](README_CROSS_MODEL.md)
- [SYSTEM_STATUS.md](SYSTEM_STATUS.md)
