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
# 1. Trigger policy: ci.yml must trigger ONLY on push to main. Validated
#    SEMANTICALLY via scripts/check-ci-yaml.sh (ruby stdlib YAML), not by
#    positional awk/grep extraction.
# ---------------------------------------------------------------------------
WORKFLOW=".github/workflows/ci.yml"
CI_YAML="scripts/check-ci-yaml.sh"
require "ci.yml exists" test -f "$WORKFLOW"
require "check-ci-yaml.sh exists" test -x "$CI_YAML"

# Behavioral: valid ci.yml passes semantic validation when ruby is present;
# missing declared runtime (ruby) fails clearly instead of silently passing.
if command -v ruby >/dev/null 2>&1; then
  require "ci.yml semantic validation passes" "$CI_YAML"
  require_not "ci.yml semantic validation rejects missing push" \
    bash -c '
      tmp=$(mktemp)
      printf "on:\n  push:\n    branches: [dev]\n" > "$tmp"
      "$1" "$tmp"; rc=$?
      rm -f "$tmp"
      exit "$rc"
    ' _ "$CI_YAML"
  # Comments / multi-line YAML are harmless to a semantic parser.
  require "ci.yml semantic validation tolerates comments and multi-line branches" \
    bash -c '
      tmp=$(mktemp)
      printf "name: CI\n# comment after name\non:\n  push:\n    branches:\n      - main\njobs:\n  rust:\n    name: archctl\n    steps:\n      - run: cargo test\n  bench-smoke:\n    name: archctl bench\n    steps:\n      - run: cargo bench\n  bench-compare:\n    name: bench compare\n    steps:\n      - run: scripts/bench-compare.sh \"\${{ github.event.before }}\"\n  web:\n    name: archview\n    steps:\n      - run: pnpm test\n" > "$tmp"
      "$1" "$tmp"; rc=$?
      rm -f "$tmp"
      exit "$rc"
    ' _ "$CI_YAML"
else
  require_not "ci.yml semantic validation fails clearly without ruby" \
    "$CI_YAML"
  # The failure must name the missing runtime, not silently pass.
  YAML_ERR="$("$CI_YAML" 2>&1 || true)"
  require "ci.yml semantic validation missing-runtime message mentions ruby" \
    grep -q 'ruby' <<<"$YAML_ERR"
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
require "bench-compare has fetch-depth 0" \
  grep -q 'fetch-depth: 0' "$WORKFLOW"
require "bench-compare uses github.event.before" \
  grep -q 'github.event.before' "$WORKFLOW"
require_not "bench-compare not PR-gated" \
  grep -q 'github.event_name == .pull_request' "$WORKFLOW"

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

# --full must wire the contract gates and refresh the benchmark baseline.
require "verify-local.sh --full runs contract gates" \
  grep -q 'test-ci-gates.sh' <<<"$DRY_FULL"
require "verify-local.sh --full fetches fresh origin/main" \
  grep -q 'git fetch origin main' <<<"$DRY_FULL"

# --baseline <ref> overrides the default origin/main and skips the fetch.
DRY_BASELINE="$("$VERIFY_SCRIPT" --dry-run --full --baseline HEAD 2>&1 || true)"
require "verify-local.sh --full --baseline uses explicit ref" \
  grep -q 'bench-compare.sh HEAD' <<<"$DRY_BASELINE"
require_not "verify-local.sh --full --baseline skips fetch" \
  grep -q 'git fetch origin main' <<<"$DRY_BASELINE"

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
# 8c. Branch protection live check: only via explicit --full flag; never from
#     cheap pre-push; no credential leakage; clear failures on gh/auth/network.
# ---------------------------------------------------------------------------
BP_SCRIPT="scripts/check-branch-protection.sh"
require "check-branch-protection.sh exists" test -x "$BP_SCRIPT"

