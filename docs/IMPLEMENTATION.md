# Guía de implementación de TIDE-X

Reglas prácticas para cambiar el crate sin romper autoridad ni evidencia.

**Ubicación:** `docs/IMPLEMENTATION.md`  
**Relacionado:** [INDEX](INDEX.md) · [ARCHITECTURE](ARCHITECTURE.md) · [directories/](directories/) · [systems/](systems/) · [QUALITY_AND_TESTING](QUALITY_AND_TESTING.md)

## 1. Reglas

- Autoridad en Rust; workers externos no deciden ni promueven.
- Evidencia observada y hasheada; receipts obligatorios en rutas relevantes.
- Fail-closed ante validación rota.
- No mocks/stubs/sintéticos presentados como evidencia de producción.

## 1.1 Honestidad de evidencia

Antes de documentar una mejora: ¿capa producción o experimental? ¿origen hasheado? ¿alguien interpreta salida experimental como autoridad?

## 2. Build rápido

```bash
make ci
# equivalente a fmt + clippy -D warnings + check + test --lib + integration + configuration_contracts + fuzz check
```

Toolchain: `rust-toolchain.toml` **1.96.0**. `Cargo.toml` declara `rust-version = "1.96"` y `publish = false`.

Mapa de código: [directories/src.md](directories/src.md). Control plane: [systems/operator-control-plane.md](systems/operator-control-plane.md).

## 3. Flujo recomendado

1. Reproducir en Rust.
2. Prueba que cubra el caso.
3. Cambio mínimo con contratos.
4. Validar identidad/hashes/schemas.
5. No presentar resultado experimental como evidencia final.

## 4. Backend HF / cross-model

Ver [README_CROSS_MODEL.md](README_CROSS_MODEL.md) y [systems/cross-model-runtime.md](systems/cross-model-runtime.md). Operaciones de worker: `generate`, `activation`, `set_steering`, `clear_steering`, `shutdown`.

Identidad de modelo: [systems/model-identity-and-catalog.md](systems/model-identity-and-catalog.md).

## 5. Documentación de directorios/sistemas

Al tocar un dominio, actualizar el doc correspondiente bajo `docs/directories/` o `docs/systems/` y el enlace en [INDEX.md](INDEX.md).
