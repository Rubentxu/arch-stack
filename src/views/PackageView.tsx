/**
 * PackageView — renders a call-graph bundle as a package/module
 * dependency diagram.
 *
 * Derives packages from the `file` field of nodes by grouping
 * nodes whose file shares a common directory prefix. A package
 * is the first path segment (e.g. "src/auth.rs" → "src").
 *
 * M17.5 MVP: card grid + relations list + cycle detection. The view
 * derives package-level edges from the underlying call-graph by
 * counting inter-package call edges (with weight = number of calls).
 * Cycles are highlighted in the relations panel.
 *
 * This is a derived view — no new bundle shape required. The
 * underlying bundle is still call-graph; the view abstracts it.
 */

import { For, Show, createMemo, type Component } from "solid-js";
import type { GraphEdge, GraphNode } from "../bundle/loader";

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

interface PackageEdge {
  source: string;
  target: string;
  weight: number;
}

function packageForFile(file: string | undefined): string {
  if (!file) return "(unknown)";
  // Drop the filename, then drop any trailing "src" segment (Rust
  // workspace convention: crates/*/src/<file>). Examples:
  //   "src/auth.rs"               → "src"
  //   "crates/cli/src/main.rs"    → "crates/cli"
  //   "lib/foo/bar.ts"            → "lib/foo"
  //   "src/auth/login.rs"         → "src/auth"
  const parts = file.split("/");
  if (parts.length <= 1) return file;
  parts.pop(); // drop filename
  while (parts.length > 1 && parts[parts.length - 1] === "src") {
    parts.pop();
  }
  return parts.join("/") || file;
}

export const PackageView: Component<PackageViewProps> = (props) => {
  /** Aggregate nodes by package (file directory). */
  const packages = createMemo<Package[]>(() => {
    const map = new Map<string, Package>();
    for (const node of props.nodes) {
      const pkg = packageForFile(node.file);
      const existing = map.get(pkg);
      if (existing) {
        existing.functionCount++;
        // functionCount increments; fileCount stays since one file
        // may host many functions
      } else {
        map.set(pkg, { name: pkg, fileCount: 1, functionCount: 1 });
      }
    }
    return [...map.values()].sort((a, b) => a.name.localeCompare(b.name));
  });

  /**
   * Build package-level edges from call-graph edges. A package edge
   * (A → B) exists if any function in A calls any function in B.
   * Weight = number of distinct call sites.
   */
  const nodePkg = createMemo<Map<string, string>>(() => {
    const m = new Map<string, string>();
    for (const n of props.nodes) m.set(n.id, packageForFile(n.file));
    return m;
  });

  const packageEdges = createMemo<PackageEdge[]>(() => {
    const map = new Map<string, number>();
    for (const e of props.edges) {
      const srcPkg = nodePkg().get(e.source);
      const dstPkg = nodePkg().get(e.target);
      if (!srcPkg || !dstPkg || srcPkg === dstPkg) continue;
      const key = `${srcPkg}\0${dstPkg}`;
      map.set(key, (map.get(key) ?? 0) + 1);
    }
    return [...map.entries()].map(([key, weight]) => {
      const [source, target] = key.split("\0");
      return { source, target, weight };
    });
  });

  /**
   * Detect cycles via DFS. Returns a Set of edge keys (source\0target)
   * that are part of a cycle.
   */
  const cycleEdges = createMemo<Set<string>>(() => {
    const adj = new Map<string, Set<string>>();
    for (const e of packageEdges()) {
      const set = adj.get(e.source) ?? new Set();
      set.add(e.target);
      adj.set(e.source, set);
    }
    const inCycle = new Set<string>();
    const WHITE = 0, GRAY = 1, BLACK = 2;
    const color = new Map<string, number>();
    for (const v of adj.keys()) color.set(v, WHITE);

    const dfs = (u: string, path: string[]) => {
      color.set(u, GRAY);
      path.push(u);
      for (const v of adj.get(u) ?? []) {
        const c = color.get(v) ?? WHITE;
        if (c === GRAY) {
          // Found a back-edge — cycle from v to current u.
          const idx = path.indexOf(v);
          if (idx >= 0) {
            for (let i = idx; i < path.length - 1; i++) {
              inCycle.add(`${path[i]}\0${path[i + 1]}`);
            }
            inCycle.add(`${path[path.length - 1]}\0${v}`);
          }
        } else if (c === WHITE) {
          dfs(v, path);
        }
      }
      color.set(u, BLACK);
      path.pop();
    };
    for (const v of adj.keys()) {
      if (color.get(v) === WHITE) dfs(v, []);
    }
    return inCycle;
  });

  return (
    <div class="package-view">
      <header class="pkg-header">
        <h2>Package diagram</h2>
        <p class="muted">
          {packages().length} packages · {packageEdges().length}{" "}
          inter-package edges
          {cycleEdges().size > 0 ? " · ⚠ cycle" : ""}
        </p>
      </header>

      <Show
        when={packages().length > 0}
        fallback={<p class="empty">No functions to derive packages from.</p>}
      >
        <div class="pkg-grid">
          <For each={packages()}>
            {(pkg) => (
              <article
                class="pkg-card"
                onClick={() => props.onSelect(pkg.name)}
                title={`${pkg.fileCount} file(s), ${pkg.functionCount} function(s)`}
              >
                <h3 class="pkg-name">{pkg.name}</h3>
                <dl class="pkg-stats">
                  <dt>files</dt>
                  <dd>{pkg.fileCount}</dd>
                  <dt>functions</dt>
                  <dd>{pkg.functionCount}</dd>
                </dl>
              </article>
            )}
          </For>
        </div>

        <Show when={packageEdges().length > 0}>
          <section class="pkg-relations">
            <h3>Inter-package dependencies</h3>
            <ul>
              <For each={packageEdges()}>
                {(e) => {
                  const isCycle = cycleEdges().has(`${e.source}\0${e.target}`);
                  return (
                    <li
                      class={`pkg-edge ${
                        isCycle ? "is-cycle" : ""
                      } weight-${Math.min(3, e.weight)}`}
                    >
                      <code>{e.source}</code>
                      <span class="pkg-arrow">
                        ─{isCycle ? "↺" : "→"}({e.weight})→
                      </span>
                      <code>{e.target}</code>
                    </li>
                  );
                }}
              </For>
            </ul>
          </section>
        </Show>
      </Show>
    </div>
  );
};
