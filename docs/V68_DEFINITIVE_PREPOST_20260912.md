# V68 definitive pre/post (2026-09-12)

Fail-closed behavioral acquisition proof on a **real receptor** (SmolLM2-1.7B-Instruct), using the V68 measured-response delta path. No optimizer/backprop on the receiver. No LoRA. No fabricated MBPP scores.

## Claim boundary (authoritative)

From the authenticated receipt `claim_boundary`:

- `rust_generated_delta_from_measured_responses`: true
- `receiver_optimizer_steps`: 0
- `backpropagation_used`: false
- `fresh_process_checkpoint_execution`: true
- `donor_model_used`: false
- `new_semantic_capability_transfer_established`: **false**
- `mbpp_transfer_established`: **false**
- `general_language_preservation_established`: **false**
- `authorizes_promotion`: **false**

This run establishes **native-parameter response identification** (final RMSNorm scale → next-token logit-margin profile), not semantic skill transplant and not MBPP transfer.

## Receptor

- Model: `HuggingFaceTB/SmolLM2-1.7B-Instruct` snapshot `31b70e2e869a7173562077fd711b654946d38674`
- `model.safetensors` SHA-256: `f55217be716b6a997b97b9d8d7eb6fad02e00858f5010ec24f64603c3a98a0e8`
- Local path used: `runtime/llms/huggingface/hub/models--HuggingFaceTB--SmolLM2-1.7B-Instruct/snapshots/31b70e2e869a7173562077fd711b654946d38674`
- Hardware: CPU only (no GPU), torch 2.5.1+cpu

## Pipeline

1. Measure PRE baseline logit margins on 12 fixed prompts (4 probes + 4 risk controls + 4 unseen controls).
2. Build response-sensitivity basis on final `LlamaRMSNorm` (2048-d) via QR of probe sensitivities (no target values in basis construction).
3. Collect 12 native calibration observations by temporary coordinate interventions (restored afterward).
4. Rust `tidex receiver compile` produces dense candidate deltas for correct/wrong targets (no Python optimizer).
5. `tidex receiver materialize` writes standalone checkpoints.
6. Fresh Python process evaluates POST margins (`--replay-model`); inverse predictor is never the evaluator.

## PRE / POST numbers (receipt)

| Metric | Value | Gate |
| --- | --- | --- |
| `pass` | **true** | — |
| `actual_target_relative_error` | **9.214685309741161e-05** | ≤ 0.05 |
| `wrong_target_relative_error` | 1.9999084479326186 | — |
| `correct_wrong_error_ratio` | **21703.49155405716** | ≥ 4.0 |
| `unseen_control_margin_rms_change` | **0.012298796185432895** | ≤ 0.1 |

Requested probe response change: `[0.04, -0.03, 0.02, -0.01]`

Correct-arm actual probe change: `[0.03999805450439453, -0.030002593994140625, 0.019998550415039062, -0.009996414184570312]`

PRE baseline margins (12): see receipt `baseline_margins`.

## Artifacts

- Run root (outside git): `/tmp/tidex-v68-definitive-prepost-20260912T1535Z`
- Receipt in-repo: `quality/evidence/v68/receipt.json`
- Receipt copy: `collected_receipts/tidex-v68-definitive-prepost-20260912T1535Z-receipt.json`
- Run pointer: `quality/evidence/v68/run_pointer.json`
- Frozen compiler used: `/tmp/tidex-v68-frozen-bin/tidex` (`736d7d6273ed5a5f0ccfe7ea27df0d2ac2517d8a84918e7f2a2c32681dc227fb`)

## Engineering notes from this session

1. Base checkpoint was missing from the broken HF cache symlink (`~/.cache/huggingface/hub` → nonexistent `cerebro3-runtime/...`). Restored by downloading the pinned Instruct snapshot into `runtime/llms/huggingface/hub`.
2. First attempt failed: `tidex receiver materialize` exceeded the probe’s 240s subprocess timeout on CPU. Probe timeouts raised to 3600s (criteria unchanged).
3. Second attempt evaluated both arms then aborted integrity check because a concurrent `cargo build --release` rewrote the compiler binary mid-run. Third attempt used a frozen copy outside the cargo target dir and produced the receipt above.
4. V66 LoRA-MBPP adapter exists under `/tmp/tidex-v66-compiled-adapter` but was **not** used as the transfer mechanism (per policy).
5. `/tmp/tidex-v67-direct-smollm/model.safetensors` is a modified 3.8GB weight (SHA ≠ BASE_SHA); not a valid V68 base.

## What this does *not* unblock

Semantic donor→receiver capability transfer / MBPP acquisition still requires a separate verifiable capability protocol with honest claim_boundary. V68 here only proves measured local response control on the receptor without training it.

## Learning admission (plasticity wire)

Top-level `dense_delta` + `parameter_layout` are sourced **only** from existing correct-arm artifacts (never invented):

- `dense_delta` ← `arms.correct.candidate` → `tidex.receiver_weight_candidate/v1.dense_delta` (2048-d `.dvec` under `artifacts/deltas/by-sha/`)
- `parameter_layout` ← that candidate’s basis (`tidex.receiver_weight_basis/v1.layout`, `model.norm.weight` 2048)

The probe (`quality/experiments/v68_receiver_response_probe.py`) now emits these on successful standalone evaluation via `admission_wire_from_correct_arm` (fail-closed if candidate/basis/dvec missing). In-repo evidence copies were re-sealed from the same live run paths/hashes; `claim_boundary` semantics are unchanged (`new_semantic_capability_transfer_established=false`, no MBPP/promotion).

```bash
export TIDEX_PRIVATE_ROOT=/path/to/private   # absolute, mode 0700
adaptive-learning-cycle start vxx-v68-definitive-prepost-20260912 \
  /tmp/v68-learning-target.json /tmp/v68-learning-policy.json
adaptive-learning-cycle next vxx-v68-definitive-prepost-20260912
tidex learning admit-vxx vxx-v68-definitive-prepost-20260912 \
  /tmp/tidex-v68-definitive-prepost-20260912T1535Z/receipt.json --assimilate
adaptive-learning-cycle next vxx-v68-definitive-prepost-20260912
```

Automated proof: `integration_real_v68_admit_assimilate_changes_next_aperture` (skips only if the absolute `.dvec` is absent).
