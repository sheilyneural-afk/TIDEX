#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
echo "TIDE-X:                      http://127.0.0.1:8793"
echo "Catálogo / recetas:          http://127.0.0.1:8793/operator"
exec "$ROOT/tidex" serve
