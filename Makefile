.PHONY: fmt check test integration ci

fmt:
	cargo fmt --all -- --check

check:
	cargo check --all-targets

test:
	cargo test --lib

integration:
	cargo test --test brain --test production_surface --test quality_properties --test convergence_pipeline

ci: fmt check test integration
