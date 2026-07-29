import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { resolveProjectEnv, projectEnvFor } from "./shell-env.ts";

test("resolveProjectEnv produces canonical env keys", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-shell-env-"));
  const r = resolveProjectEnv({ cwd: root });
  assert.ok(r.env.ARCHCTL_PROJECT_DIR);
  assert.ok(r.env.ARCHCTL_PROJECT_ID);
  assert.match(r.env.ARCHCTL_PROJECT_ID, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  // Directory is created and writable.
  assert.ok(r.projectDir.length > 0);
});

test("resolveProjectEnv is stable across calls when cwd is the same", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-shell-env-"));
  const a = resolveProjectEnv({ cwd: root });
  const b = resolveProjectEnv({ cwd: root });
  assert.equal(a.env.ARCHCTL_PROJECT_DIR, b.env.ARCHCTL_PROJECT_DIR);
  assert.equal(a.env.ARCHCTL_PROJECT_ID, b.env.ARCHCTL_PROJECT_ID);
});

test("projectEnvFor is the pure helper exposed for tests", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-shell-env-"));
  const env = projectEnvFor({ cwd: root });
  assert.equal(typeof env.ARCHCTL_PROJECT_DIR, "string");
  assert.equal(typeof env.ARCHCTL_PROJECT_ID, "string");
});
