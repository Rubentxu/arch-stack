#!/usr/bin/env bash
# build.sh — podman build helper for the archctl-bench container
set -euo pipefail

IMAGE="${IMAGE:-archctl-bench:latest}"
CONTAINERFILE="${CONTAINERFILE:-bench/Containerfile}"

usage() {
  cat <<EOF
build.sh — Build the archctl-bench container

USAGE:
  bench/build.sh [--no-cache] [--tag <name>]

ENV:
  IMAGE         image name:tag (default: archctl-bench:latest)
  CONTAINERFILE path to Containerfile (default: bench/Containerfile)
EOF
}

NO_CACHE=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-cache) NO_CACHE="--no-cache"; shift ;;
    --tag) IMAGE="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; usage; exit 1 ;;
  esac
done

echo "Building $IMAGE from $CONTAINERFILE (context: bench/)"
podman build $NO_CACHE -f "$CONTAINERFILE" -t "$IMAGE" bench/
echo "Build OK: $IMAGE"
echo "Verify with: podman run --rm $IMAGE rustc --version"
