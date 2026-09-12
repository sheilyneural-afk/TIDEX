# `src/learning`

Aprendizaje adaptativo, evidencias, portfolio y memoria procedimental.

**Ubicación:** `src/learning/`  
**Relacionado:** [src](src.md) · [INDEX](../INDEX.md)

## Para qué está

Conecta observación → candidatos → evidencias selladas sin auto-promocionar.

## Mapa de archivos

| Archivo | Líneas | Qué hace (desde docs del código) |
|---------|--------|----------------------------------|
| `experimental_evidence_admission.rs` | Fail-closed Vxx receipt → `LearningExperimentEvidence` / `DeltaObservation` (see `docs/VXX_LEARNING_ADMISSION.md`) |
| `causal_credit.rs` | 556 | Return the conservative, evidence-backed causal utility weight for every runtime field in the caller's canonical order. The lower confidence bound, rather than the point estimat… |
| `learning_finalization.rs` | 704 | Canonical hand-off from a completed adaptive-learning session to the TIDE-X reconstruction authority. This module deliberately does not reconstruct, replace an active corpus, co… |
| `learning_orchestrator.rs` | 1923 | Explicit policy for a real sequential learning cycle. The policy is kept separate from `LearningTarget` so an offline prospective design remains a pure information-design artifa… |
| `memory.rs` | 384 | Deprecated external persistence entrypoint. Memory is authority state: it can only be materialized as part of the receipt-bound engine transaction, where its digest is bound to … |
| `numerical_evolution.rs` | 1931 | End-to-end numerical candidate evolution for TIDE-X. This module connects the bounded least-squares portfolio, paired holdout evaluation, PETFC trajectory governance, and adviso… |
| `portfolio_governance.rs` | 5005 | Universal governance for candidate evolution in TIDE-X. This module derives conservative candidate decisions from paired raw observations. It never executes, promotes, or activa… |
| `procedural_memory.rs` | 4542 | Universal, advisory-only procedural experience memory. This module remembers authenticated solver attempts without introducing a second authority or persistence layer. [`Procedu… |
| `procedural_replay.rs` | Replay canónico fail-closed: `rebuild_from_authenticated_receipt` despacha por schema (numerical.evolve / operator run view / operator job). V67/V68 → LearningExperimentEvidence only. **No** `procedural_memory.json`. |
| `representation_evidence.rs` | 915 | Immutable installation of sealed, task-agnostic representation evidence. This module deliberately does not update `state/observations`. It turns a sealed capture plus fresh, sta… |
| `sleep_diagnostics.rs` | 182 | API pública: enum ConsolidationEventKind, struct ConsolidationEvent, struct SleepConsolidationDiagnostics, fn diagnose_consolidation |
| `sleep_evidence.rs` | 2031 | API pública: const NONINFERIORITY_95_Z, const MAX_SLEEP_EVIDENCE_JSON_BYTES, struct ProtectionEvidenceSummary, struct InteractionEvidenceSummary |
| `solver_portfolio.rs` | 3607 | Bounded, architecture-independent least-squares solver portfolio. Backends in this module only propose numerical candidates. They cannot attest evidence, sign a result, promote … |

## Árbol (archivos)

- `causal_credit.rs`
- `experimental_evidence_admission.rs`
- `learning_finalization.rs`
- `learning_orchestrator.rs`
- `memory.rs`
- `mod.rs`
- `numerical_evolution.rs`
- `portfolio_governance.rs`
- `procedural_memory.rs`
- `procedural_replay.rs`
- `representation_evidence.rs`
- `sleep_diagnostics.rs`
- `sleep_evidence.rs`
- `solver_portfolio.rs`

## Cómo leerlo

1. Empieza por `mod.rs` (re-exports / feature gates).
2. Lee el `//!` del archivo ancla (arriba en la tabla).
3. Cruza con el sistema correspondiente en `docs/systems/` si es operator/governance/learning/cross-model.
