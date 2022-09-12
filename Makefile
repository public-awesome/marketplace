.PHONY: optimize lint

optimize: 
	sh scripts/optimize.sh

lint:
	cargo clippy --all-targets -- -D warnings
