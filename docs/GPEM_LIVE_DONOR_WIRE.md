# Live GPEM donor wire (TIDE-X ↔ SHEI)

## How TIDE-X calls SHEI/GPEM

1. `GpemV2RecommendDonorWire::observe` (Rust, `src/capability/authenticated_capacity.rs`)
2. Spawns `tools/gpem_v2_recommend_donor.py` with JSON on stdin
3. Bridge loads SHEI `research_python` and invokes canonical interfaces:
   - `create_gpem(store_root, force_local=True)` (default) or `get_gpem(project_root=…)` when `TIDEX_GPEM_DONOR_MODE=get_gpem`
   - `GPEMService.recommend_v2(context)` → `GPEMServiceV2.recommend(context)`
4. Recommendations map to `DonorAction::{Select, Explore}` (route/prior alignment)
5. `seal_live_gpem_v2_recommend_capacity` / `acquire_procedure_selector_package` seal `AuthenticatedCapacity` with `DonorKind::GpemV2Recommend`

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
# From a Rust test or small harness:
#   wire.seed_demo_traces()?;
#   acquire_procedure_selector_package(store)?;
# Or: cargo test -p tidex --lib gpem_wire_live_recommend_seals_when_shei_available
#      cargo test -p tidex --lib productive_acquire_live_gpem_seals_when_shei_available
```

Empty demo store (`tidex demo procedure-selector`) fail-closes honestly until seeded.

## Paso 6

Wire exists. **Paso 6 remains NOT ACCEPTED** until full honest demo with seeded live GPEM + real second tick.
