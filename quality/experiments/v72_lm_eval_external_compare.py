#!/usr/bin/env python3
"""Compare the internal TIDE-X universality gate with the external LM Eval Harness.

This layer is intentionally separate from the admission protocol. The Rust
`universality_evidence` module remains the source of truth for whether a
capability is accepted. This script provides a public benchmark comparison layer
using the EleutherAI LM Evaluation Harness when it is available.

Operational model:
  - Internal gate: evidence-driven, fail-closed, used for admission.
  - External harness: benchmark-oriented, used for comparison and reporting.

The script supports dry-run generation of a benchmark plan and optional real
execution if the harness environment is installed.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Iterable

ROOT = Path(__file__).resolve().parents[2]
HARNESS_ROOT = ROOT.parent / "lm-evaluation-harness"
EXTERNAL_REPORT = ROOT / "quality" / "experiments" / "external_lm_eval_summary.json"
INTERNAL_REPORT = ROOT / "quality" / "experiments" / "universality_augmented.json"


def clamp01(value: float) -> float:
    return max(0.0, min(1.0, float(value)))


def read_internal_summary() -> dict:
    payload = {"status": "not_available"}
    if INTERNAL_REPORT.exists():
        try:
            data = json.loads(INTERNAL_REPORT.read_text())
            protocol = data.get("protocol", {})
            trials = data.get("trials", [])
            payload = {
                "status": "available",
                "capability_count": len({t.get("capability_id") for t in trials if t.get("capability_id")}),
                "trial_count": len(trials),
                "minimum_held_out_capabilities": protocol.get("minimum_held_out_capabilities", 0),
                "minimum_seeds_per_capability": protocol.get("minimum_seeds_per_capability", 0),
                "scope": protocol.get("scope", "unknown"),
            }
        except Exception:
            payload = {"status": "invalid_json"}
    return payload


def harness_python() -> str:
    venv_python = HARNESS_ROOT / ".venv" / "bin" / "python"
    if venv_python.exists():
        return str(venv_python)
    return sys.executable


def build_command(model: str, tasks: Iterable[str], extra_args: Iterable[str] = ()) -> list[str]:
    cmd = [
        harness_python(),
        "-m",
        "lm_eval",
        "--model",
        model,
        "--tasks",
        ",".join(tasks),
    ]
    cmd.extend(extra_args)
    return cmd


def dry_run_plan(model: str, tasks: list[str], extra_args: list[str]) -> dict:
    internal = read_internal_summary()
    harness_present = HARNESS_ROOT.exists()
    cmd = build_command(model, tasks, extra_args)
    return {
        "schema": "tidex.external_lm_eval_plan/v1",
        "internal_gate": {
            "source": str(INTERNAL_REPORT),
            "summary": internal,
            "policy_role": "authority_for_admission",
        },
        "external_benchmark": {
            "harness_root": str(HARNESS_ROOT),
            "present": harness_present,
            "interpreter": harness_python(),
            "model": model,
            "tasks": tasks,
            "command": cmd,
            "role": "external_comparison_layer_only",
        },
        "notes": [
            "The benchmark output is not used to admit or reject capabilities.",
            "The internal TIDE-X universality receipt remains the gate.",
            "LM Eval Harness is used for public comparison and reporting only.",
        ],
    }


def run_external_benchmark(model: str, tasks: list[str], extra_args: list[str]) -> dict:
    harness_root = HARNESS_ROOT.resolve()
    if not harness_root.exists():
        raise FileNotFoundError(f"LM Eval Harness not found at {harness_root}")

    env = os.environ.copy()
    env["PYTHONPATH"] = str(harness_root) + (os.pathsep + env["PYTHONPATH"] if env.get("PYTHONPATH") else "")
    cmd = build_command(model, tasks, extra_args)
    completed = subprocess.run(cmd, cwd=str(harness_root), env=env, capture_output=True, text=True)
    result = {
        "schema": "tidex.external_lm_eval_run/v1",
        "interpreter": harness_python(),
        "command": cmd,
        "returncode": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description="Compare internal TIDE-X evidence against the external LM Eval Harness.")
    parser.add_argument("--dry-run", action="store_true", help="Generate the benchmark plan without running an external evaluation.")
    parser.add_argument("--run", action="store_true", help="Execute LM Evaluation Harness if installed.")
    parser.add_argument("--model", default="hf", help="LM Eval model name, default: hf")
    parser.add_argument("--tasks", default="hellaswag,arc_easy", help="Comma-separated LM Eval tasks")
    parser.add_argument("--extra-args", default="", help="Extra arguments passed to lm_eval, e.g. --limit 1")
    parser.add_argument("--report", default=str(EXTERNAL_REPORT), help="Where to write the JSON summary")
    args = parser.parse_args()

    tasks = [t.strip() for t in args.tasks.split(",") if t.strip()]
    extra_args = [p for p in args.extra_args.split() if p]

    if not args.dry_run and not args.run:
        args.dry_run = True

    plan = dry_run_plan(args.model, tasks, extra_args)

    if args.run:
        try:
            result = run_external_benchmark(args.model, tasks, extra_args)
            report = {
                "plan": plan,
                "result": result,
                "status": "external_run_attempted",
            }
        except Exception as exc:  # pragma: no cover - user-facing failure path
            report = {
                "plan": plan,
                "error": str(exc),
                "status": "external_run_failed",
            }
    else:
        report = {"plan": plan, "status": "dry_run"}

    out = Path(args.report)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Wrote {out}")
    print(json.dumps({"status": report["status"], "model": args.model, "tasks": tasks}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
