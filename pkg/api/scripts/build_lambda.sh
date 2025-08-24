#!/bin/bash
set -e

# Increase file descriptor limit to avoid quota exceeded errors
ulimit -n 4096

echo "Building Lambda function..."
cargo lambda build --profile lambda --arm64 --bin lambda_server

echo "Lambda build complete. Binary available at target/lambda/lambda_server/bootstrap"
