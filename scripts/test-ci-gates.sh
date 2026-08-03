#!/usr/bin/env bash
# test-ci-gates.sh — deterministic script-level checks for the ci-main-gates
# change. Verifies trigger policy, preserved jobs, baseline wiring, zero-SHA /
# error behavior, local verify modes, hook wiring, toolchain pin and MSRV.
#
# Usage:
#   scripts/test-ci-gates.sh
#
# Exit codes:
#   0 = all checks pass
#   1 = one or more checks failed
#
# The script is intentionally hermetic: it reads static files and runs the
# synthetic (TEST_FAKE_*) paths of bench-compare.sh. It does NOT run cargo,
# pnpm, or live benchmarks, so it is fast and deterministic on any host.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FAILED=0
PASSED=0

note_fail() {
  echo "FAIL: $1"
  FAILED=$((FAILED + 1))
}

note_pass() {
  echo "ok:   $1"
  PASSED=$((PASSED + 1))
}

# require <description> <command...> — command must exit 0
require() {
  local desc="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    note_pass "$desc"
  else
    note_fail "$desc"
  fi
}

# require_not <description> <command...> — command must exit non-zero
require_not() {
  local desc="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    note_fail "$desc (unexpected success)"
  else
    note_pass "$desc"
  fi
}

# ---------------------------------------------------------------------------
# 1. Trigger policy: ci.yml must trigger ONLY on push to main.
# ---------------------------------------------------------------------------
WORKFLOW=".github/workflows/ci.yml"
require "ci.yml exists" test -f "$WORKFLOW"
require "ci.yml has 'on:'" grep -q '^on:' "$WORKFLOW"

# Extract the trigger block between 'on:' and the first job key ('name:' or 'jobs:').
TRIGGER_BLOCK="$(awk '/^on:/{f=1;next} /^jobs:/{f=0} f' "$WORKFLOW")"

require "trigger block contains push" \
  grep -q 'push:' <<<"$TRIGGER_BLOCK"
require "push targets main only" \
  grep -q 'branches: \[main\]' <<<"$TRIGGER_BLOCK"
require_not "no pull_request trigger" \
  grep -q 'pull_request' <<<"$TRIGGER_BLOCK"
require_not "no workflow_dispatch trigger" \
  grep -q 'workflow_dispatch' <<<"$TRIGGER_BLOCK"
require_not "no schedule trigger" \
  grep -q 'schedule' <<<"$TRIGGER_BLOCK"
# Every branches: line in the trigger block must target exactly [main].
BRANCH_LINES="$(grep -c 'branches:' <<<"$TRIGGER_BLOCK" || true)"
if [ "$BRANCH_LINES" -eq 1 ] && grep -q 'branches: \[main\]' <<<"$TRIGGER_BLOCK"; then
  note_pass "only branch trigger is main"
else
  note_fail "only branch trigger is main (branches lines: ${BRANCH_LINES})"
fi

# ---------------------------------------------------------------------------
# 2. Preserved post-merge jobs (all four gate groups).
# ---------------------------------------------------------------------------
require "job rust present" grep -q '^  rust:' "$WORKFLOW"
require "job bench-smoke present" grep -q '^  bench-smoke:' "$WORKFLOW"
require "job bench-compare present" grep -q '^  bench-compare:' "$WORKFLOW"
require "job web present" grep -q '^  web:' "$WORKFLOW"

# ---------------------------------------------------------------------------
# 3. bench-compare baseline wiring (post-merge, github.event.before).
# ---------------------------------------------------------------------------
BENCH_COMPARE_JOB="$(awk '/^  bench-compare:/{f=1;next} /^  [a-z-]+:/{f=0} f' "$WORKFLOW")"
require "bench-compare has fetch-depth 0" \
  grep -q 'fetch-depth: 0' <<<"$BENCH_COMPARE_JOB"
require "bench-compare uses github.event.before" \
  grep -q 'github.event.before' <<<"$BENCH_COMPARE_JOB"
require_not "bench-compare not PR-gated" \
  grep -q 'github.event_name == .pull_request' <<<"$BENCH_COMPARE_JOB"

# ---------------------------------------------------------------------------
# 4. No floating toolchain steps; pinned via root rust-toolchain.toml.
# ---------------------------------------------------------------------------
require_not "no 'rustup toolchain install' floating step" \
  grep -q 'rustup toolchain install' "$WORKFLOW"
