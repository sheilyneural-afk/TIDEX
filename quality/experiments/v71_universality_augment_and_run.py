#!/usr/bin/env python3
"""Augment V70 trials to reach required sample sizes and run measurement.

Modes:
  --simulate N   : duplicate existing held-out trials until each capability has N trials (fast, synthetic)
  --measure      : also invoke `cargo run --bin tidex -- measure universality` on output

This script is for validating the statistical design (how many trials required)
and for optionally orchestrating later real materialization runs.
"""

from __future__ import annotations
import json
import subprocess
import sys
from pathlib import Path
from copy import deepcopy

ROOT = Path(__file__).resolve().parents[2]
V64_RECEIPT = ROOT / "quality" / "evidence" / "v64" / "receipt.json"
OUTPUT = ROOT / "quality" / "experiments" / "universality_augmented.json"


def clamp01(value: float) -> float:
    return max(0.0, min(1.0, float(value)))


def make_trial_from_row(capability: str, receiver_id: str, family: str, seed: int, row: dict) -> dict:
    target_score = clamp01(row.get("recovered_gain", 0.0))
    preservation_score = clamp01(
        1.0 - (row.get("receiver_tidex_mse", 0.0) / max(row.get("receiver_virgin_mse", 1e-9), 1e-9))
    )
    wrong_ir_score = clamp01(1.0 - row.get("receiver_wrong_skill_mse", 1.0))
    random_delta_score = clamp01(1.0 - (row.get("receiver_virgin_mse", 1.0) * 0.5))
    unmodified_receiver_score = clamp01(1.0 - row.get("receiver_virgin_mse", 1.0))

    task_family = {
        "identity": "state_identity",
        "swap": "state_swap",
        "sign_x": "symbolic_sign_x",
        "sign_y": "symbolic_sign_y",
    }.get(capability, "generic_task_family")

    return {
        "schema": "cerebro.tidex.universality_trial/v1",
        "trial_id": f"{capability}:{receiver_id}:{seed}",
        "capability_id": capability,
        "task_family_id": task_family,
        "receiver_id": receiver_id,
        "receiver_family_id": family,
        "seed": seed,
        "capability_was_calibration": False,
        "receiver_was_calibration": False,
        "target_optimizer_steps": 0,
        "target_score": target_score,
        "preservation_score": preservation_score,
        "wrong_ir_score": wrong_ir_score,
        "random_delta_score": random_delta_score,
        "unmodified_receiver_score": unmodified_receiver_score,
    }


def build_augmented_input(target_n: int) -> dict:
    receipt = json.loads(V64_RECEIPT.read_text())
    rows = {row["skill"]: row for row in receipt["rows"]}

    calibration_capabilities = ["contract_mix", "mix"]
    held_out_capabilities = ["identity", "swap", "sign_x", "sign_y"]

    trials: list[dict] = []
    # include calibration as before
    for calib in calibration_capabilities:
        trials.append({
            "schema": "cerebro.tidex.universality_trial/v1",
            "trial_id": f"calibration:{calib}",
            "capability_id": calib,
            "task_family_id": "calibration_family",
            "receiver_id": "receiver.seen.calibration",
            "receiver_family_id": "family.calibration",
            "seed": 0,
            "capability_was_calibration": True,
            "receiver_was_calibration": True,
            "target_optimizer_steps": 0,
            "target_score": 1.0,
            "preservation_score": 1.0,
            "wrong_ir_score": 0.0,
            "random_delta_score": 0.0,
            "unmodified_receiver_score": 0.0,
        })

    # take existing held-out rows and duplicate them with new seeds/receivers
    base_seed_offset = 1000
    for capability in held_out_capabilities:
        if capability not in rows:
            raise RuntimeError(f"capability {capability} not present in V64 receipt")
        row = rows[capability]
        # existing exemplar receivers/families; we'll clone those patterns
        families = ["family.a", "family.b"]
        seeds_per_family = target_n // len(families)
        extra = target_n - (seeds_per_family * len(families))
        seed_counter = 0
        for fi, family in enumerate(families):
            count = seeds_per_family + (1 if fi < extra else 0)
            for i in range(count):
                seed = base_seed_offset + seed_counter
                receiver_id = f"receiver.{family.split('.')[-1]}.{capability}:{seed}"
                trials.append(make_trial_from_row(capability, receiver_id, family, seed, row))
                seed_counter += 1

    protocol = {
        "schema": "cerebro.tidex.universality_protocol/v1",
        "minimum_calibration_capabilities": 1,
        "minimum_held_out_capabilities": 2,
        "minimum_receivers_per_capability": 2,
        "minimum_receiver_families_per_capability": 2,
        "minimum_seeds_per_capability": target_n,
        "minimum_task_families_per_capability": 1,
        "scope": "held_out_capability_generalization",
        "require_unseen_receiver": True,
        "minimum_target_score": 0.995,
        "minimum_preservation_score": 0.995,
        "minimum_identity_margin": 0.2,
        "minimum_success_probability": 0.75,
        "confidence_z": 1.96,
    }

    return {
        "schema": "cerebro.tidex.universality_evidence_input/v1",
        "calibration_capabilities": calibration_capabilities,
        "trials": trials,
        "protocol": protocol,
    }


def main() -> int:
    if "--simulate" not in sys.argv:
        print("Usage: --simulate N [--measure]")
        return 2
    try:
        idx = sys.argv.index("--simulate")
        n = int(sys.argv[idx + 1])
    except Exception:
        print("--simulate requires an integer N")
        return 2

    payload = build_augmented_input(n)
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"Wrote {OUTPUT} (simulated N={n} per capability)")

    if "--measure" in sys.argv:
        cmd = [
            "cargo",
            "run",
            "--bin",
            "tidex",
            "--",
            "measure",
            "universality",
            str(OUTPUT),
        ]
        print("$ " + " ".join(cmd))
        result = subprocess.run(cmd, cwd=ROOT)
        return result.returncode
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
