---
description: Entry point for C4 + UML diagram requests. Dispatches to the right subagent.
agent: diagram-architect
---

The user wants a diagram. Forward the request to the right subagent.

```text
/diagram <kind> [args...]
```

`kind` selects the target subagent:

| `kind` | Subagent | Examples |
|---|---|---|
| `c4` | `c4-modeler` | `c4 context`, `c4 container payments`, `c4 component orders`, `c4 deployment`, `c4 dynamic "crear pedido"` |
| `usecase` / `use-cases` | `uml-modeler` | `usecase checkout`, `use-cases payments` |
| `class` | `uml-modeler` | `class order-domain`, `class modules/checkout` |
| `sequence` | `uml-modeler` | `sequence "crear pedido"`, `sequence src/orders/create.rs::create_order` |
| `state` / `activity` | `uml-modeler` | `state Order`, `activity checkout` |
| `evidence` / `explain` | `architecture-evidence` | `evidence rel:orders-payment`, `explain container:orders-api` |
| `update` / `diff` | `architecture-evidence` | `update`, `diff HEAD~1..HEAD` |
| `review` | `diagram-reviewer` | `review <view-id>` |

## Flow

1. Parse `kind` and `args`.
2. Delegate to the appropriate subagent.
3. Receive the result and forward to the user.
4. If the user requested a review or the result is `needs-fix`, route
   to `diagram-reviewer`.

## Constraints

- The agent never reads source files directly; it only invokes
  `archctl` (and skill wrappers).
- The agent never writes to the repository.
- The agent persists view specifications via `archctl diagram put`.
