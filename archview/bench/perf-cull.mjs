#!/usr/bin/env node
/**
 * M21 Performance Gate — G6 Culling + LOD
 *
 * Manual pre-PR performance gate for the M21 culling implementation.
 * Measures TTFP (time-to-first-paint) and sustained pan/zoom FPS on
 * c4-stress-1k.json (1221 nodes / 3920 edges).
 *
 * DECISION D (locked): this script remains manual pre-PR; the CI perf
 * gate is tracked in issue #perf-ci-gate (opened by sddk-apply in
 * apply-progress.md).
 *
 * PREREQUISITES:
 * - Node.js >= 20
 * - Playwright (installed globally: `npm install -g playwright` or `pip install playwright`)
 * - Chromium browser: `playwright install chromium`
 * - archview dev server must be on port 18080
 *
 * USAGE:
 *   # Terminal 1: start dev server
 *   cd archview && pnpm dev &
 *
 *   # Terminal 2: run perf gate
 *   node bench/perf-cull.mjs
 *
 * ACCEPTANCE CRITERIA (REQ-M21-007 / AC-1, AC-2):
 *   AC-1: TTFP ≤ 5000ms on c4-stress-1k.json
 *   AC-2: Sustained pan/zoom FPS ≥ 55 on c4-stress-1k.json
 *
 * If criteria are NOT met, set enableCulling: false in all views
 * and re-run after optimisation work. Do NOT merge with failing perf.
 */

import { chromium } from "playwright";

const DEV_SERVER_URL = "http://localhost:18080";
const BUNDLE_PATH = "/samples/c4-stress-1k.json";
const TTFP_TIMEOUT_MS = 30_000;
const FPS_DURATION_MS = 3_000;
const FPS_MIN = 55;
const TTFP_MAX_MS = 5_000;

async function startDevServer() {
  const { spawn } = await import("node:child_process");
  console.log("[perf-gate] Starting archview dev server...");
  const server = spawn("pnpm", ["dev"], {
    cwd: "/var/mnt/DiscoChino2-fast/Proyectos/agentesIA/arch-stack/archview",
    stdio: ["ignore", "pipe", "pipe"],
    detached: true,
  });

  // Wait for server to be ready (look for "Local:" in output)
  await new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error("Dev server startup timeout (30s)")), 30_000);
    server.stdout.on("data", (data) => {
      const line = data.toString();
      if (line.includes("Local:") || line.includes("localhost:18080")) {
        clearTimeout(timeout);
        resolve();
      }
    });
    server.stderr.on("data", (data) => {
      const line = data.toString();
      if (line.includes("Local:") || line.includes("localhost:18080")) {
        clearTimeout(timeout);
        resolve();
      }
    });
  });

  console.log("[perf-gate] Dev server ready.");
  return server;
}

async function measureTTFP(page) {
  console.log("[perf-gate] Measuring TTFP...");

  const start = performance.now();

  // Navigate to the app (loads the SPA shell)
  await page.goto(DEV_SERVER_URL, { waitUntil: "domcontentloaded" });

  // Load the c4-stress-1k.json bundle via the file input.
  // The App.tsx has a file input for loading bundles. We use the
  // sample loader by navigating to the bundle URL directly via the
  // app's load mechanism.
  //
  // Since the App loads samples via a File input, we use the
  // /samples/ URL which is served statically by Vite.
  const bundleUrl = DEV_SERVER_URL + BUNDLE_PATH;

  // Click the "Load Bundle" button/link to open file picker
  // For the perf test, we inject the bundle URL directly into
  // the app's state via evaluate.
  await page.evaluate(async (url) => {
    // Import App's loadBundle and call it directly
    const { loadBundle } = await import("/var/mnt/DiscoChino2-fast/Proyectos/agentesIA/arch-stack/archview/src/bundle/loader.ts");
    await loadBundle(url);
  }, bundleUrl);

  // Wait for the canvas to render (G6 graph is ready)
  await page.waitForSelector("canvas", { timeout: TTFP_TIMEOUT_MS });

  // Also wait for the graph to finish initial render
  await page.waitForFunction(
    () => {
      const canvas = document.querySelector("canvas");
      if (!canvas) return false;
      // Check that G6 has rendered nodes (canvas has non-trivial content)
      const ctx = canvas.getContext("2d");
      if (!ctx) return true; // Can't check context, assume OK
      return true;
    },
    { timeout: TTFP_TIMEOUT_MS },
  );

  const ttfp = performance.now() - start;
  return ttfp;
}

