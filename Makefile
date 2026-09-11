.PHONY: fmt clippy check test integration config-contracts fuzz-check ci

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets --all-features --locked -- -D warnings

check:
	cargo check --all-targets --all-features --locked

test:
	cargo test --lib --all-features --locked

integration:
	cargo test --all-features --locked --test production_surface --test quality_properties --test convergence_pipeline

config-contracts:
	cargo test --locked --all-features --test configuration_contracts

fuzz-check:
	cargo check --locked --manifest-path fuzz/Cargo.toml --all-targets

ci: fmt clippy check test integration config-contracts fuzz-check
