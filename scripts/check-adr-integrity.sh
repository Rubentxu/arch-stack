#!/usr/bin/env bash
#
# check-adr-integrity.sh — Validate ADR directory integrity
#
# Checks: unique-ids, filename-header-match, valid-status,
#         index-consistency, broken-links, gap-info
#
# Usage:
#   scripts/check-adr-integrity.sh [--json] [--adr-dir <path>]
#
# Exit codes:
#   0 = clean (warnings allowed)
#   2 = violations found (errors in unique-ids, filename-header, or broken-links)

set -uo pipefail

ADR_DIR=""
OUTPUT_JSON=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --json)     OUTPUT_JSON=true; shift ;;
        --adr-dir)  ADR_DIR="$2"; shift 2 ;;
        --adr-dir=*) ADR_DIR="${1#--adr-dir=}"; shift ;;
        -h|--help)
            echo "Usage: $0 [--json] [--adr-dir <path>]"
            echo "Default ADR dir: docs/adr"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
if [[ -z "$ADR_DIR" ]]; then
    ADR_DIR="$REPO_ROOT/docs/adr"
fi

if [[ ! -d "$ADR_DIR" ]]; then
    echo "Error: ADR directory not found: $ADR_DIR" >&2
    exit 1
fi

ERRORS=0
WARNINGS=0
VIOLATIONS=""

add_violation() {
    local check="$1" severity="$2" file="$3" message="$4"
    # Escape message for display
    local escaped_msg="${message//\"/\\\"}"
    if [[ -n "$VIOLATIONS" ]]; then
        VIOLATIONS+=$'\n'
    fi
    VIOLATIONS+="${check}|${severity}|${file}|${escaped_msg}"
    if [[ "$severity" == "error" ]]; then
        ERRORS=$((ERRORS + 1))
    elif [[ "$severity" == "warning" ]]; then
        WARNINGS=$((WARNINGS + 1))
    fi
}

# ─── Collect all ADR files ─────────────────────────────────────────

# Build a temp file with: filename|file_id|header_id
TEMP_DATA=$(mktemp)
trap 'rm -f "$TEMP_DATA"' EXIT

for f in "$ADR_DIR"/ADR-*.md; do
    [[ -f "$f" ]] || continue
    filename="$(basename "$f")"

    # Extract ID from filename
    file_id="$(echo "$filename" | grep -oP '^ADR-\d{3}' || true)"
    if [[ -z "$file_id" ]]; then
        add_violation "filename-format" "error" "$filename" "Filename does not match ADR-NNN pattern"
        continue
    fi

    # Extract ID from H1 header (first 10 lines to catch frontmatter)
    header_id="$(head -10 "$f" | grep -oP '# ADR-\d{3}' | grep -oP 'ADR-\d{3}' || true)"

    echo "${filename}|${file_id}|${header_id}" >> "$TEMP_DATA"
done

TOTAL_ADRS=$(wc -l < "$TEMP_DATA" | tr -d ' ')

# ─── C1: unique-ids ────────────────────────────────────────────────

while IFS='|' read -r _ file_id _; do
    count=$(grep -c "|${file_id}|" "$TEMP_DATA" || true)
    if [[ $count -gt 1 ]]; then
        # Collect filenames for this ID
        files_with_id=$(grep "|${file_id}|" "$TEMP_DATA" | cut -d'|' -f1 | tr '\n' ' ')
        add_violation "unique-ids" "error" "$files_with_id" "$file_id appears in $count files"
    fi
done < "$TEMP_DATA" | sort -u  # deduplicate (each ID reported once)

# Actually, the above approach is flawed — let me do it differently
# Reset and redo C1 properly
ERRORS=0
WARNINGS=0
VIOLATIONS=""

# Collect unique IDs and check for duplicates
declare -A id_count
declare -A id_files
while IFS='|' read -r filename file_id header_id; do
    id_count["$file_id"]=$(( ${id_count["$file_id"]:-0} + 1 ))
    id_files["$file_id"]+="$filename "
done < "$TEMP_DATA"

for id in "${!id_count[@]}"; do
    if [[ ${id_count[$id]} -gt 1 ]]; then
        add_violation "unique-ids" "error" "${id_files[$id]}" "$id appears in ${id_count[$id]} files: ${id_files[$id]}"
    fi
done

# ─── C2: filename-header-match ─────────────────────────────────────

while IFS='|' read -r filename file_id header_id; do
    if [[ -n "$header_id" && "$file_id" != "$header_id" ]]; then
        add_violation "filename-header-match" "error" "$filename" "Filename says $file_id but H1 says $header_id"
    fi
done < "$TEMP_DATA"

