# Live GPEM donor wire (TIDE-X ↔ SHEI)

## How TIDE-X calls SHEI/GPEM

1. `GpemV2RecommendDonorWire::observe` (Rust, `src/capability/authenticated_capacity.rs`)
2. Spawns `tools/gpem_v2_recommend_donor.py` with JSON on stdin
3. Bridge loads SHEI `research_python` and invokes canonical interfaces:
   - `create_gpem(store_root, force_local=True)` (default) or `get_gpem(project_root=…)` when `TIDEX_GPEM_DONOR_MODE=get_gpem`
   - `GPEMService.recommend_v2(context)` → `GPEMServiceV2.recommend(context)`
4. Recommendations map to `DonorAction::{Select, Explore}` (route/prior alignment)
5. `seal_live_gpem_v2_recommend_capacity` / `acquire_procedure_selector_package` seal `AuthenticatedCapacity` with `DonorKind::GpemV2Recommend`
6. Demo seed path: `GpemV2RecommendDonorWire::seed_demo_traces` / `seed_live_gpem_demo_store` → `GPEMServiceV2.ingest_payload`

**No GPEM copy inside TIDE-X.** Fixture selector remains unit-test only.

## Fail-closed

| Condition | Error |
|-----------|-------|
| Store marker `.tidex_gpem_force_unavailable` | `gpem_v2_recommend_donor_unavailable` |
| `TIDEX_GPEM_DONOR_MODE=force_unavailable` | same |
| Missing SHEI `research_python` / python / bridge | `…_unavailable` or `…_misconfigured` |
| Bridge/python failure | `gpem_v2_recommend_invoke_failed` |
| Live responses but no select+explore campaign | `gpem_v2_recommend_insufficient_live_evidence` |

Never: GPEM fails → `FixtureProcedureSelector` → continue.

## Live smoke (this machine)

```bash
export TIDEX_SHEI_ROOT=/home/yo/Projects/SHEI   # default if present
export TIDEX_HOME=/tmp/tidex-paso6-demo-home     # isolated operator home

# Full Paso 6 operator demo (seed → seal → Software stop → real B-loop):
cargo run --bin tidex -- demo procedure-selector

# Focused lib / bin tests:
cargo test -p tidex --lib seed_and_run_live_gpem_vertical_software_stop_when_shei_available
cargo test -p tidex --bin tidex demo_seeded_live_gpem_plus_real_b_loop_when_shei_available
cargo test -p tidex --lib productive_acquire_live_gpem_seals_when_shei_available
```

## Paso 6

**ACCEPTED (Software vertical)** on seeded live GPEM + honest Software residency stop + non-synthetic B-loop second tick.
Weights/Hybrid → real CapabilityIR + receptor remains **not** demonstrated (Paso 5 residual); Software stop is frozen-valid success for this vertical.