require "rust-toolchain.toml exists" test -f rust-toolchain.toml
require "toolchain channel pinned 1.97.1" \
  grep -q 'channel = "1.97.1"' rust-toolchain.toml

# ---------------------------------------------------------------------------
# 5. MSRV declared in archctl/Cargo.toml.
# NOTE: spec proposed 1.85, but the dependency tree requires 1.91 (validated
# empirically at apply: cargo-platform@0.3.3 -> rustc 1.91, idna_adapter/ignore/
# time -> 1.86-1.88). Declaring 1.85 would be false; ADR-025 records the raise.
# ---------------------------------------------------------------------------
require "MSRV rust-version 1.91 declared" \
  grep -q 'rust-version = "1.91"' archctl/Cargo.toml

# ---------------------------------------------------------------------------
# 6. Clippy repair: filesystem.rs uses .keys(), no lint suppression.
# ---------------------------------------------------------------------------
require "filesystem.rs uses .keys()" \
  grep -q 'for file_path in files.keys()' archctl/src/filesystem.rs
require_not "no clippy for_kv_map suppression" \
  grep -q 'allow(clippy::for_kv_map' archctl/src/filesystem.rs

# ---------------------------------------------------------------------------
# 7. bench-compare.sh baseline wiring and zero-SHA / error behavior.
# ---------------------------------------------------------------------------
BENCH_SCRIPT="scripts/bench-compare.sh"
require "bench-compare.sh exists" test -x "$BENCH_SCRIPT"
require "bench-compare.sh --help exits 0" \
  bash -c '"$0" --help >/dev/null 2>&1' "$BENCH_SCRIPT"

# Default baseline is origin/main (valid in a normal clone).
require "bench-compare.sh default baseline origin/main accepted" \
  bash -c '"$0" --fake-regression 0 >/dev/null 2>&1' "$BENCH_SCRIPT"

# Explicit valid baseline (HEAD) accepted.
require "bench-compare.sh explicit valid baseline accepted" \
  bash -c '"$0" --fake-regression 0 "$1" >/dev/null 2>&1' \
  "$BENCH_SCRIPT" "$(git rev-parse HEAD)"

# All-zero SHA must fail clearly (exit 2).
require_not "bench-compare.sh all-zero SHA rejected (exit 2)" \
  bash -c '"$0" --fake-regression 0 0000000000000000000000000000000000000000 >/dev/null 2>&1' \
  "$BENCH_SCRIPT"

# Invalid / unreachable baseline must fail clearly (exit 2).
require_not "bench-compare.sh invalid baseline rejected (exit 2)" \
  bash -c '"$0" --fake-regression 0 no-such-ref-xyz >/dev/null 2>&1' \
  "$BENCH_SCRIPT"

# ADR-019 synthetic regression: over threshold exits 1, within exits 0.
require_not "bench-compare.sh synthetic over-threshold exits 1" \
  bash -c '"$0" --fake-regression 11 >/dev/null 2>&1' "$BENCH_SCRIPT"
require "bench-compare.sh synthetic within-threshold exits 0" \
  bash -c '"$0" --fake-regression 5 >/dev/null 2>&1' "$BENCH_SCRIPT"

# Hidden environment bypass must be gone: TEST_FAKE_REGRESSION is a production
# path that could disable gates without an explicit CLI option.
require_not "bench-compare.sh no TEST_FAKE_REGRESSION env bypass" \
  grep -qE '\$\{?TEST_FAKE_REGRESSION' "$BENCH_SCRIPT"

# python3 prerequisite must fail clearly before creating any worktree.
MINBIN="$(mktemp -d)"
ln -s "$(command -v bash)" "$MINBIN/bash"
NOPY_MSG="$(env PATH="$MINBIN" bash -c '"$0" --fake-regression 0 2>&1' "$BENCH_SCRIPT" || true)"
set +e
env PATH="$MINBIN" bash -c '"$0" --fake-regression 0 >/dev/null 2>&1' "$BENCH_SCRIPT"
NOPY_EXIT=$?
set -e
rm -rf "$MINBIN"
require "bench-compare.sh missing python3 message mentions python3" \
  grep -q 'python3' <<<"$NOPY_MSG"
if [ "$NOPY_EXIT" -eq 2 ]; then
  note_pass "bench-compare.sh fails clearly without python3 (exit 2)"
