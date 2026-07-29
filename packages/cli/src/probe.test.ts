import { test } from "node:test";
import assert from "node:assert/strict";
import { runProbe } from "./probe.ts";
import { mkdtempSync, writeFileSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

function setupFakeRepo(): string {
  const dir = mkdtempSync(join(tmpdir(), "archctl-probe-"));
  writeFileSync(join(dir, ".opencode-version"), "opencode-version: 1.18.x\n");
  mkdirSync(join(dir, "schemas", "opencode", "1.18.x"), { recursive: true });
  writeFileSync(join(dir, "schemas", "opencode", "1.18.x", "config.json"), "{}");
  return dir;
}

test("probe fails when the vendored schema snapshot is missing", () => {
  const dir = mkdtempSync(join(tmpdir(), "archctl-probe-"));
  writeFileSync(join(dir, ".opencode-version"), "opencode-version: 1.18.x\n");
  const r = runProbe({ cwd: dir, pinnedOpenCode: "1.18.x" });
  assert.equal(r.ok, false);
  assert.ok(r.findings.some((f) => f.id === "opencode.schema-snapshot" && f.severity === "fail"));
});

test("probe passes the structural checks on a fake repo", () => {
  const dir = setupFakeRepo();
  const r = runProbe({ cwd: dir, pinnedOpenCode: "1.18.x" });
  // The probe MUST not block on missing optional tools; it must only fail on
  // missing vendored snapshot, missing XDG writability, or missing runtime.
  assert.equal(r.findings.find((f) => f.id === "opencode.schema-snapshot")?.severity, "ok");
  assert.equal(r.findings.find((f) => f.id === "xdg.writability")?.severity, "ok");
  // The CLI / hooks / renderers are advisory (warn, not fail) until OpenCode is
  // installed. Their severity must NOT block the probe.
  const cli = r.findings.find((f) => f.id === "opencode.cli");
  assert.ok(cli, "CLI finding must be present");
  assert.notEqual(cli?.severity, "fail");
});
