#!/usr/bin/env python3
"""Thin TIDE-X → SHEI GPEM v2 recommend bridge (canonical interfaces only).

Invokes SHEI governed procedural memory via:
  - get_gpem(...) when SHEI_GPEM_ROOT / project root is usable
  - else create_gpem(store_root, force_local=True) → GPEMService.recommend_v2
    (which delegates to GPEMServiceV2.recommend)

Stdin JSON:
  {
    "action": "recommend" | "seed_demo_traces",
    "store_root": "<path>",
    "shei_research_python": "<path>/research_python",
    "context": { ... },          # recommend
    "limit": 5,                  # recommend
    "prefer_get_gpem": false     # optional
  }

Stdout JSON (success):
  {"ok": true, "interface": "...", "recommendations": [...], ...}
Fail-closed:
  {"ok": false, "error": "<code>", "detail": "..."}
"""

from __future__ import annotations

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def _fail(code: str, detail: str = "") -> int:
    sys.stdout.write(json.dumps({"ok": False, "error": code, "detail": detail}, sort_keys=True))
    sys.stdout.write("\n")
    return 2


def _ok(payload: dict[str, Any]) -> int:
    out = {"ok": True, **payload}
    sys.stdout.write(json.dumps(out, sort_keys=True, default=str))
    sys.stdout.write("\n")
    return 0


def _load_shei(research_python: Path) -> None:
    rp = str(research_python.resolve())
    if not research_python.is_dir():
        raise FileNotFoundError(f"shei_research_python_missing:{rp}")
    if rp not in sys.path:
        sys.path.insert(0, rp)


def _rec_to_dict(rec: Any) -> dict[str, Any]:
    return {
        "trace_id": getattr(rec, "trace_id", None),
        "turn_id": getattr(rec, "turn_id", None),
        "route": getattr(rec, "route", None),
        "capability_id": getattr(rec, "capability_id", None),
        "score": float(getattr(rec, "score", 0.0) or 0.0),
        "rationale": list(getattr(rec, "rationale", []) or []),
        "lifecycle_state": getattr(rec, "lifecycle_state", None),
        "utility_score": float(getattr(rec, "utility_score", 0.0) or 0.0),
        "auditability_score": float(getattr(rec, "auditability_score", 0.0) or 0.0),
    }


def _open_service(store_root: Path, *, prefer_get_gpem: bool, shei_root: Path | None):
    """Open governed GPEM via canonical SHEI interfaces."""
    from memory.procedural_execution_memory.authority import GPEMAuthority

    GPEMAuthority.assert_safe()

    if prefer_get_gpem and shei_root is not None:
        os.environ["SHEI_GPEM_ROOT"] = str(store_root)
        os.environ.setdefault("SHEI_GPEM_ENABLED", "1")
        from memory.procedural_execution_memory.shei_bridge import get_gpem, reset_gpem

        reset_gpem()
        gpem = get_gpem(project_root=shei_root)
        if gpem is None:
            raise RuntimeError("get_gpem_returned_none")
        return gpem, "get_gpem->GPEMService.recommend_v2"

    from memory.procedural_execution_memory import create_gpem

    gpem = create_gpem(str(store_root), force_local=True)
    return gpem, "create_gpem->GPEMService.recommend_v2->GPEMServiceV2.recommend"


def _demo_payload(turn_id: str, route: str, capability_id: str, *, success: bool = True) -> dict[str, Any]:
    return {
        "turn_id": turn_id,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "actor": {"actor_id": "tidex_gpem_donor_bridge", "actor_type": "system"},
        "input": {"raw_user_message": "tidex-donor", "normalized_user_message": "tidex-donor"},
        "intent": {
            "user_intent": route,
            "task_signature": f"{route}:{capability_id}",
            "route": route,
        },
        "routing": {"selected_route": route},
        "quality": {
            "completeness_score": 0.9 if success else 0.0,
            "context_richness_score": 0.85,
            "retrievability_score": 0.85,
            "auditability_score": 0.9 if success else 0.55,
            "training_value_score": 0.85,
            "production_readiness_score": 0.85,
        },
        "result": {
            "turn_success": success,
            "quality_status": "ok" if success else "failed",
        },
        "errors": {"current_errors": [] if success else [{"message": "failed"}]},
        "security": {"privacy": {"safe_for_training": success}},
        "audit": {"audit_status": "captured", "evidence": []},
        "interop": {"capability_id": capability_id},
    }


