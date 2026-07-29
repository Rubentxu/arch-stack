// ADR-0003 — discriminated SourceIdentity + portable projectId.
//
// Two modes:
//   - git:     stable, sharable across machines (repositoryId + worktreeId)
//   - directory: local-only (directoryId = BLAKE3(canonical_realpath))
//
// A portable projectId is the cross-machine identity carried by export
// bundles. UUIDv4 derived from SHA-256(SOURCE_IDENTITY_CONTENT + firstExportTimestamp)
// (the operational rule added when ADR-0003 was promoted to Accepted).
// For in-memory usage we deterministically compute a stable UUIDv4 from
// the SourceIdentity content; the export layer is free to mix in a
// timestamp when the bundle is actually produced.

import { realpathSync } from "node:fs";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";

export type SourceIdentity =
  | {
      type: "git";
      repositoryId: string;
      worktreeId: string;
      rootCommit: string;
      toplevel: string;
    }
  | {
      type: "directory";
      directoryId: string;
      canonicalRealpath: string;
    };

/** Internal helper: deterministic-but-distinct BLAKE-style tag for inputs. */
function blakeLike(input: string): string {
  return `blake3:${createHash("sha256").update(input).digest("hex")}`;
}

function normDir(p: string): string {
  // Strip a trailing slash so /a/b and /a/b/ hash identically.
  return p.endsWith("/") ? p.slice(0, -1) : p;
}

export interface ResolveOptions {
  /** Optional override for the cwd; defaults to process.cwd(). */
  cwd?: string;
  /**
   * Optional explicit Git toplevel. When omitted, the resolver probes `git
   * rev-parse --show-toplevel`. This keeps the resolver testable without
   * requiring Git on PATH at probe time.
   */
  gitToplevel?: string;
}

/** BLAKE3 isn't in Node's stdlib yet; SHA-256 is acceptable for v1 hashing. */

function gitOutput(args: string[], cwd: string): string | null {
  const r = spawnSync("git", args, { cwd, encoding: "utf8" });
  if (r.status !== 0) return null;
  return `${r.stdout ?? ""}`.trim();
}

function normalizeRemote(url: string): string {
  // Strip credentials and trailing `.git` so the same remote always hashes
  // identically across machines / clones.
  return url.replace(/^https?:\/\/[^@]+@/, "").replace(/\.git$/, "").trim();
}

/**
 * Resolve the SourceIdentity of a path.
 *
 * - If `gitToplevel` is supplied OR `git rev-parse --show-toplevel` succeeds
 *   inside `cwd`, returns the Git mode (sharable across machines).
 * - Otherwise returns the directory mode (local-only).
 *
 * Never throws: an unreadable path, a broken Git repo, or no Git on PATH
 * all fall back to the directory mode. The caller decides whether that is
 * acceptable; the resolver's contract is "best-effort, always defined".
 */
export function resolveSourceIdentity(opts: ResolveOptions = {}): SourceIdentity {
  const cwd = opts.cwd ?? process.cwd();
  let toplevel = opts.gitToplevel ?? null;
  if (!toplevel) toplevel = gitOutput(["rev-parse", "--show-toplevel"], cwd);

  if (toplevel) {
    const remote = normalizeRemote(gitOutput(["config", "--get", "remote.origin.url"], toplevel) ?? "");
    const rootCommit = gitOutput(["rev-parse", "HEAD"], toplevel) ?? "";
    const canonicalTop = normDir(safeRealpath(toplevel));
    // WorktreeId must change when the *toplevel* changes, but it must NOT
    // change merely because rootCommit advanced on the same worktree.
    // We derive it from the worktree path alone. The repositoryId carries
    // the commit + remote and is the cross-machine anchor.
    const repositoryId = blakeLike(`git|${remote}|${rootCommit}`);
    const worktreeId = blakeLike(`worktree|${canonicalTop}`);
    return {
      type: "git",
      repositoryId,
      worktreeId,
      rootCommit,
      toplevel: canonicalTop,
    };
  }

  const canonical = normDir(safeRealpath(cwd));
  return {
    type: "directory",
    directoryId: blakeLike(`dir|${canonical}`),
    canonicalRealpath: canonical,
  };
}

function safeRealpath(p: string): string {
  try {
    return realpathSync.native(p);
  } catch {
    return p;
  }
}

/**
 * Compute the portable `projectId` from a SourceIdentity. UUIDv4-shaped
 * (16 random bytes) deterministically derived from SHA-256 of the identity
 * content (ADR-0003 acceptance rule). The caller may append a timestamp when
 * exporting so distinct bundles differ; for in-memory identity the digest
 * alone is enough.
 */
export function portableProjectId(identity: SourceIdentity): string {
  const content = JSON.stringify(identity, Object.keys(identity).sort());
  const bytes = createHash("sha256").update(content).digest();
  // Set version (4) and variant (10) per RFC 4122.
  bytes[6] = ((bytes[6] ?? 0) & 0x0f) | 0x40;
  bytes[8] = ((bytes[8] ?? 0) & 0x3f) | 0x80;
  const hex = bytes.toString("hex");
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20, 32),
  ].join("-");
}
