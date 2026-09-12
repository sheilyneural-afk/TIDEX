# Sistema: quality gates y CI

**Ubicación:** `quality/gate*.sh`, `Makefile`, `.github/workflows/ci.yml`, `fuzz/`  
**Relacionado:** [QUALITY_AND_TESTING](../QUALITY_AND_TESTING.md) · [directories/quality](../directories/quality.md) · [INDEX](../INDEX.md)

## Qué es

La red de seguridad reproducible: fmt/clippy/check/tests + gates P0–P4 + fuzz ASan + deny/audit.

## Capas

| Capa | Dónde | Qué garantiza |
|------|-------|----------------|
| CI mínima | `Makefile` / `ci.yml` | fmt, clippy, check, test lib, integration, config-contracts, fuzz-check |
| Gate0 | `quality/gate0-*.sh` | Release/empty-state; layout runtime limpio (rechaza `runtime/cargo-target`) |
| Gate1 | `gate1-tooling.sh` | Cobertura, Miri, ASan/TSan, fuzz 3 targets (`QUALITY_FUZZ_RUNS`, default 100k) |
| Gate2–4 | `gate2`…`gate4` | Verification → assurance → release-readiness |
| Fuzz | `fuzz/fuzz_targets/*` | `multi-case-solver`, `persisted-inputs`, `identity-wire` |

## Notas operativas

- Gate1 pinnea `QUALITY_EXPECTED_COMMIT` (histórico `0ed41eb…`): hay que alinearlo al HEAD real antes de certificar.
- Fuzz debe usar `CARGO_TARGET_DIR` **fuera** del checkout (como hace gate1) para no reintroducir build dirs.
- Campaña 2026-09-12: 100k×3 PASS, 0 artifacts/crashes.