# Behavioral via a fake gh shim (no network; nothing is pushed or mutated).
FAKE_BIN="$(mktemp -d)"
cat > "$FAKE_BIN/gh" <<'SHIM'
#!/usr/bin/env bash
if [ "${1:-}" = "auth" ]; then
  [ "$FAKE_GH_AUTH" = "ok" ] || { echo "not logged in" >&2; exit 1; }
  echo "logged in"; exit 0
fi
if [ "${1:-}" = "api" ]; then
  [ "$FAKE_GH_API_FAIL" = "1" ] && { echo "network error" >&2; exit 1; }
  printf '%s' "$FAKE_GH_JSON"
  exit 0
fi
exit 2
SHIM
chmod +x "$FAKE_BIN/gh"

COMPLIANT='{"required_pull_request_reviews":{"required_approving_review_count":1},"enforce_admins":{"enabled":true},"allow_force_pushes":{"enabled":false},"allow_deletions":{"enabled":false},"required_status_checks":null}'
NONCOMPLIANT='{"required_pull_request_reviews":null,"enforce_admins":{"enabled":false},"allow_force_pushes":{"enabled":true},"allow_deletions":{"enabled":false},"required_status_checks":null}'

require "branch protection passes on compliant live settings" \
  env PATH="$FAKE_BIN:$PATH" FAKE_GH_AUTH=ok FAKE_GH_JSON="$COMPLIANT" \
  bash -c '"$0" >/dev/null 2>&1' "$BP_SCRIPT"
require_not "branch protection fails on non-compliant settings" \
  env PATH="$FAKE_BIN:$PATH" FAKE_GH_AUTH=ok FAKE_GH_JSON="$NONCOMPLIANT" \
  bash -c '"$0" >/dev/null 2>&1' "$BP_SCRIPT"
require_not "branch protection fails clearly without gh auth" \
  env PATH="$FAKE_BIN:$PATH" FAKE_GH_AUTH=no FAKE_GH_JSON="$COMPLIANT" \
  bash -c '"$0" >/dev/null 2>&1' "$BP_SCRIPT"
require_not "branch protection fails clearly on API/network error" \
  env PATH="$FAKE_BIN:$PATH" FAKE_GH_AUTH=ok FAKE_GH_JSON="$COMPLIANT" FAKE_GH_API_FAIL=1 \
  bash -c '"$0" >/dev/null 2>&1' "$BP_SCRIPT"
require_not "branch protection fails clearly without gh CLI" \
  env PATH="/usr/bin:/bin" bash -c '"$0" >/dev/null 2>&1' "$BP_SCRIPT"

# Credentials must never be printed, even when the API response contains one.
LEAK_JSON='{"required_pull_request_reviews":null,"enforce_admins":{"enabled":false},"allow_force_pushes":{"enabled":false},"allow_deletions":{"enabled":false},"required_status_checks":null,"note":"token gho_FAKE_SECRET_123"}'
BP_OUT="$(env PATH="$FAKE_BIN:$PATH" FAKE_GH_AUTH=ok FAKE_GH_JSON="$LEAK_JSON" \
  bash -c '"$0" 2>&1 || true' "$BP_SCRIPT")"
require_not "branch protection never prints credentials" \
  grep -q 'gho_FAKE_SECRET_123' <<<"$BP_OUT"
require "branch protection failure message is human-readable" \
  grep -q 'check-branch-protection' <<<"$BP_OUT"
rm -rf "$FAKE_BIN"

# Cheap pre-push mode must NEVER query branch protection; only --full
# --check-branch-protection may. And --check-branch-protection requires --full.
require_not "verify-local.sh cheap mode never queries branch protection" \
  grep -q 'check-branch-protection' <<<"$DRY_CHEAP"
BP_FULL="$("$VERIFY_SCRIPT" --dry-run --full --check-branch-protection 2>&1 || true)"
require "verify-local.sh --full --check-branch-protection runs it" \
  grep -q 'check-branch-protection.sh' <<<"$BP_FULL"
set +e
"$VERIFY_SCRIPT" --check-branch-protection >/dev/null 2>&1
BP_FLAG_EXIT=$?
set -e
if [ "$BP_FLAG_EXIT" -eq 2 ]; then
  note_pass "verify-local.sh --check-branch-protection without --full exits 2"
