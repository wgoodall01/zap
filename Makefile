.SUFFIXES: # disable builtin rules

.PHONY: all
all: api web api_lambda

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

.PHONY: api/target/openapi.json
api/target/openapi.json: api
	cd pkg/api && ./scripts/generate_openapi_spec.sh

.PHONY: web
web: api/target/openapi.json
	cd pkg/web && pnpm build
