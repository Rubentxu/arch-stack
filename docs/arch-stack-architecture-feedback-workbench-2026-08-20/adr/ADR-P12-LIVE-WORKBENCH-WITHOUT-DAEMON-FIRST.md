# ADR-P12 — Live Workbench Without a Daemon First
**Status:** Proposed

## Decision
`archctl view --watch` + worker + GraphRevision/GraphDelta + polling HTTP.

## Reopen
Axum/Tokio/SSE/WebSocket o `archctld` sólo con necesidad medida.
