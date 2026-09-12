# Sistema: receiver y materialization

**Ubicación:** `src/receiver/` + `src/materialization/`  
**Relacionado:** [src-receiver](../directories/src-receiver.md) · [src-materialization](../directories/src-materialization.md) · [INDEX](../INDEX.md)

## Qué es

Cómo una capacidad/adaptación se **perfilá** sobre una familia de arquitectura (receiver) y cómo se **materializa** en sombras/steering evaluables sin confundir eso con promoción productiva.

## Receiver

Compiler, layout, profiler, weight binding, checkpoint adapter, model adaptation, architecture families. Digest de compiler congelado: mismatches → errores de integridad (`frozen_receiver_compiler_source_mismatch` en pipelines de convergencia).

## Materialization

Pipeline + selector de backend; materializers dense/low-rank/sparse/shadow; activation steering; shadow evaluation; universal capability compiler; universality evidence. Políticas en `config/materialization/*.json`.

## Invariante

Materializar un candidato ≠ autorizar su activación en producción (eso pasa por governance / AdapterBank).

