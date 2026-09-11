# Calidad y validación de TIDE-X

Este repositorio no sigue una política de "tests que pasan por casualidad". La calidad se define con tres capas:

1. Verificación de compilación real.
2. Validación de lógica de autoridad y evidencia.
3. Verificación del comportamiento mediante suites explícitas por dominio.

## 1. Principios del proyecto

- La verdad no se inventa: cada capacidad debe declararse con estado, autoridad y evidencia reales.
- La producción y la experimentación son capas distintas.
- Los workflows y los artefactos deben estar vinculados a una necesidad observable.
- La acción de ejecutar debe estar respaldada por checks de autoridad, no por nombres bonitos.

## 2. Sucesos esperados del pipeline

Se recomienda seguir este orden:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --offline --locked -- -D warnings
cargo check --all-targets --all-features --locked
cargo test --lib --all-features --locked
cargo test --all-features --locked --test production_surface --test quality_properties --test convergence_pipeline
cargo test --locked --all-features --test configuration_contracts
cargo check --locked --manifest-path fuzz/Cargo.toml --all-targets
```

**Importante:** no lanzar varios `cargo test` o `cargo build` en paralelo sobre el mismo checkout. `TIDEX_SOURCE_TREE_DIGEST` se congela en compile time (`env!()` en `receiver_compiler.rs`); builds concurrentes pueden desalinear binarios y artefactos congelados y provocar `frozen_receiver_compiler_source_mismatch` en `convergence_pipeline`. Usar un solo proceso de Cargo a la vez, o el target `make integration` que agrupa los tests de integración en una invocación.

## 3. Estructura de pruebas real

El proyecto no depende de una única suite global; su catálogo de pruebas está segmentado por dominio. Eso es una buena práctica en Rust cuando el repositorio tiene múltiples módulos con distintas responsabilidades.

Comandos útiles:

```bash
cargo test --lib --all-features
cargo test --all-features --test production_surface
cargo test --all-features --test quality_properties
cargo test --all-features --test convergence_pipeline
cargo test --all-features --test configuration_contracts
```

La clave es ser explícito: cuando se llama a `cargo test` con un filtro, la salida puede mostrar 0 tests para otros targets. Eso no indica fallo; indica que el filtro no coincide con ese conjunto.

`Cargo.toml` tiene `autotests = false`. Sólo se conservan tests externos registrados explícitamente; los antiguos tests huérfanos se retiraron o consolidaron en la autoridad canónica correspondiente.

GitHub Actions replica este pipeline, ejecuta Clippy con `-D warnings` y usa Rust 1.96.0, igual que `rust-toolchain.toml`. Las puertas de `quality/` añaden audit, fuzzing, sanitizadores y destinos temporales.

## 4. Criterio de calidad

Un cambio es aceptable cuando:

- compila con el árbol real del proyecto;
- pasa la validación de la librería principal;
- conserva la semántica de autoridad declarada;
- no fuerza capacidades operativas cuando el módulo sigue siendo advisory o experimental;
- deja evidencia verificable y receipts claros.

## 5. Política de mejora continua

La meta del repositorio no es una UI bonita ni una narración elegante. La meta es una arquitectura honesta:

- nombres con significado real,
- autoría explícita,
- workflows con uso concreto,
- evidencia y receipts,
- estados de ejecución fiables,
- validación repetible en CI.

Eso es lo que convierte un proyecto experimental en una base de ingeniería sólida.
