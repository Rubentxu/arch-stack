#!/usr/bin/env bash
# entrypoint.sh — invoked by the Quadlet container
# Default: print toolchain versions, then exec $@ (the orchestrator)
set -euo pipefail
echo "=== archctl-bench container ==="
echo "rustc: $(rustc --version)"
echo "cargo: $(cargo --version)"
echo "git:   $(git --version)"
echo "jq:    $(jq --version)"
echo "=============================="
exec "$@"
