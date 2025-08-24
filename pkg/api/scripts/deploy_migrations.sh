#!/usr/bin/env bash
set -euo pipefail
shopt -s inherit_errexit
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"


# First, get the production DB connection secret, in AWS RDS format.
echo "[+] Fetching DATABASE_URL secret from AWS Secrets Manager..."
db_secret="$($DIR/../../infra/scripts/get_secret.sh --db)"
if [ -z "$db_secret" ]; then
  echo "Error: DATABASE_URL secret not found."
  exit 1
fi

# Build the DATABASE_URL from the `host`, `port`, `username`, `password`, and `dbname` fields.
host=$(echo "$db_secret" | jq -r '.host')
port=$(echo "$db_secret" | jq -r '.port')
username=$(echo "$db_secret" | jq -r '.username')
password=$(echo "$db_secret" | jq -r '.password')
dbname=$(echo "$db_secret" | jq -r '.dbname')
export DATABASE_URL="postgresql://$username:$password@$host:$port/$dbname"
echo "[+] Fetched DATABASE_URL for host $host"

# Run `cargo sqlx migrate run` with the DATABASE_URL.
exec cargo sqlx migrate run --source ./migrations "$@"
