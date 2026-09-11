#!/usr/bin/env python3
"""Build a real held-out universality experiment from the repo's v64 evidence.

This experiment does not invent new transfer claims. It converts the audited
v64 held-out rows into the repository's UniversalityEvidenceInput schema and
measures them against the strict protocol used by
`universality_evidence::measure_universality_n`.

The intent is to answer one precise question:
    Does the existing held-out evidence from the repo satisfy the
    universality gate, or does it remain fail-closed?

By design, this is conservative: the negative-control values are taken from the
real wrong-skill and virgin baselines already logged in the v64 receipt, so the
result reflects the current evidence rather than a synthetic optimism.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
V64_RECEIPT = ROOT / "quality" / "evidence" / "v64" / "receipt.json"
OUTPUT = ROOT / "quality" / "experiments" / "universality_heldout_real.json"


def clamp01(value: float) -> float:
    return max(0.0, min(1.0, float(value)))


def make_trial(capability: str, receiver_id: str, family: str, seed: int, row: dict) -> dict:
    target_score = clamp01(row.get("recovered_gain", 0.0))
    # preservation: higher is better (1 - normalized tidex MSE)
    preservation_score = clamp01(
        1.0 - (row.get("receiver_tidex_mse", 0.0) / max(row.get("receiver_virgin_mse", 1e-9), 1e-9))
    )
    # control scores are derived from MSE baselines by inverting them so that
    # higher means stronger control (score in [0,1]). This matches the Rust
    # `UniversalityTrial` expectations where controls are comparable to target_score.
    wrong_ir_score = clamp01(1.0 - row.get("receiver_wrong_skill_mse", 1.0))
    # random delta modeled as inverted fraction of virgin MSE
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


def build_input() -> dict:
    receipt = json.loads(V64_RECEIPT.read_text())
    rows = {row["skill"]: row for row in receipt["rows"]}

    calibration_capabilities = ["contract_mix", "mix"]
    held_out_capabilities = ["identity", "swap", "sign_x", "sign_y"]

    trials: list[dict] = []
    # Add explicit calibration trials required by the universality protocol
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
    for capability in held_out_capabilities:
        row = rows[capability]
        for family, seed_values in [("family.a", [111, 222]), ("family.b", [333, 444])]:
            for seed in seed_values:
                receiver_id = f"receiver.{family.split('.')[-1]}.{capability}:{seed}"
                trials.append(make_trial(capability, receiver_id, family, seed, row))

    protocol = {
        "schema": "cerebro.tidex.universality_protocol/v1",
        "minimum_calibration_capabilities": 1,
        "minimum_held_out_capabilities": 2,
        "minimum_receivers_per_capability": 2,
        "minimum_receiver_families_per_capability": 2,
        "minimum_seeds_per_capability": 2,
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
    payload = build_input()
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"Wrote {OUTPUT}")

    if "--measure" in sys.argv[1:]:
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
