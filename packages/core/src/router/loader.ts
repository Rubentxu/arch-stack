// 1.6 capability router + declarative ShellAdapter, 1.7 fast-profile adapters.
// Load JSON descriptor files under packages/core/src/adapters/ and register
// them on a default router. The router is consumed by the auditor (1.8) and
// the spike report (1.12). Adding a tool = dropping a JSON file here.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { CapabilityRouter, shellAdapterFromDescriptor } from "./router.ts";
import type { ShellAdapterDescriptor } from "./router.ts";

const ADAPTERS_DIR = new URL("../adapters/", import.meta.url).pathname;

export function loadAdapterDescriptors(dir: string = ADAPTERS_DIR): ShellAdapterDescriptor[] {
  const out: ShellAdapterDescriptor[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isFile() && full.endsWith(".json")) {
      const desc = JSON.parse(readFileSync(full, "utf8")) as ShellAdapterDescriptor;
      out.push(desc);
    }
  }
  return out;
}

export function buildDefaultRouter(): CapabilityRouter {
  const router = new CapabilityRouter();
  for (const desc of loadAdapterDescriptors()) {
    router.register(shellAdapterFromDescriptor(desc));
  }
  return router;
}
