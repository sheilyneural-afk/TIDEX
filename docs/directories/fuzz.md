# `fuzz/`

Crate `tidex-fuzz` con tres arneses libFuzzer (+ ASan vía `cargo fuzz`).

**Ubicación:** `fuzz/`  
**Relacionado:** [quality-gates-and-ci](../systems/quality-gates-and-ci.md) · [INDEX](../INDEX.md)

## Targets

| Target | Ejercita |
|--------|----------|
| `multi-case-solver` | Portfolio least-squares / reports autenticados |
| `persisted-inputs` | Round-trip serde de receipts/IR/bundles/solver records |
| `identity-wire` | Parse + round-trip de IDs foundation + `Sha256Digest` |

## Seeds

`fuzz/seeds/<target>/` — corpus inicial mínimo. Gate1 copia seeds a un tmp y **no** debe dejar `fuzz/artifacts` en el checkout.

## Resultado reciente (100k runs × 3)

PASS, 0 crashes, 0 artifacts; corpora expandidos en `/tmp` de la corrida.
