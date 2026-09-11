#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUNTIME="${TIDEX_RUNTIME_ROOT:-$ROOT/runtime}"
VENV="$RUNTIME/python/tidex-mechinterp"
HUB="$RUNTIME/llms/huggingface/hub"
LOCK="$ROOT/src/cross_model/runtime/hf-runtime.lock.txt"
export HF_HOME="$RUNTIME/llms/huggingface"
export HUGGINGFACE_HUB_CACHE="$HUB"
export PYTHONNOUSERSITE=1
export HF_HUB_DISABLE_TELEMETRY=1

if [[ ! -f "$LOCK" ]]; then
  echo "missing certified HF runtime lock: $LOCK" >&2
  exit 1
fi

mkdir -p "$VENV" "$HUB" "$RUNTIME/tidex" "$RUNTIME/private"
chmod 700 "$RUNTIME" "$VENV" "$HUB" "$RUNTIME/tidex" "$RUNTIME/private"

if [[ ! -x "$VENV/bin/python" ]]; then
  python3 -m venv "$VENV"
fi

"$VENV/bin/python" - <<'PY'
import sys
if sys.version_info[:2] != (3, 12):
    raise SystemExit(f"hf_runtime_python_unsupported:{sys.version}")
PY

"$VENV/bin/python" -m pip install --require-hashes -r "$LOCK"
"$VENV/bin/python" -m pip check
"$VENV/bin/python" - "$LOCK" <<'PY'
import importlib.metadata
import re
import sys
from pathlib import Path

def canonicalize(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).lower()

lock = Path(sys.argv[1])
expected = {}
for raw in lock.read_text().splitlines():
    line = raw.strip()
    if not line or line.startswith("#") or line.startswith("-"):
        continue
    spec = line.split()[0]
    name, sep, version = spec.partition("==")
    if sep != "==" or not name or not version or "--hash=sha256:" not in line:
        raise SystemExit(f"hf_runtime_lock_line_invalid:{raw}")
    key = canonicalize(name)
    if key in expected:
        raise SystemExit(f"hf_runtime_lock_duplicate:{name}")
    expected[key] = version

installed = {
    canonicalize(distribution.metadata["Name"]): distribution.version
    for distribution in importlib.metadata.distributions()
    if distribution.metadata["Name"]
}
for name, version in expected.items():
    got = installed.get(name)
    if got != version:
        raise SystemExit(f"hf_runtime_lock_mismatch:{name}:expected={version}:got={got}")

extras = sorted(set(installed) - set(expected))
if extras:
    raise SystemExit("hf_runtime_uncertified_extra:" + ",".join(extras))

import nnsight
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer  # noqa: F401

if "+cpu" not in torch.__version__:
    raise SystemExit(f"hf_runtime_torch_not_cpu:{torch.__version__}")
if nnsight.__version__ != expected[canonicalize("nnsight")]:
    raise SystemExit(f"hf_runtime_nnsight_mismatch:{nnsight.__version__}")
PY

download() {
  local repo="$1"
  echo "Downloading $repo into $HUB"
  "$VENV/bin/huggingface-cli" download "$repo"
}

download HuggingFaceTB/SmolLM2-135M
download HuggingFaceTB/SmolLM2-135M-Instruct

echo "Runtime listo."
echo "Python: $VENV/bin/python"
echo "Lock:   $LOCK"
echo "Hub:    $HUB"
echo "Arranque: $ROOT/tidex serve"