else
  note_fail "verify-local.sh --check-branch-protection without --full exits 2 (got $BP_FLAG_EXIT)"
fi

# ---------------------------------------------------------------------------
# 8b. Bundle cap is a single-source executable called by both callers.
# ---------------------------------------------------------------------------
BUNDLE_CAP="scripts/check-bundle-cap.sh"
require "check-bundle-cap.sh exists" test -f "$BUNDLE_CAP"
require "check-bundle-cap.sh executable" test -x "$BUNDLE_CAP"

# Both CI and local verification must call the shared script, and neither may
# keep a private duplicate of the gzip loop.
require "verify-local.sh calls check-bundle-cap.sh" \
  grep -q 'check-bundle-cap.sh' "$VERIFY_SCRIPT"
require "ci.yml calls check-bundle-cap.sh" \
  grep -q 'check-bundle-cap.sh' "$WORKFLOW"
require_not "verify-local.sh has no duplicate bundle-cap loop" \
  grep -q 'gzip -c' "$VERIFY_SCRIPT"
require_not "ci.yml has no duplicate bundle-cap loop" \
  grep -q 'gzip -c' "$WORKFLOW"

# Behavioral: over-limit fixture fails, within-limit passes, using the script.
CAP_DIR="$(mktemp -d)"
head -c 3000000 /dev/urandom > "$CAP_DIR/big.js"
printf 'console.log(1)\n' > "$CAP_DIR/small.js"
require_not "check-bundle-cap.sh rejects over-limit bundle" \
  bash -c '"$0" "$1"/*.js >/dev/null 2>&1' "$BUNDLE_CAP" "$CAP_DIR"
require "check-bundle-cap.sh accepts within-limit bundle" \
  bash -c '"$0" "$1"/small.js >/dev/null 2>&1' "$BUNDLE_CAP" "$CAP_DIR"
rm -rf "$CAP_DIR"

# ---------------------------------------------------------------------------
# 9. Pre-push hook wiring + behavioral ref validation.
# ---------------------------------------------------------------------------
PRE_PUSH=".githooks/pre-push"
require "pre-push hook exists" test -f "$PRE_PUSH"
require "pre-push hook executable" test -x "$PRE_PUSH"
require "install-hooks.sh sets core.hooksPath" \
  grep -q 'core.hooksPath' scripts/install-hooks.sh
require "install-hooks.sh covers pre-push" \
  grep -q 'pre-push' scripts/install-hooks.sh

# Behavioral: the hook must consume Git's stdin ref lines (local_ref
# local_sha remote_ref remote_sha), validate the pushed commits in a temp
# worktree (never the ambient tree), skip zero-SHA deletions, handle
# multiple refs, and clean up worktrees + git metadata on success/failure.
SCRATCH="$(mktemp -d)"
git -C "$SCRATCH" init -q
git -C "$SCRATCH" config user.email test@example.com
git -C "$SCRATCH" config user.name test
mkdir -p "$SCRATCH/scripts" "$SCRATCH/.githooks"
printf 'hello\n' > "$SCRATCH/f.txt"
git -C "$SCRATCH" add f.txt
git -C "$SCRATCH" commit -qm one
SHA1="$(git -C "$SCRATCH" rev-parse HEAD)"
printf 'world\n' >> "$SCRATCH/f.txt"
git -C "$SCRATCH" commit -qam two
SHA2="$(git -C "$SCRATCH" rev-parse HEAD)"

# A fake verify-local.sh that records the worktree path it was invoked from.
cat > "$SCRATCH/scripts/verify-local.sh" <<'FAKE'
#!/usr/bin/env bash
pwd >> "$PP_MARKER"
exit "${PP_EXIT:-0}"
FAKE
chmod +x "$SCRATCH/scripts/verify-local.sh"
cp "$PRE_PUSH" "$SCRATCH/.githooks/pre-push"
chmod +x "$SCRATCH/.githooks/pre-push"

