/**
 * PackageView — renders a call-graph bundle as a package/module
 * dependency diagram.
 *
 * Derives packages from the `file` field of nodes by grouping
 * nodes whose file shares a common directory prefix. A package
 * is the first path segment (e.g. "src/auth.rs" → "src").
 *
 * M17.1.5 replaced the previous card-grid render with a G6
 * dagre horizontal graph. The pure helpers in PackageGraph.ts
 * are unchanged.
 */

import {
  For,
  Show,
  createEffect,
  createMemo,
  onCleanup,
  onMount,
  type Component,
} from "solid-js";
import type { GraphEdge, GraphNode } from "../bundle/loader";
import { GraphRenderer } from "../renderer/g6";
import { LR_LAYERED } from "../renderer/layout-presets";
import {
  buildPackageEdges,
  detectCycles,
  packageForFile,
} from "./PackageGraph";

export interface PackageViewProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  onSelect: (id: string | null) => void;
}

interface Package {
  name: string;
  fileCount: number;
  functionCount: number;
}

/** Map a `Package` to a node the renderer can consume. The id
 *  is the package name (must be unique). */
function packageToNode(p: Package): GraphNode {
  return {
    id: p.name,
    label: p.name,
    kind: "package",
    file: p.name,
  };
}

export const PackageView: Component<PackageViewProps> = (props) => {
  let containerRef!: HTMLDivElement;
  let renderer: GraphRenderer | undefined;

  const packages = createMemo<Package[]>(() => {
    const map = new Map<string, Package>();
    for (const node of props.nodes) {
      const pkg = packageForFile(node.file);
      const existing = map.get(pkg);
      if (existing) {
        existing.functionCount++;
      } else {
        map.set(pkg, { name: pkg, fileCount: 1, functionCount: 1 });
      }
    }
    return [...map.values()].sort((a, b) => a.name.localeCompare(b.name));
  });

  const packageEdges = createMemo(() =>
    buildPackageEdges(props.nodes, props.edges),
  );

  const cycles = createMemo(() => detectCycles(packageEdges()));

  const packageNodes = createMemo<GraphNode[]>(() =>
    packages().map(packageToNode),
  );

  const packageEdgeList = createMemo<GraphEdge[]>(() =>
    packageEdges().map((e) => ({
      id: `${e.from}->${e.to}`,
      source: e.from,
      target: e.to,
      label: `${e.weight}`,
      kind: e.inCycle ? "cycle" : "depends",
    })),
  );

  onMount(() => {
    renderer = new GraphRenderer({
      container: containerRef,
      width: containerRef?.clientWidth || 800,
      height: containerRef?.clientHeight || 600,
      // M19: ELK layered in Web Worker, left-to-right.
      layoutOptions: LR_LAYERED,
      onNodeClick: (id) => {
        props.onSelect(id);
      },
    });
    renderer.setData({
      schemaVersion: "0.0.0",
      source: "package-view",
      loadedAt: "0",
      rawKind: "call-graph",
      nodes: packageNodes(),
      edges: packageEdgeList(),
    });
  });

  createEffect(() => {
    if (!renderer) return;
    renderer.setData({
      schemaVersion: "0.0.0",
      source: "package-view",
      loadedAt: "0",
      rawKind: "call-graph",
      nodes: packageNodes(),
      edges: packageEdgeList(),
    });
  });

  onMount(() => {
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry || !renderer) return;
      const { width, height } = entry.contentRect;
      if (width > 0 && height > 0) renderer.resize(width, height);
    });
    ro.observe(containerRef);
    onCleanup(() => ro.disconnect());
  });

  onCleanup(() => {
    renderer?.destroy();
    renderer = undefined;
  });

  return (
    <div class="package-view">
      <header class="pkg-header">
        <h2>Packages</h2>
        <p class="muted">
          {packages().length} packages · {packageEdges().length} inter-package
          edges
        </p>
      </header>

      <Show
        when={packages().length > 0}
        fallback={<p class="empty">No functions to derive packages from.</p>}
      >
        <div ref={containerRef} class="pkg-canvas" />

        <Show when={cycles().length > 0}>
          <section class="pkg-cycles">
            <h3>Dependency cycles</h3>
            <ul>
              <For each={cycles()}>
                {(c) => (
                  <li>
                    <For each={c}>
                      {(p, i) => (
                        <>
                          <Show when={i() > 0}>
                            <span class="arrow"> → </span>
                          </Show>
                          <code>{p}</code>
                        </>
                      )}
                    </For>
                    <span class="arrow"> → {c[0]}</span>
                  </li>
                )}
              </For>
            </ul>
          </section>
        </Show>
      </Show>
    </div>
  );
};
