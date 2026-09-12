# Runtime cross-model de TIDE-X

Este subsistema vive bajo `src/cross_model/` y se compila con la feature `cross-model-plasticity`. Amplía la base existente con ejecución real de modelos, observación de activaciones y validación de evidencia. No trata la respuesta del modelo, el nombre del modelo o un vector de activación como prueba de que se ha transferido una capacidad.

## Límite de autoridad

La ruta de producción separa cuatro tipos de autoridad:

1. inferencia conductual;
2. medición de activaciones internas;
3. materialización física de pesos;
4. autorización de producción.

Ninguna fase puede inventar la evidencia requerida por la siguiente. La autorización final sigue fuera del backend cross-model.

## Advertencia importante de honestidad

Este subsistema es real, pero no es un sistema autónomo ni un substituto de la autoridad. Lo que hace es:

- ejecutar un backend real;
- medir activaciones y salidas reales;
- aportar evidencia observada;
- dejar que la capa de autoridad de Rust decida si esa evidencia es suficiente para avanzar.

No conviene leer el runtime cross-model como una entidad que “decide por sí misma” ni como una prueba de que cualquier cambio de activación implica una mejor capacidad funcional. La mejora real exige validación adicional, bind a identidad, checks de contrato y decisión final de la autoridad.

## Backends reales

| Backend | Inferencia conductual | Activaciones internas | Intervención | Peso físico |
| --- | --- | --- | --- | --- |
| Ollama | sí | no | no | no |
| HF Transformers | sí | sí | sí | no |
| AdapterBank / weight actuator | no | no | no | sí |

Cada backend solo expone lo que realmente puede sostener físicamente.

## HF Transformers

El runtime HF crea un worker persistente local. La fuente de `src/cross_model/runtime/hf_worker.py` se embebe en el binario con `include_str!`. Rust valida:

- la ejecutable Python;
- el script del worker;
- el snapshot del modelo;
- los archivos `model.safetensors`, `config.json` y `tokenizer.json`;
- la identidad del modelo y la geometría reportada;
- los hashes y la metadata del runtime.

Esto permite que la ejecución real se mida y verifique sin caer en una autoridad delegada.

## Descubrimiento y evidencia

La lógica de benchmark usa evaluadores deterministas, no "llm-as-judge". La respuesta del modelo se compara con una verificación explícita y su identidad se liga a hashes.

Una diferencia observada positiva o un vector medido no basta por sí solo para convertir un ajuste local en capacidad efectiva. Deben mantenerse la validación, el bound conservador y la capa de promoción final.

## Capas clave

- `HierarchicalSteeringExtractor`: extrae dirección real de activaciones medidas.
- `CrossModelAligner`: aprende un mapado de calibración con separación train/validation.
- `LoRASynthesizer`: requiere un delta físico real, no un steering como si fuese un peso.
- `AdapterBank`: conserva la autoridad real para activación, revocación y rollback.

## Estado actual

La base actual demuestra que el sistema puede:

- ejecutar modelos reales;
- medir activaciones reales;
- instalar y limpiar steering real;
- validar identidad del backend y de los archivos;
- mantener una capa de receipts con procedencia observada;
- mantener la autoridad del sistema en Rust.

No demuestra por sí solo una transferencia útil universal entre architectures ni una promoción automática de producción.

## Verificación mínima

```bash
cargo test --locked --offline --lib --quiet
```

Para más contexto, revisa:

- [README.md](../README.md)
- [ARCHITECTURE.md](ARCHITECTURE.md)
- [SYSTEM_STATUS.md](SYSTEM_STATUS.md)
