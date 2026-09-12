# Sistema: identidad de modelo y catálogo

Cómo un snapshot local estilo Hugging Face se convierte en un `model_id: Sha256Digest` catalogado y referenciable por jobs/workflows.

**Ubicación:** `src/operator/control_plane.rs` — `model_candidate_identity`, `discover_local_models`, `catalog_local_models`, `load_catalog_model`  
**Relacionado:** [operator-control-plane](operator-control-plane.md) · [INDEX](../INDEX.md)

## Propósito de autoridad

El `model_id` es identidad de **artefacto** (bytes), no nickname de carpeta ni revision HF “de nombre”.

## Algoritmo content-bound (vigente)

`model_candidate_identity`:

1. Hashea bytes de `config.json` y `tokenizer.json` (`sha256_file`).
2. Según `LocalModelLayout`:
   - **single**: `model.safetensors`
   - **sharded**: `model.safetensors.index.json` + todos los `*.safetensors` ordenados
3. `Sha256Digest::digest_domain(b"TIDEX:OPERATOR-MODEL-CANDIDATE:v2\0", serde(layout, files))`.

Discovery es **read-only**. `confine_model_scan_root` + `hub_snapshot_file` impiden escapes fuera del hub.

## Invariantes verificadas en test

`model_id_is_content_bound_and_stale_catalog_identity_is_rejected`:

- mismo snapshot, mismos bytes → mismo `model_id` tras rescan;
- mutar bytes de `model.safetensors` → **nuevo** `model_id`;
- el id antiguo **deja de cargar** (`load_catalog_model` falla).

**No hay** `OperatorModelAlias` / aliases de compatibilidad en el árbol actual.

## Catálogo

- `catalog_local_models` escribe `operator/models/by-sha/{model_id}.json`.
- `validate_catalog_model_candidate` recomputa identity; mismatch → `operator_model_catalog_content_changed`.

## Síntoma histórico (ya explicado)

Jobs con IDs viejos tras wipe/rescan/cambio de dominio → `invalid:operator_model_not_cataloged`. No se “arregla” con aliases: se re-escanea y se usan IDs content-bound actuales.
