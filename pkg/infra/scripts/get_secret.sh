#!/usr/bin/env bash
set -euo pipefail
shopt -s inherit_errexit
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"


# Function to display usage
usage() {
    echo "Usage: $0 [--db|--api-keys] > output.json"
    echo ""
    echo "Options:"
    echo "  --db        Fetch database credentials secret"
    echo "  --api-keys  Fetch API keys secret"
    echo ""
    echo "Examples:"
    echo "  $0 --db > db_secret_contents.json"
    echo "  $0 --api-keys > api_keys_contents.json"
    exit 1
}

# Check if argument is provided
if [ $# -ne 1 ]; then
    echo "Error: Exactly one argument required" >&2
    usage
fi

# Get stack outputs
STACK_OUTPUTS="$(aws cloudformation describe-stacks --stack-name ZapStackProd | jq '.Stacks[].Outputs')"

case "$1" in
    --db)
        SECRET_ARN="$(echo "$STACK_OUTPUTS" | jq -r '.[] | select(.OutputKey=="DbSecretArn") | .OutputValue')"
        ;;
    --api-keys)
        SECRET_ARN="$(echo "$STACK_OUTPUTS" | jq -r '.[] | select(.OutputKey=="ApiSecretArn") | .OutputValue')"
        ;;
    *)
        echo "Error: Invalid argument '$1'" >&2
        usage
        ;;
esac

# Fetch and output the secret value as JSON
aws secretsmanager get-secret-value --secret-id "$SECRET_ARN" --query 'SecretString' --output text
