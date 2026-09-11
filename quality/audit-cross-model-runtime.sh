#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo '[1/3] Rust all-feature compile gate'
cargo check --all-features --all-targets --offline --locked

echo '[2/3] Forbidden simulation-pattern scan'
if rg -n -i 'placeholder|mock|stub|fallback|hardcod|canned|fake|simulate|rand::random|synthetic|not yet implemented|in a real implementation' src/cross_model; then
  echo 'forbidden simulation marker found in src/cross_model' >&2
  exit 2
fi

echo '[3/3] Real Ollama runtime smoke'
python3 quality/smoke/test_direct_model.py

echo 'PASS: code compiles and runtime inference smoke completed.'
echo 'No production activation or transfer is claimed by this script.'
