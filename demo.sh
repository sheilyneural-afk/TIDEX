#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

echo '[1/5] Rust all-feature compile gate'
cargo check --all-features --all-targets --offline

echo '[2/5] Source simulation scan'
if rg -n -i 'placeholder|mock|stub|fallback|hardcod|canned|fake|simulate|rand::random|synthetic|not yet implemented|in a real implementation' src/cross_model; then
  echo 'forbidden simulation marker found in src/cross_model' >&2
  exit 2
fi

echo '[3/5] Real Ollama runtime smoke'
python3 test_direct_model.py

echo '[4/5] Preserved V66 functional evidence verification'
python3 test_transfer_effectiveness.py

echo '[5/5] Real physical artifact validation'
python3 test_real_transfer.py

echo 'PASS: code compiles, runtime inference is real, preserved functional evidence hashes verify, and physical transfer inputs are authentic.'
echo 'No production activation or new transfer is claimed by this script.'
