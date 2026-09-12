# `tests/`

Tests de integración / propiedades / contratos **fuera** de `src/` (crate tests).

**Ubicación:** `tests/`  
**Relacionado:** [QUALITY_AND_TESTING](../QUALITY_AND_TESTING.md) · [INDEX](../INDEX.md)

## Contenidos

| Archivo | Rol |
|---------|-----|
| `configuration_contracts.rs` | Contratos de `config/` |
| `convergence_pipeline.rs` | Pipeline de convergencia / integridad de digests |
| `production_surface.rs` | Superficie de producción |
| `quality_properties.rs` | Propiedades de calidad |
| `support/` | Helpers compartidos |

## Nota histórica

`tests/brain.rs` fue eliminado en el reorg: pérdida de red de seguridad sobre analysis/phantom/sleep. Restaurar o reemplazar sigue abierto si se quiere paridad de cobertura.
