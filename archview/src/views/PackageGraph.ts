/**
 * Pure helpers for package-level graph analysis.
 *
 * Extracted from PackageView so they can be unit-tested without
 * pulling JSX into the test pipeline. Each helper is a deterministic
 * transformation over the call-graph bundle — no Solid reactivity.
 */

export interface PackageEdge {
  source: string;
  target: string;
  weight: number;
}

/**
 * Derive a package name from a file path. Drops the filename, then
 * any trailing "src" segment (Rust workspace convention:
 * crates/[name]/src/[file]). Examples:
 *   "src/auth.rs"               → "src"
 *   "crates/cli/src/main.rs"    → "crates/cli"
 *   "lib/foo/bar.ts"            → "lib/foo"
 *   "src/auth/login.rs"         → "src/auth"
 *   undefined                   → "(unknown)"
 */
export function packageForFile(file: string | undefined): string {
  if (!file) return "(unknown)";
  const parts = file.split("/");
  if (parts.length <= 1) return file;
  parts.pop(); // drop filename
  while (parts.length > 1 && parts[parts.length - 1] === "src") {
    parts.pop();
  }
  return parts.join("/") || file;
}

/**
 * Compute inter-package edges from a call-graph. An edge (A → B) exists
 * if any function in A calls any function in B. Weight = number of
 * distinct call sites (sum across all caller-callee pairs in A→B).
 */
export function buildPackageEdges(
  nodes: { id: string; file?: string }[],
  edges: { source: string; target: string }[],
): PackageEdge[] {
  const nodePkg = new Map<string, string>();
  for (const n of nodes) nodePkg.set(n.id, packageForFile(n.file));
  const map = new Map<string, number>();
  for (const e of edges) {
    const srcPkg = nodePkg.get(e.source);
    const dstPkg = nodePkg.get(e.target);
    if (!srcPkg || !dstPkg || srcPkg === dstPkg) continue;
    const key = `${srcPkg}\0${dstPkg}`;
    map.set(key, (map.get(key) ?? 0) + 1);
  }
  return [...map.entries()].map(([key, weight]) => {
    const [source, target] = key.split("\0");
    return { source, target, weight };
  });
}

/**
 * Detect cycles in a directed graph using DFS back-edges (color
 * algorithm). Returns a Set of edge keys (`source\0target`) that
 * are part of any cycle.
 */
export function detectCycles(edges: PackageEdge[]): Set<string> {
  const adj = new Map<string, Set<string>>();
  for (const e of edges) {
    const set = adj.get(e.source) ?? new Set();
    set.add(e.target);
    adj.set(e.source, set);
  }
  const inCycle = new Set<string>();
  const WHITE = 0,
    GRAY = 1,
    BLACK = 2;
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
}
