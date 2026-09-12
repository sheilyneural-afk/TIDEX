# `src/` — mapa de dominios

Código Rust organizado por **dominio de responsabilidad**, no por capas MVC.

**Ubicación:** `src/`  
**Relacionado:** [ARCHITECTURE](../ARCHITECTURE.md) · [INDEX](../INDEX.md)

## Principio

Cada subdirectorio es un bounded context. `lib.rs` solo declara módulos (y `cross_model` bajo feature).

## Dominios

| Módulo | Doc | Una frase |
|--------|-----|-----------|
| `foundation` | [src-foundation](src-foundation.md) | Identidad, digest, autoridad de ficheros, ledger |
| `knowledge` | [src-knowledge](src-knowledge.md) | Autoridad epistémica (KnowledgeEngine) |
| `governance` | [src-governance](src-governance.md) | AdapterBank, residencia, promotion gate |
| `operator` | [src-operator](src-operator.md) | HTTP control plane, registry, grafo |
| `learning` | [src-learning](src-learning.md) | Ciclo adaptativo y evidencias |
| `receiver` | [src-receiver](src-receiver.md) | Perfilado/binding a arquitecturas |
| `materialization` | [src-materialization](src-materialization.md) | Sombras / steering evaluables |
| `capability` | [src-capability](src-capability.md) | IR/bundles/vault |
| `analysis` | [src-analysis](src-analysis.md) | Tomografía / geometría / tracking |
| `engine` | [src-engine](src-engine.md) | Runtime interno de engine |
| `runtime` | [src-runtime](src-runtime.md) | Aislamiento de ejecución (módulo Rust) |
| `cross_model` | [src-cross_model](src-cross_model.md) | Feature HF/NNsight/plasticity |
| `bin` | [src-bin](src-bin.md) | CLIs y benches |

## Cómo no perderse

- ¿Quién decide? → knowledge + governance + foundation.
- ¿Quién ejecuta modelos externos? → cross_model (feature) vía operator workflows.
- ¿Quién materializa candidatos? → materialization (+ receiver).
- ¿Quién opera el lab? → operator + web-console.