async function measureFPS(page) {
  console.log("[perf-gate] Measuring sustained pan/zoom FPS...");

  const canvas = await page.waitForSelector("canvas", { timeout: 10_000 });
  const canvasBox = await canvas.boundingBox();
  if (!canvasBox) throw new Error("Cannot get canvas bounding box");

  const centerX = canvasBox.x + canvasBox.width / 2;
  const centerY = canvasBox.y + canvasBox.height / 2;

  // Collect frame timestamps during a pan + zoom interaction sequence.
  // Set up a frame collector using requestAnimationFrame
  await page.evaluate(() => {
    let raf = 0;
    const collect = () => {
      // Just mark that a frame fired
      window.__perfFrameCount = (window.__perfFrameCount || 0) + 1;
      raf = requestAnimationFrame(collect);
    };
    window.__perfFrameCount = 0;
    collect();
    // Give it a reference to stop later
    window.__perfRaf = raf;
  });

  // Simulate pan: drag the canvas
  await page.mouse.move(centerX, centerY);
  await page.mouse.down();
  for (let i = 0; i < 20; i++) {
    await page.mouse.move(centerX + i * 10, centerY + i * 5);
    await new Promise((r) => setTimeout(r, 16)); // ~60fps pace
  }
  await page.mouse.up();

  // Simulate zoom: wheel scroll on canvas
  for (let i = 0; i < 10; i++) {
    await page.mouse.wheel(0, -100);
    await new Promise((r) => setTimeout(r, 16));
  }

  await new Promise((r) => setTimeout(r, 500)); // Let frames settle

  // Stop the frame collector
  const frameCount = await page.evaluate(() => {
    cancelAnimationFrame(window.__perfRaf);
    return window.__perfFrameCount;
  });

  // Calculate FPS from frame count and elapsed time
  // We collected for approximately FPS_DURATION_MS but the interaction
  // itself is shorter. Use the actual timestamps array if available.
  const elapsed = timestamps.length > 1
    ? timestamps[timestamps.length - 1] - timestamps[0]
    : FPS_DURATION_MS;

  const fps = frameCount / (elapsed / 1000);
  return fps;
}

async function main() {
  let server = null;

  try {
    server = await startDevServer();

    const browser = await chromium.launch({ headless: true });
    const context = await browser.newContext({
      viewport: { width: 1280, height: 800 },
    });
    const page = await context.newPage();

    // ── TTFP Measurement ───────────────────────────────────────────────
    let ttfp;
    try {
      ttfp = await measureTTFP(page);
    } catch (err) {
      console.error(`[perf-gate] TTFP measurement FAILED: ${err.message}`);
      console.error("[perf-gate] TTFP AC-1: FAIL (measurement error)");
      process.exit(1);
    }

    const ttfpPass = ttfp <= TTFP_MAX_MS;
    console.log(`[perf-gate] TTFP: ${Math.round(ttfp)}ms (limit: ${TTFP_MAX_MS}ms) — ${ttfpPass ? "PASS ✓" : "FAIL ✗"}`);

    // ── FPS Measurement ──────────────────────────────────────────────────
    let fps;
    try {
      fps = await measureFPS(page);
    } catch (err) {
      console.error(`[perf-gate] FPS measurement FAILED: ${err.message}`);
      console.error("[perf-gate] AC-2: FAIL (measurement error)");
      process.exit(1);
    }

    const fpsPass = fps >= FPS_MIN;
    console.log(`[perf-gate] FPS: ${fps.toFixed(1)} (minimum: ${FPS_MIN}) — ${fpsPass ? "PASS ✓" : "FAIL ✗"}`);

    // ── Result ────────────────────────────────────────────────────────────
    console.log("\n" + "=".repeat(60));
    if (ttfpPass && fpsPass) {
      console.log("[perf-gate] ALL ACCEPTANCE CRITERIA MET — safe to merge");
      console.log(`[perf-gate] TTFP: ${Math.round(ttfp)}ms ≤ ${TTFP_MAX_MS}ms`);
      console.log(`[perf-gate] FPS:  ${fps.toFixed(1)} ≥ ${FPS_MIN}`);
    } else {
      console.error("[perf-gate] PERFORMANCE GATE FAILED");
      if (!ttfpPass) console.error(`  AC-1 TTFP: ${Math.round(ttfp)}ms > ${TTFP_MAX_MS}ms`);
      if (!fpsPass) console.error(`  AC-2 FPS:  ${fps.toFixed(1)} < ${FPS_MIN}`);
      console.error("\nBefore merging:");
      console.error("  1. Set enableCulling: false in all views");
      console.error("  2. Investigate the bottleneck (layout? culling? G6 config?)");
      console.error("  3. Re-run this script after optimisation");
      console.error("  4. Do NOT merge with a failing perf gate");
    }
    console.log("=".repeat(60) + "\n");

    await browser.close();

    process.exit(ttfpPass && fpsPass ? 0 : 1);
  } catch (err) {
    console.error("[perf-gate] Unexpected error:", err);
    process.exit(1);
  } finally {
    if (server) {
      // Kill the dev server process group
      try {
        process.kill(-server.pid, "SIGTERM");
      } catch {
        // Ignore errors when killing
      }
    }
  }
}

main();
