#!/usr/bin/env bash
# routing-check.sh — CI eval for arch-stack skills routing
# Verifies: no trigger collisions, each skill has triggers, routing matrix is consistent.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILLS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"  # eval → skills
TRIGGERS_FILE="$SCRIPT_DIR/triggers.yaml"
CI="${CI:-false}"

# Colors (disabled in CI)
if [ "$CI" = "false" ] && [ -t 1 ]; then
  RED=$'\033[0;31m'; GREEN=$'\033[0;32m'; YELLOW=$'\033[0;33m'; NC=$'\033[0m'
else
  RED=''; GREEN=''; YELLOW=''; NC=''
fi

errors=0; warnings=0

log_pass()  { echo "${GREEN}PASS${NC}: $1"; }
log_fail()  { echo "${RED}FAIL${NC}: $1"; ((errors++)); }
log_warn()  { echo "${YELLOW}WARN${NC}: $1"; ((warnings++)); }
log_info()  { echo "INFO: $1"; }

# ---------------------------------------------------------------------------
# 1. Parse frontmatter from all SKILL.md files
# ---------------------------------------------------------------------------
declare -A skill_names
declare -A skill_triggers

log_info "Scanning skills in $SKILLS_DIR..."

for skill_md in "$SKILLS_DIR"/*/SKILL.md; do
  [ -f "$skill_md" ] || continue
  skill_key="${skill_md%/*}"
  skill_key="${skill_key##*/}"

  name_value=$(sed -n 's/^name:[[:space:]]*//p' "$skill_md" | tr -d '"' || echo "")
  desc_value=$(sed -n 's/^description:[[:space:]]*//p' "$skill_md" || echo "")

  [ -z "$name_value" ] && name_value="$skill_key"
  skill_names["$skill_key"]="$name_value"

  # Extract quoted phrases from description (trigger candidates)
  quoted=$(echo "$desc_value" | grep -oE '"[^"]+"' 2>/dev/null \
    | sed 's/"//g' | tr '[:upper:]' '[:lower:]' | tr -d ',' \
    | grep -v '^[[:space:]]*$' | sort -u | tr '\n' ',' | sed 's/,$//')
  skill_triggers["$skill_key"]="$quoted"
done

skill_count=${#skill_names[@]}
log_info "Found $skill_count skills."

# ---------------------------------------------------------------------------
# 2. Collision detection — no two skills share the same trigger phrase
# ---------------------------------------------------------------------------
log_info "Checking for trigger phrase collisions..."

declare -A phrase_owner

for skill_key in "${!skill_triggers[@]}"; do
  triggers="${skill_triggers[$skill_key]}"
  [ -z "$triggers" ] && continue
  IFS=',' read -ra PHRASES <<< "$triggers"
  for phrase in "${PHRASES[@]}"; do
    phrase="$(echo "$phrase" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    [ -z "$phrase" ] && continue
    if [[ -v phrase_owner["$phrase"] ]]; then
      owner="${phrase_owner[$phrase]}"
      [ "$owner" != "$skill_key" ] && log_fail "Collision: '$phrase' claimed by '$owner' and '$skill_key'"
    else
      phrase_owner["$phrase"]="$skill_key"
    fi
  done
done

# ---------------------------------------------------------------------------
# 3. Coverage — each skill has at least one trigger phrase
# ---------------------------------------------------------------------------
log_info "Checking coverage (each skill has ≥1 trigger phrase)..."
for skill_key in "${!skill_triggers[@]}"; do
  [ -z "${skill_triggers[$skill_key]}" ] && log_fail "Skill '$skill_key' has no quoted trigger phrases"
done

# ---------------------------------------------------------------------------
# 4. Validate triggers.yaml coverage (Python YAML parser)
# ---------------------------------------------------------------------------
if [ -f "$TRIGGERS_FILE" ] && command -v python3 &>/dev/null; then
  log_info "Validating triggers.yaml..."
  python3 - "$SKILLS_DIR" "$TRIGGERS_FILE" "${!skill_names[@]}" <<'PYEOF'
import sys, yaml

skills_dir, triggers_file = sys.argv[1], sys.argv[2]
skill_keys = set(sys.argv[3:])

with open(triggers_file) as f:
    data = yaml.safe_load(f)

errors = 0
warnings = 0

for skill_key, v in data.items():
    if skill_key not in skill_keys:
        print(f"WARN: triggers.yaml references unknown skill: '{skill_key}'")
        warnings += 1
    if isinstance(v, dict):
        should = v.get("should_trigger", [])
        should_not = v.get("should_not_trigger", [])
        if not should:
            print(f"WARN: triggers.yaml '{skill_key}' has no should_trigger entries")
            warnings += 1

print(f"INFO: triggers.yaml validates ({len(data)} skills)")
sys.exit(0)
PYEOF
fi

# ---------------------------------------------------------------------------
# 5. Summary
# ---------------------------------------------------------------------------
echo ""
if [ "$errors" -eq 0 ] && [ "$warnings" -eq 0 ]; then
  log_pass "All routing checks passed ($skill_count skills)"
  exit 0
elif [ "$errors" -eq 0 ]; then
  log_pass "Routing passed with $warnings warning(s)"
  exit 0
else
  log_fail "Routing failed: $errors error(s), $warnings warning(s)"
  exit 1
fi
