#!/usr/bin/env bash
set -euo pipefail
shopt -s inherit_errexit
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd "$DIR/.." # cd to pkg/api

echo "--- Generating OpenAPI spec"
cargo run --bin gen_openapi_spec > target/openapi.json
echo "--- OpenAPI spec generated successfully"
