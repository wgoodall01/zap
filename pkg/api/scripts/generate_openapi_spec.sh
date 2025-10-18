#!/usr/bin/env bash
set -euo pipefail
shopt -s inherit_errexit
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd "$DIR/.." # cd to pkg/api

# kill all child processes on exit
trap 'kill $(jobs -p)' EXIT

# Build the app
echo "--- Building API server"
cargo build -q

# Run the app in the background as a job on a non-default port
echo "--- Starting API server"
PORT=9000 cargo run --quiet >/dev/null &

# Poll for the JSON file every 50ms until it exists, max 10s
# `GET localhost:9000/api/v1/openapi.json`
echo "--- Waiting for OpenAPI spec to be generated"
done="false"
for i in {1..200}; do
  if curl -s http://localhost:9000/api/v1/openapi.json -o target/openapi.json; then
	echo "--- OpenAPI spec generated successfully"
    done="true"
	break
  fi
  sleep 0.05
done

# If the file doesn't exist after 10s, exit with an error
if [ "$done" = "false" ]; then
  echo "--- OpenAPI spec generation failed"
  exit 1
fi