def action_recommend(req: dict[str, Any]) -> int:
    store_root = Path(str(req["store_root"]))
    research_python = Path(str(req["shei_research_python"]))
    context = req.get("context") or {}
    if not isinstance(context, dict):
        return _fail("gpem_v2_recommend_context_invalid", "context_must_be_object")
    limit = int(req.get("limit") or 5)
    prefer_get_gpem = bool(req.get("prefer_get_gpem") or False)
    shei_root = research_python.parent if research_python.name == "research_python" else None

    try:
        _load_shei(research_python)
        store_root.mkdir(parents=True, exist_ok=True)
        gpem, interface = _open_service(
            store_root, prefer_get_gpem=prefer_get_gpem, shei_root=shei_root
        )
        # Canonical facade: GPEMService.recommend_v2 → GPEMServiceV2.recommend
        if hasattr(gpem, "recommend_v2"):
            recs = gpem.recommend_v2(context, limit=limit)
            interface_used = interface
        else:
            # Direct V2 path if a raw GPEMServiceV2 was returned
            recs = gpem.recommend(context, limit=limit)
            interface_used = "GPEMServiceV2.recommend"
        payload = {
            "interface": interface_used,
            "store_root": str(store_root),
            "recommendations": [_rec_to_dict(r) for r in (recs or [])],
            "authorizes_production": False,
            "advisory_only": True,
        }
        return _ok(payload)
    except Exception as exc:  # noqa: BLE001 — bridge must fail-closed with code
        return _fail("gpem_v2_recommend_invoke_failed", f"{type(exc).__name__}:{exc}")


def action_seed_demo_traces(req: dict[str, Any]) -> int:
    """Seed a local governed store for live donor integration / demo smoke.

    Uses GPEMServiceV2.ingest_payload (canonical write path). Not a fixture
    procedure selector — real GPEM ledger rows.
    """
    store_root = Path(str(req["store_root"]))
    research_python = Path(str(req["shei_research_python"]))
    try:
        _load_shei(research_python)
        store_root.mkdir(parents=True, exist_ok=True)
        from memory.procedural_execution_memory import create_gpem

        gpem = create_gpem(str(store_root), force_local=True)
        traces = []
        for turn_id, route, cap in (
            ("tidex_demo_t1", "analysis", "proc.alpha"),
            ("tidex_demo_t2", "analysis", "proc.alpha"),
            ("tidex_demo_t3", "repair", "proc.beta"),
        ):
            result = gpem._memory.ingest_payload(
                _demo_payload(turn_id, route, cap, success=True), redact=True
            )
            traces.append(result.trace.header.trace_id)
        return _ok(
            {
                "interface": "create_gpem->GPEMServiceV2.ingest_payload",
                "store_root": str(store_root),
                "trace_ids": traces,
                "authorizes_production": False,
            }
        )
    except Exception as exc:  # noqa: BLE001
        return _fail("gpem_v2_seed_invoke_failed", f"{type(exc).__name__}:{exc}")


def main() -> int:
    try:
        raw = sys.stdin.read()
        req = json.loads(raw) if raw.strip() else {}
    except json.JSONDecodeError as exc:
        return _fail("gpem_v2_recommend_request_invalid", str(exc))
    if not isinstance(req, dict):
        return _fail("gpem_v2_recommend_request_invalid", "request_must_be_object")
    for key in ("store_root", "shei_research_python"):
        if not str(req.get(key) or "").strip():
            return _fail("gpem_v2_recommend_donor_misconfigured", f"missing_{key}")
    action = str(req.get("action") or "recommend").strip()
    if action == "recommend":
        return action_recommend(req)
    if action == "seed_demo_traces":
        return action_seed_demo_traces(req)
    return _fail("gpem_v2_recommend_action_unsupported", action)


if __name__ == "__main__":
    sys.exit(main())
