lint:
	cargo clippy --all-targets -- -D warnings

schema:
	sh scripts/schema.sh

optimize:
	sh scripts/optimize.sh

deploy-local:
	#!/usr/bin/env bash
	TEST_ADDRS=`jq -r '.[].address' ./tests/e2e/configs/test_accounts.json | tr '\n' ' '`
	docker kill stargaze || true
	docker volume rm -f stargaze_data
	docker run --rm -d --name stargaze \
		-e DENOM=ustars \
		-e CHAINID=testing \
		-e GAS_LIMIT=75000000 \
		-p 1317:1317 \
		-p 26656:26656 \
		-p 26657:26657 \
		-p 9090:9090 \
		--mount type=volume,source=stargaze_data,target=/root \
		publicawesome/stargaze:10.0.1 /data/entry-point.sh $TEST_ADDRS

deploy-local-arm:
	#!/usr/bin/env bash
	TEST_ADDRS=`jq -r '.[].address' ./tests/e2e/configs/test_accounts.json | tr '\n' ' '`
	docker kill stargaze || true
	docker volume rm -f stargaze_data
	docker run --rm -d --name stargaze \
		-e DENOM=ustars \
		-e CHAINID=testing \
		-e GAS_LIMIT=75000000 \
		-p 1317:1317 \
		-p 26656:26656 \
		-p 26657:26657 \
		-p 9090:9090 \
		--mount type=volume,source=stargaze_data,target=/root \
		--platform linux/amd64 \
		publicawesome/stargaze:10.0.1 /data/entry-point.sh $TEST_ADDRS

e2e-test: deploy-local
	RUST_LOG=info CONFIG=configs/cosm-orc.yaml RUST_BACKTRACE=1 cargo e2e-test

e2e-test-arm: deploy-local-arm
	RUST_LOG=info CONFIG=configs/cosm-orc.yaml RUST_BACKTRACE=1 cargo e2e-test

# e2e-test-full: dl-artifacts optimize e2e-test

# e2e-test-full-arm: dl-artifacts optimize-arm e2e-test-arm
