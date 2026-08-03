#!/usr/bin/env bash
# check-ci-yaml.sh — semantic validation of .github/workflows/ci.yml (ADR-025).
#
# Replaces brittle awk/grep positional YAML parsing with real YAML parsing
# via ruby's stdlib (declared runtime; no app dependency). If ruby is
# unavailable the check fails clearly instead of silently passing on broken
# YAML. Handles YAML 1.1 gotcha: Psych parses the workflow `on:` key as
# boolean true, so both `doc["on"]` and `doc[true]` are considered.
#
# Usage:
#   scripts/check-ci-yaml.sh [workflow-file]
#
# Exit codes:
#   0 = workflow YAML matches ADR-025/ADR-019 policy
#   1 = semantic violation or malformed YAML
#   2 = prerequisite missing (ruby) or usage error

set -euo pipefail

WORKFLOW="${1:-.github/workflows/ci.yml}"

if ! command -v ruby >/dev/null 2>&1; then
    echo "check-ci-yaml: error: ruby is required for semantic YAML validation (stdlib yaml); install ruby or skip this check" >&2
    exit 2
fi

if [ ! -f "$WORKFLOW" ]; then
    echo "check-ci-yaml: error: workflow file not found: ${WORKFLOW}" >&2
    exit 2
fi

ruby -ryaml - "$WORKFLOW" <<'RUBY'
path = ARGV[0]
begin
  doc = YAML.safe_load(File.read(path), aliases: true)
rescue => e
  warn "check-ci-yaml: malformed YAML: #{e.message}"
  exit 1
end
abort "check-ci-yaml: workflow root is not a mapping" unless doc.is_a?(Hash)
on = doc["on"] || doc[true]
abort "check-ci-yaml: missing on:" unless on.is_a?(Hash)
push = on["push"]
abort "check-ci-yaml: missing on.push" unless push.is_a?(Hash)
abort "check-ci-yaml: on.push.branches != [main]: #{push["branches"].inspect}" unless push["branches"] == ["main"]
abort "check-ci-yaml: unexpected pull_request trigger" if on["pull_request"]
abort "check-ci-yaml: unexpected workflow_dispatch trigger" if on["workflow_dispatch"]
abort "check-ci-yaml: unexpected schedule trigger" if on["schedule"]
jobs = doc["jobs"]
abort "check-ci-yaml: missing jobs" unless jobs.is_a?(Hash)
%w[rust bench-smoke bench-compare web].each do |j|
  abort "check-ci-yaml: missing job #{j}" unless jobs.key?(j)
end
bc = jobs["bench-compare"]
abort "check-ci-yaml: bench-compare has no steps" unless bc["steps"].is_a?(Array)
runs = bc["steps"].map { |s| s["run"].to_s }.join("\n")
abort "check-ci-yaml: bench-compare must invoke scripts/bench-compare.sh" unless runs.include?("scripts/bench-compare.sh")
abort "check-ci-yaml: bench-compare must use github.event.before" unless runs.include?("github.event.before")
puts "check-ci-yaml: OK #{path}"
RUBY
