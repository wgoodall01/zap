.SUFFIXES: # disable builtin rules

.PHONY: all
all: api web

.PHONY: api
api:
	cd pkg/api && cargo build

api/target/openapi.json: api
	cd pkg/api && ./scripts/generate_openapi_spec.sh

.PHONY: web
web: api/target/openapi.json
	cd pkg/web && pnpm build
