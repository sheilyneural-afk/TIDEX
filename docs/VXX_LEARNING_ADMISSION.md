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

## Real V67 layout binding (legacy schema alias)

Collected `tidex-v67-real-*.json` receipts embed `parameter_layout.schema =
tidex.parameter_block_layout/v1` (current validate contract) while
`materialization.parameter_layout_sha256` was sealed under the pre-rename
string `cerebro.tidex.parameter_block_layout/v1` with **identical** blocks.
Admission accepts that alias only when rewriting the schema string recovers the
sealed digest; any other mismatch stays `vxx_admission_v67_layout_semantic_mismatch`.

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
- Mapping V67/V68 into `SolverAttempt` / `ProceduralMemory` (see boundary below)

## Boundary vs ProceduralMemory replay

V67/V68 stay on this LearningExperimentEvidence path. They do **not** carry
`AttemptBindings`, `Applicability`, `SolverConfiguration`, lineage, or sealed
research evaluation — the fields `SolverAttempt::seal` requires. Inventing
those would fabricate procedural attempts.

`src/learning/procedural_replay.rs` therefore fail-closes both schemas with
`procedural_replay_schema_learning_evidence_only`. Procedural replay accepts
only receipts that already embed sealed attempts:

- `tidex.numerical_evolution_receipt/v1`
- `tidex.operator_run_view/v1` (Operator run receipt + embedded stdout)
- `tidex.operator_job/v1` when `run` embeds that view

A bare `tidex.operator_run_receipt/v1` is path-only; the CLI wraps it as a run
view after loading stdout. Job evidence receipts are hashes only
(`procedural_replay_schema_no_attempt_structure`).

## Real collected receipts

`collected_receipts/tidex-v67-real-*.json` match the V67 schema. Admission still
requires the dense artifact file referenced by `dense_delta.path` (or an already
installed canonical private copy) plus a pending aperture whose weight dimension
is 2.


## Reproduce (real V67 on-disk receipt)

Requires the dense artifact still present at the absolute `dense_delta.path`
recorded in `collected_receipts/tidex-v67-real-*.json` (typically under
`/tmp/cerebro3-v67-real-…/artifacts/deltas/by-sha/…dvec`).

```bash
# private root (installer-owned 0700 directory)
export TIDEX_PRIVATE_ROOT=/path/to/private   # absolute, mode 0700
export TIDEX_HOME=/path/to/tidex-home

# 1) start adaptive session (2 capabilities = V67 mapped metric arity)
cat > /tmp/v67-learning-target.json <<'JSON'
{
  "target_id": "vxx-v67-real-cli",
  "capability_ids": [
    "v67.actuation_established",
    "v67.v66_behavioral_delta_source_used"
  ],
  "candidate_budget": 8,
  "plan_steps": 4,
  "noise_variance": 0.1,
  "cost_weight": 0.0,
  "risk_weight": 0.0
}
JSON
cat > /tmp/v67-learning-policy.json <<'JSON'
{
  "schema": "tidex.adaptive_learning_policy/v1",
  "outcome_utility_weight": 1.0,
  "maximize_observed_value": true
}
JSON

./tidex  # builds bins as needed; or cargo build --locked --offline --bin adaptive-learning-cycle --bin tidex
adaptive-learning-cycle start vxx-v67-real-cli /tmp/v67-learning-target.json /tmp/v67-learning-policy.json
adaptive-learning-cycle next vxx-v67-real-cli > /tmp/aperture-before.json

# 2) admit + assimilate real receipt (binds dense via path on receipt)
tidex learning admit-vxx vxx-v67-real-cli \
  collected_receipts/tidex-v67-real-11kb4b6z-receipt.json --assimilate

# 3) next aperture / strategy after assimilate
adaptive-learning-cycle next vxx-v67-real-cli > /tmp/aperture-after.json
adaptive-learning-cycle show vxx-v67-real-cli
```

Automated proof (skips only if dense file absent — never fabricates success):

```bash
# Real V67 sketches ~201M params — use --release (debug can take many minutes).
cargo test --locked --offline --release --test vxx_learning_admission -- --nocapture
cargo test --locked --offline --lib experimental_evidence_admission -- --nocapture
```

Lib unit `real_v67_receipt_admits_assimilates_and_changes_next_aperture` is opt-in (`TIDEX_RUN_REAL_V67=1`); prefer the release integration test above.

Relevant tests:

- `integration_real_v67_admit_assimilate_changes_next_aperture`
- `integration_v68_forbidden_transfer_claim_fails_closed`
- `real_v67_receipt_admits_assimilates_and_changes_next_aperture`
- `v68_transfer_claim_true_is_rejected`
- `v67_fixture_admits_and_assimilates_changing_next_aperture`
