import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, existsSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { resolveXdg, ensureXdg } from "./xdg.ts";

test("resolveXdg uses XDG_*_HOME when set", () => {
  const fake = "/tmp/xdg-fake";
  const layout = resolveXdg({
    XDG_DATA_HOME: `${fake}/share`,
    XDG_STATE_HOME: `${fake}/state`,
    XDG_CACHE_HOME: `${fake}/cache`,
    XDG_CONFIG_HOME: `${fake}/config`,
  } as NodeJS.ProcessEnv);
  assert.equal(layout.data, `${fake}/share/archctl`);
  assert.equal(layout.state, `${fake}/state/archctl`);
  assert.equal(layout.cache, `${fake}/cache/archctl`);
  assert.equal(layout.config, `${fake}/config/archctl`);
});

test("resolveXdg falls back to $HOME when XDG_*_HOME unset", () => {
  const home = "/tmp/xdg-home";
  const layout = resolveXdg({ HOME: home } as NodeJS.ProcessEnv);
  assert.equal(layout.data, `${home}/.local/share/archctl`);
  assert.equal(layout.projectsRoot(), `${home}/.local/share/archctl/projects`);
  assert.equal(layout.runsRoot(), `${home}/.local/state/archctl/runs`);
});

test("ensureXdg creates all six directories", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-xdg-"));
  const fake = join(root, "xdg");
  const layout = resolveXdg({
    XDG_DATA_HOME: `${fake}/share`,
    XDG_STATE_HOME: `${fake}/state`,
    XDG_CACHE_HOME: `${fake}/cache`,
    XDG_CONFIG_HOME: `${fake}/config`,
  } as NodeJS.ProcessEnv);
  ensureXdg(layout);
  for (const dir of [
    layout.data,
    layout.state,
    layout.cache,
    layout.config,
    layout.projectsRoot(),
    layout.runsRoot(),
  ]) {
    assert.ok(existsSync(dir), `${dir} should exist`);
    assert.ok(statSync(dir).isDirectory());
  }
});
