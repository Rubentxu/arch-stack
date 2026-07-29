import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { resolveSourceIdentity, portableProjectId } from "./identity.ts";

function makeFakeRepo(): string {
  const dir = mkdtempSync(join(tmpdir(), "archctl-fake-"));
  const r = spawnSync("git", ["init", "-q", "--initial-branch=main"], { cwd: dir, encoding: "utf8" });
  if (r.status !== 0) throw new Error(`git init failed: ${r.stderr}`);
  spawnSync("git", ["config", "user.email", "test@archctl"], { cwd: dir, encoding: "utf8" });
  spawnSync("git", ["config", "user.name", "archctl-test"], { cwd: dir, encoding: "utf8" });
  writeFileSync(join(dir, "README.md"), "test\n");
  spawnSync("git", ["add", "README.md"], { cwd: dir, encoding: "utf8" });
  spawnSync("git", ["commit", "-q", "-m", "init"], { cwd: dir, encoding: "utf8" });
  return dir;
}

test("resolver returns directory-mode for a non-Git path", () => {
  const dir = mkdtempSync(join(tmpdir(), "archctl-id-"));
  const id = resolveSourceIdentity({ cwd: dir });
  assert.equal(id.type, "directory");
  if (id.type !== "directory") return;
  assert.ok(id.directoryId.startsWith("blake3:"));
  assert.ok(id.canonicalRealpath.length > 0);
  assert.ok(existsSync(id.canonicalRealpath));
});

test("resolver returns directory-mode when the path is not inside a Git repo", () => {
  const outer = mkdtempSync(join(tmpdir(), "archctl-id-outer-"));
  const inner = join(outer, "nested");
  mkdirSync(inner, { recursive: true });
  const id = resolveSourceIdentity({ cwd: inner });
  assert.equal(id.type, "directory");
});

test("resolver returns git-mode for an actual Git repo (real probe)", () => {
  const repo = makeFakeRepo();
  const id = resolveSourceIdentity({ cwd: repo });
  assert.equal(id.type, "git");
  if (id.type !== "git") return;
  assert.ok(id.repositoryId.startsWith("blake3:"));
  assert.ok(id.worktreeId.startsWith("blake3:"));
  assert.match(id.rootCommit, /^[0-9a-f]{40}$/);
  assert.equal(id.toplevel, repo);
});

test("resolver is stable across calls when nothing changes", () => {
  const dir = mkdtempSync(join(tmpdir(), "archctl-id-"));
  const a = resolveSourceIdentity({ cwd: dir });
  const b = resolveSourceIdentity({ cwd: dir });
  assert.deepEqual(a, b);
});

test("portableProjectId is UUIDv4-shaped and stable per identity", () => {
  const id = resolveSourceIdentity({ cwd: mkdtempSync(join(tmpdir(), "archctl-id-")) });
  const pid = portableProjectId(id);
  assert.match(pid, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  const pid2 = portableProjectId(id);
  assert.equal(pid, pid2);
});

test("two distinct repos produce distinct repositoryId and projectId", () => {
  const a = makeFakeRepo();
  const b = makeFakeRepo();
  const ia = resolveSourceIdentity({ cwd: a });
  const ib = resolveSourceIdentity({ cwd: b });
  assert.equal(ia.type, "git");
  assert.equal(ib.type, "git");
  if (ia.type !== "git" || ib.type !== "git") return;
  assert.notEqual(ia.repositoryId, ib.repositoryId);
  assert.notEqual(portableProjectId(ia), portableProjectId(ib));
});