ZERO="0000000000000000000000000000000000000000"
PP_MARKER="$SCRATCH/marker"
# One real push (SHA1) plus one deletion (zero SHA) on stdin.
printf 'refs/heads/main %s refs/heads/main %s\nrefs/heads/del %s refs/heads/del %s\n' \
  "$SHA1" "$SHA1" "$ZERO" "$ZERO" \
  | (cd "$SCRATCH" && PP_MARKER="$PP_MARKER" .githooks/pre-push >/dev/null 2>&1)
PP_EXIT=$?
if [ "$PP_EXIT" -eq 0 ] && [ "$(wc -l < "$PP_MARKER")" -eq 1 ] \
   && [ "$(cat "$PP_MARKER")" != "$SCRATCH" ] \
   && [ "$(git -C "$SCRATCH" worktree list | wc -l)" -eq 1 ]; then
  note_pass "pre-push validates pushed commit in worktree and skips deletion"
else
  note_fail "pre-push validates pushed commit in worktree and skips deletion (exit=$PP_EXIT, lines=$(wc -l < "$PP_MARKER" 2>/dev/null || echo 0), wts=$(git -C "$SCRATCH" worktree list | wc -l))"
fi

# Multiple refs: each unique pushed commit is validated, none from ambient tree.
PP_MARKER="$SCRATCH/marker2"
printf 'refs/heads/a %s refs/heads/a %s\nrefs/heads/b %s refs/heads/b %s\n' \
  "$SHA1" "$SHA1" "$SHA2" "$SHA2" \
  | (cd "$SCRATCH" && PP_MARKER="$PP_MARKER" .githooks/pre-push >/dev/null 2>&1)
if [ "$(wc -l < "$PP_MARKER")" -eq 2 ] \
   && [ "$(git -C "$SCRATCH" worktree list | wc -l)" -eq 1 ]; then
  note_pass "pre-push validates multiple pushed commits"
else
  note_fail "pre-push validates multiple pushed commits (lines=$(wc -l < "$PP_MARKER" 2>/dev/null || echo 0), wts=$(git -C "$SCRATCH" worktree list | wc -l))"
fi

# Failure inside verify-local must propagate exit 1 AND still clean up.
PP_MARKER="$SCRATCH/marker3"
set +e
printf 'refs/heads/main %s refs/heads/main %s\n' "$SHA1" "$SHA1" \
  | (cd "$SCRATCH" && PP_MARKER="$PP_MARKER" PP_EXIT=1 .githooks/pre-push >/dev/null 2>&1)
PP_FAIL_EXIT=$?
set -e
if [ "$PP_FAIL_EXIT" -eq 1 ] \
   && [ "$(git -C "$SCRATCH" worktree list | wc -l)" -eq 1 ]; then
  note_pass "pre-push failure exits 1 and cleans worktrees"
else
  note_fail "pre-push failure exits 1 and cleans worktrees (exit=$PP_FAIL_EXIT, wts=$(git -C "$SCRATCH" worktree list | wc -l))"
fi

# Missing verify-local.sh: fail closed with the documented remediation.
rm "$SCRATCH/scripts/verify-local.sh"
set +e
printf 'refs/heads/main %s refs/heads/main %s\n' "$SHA1" "$SHA1" \
  | (cd "$SCRATCH" && .githooks/pre-push >/dev/null 2>&1)
PP_MISS_EXIT=$?
PP_MISS_MSG="$(cd "$SCRATCH" && printf 'refs/heads/main %s refs/heads/main %s\n' "$SHA1" "$SHA1" | .githooks/pre-push 2>&1 || true)"
set -e
if [ "$PP_MISS_EXIT" -eq 1 ] && grep -q 'refusing push' <<<"$PP_MISS_MSG"; then
  note_pass "pre-push fails closed when verify-local.sh missing"
else
  note_fail "pre-push fails closed when verify-local.sh missing (exit=$PP_MISS_EXIT)"
fi
rm -rf "$SCRATCH"

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
