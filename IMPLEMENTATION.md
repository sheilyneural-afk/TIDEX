# Guía de implementación de CEREBRO3

## 1. Reglas de implementación

La implementación actual de CEREBRO3 debe seguir estas reglas:

- la autoridad va en Rust;
- los workers externos no deciden ni promueven;
- la evidencia debe ser observada y hashada;
- los receipts son obligatorios en cualquier ruta de ejecución relevante;
- los fallos de validación se cierran en fail-closed.
- la experimentación y la synthetic data nunca se presentan como producción sin separar explícitamente su rol.

No se aceptan "stubs" para la lógica de autoridad ni para la validación de ejecución. Tampoco se aceptan rutas que conviertan una prueba experimental en una afirmación de autoridad sin cadena de evidencia.

## 1.1 Política de honestidad de evidencia

Antes de cerrar una entrega o de documentar una mejora, conviene responder estas preguntas:

1. ¿La afirmación pertenece a la capa de producción o a la capa experimental?
2. ¿Hay un origen real, verificable y ligado a un hash o a un receipt?
3. ¿La salida está siendo interpretada como evidencia de autoridad sin que la autoridad la haya validado?
4. ¿La ruta es un bench, un script de diagnóstico o una prueba de transfer y no una decisión operativa?

Si la respuesta a cualquiera de estas preguntas es ambigua, la documentación debe decirlo claramente y no promocionar la evidencia como si fuese definitiva.

## 2. Build y comprobación rápida

```bash
cargo fmt --all
cargo test --locked --offline --lib --quiet
```

También puede usarse una comprobación más amplia del proyecto:

```bash
cargo check --all-targets --locked --offline
cargo test --locked --offline --all-targets
```

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

### Autoridad

- `src/knowledge_engine.rs`
- `src/adapter_bank.rs`
- `src/adaptive_staircase.rs`

### Cross-model

- `src/cross_model/models/`
- `src/cross_model/runtime/`
- `src/cross_model/plasticity*`
- `src/cross_model/` varios módulos de descubrimiento, alineación y validación.

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

- [README.md](README.md)
- [ARCHITECTURE.md](ARCHITECTURE.md)
- [README_CROSS_MODEL.md](README_CROSS_MODEL.md)
- [SYSTEM_STATUS.md](SYSTEM_STATUS.md)
