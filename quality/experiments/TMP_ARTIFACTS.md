# Artefactos experimentales conservados fuera de /tmp

Esta nota es un log de una máquina concreta. Las rutas absolutas no viven en el repositorio y no son autoridad del runtime.

La copia persistente de los artefactos V65–V67, el snapshot anterior y las
exploraciones V69 está en:

`/home/yo/.local/share/tidex/research/preserved-tmp-20260908T174152Z`

Consultar `manifest.json`: los archivos incluidos se cotejan por SHA-256
contra el origen. Los logs son instantáneas y no autoridad científica.
Los originales de los modelos se mantienen para no romper recibos ni scripts.

## Integración con el código existente

Las exploraciones V69 quedan conservadas únicamente en el archivo externo indicado arriba. No forman parte del código ejecutable ni de la evidencia vigente de TIDE-X: trabajaban sobre una familia sintética y, por tanto, no satisfacen la política actual de evidencia real. No deben restaurarse como implementación, test, fallback ni fuente de autoridad.

Las capacidades que siguen vigentes viven en los módulos Rust canónicos de receiver/materialization y en los experimentos que ejecutan checkpoints y evidencia reales. Los recibos históricos V69 permanecen históricos; no autorizan afirmaciones actuales.

## Almacenamiento y limpieza

Código y tests en Git; pesos, adaptadores, deltas y recibos fuera del repositorio.
Los árboles de compilación referenciados por .cargo/config.toml y experimentos
no se eliminan mientras haya tests activos. No modificar recibos históricos para
cambiar sus rutas: conservar el origen y registrar el destino de la copia.

Los directorios antiguos de tests retirados de /tmp quedan en
`/home/yo/.local/share/tidex/research/tmp-quarantine-20260908`.
Su journal registra cada rename y permite restaurar la ruta de origen si está
libre. La cuarentena no libera espacio; no equivale a borrado definitivo.
