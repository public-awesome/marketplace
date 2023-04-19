.PHONY: optimize lint

e2etest:
	mkdir -p e2e/contracts
	cp artifacts/*.wasm e2e/contracts
	cd e2e && go test -v

optimize: 
	sh scripts/optimize.sh

lint:
	cargo clippy --all-targets -- -D warnings

schema:
	sh scripts/schema.sh
