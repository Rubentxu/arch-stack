/**
 * Vitest global setup — stubs `fetch` so tests don't accidentally hit the
 * network. Per-test overrides (e.g. `__setFetchForTests` from
 * `lib/workspace`) compose with this baseline.
 */

import { vi } from "vitest";

const noopFetch = vi.fn(async (input: RequestInfo | URL) => {
  const url = typeof input === "string" ? input : input.toString();
  // Default to a benign "not found" so pre-existing tests that probe
  // optional endpoints fail soft rather than blowing up.
  return new Response(JSON.stringify({ error: "fetch_stubbed", url }), {
    status: 404,
    headers: { "Content-Type": "application/json" },
  });
});

(globalThis as { fetch?: typeof fetch }).fetch =
  noopFetch as unknown as typeof fetch;
