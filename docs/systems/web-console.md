# Sistema: web-console

**Ubicación:** `web-console/`  
**Relacionado:** [operator-control-plane](operator-control-plane.md) · [directories/web-console](../directories/web-console.md) · [INDEX](../INDEX.md)

## Qué es

UI estática (“Interfaz del cerebro”) que habla con el operator en `127.0.0.1:8793`.

## Contenido

`index.html`, `app.js`, `styles.css`, `start.sh` (levanta UI + asume/`serve`).

## Invariante

La UI **no** es autoridad. Solo opera APIs allow-listed del control plane.

