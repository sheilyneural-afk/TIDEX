# `src/cross_model`

Runtime cross-model bajo feature `cross-model-plasticity`.

**Ubicación:** `src/cross_model/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

Solo se compila con la feature. Incluye discovery/extraction/integration/models/plasticity/promotion/runtime. Ver sistema cross-model. Candle local fue retirado; HF/NNsight es la vía certificada actual.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `plasticity_daemon.rs` | 195 | Production cross-model evidence daemon. Usage: plasticity-daemon once <runtime.json> <benchmark.json> plasticity-daemon loop <runtime.json> <benchmark.json> <interval-seconds> R… |
| `plasticity_engine.rs` | 502 | Evidence-governed cross-model orchestrator. The engine separates four phases that earlier code conflated: behavioral discovery, internal evidence acquisition, calibrated transfo… |
| `bidirectional_loop.rs` | 209 | Evidence-driven co-evolution history reducer. A cycle is recorded only after a real discovery/evaluation round. This module does not mutate models, fabricate discoveries, or ass… |
| `consensus_builder.rs` | 345 | Fail-closed consensus reducer for candidate governance. Consensus is not production activation authority. It binds an exact proposal payload, an explicit voter set, immutable vo… |
| `mod.rs` | 16 | Co-evolution module This module contains components for implementing co-evolution between models and capabilities in the cross-model system. |
| `domain_analyzer.rs` | 196 | Domain profiles derived only from verified benchmark evaluations. |
| `emergent_detector.rs` | 170 | Emergence analysis over measured model-scale observations. No score is inferred from model names or prompt length. The detector only consumes authenticated `ModelEvaluation` val… |
| `gap_detector.rs` | 296 | Paired behavioral gap detection with distribution-free confidence bounds. |
| `mod.rs` | 333 | Verified behavioral capability discovery. A model is never scored from activation magnitude, response length, model name, or another heuristic. Every score is produced by an exp… |
| `prioritizer.rs` | 163 | Evidence-only prioritization of verified capability gaps. |
| `proposal_generator.rs` | 222 | Governance proposals derived from verified discovery evidence. A behavioral gap is not itself a transferable artifact. The first allowed proposal stage is therefore evidence acq… |
| `counterfactual_analyzer.rs` | 272 | Executed counterfactual analysis. The caller supplies the intervention and verifier. TIDE-X executes both prompts on the same runtime and measures the change; it never invents m… |
| `cross_model_aligner.rs` | 367 | Calibrated cross-model activation alignment. Alignment is learned only from paired measured activations. No dimension padding, truncation, interpolation, layer-ratio scaling, or… |
| `hierarchical_steering_extractor.rs` | 322 | Steering extraction from measured internal activations. |
| `lora_synthesizer.rs` | 246 | Verified low-rank/LoRA factorization of an already compiled weight delta. This module does not convert an activation steering vector into weights. It delegates numerical factori… |
| `mod.rs` | 22 | Capability extraction and calibrated cross-model transformation. |
| `activation_steering_bridge.rs` | 40 | Thin adapter to the canonical activation-steering materializer. |
| `adapter_bank_bridge.rs` | 64 | Thin adapter to the canonical governed adapter bank. |
| `capability_discovery_bridge.rs` | 126 | Bridge from TIDE-X's canonical discovery report to cross-model metadata. |
| `causal_credit_bridge.rs` | 25 | Thin adapter to the canonical causal-credit reducer. |
| `ledger_bridge.rs` | 60 | Thin adapter to the canonical hash-chained ledger. |
| `mod.rs` | 24 | Integration bridges module This module contains bridges to existing TIDE-X systems for seamless integration with the cross-model system. |
| `pythagoras_bridge.rs` | 24 | Thin adapter to the canonical persistent-topology implementation. |
| `shadow_evaluation_bridge.rs` | 32 | Thin adapter to the canonical isolated shadow evaluator. |
| `temporal_tracking_bridge.rs` | 25 | Thin adapter to the canonical SBAS temporal reconstruction. |
| `weight_tomography_bridge.rs` | 29 | Thin adapter to TIDE-X's canonical weight tomography. |
| `llama.rs` | 34 | Llama-family behavioral runtime backed by a real Ollama model. |
| `mistral.rs` | 34 | Mistral-family behavioral runtime backed by a real Ollama model. |
| `mod.rs` | 1556 | Real behavioral model runtimes for cross-model evidence acquisition. The Ollama backend is used only for behavior that it actually exposes. Hidden activations and model mutation… |
| `qwen.rs` | 34 | Qwen-family behavioral runtime backed by a real Ollama model. |
| `traits.rs` | 595 | Evidence-preserving model contracts for cross-model operations. Behavioral inference, internal activations, and physical intervention are distinct authorities. A backend may exp… |
| `bcm_metaplasticity.rs` | 160 | BCM-style adaptive threshold controller over normalized measured signals. This is a numerical controller only. It never claims to modify model weights. |

## Árbol (archivos)

- `co_evolution/bidirectional_loop.rs`
- `co_evolution/consensus_builder.rs`
- `co_evolution/mod.rs`
- `discovery/domain_analyzer.rs`
- `discovery/emergent_detector.rs`
- `discovery/gap_detector.rs`
- `discovery/mod.rs`
- `discovery/prioritizer.rs`
- `discovery/proposal_generator.rs`
- `extraction/counterfactual_analyzer.rs`
- `extraction/cross_model_aligner.rs`
- `extraction/hierarchical_steering_extractor.rs`
- `extraction/lora_synthesizer.rs`
- `extraction/mod.rs`
- `integration/activation_steering_bridge.rs`
- `integration/adapter_bank_bridge.rs`
- `integration/capability_discovery_bridge.rs`
- `integration/causal_credit_bridge.rs`
- `integration/ledger_bridge.rs`
- `integration/mod.rs`
- `integration/pythagoras_bridge.rs`
- `integration/shadow_evaluation_bridge.rs`
- `integration/temporal_tracking_bridge.rs`
- `integration/weight_tomography_bridge.rs`
- `mod.rs`
- `models/llama.rs`
- `models/mistral.rs`
- `models/mod.rs`
- `models/qwen.rs`
- `models/traits.rs`
- `plasticity/bcm_metaplasticity.rs`
- `plasticity/content_plasticity.rs`
- `plasticity/eligibility_traces.rs`
- `plasticity/elo_system.rs`
- `plasticity/mod.rs`
- `plasticity/neuromodulation.rs`
- `plasticity/pi_controller.rs`
- `plasticity/routing_plasticity.rs`
- `plasticity_daemon.rs`
- `plasticity_engine.rs`

_(+7 entradas)_

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
