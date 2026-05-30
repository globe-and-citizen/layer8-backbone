#!/bin/sh
# wait-for-services.sh
# Polls key service healthcheck endpoints until they respond with HTTP 200
# (or until MAX_WAIT seconds have elapsed).
#
# Usage:
#   sh wait-for-services.sh
#
# Override defaults with environment variables:
#   FP_URL, BACKEND_URL, MOCK_AUTH_URL, MAX_WAIT

set -e

FP_URL=${FP_URL:-http://localhost:6191}
BACKEND_URL=${BACKEND_URL:-http://localhost:3000}
MOCK_AUTH_URL=${MOCK_AUTH_URL:-http://localhost:5001}
MAX_WAIT=${MAX_WAIT:-300}
INTERVAL=5

wait_for_url() {
    name=$1
    url=$2
    elapsed=0

    printf 'Waiting for %s at %s ...\n' "$name" "$url"
    while true; do
        if curl -fsS --max-time 3 "$url/healthcheck" > /dev/null 2>&1; then
            printf '  ✓ %s is ready\n' "$name"
            return 0
        fi
        if [ "$elapsed" -ge "$MAX_WAIT" ]; then
            printf '  ✗ Timed out waiting for %s after %ds\n' "$name" "$MAX_WAIT"
            return 1
        fi
        sleep "$INTERVAL"
        elapsed=$((elapsed + INTERVAL))
    done
}

wait_for_url "Forward Proxy" "$FP_URL"
wait_for_url "Mock Auth"     "$MOCK_AUTH_URL"
wait_for_url "Backend"       "$BACKEND_URL"

printf '\nAll services are ready.\n'
