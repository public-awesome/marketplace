lint:
	cargo clippy --all-targets -- -D warnings

schema:
	sh scripts/schema.sh

artifacts:
	mkdir -p artifacts

download-artifacts: artifacts
	scripts/download-core-artifacts.sh
	scripts/download-launchpad-artifacts.sh
	scripts/download-marketplace-artifacts.sh

optimize:
	sh scripts/optimize.sh

optimize-arm:
	sh scripts/optimize-arm.sh

deploy-local:
	#!/usr/bin/env bash
	TEST_ADDRS=`jq -r '.[].address' ./typescript/packages/e2e-tests/configs/test_accounts.json | tr '\n' ' '`
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
	TEST_ADDRS=`jq -r '.[].address' ./typescript/packages/e2e-tests/configs/test_accounts.json | tr '\n' ' '`
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

e2e-test:
	#!/usr/bin/env bash -e
	START_DIR=$(pwd)
	cd typescript/packages/e2e-tests
	yarn install
	yarn test
	cd "$START_DIR"

e2e-test-full: download-artifacts optimize deploy-local e2e-test

e2e-test-full-arm: download-artifacts optimize-arm deploy-local-arm e2e-test

e2e-watch: deploy-local-arm
	#!/usr/bin/env bash -e
	START_DIR=$(pwd)
	cd typescript/packages/e2e-tests
	yarn test
	yarn test:watch
	cd "$START_DIR"