# ─── C3+C4: index-consistency and valid-status ─────────────────────

README="$ADR_DIR/README.md"
declare -A INDEXED_ADRS=()

if [[ -f "$README" ]]; then
    while IFS= read -r line; do
        # Extract first ADR-NNN from the line (from link text)
        indexed_id="$(echo "$line" | grep -oP 'ADR-\d{3}' | head -1 || true)"
        if [[ -n "$indexed_id" ]]; then
            INDEXED_ADRS["$indexed_id"]="1"

            # Check status — extract text after last | before trailing |
            status_text="$(echo "$line" | awk -F'|' '{print $(NF-1)}' | xargs)"
            status_lower="$(echo "$status_text" | tr '[:upper:]' '[:lower:]')"
            if ! [[ "$status_lower" =~ (aceptado|accepted|propuesto|proposed|drafted|superseded|deprecated|deprecado) ]]; then
                add_violation "valid-status" "warning" "README.md" "$indexed_id has unrecognized status: $status_text"
            fi
        fi
    done < <(grep '| \[ADR-' "$README" 2>/dev/null || true)
fi

# C4: check all files are in index
while IFS='|' read -r filename file_id header_id; do
    if [[ -z "${INDEXED_ADRS[$file_id]:-}" ]]; then
        add_violation "index-consistency" "warning" "$filename" "$file_id not found in README.md index"
    fi
done < "$TEMP_DATA"

# ─── C5: broken-links ──────────────────────────────────────────────

for f in "$ADR_DIR"/ADR-*.md; do
    [[ -f "$f" ]] || continue
    filename="$(basename "$f")"
    # Find markdown links to ADR files
    while IFS= read -r match; do
        [[ -z "$match" ]] && continue
        # Extract target filename from [text](target.md)
        target="$(echo "$match" | grep -oP '\]\(\K[^)]+')"
        target_file="$ADR_DIR/$target"
        if [[ ! -f "$target_file" ]]; then
            link_text="$(echo "$match" | grep -oP '\[\K[^\]]+')"
            add_violation "broken-links" "error" "$filename" "Link [$link_text]($target) → file not found"
        fi
    done < <(grep -oP '\[ADR-\d{3}\]\([^)]+\)' "$f" 2>/dev/null || true)
done

# ─── C6: gap-info ──────────────────────────────────────────────────

all_nums=$(cut -d'|' -f2 "$TEMP_DATA" | grep -oP '\d+' | sed 's/^0*//' | sort -n | uniq)
prev=-1
for num in $all_nums; do
    num=$((10#$num))
    if [[ $prev -ge 0 ]] && [[ $((num - prev)) -gt 1 ]]; then
        for ((g=prev+1; g<num; g++)); do
            printf -v gap_id "ADR-%03d" "$g"
            # Info violations don't count as errors or warnings
            if [[ -n "$VIOLATIONS" ]]; then
                VIOLATIONS+=$'\n'
            fi
            VIOLATIONS+="gap-info|info|—|Gap: $gap_id not found"
        done
    fi
    prev=$num
done

# ─── Report ────────────────────────────────────────────────────────

if $OUTPUT_JSON; then
    echo "{"
    echo "  \"summary\": { \"total_adrs\": $TOTAL_ADRS, \"errors\": $ERRORS, \"warnings\": $WARNINGS },"
    echo -n "  \"violations\": ["
    first=true
    while IFS='|' read -r check severity file message; do
        [[ "$severity" == "info" ]] && continue
        if $first; then
            first=false
        else
            echo -n ","
        fi
        printf '\n    {"check": "%s", "severity": "%s", "file": "%s", "message": "%s"}' \
            "$check" "$severity" "$file" "$message"
    done <<< "$VIOLATIONS"
    if ! $first; then echo; echo -n "  "; fi
    echo "]"
    echo "}"
else
    if [[ -z "$VIOLATIONS" ]]; then
        echo "OK: $TOTAL_ADRS ADRs verified, 0 violations"
        exit 0
    fi

    while IFS='|' read -r check severity file message; do
        case "$severity" in
            info)    printf '[INFO]  %-25s %s\n' "$check:" "$message" ;;
            warning) printf '[WARN]  %-25s %s (%s)\n' "$check:" "$message" "$file" ;;
            error)   printf '[ERROR] %-25s %s (%s)\n' "$check:" "$message" "$file" ;;
        esac
    done <<< "$VIOLATIONS"

    echo ""
    echo "Summary: $TOTAL_ADRS ADRs, $ERRORS error(s), $WARNINGS warning(s)"
fi

if [[ $ERRORS -gt 0 ]]; then
    exit 2
fi
exit 0
