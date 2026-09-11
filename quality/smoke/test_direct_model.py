#!/usr/bin/env python3
"""Real Ollama runtime smoke for explicit runtime/v2 entries.

This script validates only `backend=ollama` entries. It does not perform prompt
steering, capability transfer, hidden-state extraction, or promotion.
"""

import argparse
import hashlib
import json
import urllib.request
from pathlib import Path


def post_json(url: str, payload: dict, timeout: int) -> dict:
    request = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
        if response.status != 200:
            raise RuntimeError(f"HTTP {response.status}")
        return json.load(response)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--runtime",
        default=str(Path(__file__).with_name("real_model_capabilities_test.json")),
    )
    parser.add_argument("--prompt", default="Return only the decimal integer for 17 * 19.")
    args = parser.parse_args()
    runtime = json.loads(Path(args.runtime).read_text())
    if runtime.get("schema") != "cerebro.cross_model.runtime/v2":
        raise SystemExit("invalid runtime schema")
    models = runtime.get("models")
    if not isinstance(models, list):
        raise SystemExit("runtime models missing")
    ollama = [entry for entry in models if entry.get("backend") == "ollama"]
    if not ollama:
        raise SystemExit("runtime has no Ollama entries")
    failures = []
    for entry in ollama:
        if set(entry) != {"backend", "model", "endpoint", "generation"}:
            failures.append("Ollama runtime entry fields invalid")
            continue
        model = entry["model"]
        endpoint = entry["endpoint"].rstrip("/")
        policy = entry["generation"]
        timeout = int(policy["request_timeout_seconds"])
        show = post_json(f"{endpoint}/api/show", {"model": model}, timeout)
        info = show.get("model_info") or {}
        architecture = info.get("general.architecture")
        parameter_count = info.get("general.parameter_count")
        tensors = show.get("tensors") or []
        if not architecture or not isinstance(parameter_count, int) or parameter_count <= 0 or not tensors:
            failures.append(f"{model}: incomplete /api/show metadata")
            continue
        generation = post_json(
            f"{endpoint}/api/generate",
            {
                "model": model,
                "prompt": args.prompt,
                "stream": False,
                "think": bool(policy["think"]),
                "keep_alive": policy["keep_alive"],
                "options": {
                    "temperature": float(policy["temperature"]),
                    "top_p": float(policy["top_p"]),
                    "num_predict": int(policy["max_tokens"]),
                    "seed": int(policy["seed"]),
                },
            },
            timeout,
        )
        if generation.get("model") != model:
            failures.append(f"{model}: response model identity mismatch")
            continue
        text = generation.get("response")
        if not isinstance(text, str):
            failures.append(f"{model}: response field missing")
            continue
        digest = hashlib.sha256(text.encode("utf-8")).hexdigest()
        print(
            json.dumps(
                {
                    "model": model,
                    "architecture": architecture,
                    "parameters": parameter_count,
                    "tensor_count": len(tensors),
                    "response_sha256": digest,
                    "response": text,
                },
                ensure_ascii=False,
            )
        )
    if failures:
        for failure in failures:
            print(failure)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