else
  note_fail "bench-compare.sh fails clearly without python3 (exit 2, got $NOPY_EXIT)"
fi

# ---------------------------------------------------------------------------
# 8. verify-local.sh tiered local verification.
# ---------------------------------------------------------------------------
VERIFY_SCRIPT="scripts/verify-local.sh"
require "verify-local.sh exists" test -f "$VERIFY_SCRIPT"
require "verify-local.sh executable" test -x "$VERIFY_SCRIPT"

# Dry-run mode prints the cheap Rust gates without executing them.
DRY_CHEAP="$("$VERIFY_SCRIPT" --dry-run 2>&1 || true)"
require "verify-local.sh cheap mode runs cargo test" \
  grep -q 'cargo test' <<<"$DRY_CHEAP"
require "verify-local.sh cheap mode runs cargo clippy" \
  grep -q 'cargo clippy' <<<"$DRY_CHEAP"
require "verify-local.sh cheap mode runs cargo fmt" \
  grep -q 'cargo fmt' <<<"$DRY_CHEAP"
require "verify-local.sh cheap mode runs doctor" \
  grep -q 'doctor' <<<"$DRY_CHEAP"
require_not "verify-local.sh cheap mode skips bench-compare" \
  grep -q 'bench-compare' <<<"$DRY_CHEAP"

# Full mode adds web gates + ADR-019 comparison vs origin/main.
DRY_FULL="$("$VERIFY_SCRIPT" --dry-run --full 2>&1 || true)"
require "verify-local.sh --full runs pnpm test" \
  grep -q 'pnpm test' <<<"$DRY_FULL"
require "verify-local.sh --full runs pnpm build" \
  grep -q 'pnpm build' <<<"$DRY_FULL"
require "verify-local.sh --full runs bench-compare origin/main" \
  grep -q 'bench-compare.sh origin/main' <<<"$DRY_FULL"
require "verify-local.sh --full runs bench smoke" \
  grep -q 'bench --bench' <<<"$DRY_FULL"

# The former hidden env bypass must be gone: dry-run is now an explicit flag.
require_not "verify-local.sh no VERIFY_LOCAL_DRY_RUN env bypass" \
  grep -q 'VERIFY_LOCAL_DRY_RUN' "$VERIFY_SCRIPT"
require "verify-local.sh --help documents --dry-run" \
  bash -c '"$0" --help 2>&1 | grep -q -- "--dry-run"' "$VERIFY_SCRIPT"

# Usage / prerequisite errors exit 2 (unknown flag).
set +e
"$VERIFY_SCRIPT" --bogus >/dev/null 2>&1
VERIFY_USAGE_EXIT=$?
set -e
if [ "$VERIFY_USAGE_EXIT" -eq 2 ]; then
  note_pass "verify-local.sh unknown flag exits 2"
else
  note_fail "verify-local.sh unknown flag exits 2 (got $VERIFY_USAGE_EXIT)"
fi

# ---------------------------------------------------------------------------
# 9. Pre-push hook wiring.
# ---------------------------------------------------------------------------
PRE_PUSH=".githooks/pre-push"
require "pre-push hook exists" test -f "$PRE_PUSH"
require "pre-push hook executable" test -x "$PRE_PUSH"
require "pre-push hook calls verify-local.sh" \
  grep -q 'verify-local.sh' "$PRE_PUSH"
require "install-hooks.sh sets core.hooksPath" \
  grep -q 'core.hooksPath' scripts/install-hooks.sh
require "install-hooks.sh covers pre-push" \
  grep -q 'pre-push' scripts/install-hooks.sh

# ---------------------------------------------------------------------------
# 10. ADR-025 recorded and indexed.
# ---------------------------------------------------------------------------
require "ADR-025 file exists" \
  bash -c 'compgen -G "docs/adr/ADR-025-*.md" >/dev/null'
require "ADR index lists ADR-025" \
  grep -q 'ADR-025' docs/adr/README.md
require "docs manifest lists ADR-025" \
  grep -q 'ADR-025' docs/manifest.json
require "CHANGELOG references ci-main-gates" \
  grep -q 'ci-main-gates' CHANGELOG.md

# ---------------------------------------------------------------------------
echo ""
echo "----------------------------------------"
echo "test-ci-gates.sh: ${PASSED} passed, ${FAILED} failed"
if [ "$FAILED" -ne 0 ]; then
  exit 1
fi
exit 0
