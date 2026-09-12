# `src/receiver`

Perfilado y binding de receivers sobre familias de arquitectura.

**Ubicación:** `src/receiver/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

Compiler/layout/profiler/weight binding. Digest congelado del compiler es parte de la integridad.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `architecture_families.rs` | 301 | Evidence-based architecture and internal module-family fingerprinting. |
| `capability_discovery.rs` | 330 | Behavioral discovery evidence preceding CapabilityIR construction. |
| `checkpoint_adapter.rs` | 475 | Read-only, content-authenticated adapter for real SafeTensors checkpoints. It inspects headers and exact file bytes without loading tensor payloads. Unsupported or ambiguous enc… |
| `model_adaptation.rs` | 1112 | Authenticated profiling of concrete receiver checkpoints. A receiver profile binds a semantic model identity to exact checkpoint, configuration, tokenizer and physical parameter… |
| `receiver_compiler.rs` | 2878 | Receiver-specific compilation from canonical functional semantics. The numerical compiler is calibrated from capability-independent functional signatures and receiver-native sol… |
| `receiver_layout.rs` | 382 | Receiver-specific physical layout metadata above the reusable flat geometry. Capability identity never depends on this module. These contracts describe one receiver's machine-le… |
| `receiver_profile.rs` | 375 | Receiver facts and fail-closed planning for the capability runtime. This is intentionally a planning boundary. A plan is not an actuator and every plan is shadow-only; no type i… |
| `receiver_profiler.rs` | 117 | Authenticated receiver snapshot bindings. Architecture adapters may inspect a model format, but this core only accepts their resulting exact artifact commitments. It never treat… |
| `receiver_weight_binding.rs` | 3620 | Candidate-only binding from measured functional responses to real weights. Reuses the receiver compiler, ParameterBlockLayout, immutable dvec algebra and WeightActuator. It does… |
| `validation.rs` | 90 | API pública: fn validate_values, fn readout_dot_roundoff_bound, fn families_include, fn is_full_layer_coverage |
| `weight_actuator.rs` | 3160 | Direct model-weight materialization for SafeTensors checkpoints. This module is deliberately a data-plane actuator, not a capability compiler. It accepts only an already authent… |

## Árbol (archivos)

- `architecture_families.rs`
- `capability_discovery.rs`
- `checkpoint_adapter.rs`
- `mod.rs`
- `model_adaptation.rs`
- `receiver_compiler.rs`
- `receiver_layout.rs`
- `receiver_profile.rs`
- `receiver_profiler.rs`
- `receiver_weight_binding.rs`
- `validation.rs`
- `weight_actuator.rs`

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
