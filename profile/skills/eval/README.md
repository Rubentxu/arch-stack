# Skills Eval — Routing Tests

Framework for verifying skill routing correctness in CI.

## What it tests

| Test | Description |
|---|---|
| **Routing** | Each trigger phrase maps to exactly one skill |
| **Collision** | No two skills share the same trigger phrase |
| **Coverage** | Each skill has at least one trigger phrase |
| **Vocabulary** | Trigger phrases use language users actually say |

## Running

```bash
# Local (from repo root)
bash profile/skills/eval/routing-check.sh

# CI (from repo root)
bash profile/skills/eval/routing-check.sh --ci
```

Exit codes: `0` = pass, `1` = routing/collision error, `2` = parse error.

## Trigger file format

`triggers.yaml` defines the expected routing matrix:

```yaml
architecture-discovery:
  should_trigger:
    - "discover the architecture"
    - "scan the codebase"
    - "map the modules"
  should_not_trigger:
    - "generate a class diagram"
    - "review this diagram"
c4-from-graph:
  should_trigger:
    - "generate a C4 diagram"
    - "show me the containers"
  should_not_trigger:
    - "discover services"
```

## Adding a new skill

1. Add the skill to `triggers.yaml` under its canonical name
2. Add `should_trigger` phrases (use actual user vocabulary)
3. Add `should_not_trigger` phrases (phrases that belong to other skills)
4. Run `routing-check.sh --update-snapshots` to regenerate collision matrix
