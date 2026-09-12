# Raíz del crate TIDEX

Empaquetado Rust del sistema TIDE-X (`tidex` 0.1.0): identidad del paquete, toolchain, bins/tests explícitos y pipeline mínimo.

**Ubicación:** `/home/yo/Future` (raíz del repo)  
**Relacionado:** [src](src.md) · [quality](quality.md) · [ARCHITECTURE](../ARCHITECTURE.md) · [INDEX](../INDEX.md)

## Para qué existe

Sin esta capa el código de `src/` no es un producto verificable: no hay pins de toolchain, no hay lista cerrada de bins, no hay `make ci`.

## Archivos clave

| Archivo | Rol real |
|---------|----------|
| `Cargo.toml` | `publish = false`, `autobins/autotests = false`, feature `cross-model-plasticity`, lista explícita de `[[bin]]` y tests |
| `Cargo.lock` | Grafo de deps reproducible |
| `build.rs` | Digiere entradas canónicas → `TIDEX_SOURCE_TREE_DIGEST` (dominio `TIDEX:COMPILED-INPUTS:v3`) |
| `Makefile` | `fmt` `clippy` `check` `test` `integration` `config-contracts` `fuzz-check` `ci` |
| `rust-toolchain.toml` | Canal fijo (p.ej. 1.96.0) + componentes |
| `rustfmt.toml` / `clippy.toml` / `deny.toml` | Estilo y política de dependencias |
| `tidex` | Wrapper shell hacia el binario / `serve` |
| `.gitignore` | Ignora `/runtime/`, targets, `.env*`, fuzz artifacts |

## Invariantes de packaging

- Nada se auto-descubre como bin/test: si no está en `Cargo.toml`, no forma parte de la superficie publicada del crate.
- `publish = false`: no es crate crates.io.
- El digest de árbol compilado acopla release identity al fuente; mismatches rompen pipelines de convergencia.

## Navegación

1. `Cargo.toml` → qué bins/features existen.
2. `make ci` → mismo espíritu que `.github/workflows/ci.yml`.
3. Dominios en [src.md](src.md).
