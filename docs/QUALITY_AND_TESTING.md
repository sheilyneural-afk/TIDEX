# Calidad y validación de TIDE-X

Tres capas: compilación real, lógica de autoridad/evidencia, suites explícitas por dominio.

**Ubicación:** `docs/QUALITY_AND_TESTING.md`  
**Relacionado:** [INDEX](INDEX.md) · [systems/quality-gates-and-ci](systems/quality-gates-and-ci.md) · [directories/tests](directories/tests.md) · [directories/quality](directories/quality.md) · auditorías `AUDITORIA_*`

## 1. Principios

- Declarar estado/autoridad/evidencia reales.
- Producción ≠ experimentación.
- Workflows ligados a necesidad observable.
- Ejecutar tras checks de autoridad, no por nombres.

## 2. Pipeline

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --offline --locked -- -D warnings
cargo check --all-targets --all-features --locked
cargo test --lib --all-features --locked
cargo test --all-features --locked --test production_surface --test quality_properties --test convergence_pipeline
cargo test --locked --all-features --test configuration_contracts
cargo check --locked --manifest-path fuzz/Cargo.toml --all-targets
```

**Importante:** un solo proceso Cargo a la vez (digest congelado del receiver compiler).

Detalle de gates: [systems/quality-gates-and-ci.md](systems/quality-gates-and-ci.md).

## 3. Estructura de pruebas

`autotests = false`. Externos registrados: ver [directories/tests.md](directories/tests.md).  
`tests/brain.rs` no existe en `8091cc8`.

## 4. Criterio de calidad

Compila; pasa lib + integración relevante; conserva semántica de autoridad; no fuerza módulos advisory a producción; deja receipts claros.

## 5. Auditorías

Informes históricos en `AUDITORIA_COMPLETA.md`, `AUDITORIA_FUNCIONAL.md`, `CONTRA_AUDITORIA_FUNCIONAL.md` — útiles como bitácora, no como scoreboard actual.
