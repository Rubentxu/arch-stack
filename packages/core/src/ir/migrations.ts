// ADR-0002 + ADR-0004 — Architecture IR migrations registry.
//
// Migration order is numeric (from → to). Only the *current* schemaVersion is
// recognised by archctl; older inputs MUST pass through this registry. New
// fields land here before they are adopted in code — keeping migration
// registry as the only writer of schemaVersion transitions is what makes
// `auditIR`'s schemaVersion strict check safe.
import type { Migration } from "./ir.ts";

export const IR_MIGRATIONS: Migration[] = [
  // Future example (commented out so v1 is the only live schema):
  // {
  //   from: 1,
  //   to: 2,
  //   apply: (ir) => {
  //     const o = ir as { elements: unknown[] };
  //     return { ...(ir as object), elements: o.elements.map((e) => ({ ...(e as object), deprecatedField: null })) };
  //   },
  // },
];
