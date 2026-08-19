#!/usr/bin/env bash
# bench/build-in-sandbox.sh — compile archctl INSIDE the archctl-bench
# container and export a container-compatible release binary.
#
# Why: a host-built binary links against the host's libstdc++
# (GLIBCXX_3.4.35+) which ubuntu:24.04 does not ship. The M27 design
# compiles in-container; this helper makes that a single command and
# exports the artifact so subsequent runs can mount it read-only.
#
# Usage:
#   bench/build-in-sandbox.sh [--out <path>]
#
# Env:
#   OUT   output path (default: /tmp/opencode/archctl-ubuntu24)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="archctl-bench:latest"
OUT="${OUT:-/tmp/opencode/archctl-ubuntu24}"

command -v podman >/dev/null || { echo "podman required" >&2; exit 1; }
podman image exists "$IMAGE" || { echo "image missing: $IMAGE — run bench/build.sh" >&2; exit 1; }
mkdir -p "$(dirname "$OUT")"

echo "== compile archctl in-container (target /tmp/target) =="
# CARGO_TARGET_DIR is overridden explicitly: the host ~/.cargo/config.toml
# (mounted for the registry cache) points at /var/home/rubentxu/cargo-targets,
# which does not exist inside the container and would silently discard the
# build artifacts. Env override wins over the config file.
podman run --rm --security-opt label=disable \
  -v "$REPO_ROOT:/src:rw" \
  -v "$HOME/.cargo:/root/.cargo:rw" \
  -v "$(dirname "$OUT"):/out:rw" \
  -e CARGO_TARGET_DIR=/tmp/target \
  "$IMAGE" bash -c 'cd /src/archctl && cargo build --release --quiet && cp /tmp/target/release/archctl /out/'"$(basename "$OUT")"

echo "OK: $(basename "$OUT") ($(podman run --rm --security-opt label=disable \
  -v "$OUT:/usr/local/bin/archctl:ro" "$IMAGE" bash -c '/usr/local/bin/archctl --version' 2>/dev/null | tail -1))"
