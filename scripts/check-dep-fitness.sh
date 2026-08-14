#!/usr/bin/env bash
# check-dep-fitness.sh — architectural dependency fitness check (P1-09).
#
# Implements the self-dogfood rules from
# docs/arch-stack-proposals-2026-08-13/02-TARGET-ARCHITECTURE.md §Self-dogfood
# and §Architectural fitness (05-QUALITY-GATES.md):
#
#   domain      !-> lbug, reqwest            (graph.rs stays pure)
#   application !-> tiny_http, std::process  (no HTTP/server, no subprocess)
#   projection  !-> cli                      (diagram/ never imports cli.rs)
#   analysis    !-> view                     (code/, cognitive/ never import view)
#
# Modes:
#   default     report-only: prints findings + baseline comparison, exit 0
#               unless the finding COUNT exceeds the committed baseline
#               (ratchet).
#   --strict    exit 1 on any finding (future CI-blocking mode, P1-09 DoD).
#   --json      machine-readable output (one JSON object).
#
# The check is deliberately grep-based (no compilation): it inspects `use`
# and `extern crate` statements per module group. It is a baseline tool,
# not a type checker; false positives should be fixed by refining the
# LAYER maps below, not by deleting rules.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
SRC="${REPO_ROOT}/archctl/src"
STRICT=0
JSON=0
for arg in "$@"; do
  case "$arg" in
    --strict) STRICT=1 ;;
    --json)   JSON=1 ;;
    *) echo "check-dep-fitness: unknown flag ${arg}" >&2; exit 2 ;;
  esac
done

# Layer -> glob mapping (relative to archctl/src/).
# Keep in sync with AGENTS.md "Architecture and Design Rules" §Capas.
domain_globs=("graph.rs")
application_globs=("diagram" "evidence.rs" "evaluation.rs" "doctor" "code" "cognitive")
projection_globs=("diagram")
analysis_globs=("code" "cognitive")

expand() { # expand layer globs to files (recursive, .rs only)
  local out=()
  for g in "$@"; do
    while IFS= read -r f; do out+=("$f"); done < <(find "$SRC/$g" -name '*.rs' 2>/dev/null || true)
  done
  printf '%s\n' "${out[@]-}"
}

findings=() # "RULE\tfile:line\tmatch"

check_rule() { # name, forbidden pattern (ERE), files...
  local rule="$1" pattern="$2"; shift 2
  local f line_no line
  for f in "$@"; do
    [ -f "$f" ] || continue
    while IFS=: read -r line_no line; do
      [ -n "$line_no" ] || continue
      findings+=("${rule}	${f#"${REPO_ROOT}/"}:${line_no}	${line#\`}")
    done < <(grep -nE "$pattern" "$f" || true)
  done
}

# ---- collect layer files ---------------------------------------------------
mapfile -t domain_files      < <(expand "${domain_globs[@]}")
mapfile -t application_files < <(expand "${application_globs[@]}")
mapfile -t projection_files  < <(expand "${projection_globs[@]}")
mapfile -t analysis_files    < <(expand "${analysis_globs[@]}")

# ---- rules (02-TARGET-ARCHITECTURE.md §Self-dogfood) ----------------------
# Domain purity: graph.rs must not depend on the storage engine or network.
check_rule "domain!->lbug"     '^[[:space:]]*(pub )?use (crate::)?lbug|^[[:space:]]*extern crate lbug' "${domain_files[@]-}"
check_rule "domain!->reqwest"  '^[[:space:]]*(pub )?use (crate::)?reqwest|^[[:space:]]*extern crate reqwest' "${domain_files[@]-}"

# Application purity: no HTTP server, no subprocess (use adapters).
check_rule "application!->tiny_http"     '^[[:space:]]*(pub )?use (crate::)?tiny_http|^[[:space:]]*extern crate tiny_http' "${application_files[@]-}"
check_rule "application!->std::process"  '^[[:space:]]*(pub )?use std::process' "${application_files[@]-}"

# Projection independence: diagram/ must not import the CLI layer.
check_rule "projection!->cli" '^[[:space:]]*(pub )?use (crate|archctl)::cli' "${projection_files[@]-}"

# Analysis independence: code/, cognitive/ must not import the view server.
check_rule "analysis!->view"  '^[[:space:]]*(pub )?use (crate|archctl)::view' "${analysis_files[@]-}"

# ---- baseline ratchet ------------------------------------------------------
# Committed baseline (see docs/reports/dep-fitness-baseline.md). The count of
# findings must never EXCEED the baseline; shrinking it is celebrated by
# editing this number down. Rationale: legacy violations (e.g. graph.rs -> lbug)
# are paid down incrementally; new ones must not appear.
BASELINE_FILE="${REPO_ROOT}/scripts/dep-fitness-baseline.txt"
BASELINE=0
if [ -f "$BASELINE_FILE" ]; then
  BASELINE="$(head -1 "$BASELINE_FILE")"
fi

count="${#findings[@]}"

# ---- output ----------------------------------------------------------------
if [ "$JSON" -eq 1 ]; then
  printf '{"findings": %d, "baseline": %d, "strict": %d, "details": [' "$count" "$BASELINE" "$STRICT"
  first=1
  for entry in "${findings[@]-}"; do
    IFS=$'	' read -r rule loc match <<<"$entry"
    [ $first -eq 0 ] && printf ', '
    first=0
    printf '{"rule": "%s", "location": "%s", "match": "%s"}' "$rule" "$loc" "${match//\"/\\\"}"
  done
  printf ']}\n'
else
  if [ "$count" -eq 0 ]; then
    echo "dep-fitness: OK — 0 findings (baseline ${BASELINE})"
  else
    echo "dep-fitness: ${count} finding(s) (baseline ${BASELINE}):"
    for entry in "${findings[@]-}"; do
      IFS=$'	' read -r rule loc match <<<"$entry"
      printf '  %-28s %s  %s\n' "$rule" "$loc" "${match:0:60}"
    done
  fi
fi

# ---- verdict ----------------------------------------------------------------
if [ "$STRICT" -eq 1 ]; then
  if [ "$count" -gt 0 ]; then
    echo "dep-fitness: STRICT mode — failing on ${count} finding(s)" >&2
    exit 1
  fi
else
  if [ "$count" -gt "$BASELINE" ]; then
    echo "dep-fitness: RATCHET breached — ${count} > baseline ${BASELINE}." \
         "Fix the new violation or (if intentional) update ${BASELINE_FILE#"${REPO_ROOT}/"} with justification." >&2
    exit 1
  fi
fi
exit 0
