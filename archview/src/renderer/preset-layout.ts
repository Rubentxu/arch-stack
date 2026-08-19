/**
 * PresetLayout — a no-op G6 v5 layout that respects pre-set
 * node positions.
 *
 * M19 replaces the M17.1 dagre layout with ELK layered running in
 * a Web Worker. The renderer computes positions via ELK and
 * embeds them as `style.x` / `style.y` on each node when calling
 * `graph.setData(...)`. G6 v5 still runs a layout pass after
 * `setData` — the dagre/force/grid algorithms all overwrite the
 * embedded positions, so we register this minimal layout that
 * leaves them alone.
 *
 * The class extends G6's `BaseLayout` (re-exported by `@antv/g6`,
 * originally from `@antv/layout`) and implements `execute`. The
 * implementation just walks the model and copies each node's
 * embedded `x` / `y` to the model — G6 reads from the model when
 * rendering.
 *
 * Registration:
 *   `register(ExtensionCategory.LAYOUT, "preset", PresetLayout)`
 *
 * Once registered, views (or the renderer) can pass
 * `{ type: "preset" }` as their layout config.
 */

import { BaseLayout } from "@antv/g6";
import type { GraphData } from "@antv/g6";

export interface PresetLayoutOptions {
  // Empty — the layout is parameterless. The positions come from
  // the model (`graph.setData({ nodes: [{ id, style: { x, y } }] })`).
  // `type` is required by G6's `BaseLayoutOptions` discriminator.
  type: string;
  // G6's `BaseLayoutOptions` type chain expects an index signature;
  // we declare it explicitly so TS accepts the interface as a
  // valid constraint for `BaseLayout<O>`.
  [key: string]: unknown;
}

export class PresetLayout extends BaseLayout<PresetLayoutOptions> {
  readonly id: string = "preset";

  async execute(model: GraphData): Promise<GraphData> {
    // No-op: positions are already on the model from the
    // renderer's `setData` call. G6 reads them from the model
    // when rendering. We return the model unchanged.
    return model;
  }
}
