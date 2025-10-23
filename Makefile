.SUFFIXES: # disable builtin rules

.PHONY: all
all: api web api_lambda

.PHONY: fix
fix:
	claude --permission-mode acceptEdits \
		"Run 'make check' and fix any issues found. Running `cd pkg/api && cargo clippy --fix --allow-dirty` may help you. Repeat until it finishes successfully. Once 'make check' passes, your job is done."

.PHONY: fmt
fmt: 
	cd pkg/api && cargo fmt
	cd pkg/web && pnpm fmt

.PHONY: check
check: fmt
	cd pkg/api && SQLX_OFFLINE=true cargo clippy -- -D warnings && cargo fmt --check
	cd pkg/web && pnpm build && pnpm test

.PHONY: gen
gen: \
	pkg/api/.sqlx \
	pkg/api/target/openapi.json \
	pkg/web/src/api_client

.PHONY: deploy
deploy: api_lambda web
	cd pkg/api && ./scripts/deploy_migrations.sh
	cd pkg/infra && pnpm cdk deploy

.PHONY: api
api:
	cd pkg/api && cargo build

.PHONY: api_lambda
api_lambda: 
	cd pkg/api && ./scripts/build_lambda.sh

.PHONY: pkg/api/target/openapi.json
pkg/api/target/openapi.json: api
	cd pkg/api && ./scripts/generate_openapi_spec.sh

.PHONY: pkg/api/.sqlx
pkg/api/.sqlx: 
	cd pkg/api && cargo sqlx prepare

.PHONY: web
web: pkg/web/src/api_client
	cd pkg/web && pnpm build

.PHONY: pkg/web/src/api_client
pkg/web/src/api_client: pkg/api/target/openapi.json
	cd pkg/web && pnpm gen
