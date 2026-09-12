# Sistema: learning y plasticidad

**Ubicación:** `src/learning/` + `src/cross_model/plasticity*` (feature) + `/api/plasticity`  
**Relacionado:** [src-learning](../directories/src-learning.md) · [cross-model-runtime](cross-model-runtime.md) · [INDEX](../INDEX.md)

## Qué es

Ciclo de aprendizaje adaptativo, evidencias de representación, memoria procedimental advisory, portfolio numérico, y (con feature) plasticidad cross-model / consejo operator.

## Piezas

| Módulo | Rol |
|--------|-----|
| `learning_orchestrator.rs` | Política y sesión de learning secuencial real |
| `experimental_evidence_admission.rs` | Admisión fail-closed Vxx → evidencia de learning ([VXX_LEARNING_ADMISSION](../VXX_LEARNING_ADMISSION.md)) |
| `learning_finalization.rs` | Hand-off a autoridad de reconstrucción (no reconstruye aquí) |
| `representation_evidence.rs` | Instala evidencia sellada task-agnostic |
| `numerical_evolution.rs` | Evolución de candidatos least-squares + holdout |
| `portfolio_governance.rs` | Decisiones conservadoras desde observaciones pareadas |
| `procedural_memory.rs` | Memoria de intentos de solver, advisory-only |
| `causal_credit.rs` | Pesos de prioridad causal con lower confidence bound |
| Operator `/api/plasticity` | Plasticidad v2 advisory (BCM, eligibility, neuromod, PI); puede `available:false` |

## Invariantes

- Memoria no es segunda autoridad: bindings van en la transacción del engine.
- Plasticidad del operator **no** activa producción.
- Feature `cross-model-plasticity` separa el runtime HF/NNsight del núcleo siempre presente.

