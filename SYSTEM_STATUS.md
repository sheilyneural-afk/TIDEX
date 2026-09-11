# Estado del proyecto

## Estado vigente

La base actual del proyecto está en una línea estable y verificable:

- la autoridad central sigue en Rust;
- la ejecución de backends reales queda acotada y validada;
- la capa de planificación y receipts está separada de la capa de ejecución externa;
- no se acepta evidencia que no tenga origen verificable;
- el backend HF se mantiene como ejecutor acotado, no como autoridad.

## Verificación reciente

Se ejecutó la comprobación real del proyecto:

```bash
cargo test --locked --offline --lib --quiet
```

Resultado verificado:

- 592 pruebas pasadas;
- 0 fallidas.

## Componentes activos y verificados

### Authority layer

- `KnowledgeEngine`: planifica, valida y emite receipts.
- `AdapterBank`: resuelve y autentica adaptadores.
- `AdaptiveStaircase`: coordina pasos reales sin inventar ejecución.

### Runtime cross-model

- backend Ollama con inferencia real;
- backend Candle con validación de archivos reales;
- backend HF con worker persistente y protocolo hashado;
- extracción de activaciones del layer solicitado;
- intervención de steering acotada y limpieza explícita.

### Seguridad y contrato

- rutas absolutas; validación de archivos y directorios;
- hashes SHA-256 para identidad y checks del runtime;
- schemas y request ids en cada request/response;
- fail-closed en invalid identity, mismatch, corrupt file, unsupported capability.

## Qué no está establecido

El proyecto no afirma todavía:

- que un steering cause transferencia útil de comportamiento;
- que exista una generalización universal entre arquitecturas;
- que una activación interna sea producción lista sin autorización final;
- que todos los backends sean iguales en capacidad o calidad;
- que la experimentación o el synthetic data puedan promocionarse a producción sin validación adicional.

Lo que sí está demostrado es la integridad del runtime, la capa de coordinación y la separación entre ejecución real y autoridad de decisión. Esa separación es una parte central del diseño del sistema y debe seguir visible en la documentación.

## Política actual

La política sigue siendo de evidencia antes de promoción:

- si no hay origen real, la evidencia no se acepta;
- si falta validación, la ejecución no avanza;
- si cambia la identidad del artifact, se rechaza;
- si la capa externa intenta gobernar por sí sola, la operación falla cerrada.

## Documentación base

- [README.md](README.md)
- [ARCHITECTURE.md](ARCHITECTURE.md)
- [IMPLEMENTATION.md](IMPLEMENTATION.md)
- [README_CROSS_MODEL.md](README_CROSS_MODEL.md)
