# Sistema: autoridad y receipts

**Ubicación:** `src/foundation/` + `src/knowledge/knowledge_engine.rs` + `src/governance/`  
**Relacionado:** [src-foundation](../directories/src-foundation.md) · [src-knowledge](../directories/src-knowledge.md) · [src-governance](../directories/src-governance.md) · [INDEX](../INDEX.md)

## Qué es

La capa que **decide** qué puede avanzar. No es la UI, no es HF, no es el operator HTTP.
TIDE-X separa:

1. **Autoridad epistémica** — `KnowledgeEngine`: planifica, valida obligaciones/hipótesis/evidencia, emite receipts y transiciones. `living_staircase()` es proyección de lectura, no orquestador.
2. **Autoridad de ciclo de vida de adapters** — `AdapterBank`: autentica, activa, revoca, rollback. En el grafo de operator es el único executor con `production_authority=true` (`adapter.bank`).
3. **Decisión de residencia** — `ResidencyDecision`: fail-closed; la residencia no es preferencia del caller.
4. **Gate de promoción** — `universal_promotion_gate`: readiness para *otra* autoridad; este módulo **no** escribe modelos ni activa producción.
5. **Primitivas** — `foundation::{digest,authority,artifact,ledger,identity}`: SHA-256 canónico, staging privado `CREAT|EXCL`, artefactos inmutables, ledger V2.

## Por qué existe

Sin esta capa, un workflow o un backend HF podrían “decidir” promoción. El diseño explícito es: **ejecutar ≠ autorizar**.

## Flujo típico

```text
caller / operator recipe
  → KnowledgeEngine (plan + obligaciones)
  → evidencia hasheada (receipt / artifact)
  → ResidencyDecision / PromotionGate (si aplica)
  → AdapterBank solo si hay autoridad de lifecycle
```

## Invariantes

- `KnowledgeEngine::open` falla sin instancia de autoridad (`authority_instance_required`); el camino real es `open_with_authority_instance`.
- Digests de autoridad son domain-separated (`digest_domain`), no “un sha genérico”.
- Ficheros privados: install atómico, sin TOCTOU de pathname tras el fd.
- Fail-closed: ausencia de evidencia ≠ permiso.

## Archivos ancla

| Pieza | Path | Líneas (aprox.) | Rol |
|-------|------|-----------------|-----|
| KnowledgeEngine | `src/knowledge/knowledge_engine.rs` | ~7.5k | Única autoridad epistémica ejecutable por capacidad gobernada |
| AdapterBank | `src/governance/adapter_bank.rs` | ~4k | Banco de adapters content-addressed |
| Residency | `src/governance/residency_decision.rs` | ~2.3k | Decisión de residencia fail-closed |
| Promotion gate | `src/governance/universal_promotion_gate.rs` | ~0.4k | Readiness, no activación |
| Digest / Authority | `src/foundation/digest.rs`, `authority.rs` | ~0.7k / ~2k | Identidad y persistencia privada |

## Qué no es

- No es el catálogo HF del operator.
- No concede autoridad el `web-console` ni `POST /api/workflows/*`.

