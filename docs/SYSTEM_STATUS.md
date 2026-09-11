# Estado del proyecto

## Estado vigente

La base actual del proyecto está en una línea estable y verificable:

- la autoridad central sigue en Rust;
- `KnowledgeEngine::open` falla cerrado (`authority_instance_required`); el arranque real es `open_with_authority_instance`;
- la ejecución de backends reales queda acotada y validada, detrás de la feature `cross-model-plasticity`;
- la capa de planificación y receipts está separada de la capa de ejecución externa;
- no se acepta evidencia que no tenga origen verificable;
- el backend HF se mantiene como ejecutor acotado, no como autoridad;
- un receipt de la interfaz marca `authorizes_production: false` y no activa AdapterBank por sí mismo.

## Verificación

Este documento no afirma un recuento de pruebas pasadas sin re-ejecutar la suite.

En el árbol actual hay 620 atributos `#[test]` bajo `src/` y 33 bajo `tests/` (653 en total). Eso es un recuento de atributos, no un resultado de `cargo test`. Una nota anterior decía “592 pruebas pasadas”; esa cifra está desfasada y no debe reutilizarse.

El pipeline cableado en `Makefile` y `.github/workflows/ci.yml` usa el mismo toolchain fijado por `rust-toolchain.toml` (1.96.0) y ejecuta formato, Clippy con `-D warnings`, compilación all-features, tests de librería, integración registrada, contratos de configuración y compilación del arnés de fuzzing:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --offline --locked -- -D warnings
cargo check --all-targets --all-features --locked
cargo test --lib --all-features --locked
cargo test --all-features --locked --test production_surface --test quality_properties --test convergence_pipeline
cargo test --locked --all-features --test configuration_contracts
cargo check --locked --manifest-path fuzz/Cargo.toml --all-targets
```

`Cargo.toml` mantiene `autotests = false`: sólo existen como targets los tests explícitamente registrados. Los antiguos tests huérfanos se retiraron o consolidaron en sus autoridades canónicas; el árbol no conserva tests externos no registrados.

## Componentes activos y verificados

### Authority layer

- `KnowledgeEngine`: planifica, valida y emite receipts. Expone `living_staircase()` como proyección de solo lectura.
- `AdapterBank`: resuelve y autentica adaptadores. Único descriptor con `production_authority=true`.

No existe el tipo ni el archivo `AdaptiveStaircase`. Presentarlo como componente activo era un error documental.

### Runtime cross-model

Disponible cuando se construye con `--features cross-model-plasticity`:

- backend Ollama con inferencia real;
- backend Candle con validación de archivos reales;
- backend HF con worker persistente (fuente embebida) y protocolo hashado;
- extracción de activaciones del layer solicitado;
- intervención de steering acotada y limpieza explícita.

### Seguridad y contrato

- rutas absolutas; validación de archivos y directorios en las rutas de identidad de modelo;
- hashes SHA-256 para identidad y checks del runtime;
- schemas y request ids en cada request/response del worker HF;
- fail-closed en invalid identity, mismatch, corrupt file, unsupported capability.

La superficie HTTP de la interfaz no hereda automáticamente esas garantías: `POST /api/models/scan` exige un path absoluto *dentro* del hub HF del runtime, y los assets de un job deben quedar bajo `TIDEX_HOME`. Eso es estado del código, no una afirmación de que el perímetro esté cerrado.

## Qué no está establecido

El proyecto no afirma todavía:

- que un steering cause transferencia útil de comportamiento;
- que exista una generalización universal entre arquitecturas;
- que una activación interna sea producción lista sin autorización final;
- que todos los backends sean iguales en capacidad o calidad;
- que la experimentación o el synthetic data puedan promocionarse a producción sin validación adicional;
- que la escalera viva unifique el árbol (plasticidad, numerics, residency, materialization, promotion) en un solo ciclo.

Lo que sí está demostrado es la integridad del runtime de autoridad, la proyección de obligaciones y la separación entre ejecución real y decisión. Esa separación debe seguir visible en la documentación.

## Política actual

La política sigue siendo de evidencia antes de promoción:

- si no hay origen real, la evidencia no se acepta;
- si falta validación, la ejecución no avanza;
- si cambia la identidad del artifact, se rechaza;
- si la capa externa intenta gobernar por sí sola, la operación falla cerrada.

## Documentación base

- [README.md](../README.md)
- [ARCHITECTURE.md](ARCHITECTURE.md)
- [IMPLEMENTATION.md](IMPLEMENTATION.md)
- [README_CROSS_MODEL.md](README_CROSS_MODEL.md)
