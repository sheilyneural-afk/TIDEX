# Vxx → LearningExperimentEvidence admission

Fail-closed adapter: `src/learning/experimental_evidence_admission.rs`.

CLI: `tidex learning admit-vxx <session-id> <receipt.json> [--assimilate]`

## What this is

A converter from authenticated `quality/experiments` Vxx receipts into the
existing learning stack (`DeltaObservation` + `LearningExperimentEvidence` →
`assimilate_persistent_learning_evidence`). It does **not** invent a new
plasticity controller, ProceduralMemory JSON store, or capability-transfer claim.

## Supported schemas

| Schema | Dense + layout | Functional metrics mapped (exact order) |
|--------|----------------|-----------------------------------------|
| `tidex.v67_weight_actuator_smoke/v1` | required | `rust_direct_weight_actuation_established`, `v66_behavioral_delta_source_used` (bool→0/1) |
| `tidex.v68_receiver_response_probe/v1` | required on the wire | `pass` (bool→0/1), `correct_wrong_error_ratio` (finite f64) |

Pending aperture `capability_weights.len()` **must equal** the metric vector
length (2 for both mappings above). Otherwise admission fails closed.

## Required session context

- Live adaptive-learning session under the configured private root
- **Pending aperture** already issued (`adaptive_learning_cycle next` / equivalent)
- Fail-closed if missing: `adaptive_learning_no_pending_aperture`

## What maps

- Dense delta bytes → content-addressed `artifacts/deltas/by-sha/{sha}.dvec`
- `parameter_layout` → `state/parameter_layouts/by-sha/{content_sha}.json`
- `claim_boundary` → preserved verbatim in support JSON + observation confounders
- `observed_value` → **only** `dot(capability_weights, mapped_metrics)` (never invented)
- Negative outcomes (e.g. V68 `pass=false`) are valid experience

## Forbidden

- Inventing `observed_value` or functional responses not present on the receipt
- Promoting V68 into “semantic transfer established” / MBPP transfer / promotion
- Accepting V67 receipts that claim `universal_portability_established`,
  `authorizes_promotion`, or `trains_target_capability`
- Accepting V68 receipts that claim `new_semantic_capability_transfer_established`,
  `mbpp_transfer_established`, `general_language_preservation_established`, or
  `authorizes_promotion`
- V64-like (and any schema) **without** `dense_delta` + `parameter_layout`
- Broadening `procedural_replay` to Vxx in this pass

## Real collected receipts

`collected_receipts/tidex-v67-real-*.json` match the V67 schema. Admission still
requires the dense artifact file referenced by `dense_delta.path` (or an already
installed canonical private copy) plus a pending aperture whose weight dimension
is 2.
