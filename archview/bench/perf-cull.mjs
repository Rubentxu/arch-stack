#!/usr/bin/env node
/**
 * M21 Performance Gate — G6 Culling + LOD  (refactored for M23 CI gate)
 *
 * Measures TTFP (time-to-first-paint) and sustained pan/zoom FPS on
 * c4-stress-1k.json (1221 nodes / 3920 edges).
 *
 * PREREQUISITES:
 * - Node.js >= 20
 * - Playwright (installed via `pnpm add -D playwright`)
 * - Chromium browser: `pnpm exec playwright install chromium`
 * - archview must be built (pnpm build) OR a custom server command provided
 *
 * USAGE:
 *   node bench/perf-cull.mjs [--server-cmd 'pnpm preview --port 18080']
 *                            [--output /path/to/perf.json]
 *                            [--warmup N]
 *
 *   --server-cmd  Command to start the preview server (default: pnpm preview --port 18080)
 *   --output      Write JSON output to this file (default: stdout)
 *   --warmup      Number of warmup runs before measurement (default: 1)
 *                 The script runs (warmup + 1) iterations; first `warmup` are discarded.
 *
 * ACCEPTANCE CRITERIA (ADR-019 regression gate):
 *   AC-1: TTFP <= 5000ms on c4-stress-1k.json
 *   AC-2: Sustained pan/zoom FPS >= 55 on c4-stress-1k.json
 *
 * Exit codes:
 *   0 = measurement complete + both ACs pass
 *   1 = AC failure (TTFP > 5000ms or FPS < 55)
 *   2 = instrumentation error (server failed, browser error, etc.)
 *
 * JSON output schema:
 * {
 *   "ttfp_ms": number,
 *   "fps_avg": number,
 *   "fps_min": number,
 *   "sample": "c4-stress-1k.json",
 *   "runner": "archview-bench",
 *   "timestamp": "ISO8601",
 *   "duration_ms": number,
 *   "samples": [{ "ts_ms": number, "fps": number }]
 * }
 */

import { chromium } from "playwright";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:process";
import { writeFileSync } from "node:fs";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const REPO_ROOT = resolve(__dirname, "..", "..");

// ---- defaults ---------------------------------------------------------------
const DEFAULT_SERVER_CMD = "pnpm preview --port 18080";
const DEFAULT_OUTPUT = null;       // stdout
const DEFAULT_WARMUP = 1;

// ---- acceptance thresholds --------------------------------------------------
const BUNDLE_PATH = "/samples/c4-stress-1k.json";
const TTFP_TIMEOUT_MS = 30_000;
const FPS_DURATION_MS = 5_000;   // 5s sustained measurement per iteration
const FPS_MIN = 55;
const TTFP_MAX_MS = 5_000;

// ---- CLI parsing ------------------------------------------------------------
function parseArgs() {
    const args = process.argv.slice(2);
    let serverCmd = DEFAULT_SERVER_CMD;
    let outputPath = DEFAULT_OUTPUT;
    let warmup = DEFAULT_WARMUP;

    for (let i = 0; i < args.length; i++) {
        switch (args[i]) {
            case "--server-cmd":
                serverCmd = args[++i];
                break;
            case "--output":
                outputPath = args[++i];
                break;
            case "--warmup":
                warmup = parseInt(args[++i], 10);
                if (isNaN(warmup) || warmup < 0) warmup = 0;
                break;
            default:
                if (!args[i].startsWith("--")) break;
                console.error(`[perf-cull] Unknown flag: ${args[i]}`);
                process.exit(2);
        }
    }
    return { serverCmd, outputPath, warmup };
}

const { serverCmd, outputPath, warmup } = parseArgs();
const SERVER_URL = `http://localhost:${extractPort(serverCmd) || 18080}`;

function extractPort(cmd) {
    const m = cmd.match(/--port\s+(\d+)/);
    return m ? m[1] : null;
}

// ---- server lifecycle ------------------------------------------------------
function parseServerCommand(cmd) {
    // Parse "pnpm preview --port 18080 --strictPort" into parts
    const parts = cmd.split(/\s+/);
    return { cmd: parts[0], args: parts.slice(1) };
}

async function waitForServer(url, timeout = 30_000) {
    const start = Date.now();
    while (Date.now() - start < timeout) {
        try {
            const res = await fetch(url);
            if (res.ok) return true;
        } catch {
            // not ready yet
        }
        await new Promise((r) => setTimeout(r, 500));
    }
    throw new Error(`Server at ${url} did not become ready within ${timeout}ms`);
}

async function startServer(serverCmd) {
    const { cmd, args } = parseServerCommand(serverCmd);
    console.log(`[perf-cull] Starting server: ${cmd} ${args.join(" ")}`);
    console.log(`[perf-cull]   cwd: ${REPO_ROOT}`);

    const child = spawn(cmd, args, {
        cwd: REPO_ROOT,
        stdio: ["ignore", "pipe", "pipe"],
        detached: true,
    });

    await new Promise((resolve, reject) => {
        const timeout = setTimeout(() => reject(new Error("Server startup timeout (30s)")), 30_000);
        child.stdout.on("data", (data) => {
            const line = data.toString();
            if (line.includes("localhost") || line.includes("Local:")) {
                clearTimeout(timeout);
                resolve();
            }
        });
        child.stderr.on("data", (data) => {
            const line = data.toString();
            if (line.includes("localhost") || line.includes("Local:")) {
                clearTimeout(timeout);
                resolve();
            }
        });
    });

    await waitForServer(SERVER_URL);
    console.log(`[perf-cull] Server ready at ${SERVER_URL}`);
    return child;
}

function killServer(child) {
    if (!child) return;
    try {
        // Kill the whole process group
        process.kill(-child.pid, "SIGTERM");
    } catch {
        // ignore
    }
}

// ---- measurements -----------------------------------------------------------
async function measureTTFP(page) {
    console.log("[perf-cull] Measuring TTFP...");

    const start = performance.now();

    await page.goto(SERVER_URL, { waitUntil: "domcontentloaded", timeout: TTFP_TIMEOUT_MS });

    // Load the c4-stress-1k.json bundle via the app's load mechanism.
    const bundleUrl = SERVER_URL + BUNDLE_PATH;

    await page.evaluate(async (url) => {
        const { loadBundle } = await import(resolve(REPO_ROOT, "src/bundle/loader.ts"));
        await loadBundle(url);
    }, bundleUrl);

    // Wait for canvas to render
    await page.waitForSelector("canvas", { timeout: TTFP_TIMEOUT_MS });

    await page.waitForFunction(
        () => {
            const canvas = document.querySelector("canvas");
            return !!canvas;
        },
        { timeout: TTFP_TIMEOUT_MS },
    );

    const ttfp = performance.now() - start;
    return ttfp;
}

async function measureFPS(page) {
    console.log("[perf-cull] Measuring sustained FPS over ${FPS_DURATION_MS}ms...");

    const canvas = await page.waitForSelector("canvas", { timeout: 10_000 });
    const canvasBox = await canvas.boundingBox();
    if (!canvasBox) throw new Error("Cannot get canvas bounding box");

    const centerX = canvasBox.x + canvasBox.width / 2;
    const centerY = canvasBox.y + canvasBox.height / 2;

    // Collect frame timestamps during a pan + zoom interaction sequence.
    await page.evaluate(() => {
        window.__perfTimestamps = [];
        window.__perfRaf = 0;
        const collect = () => {
            window.__perfTimestamps.push(performance.now());
            window.__perfRaf = requestAnimationFrame(collect);
        };
        window.__perfRaf = requestAnimationFrame(collect);
    });

    // Simulate pan: drag the canvas
    await page.mouse.move(centerX, centerY);
    await page.mouse.down();
    for (let i = 0; i < 20; i++) {
        await page.mouse.move(centerX + i * 10, centerY + i * 5);
        await new Promise((r) => setTimeout(r, 16));
    }
    await page.mouse.up();

    // Simulate zoom: wheel scroll on canvas
    for (let i = 0; i < 10; i++) {
        await page.mouse.wheel(0, -100);
        await new Promise((r) => setTimeout(r, 16));
    }

    // Wait for FPS collection window to elapse
    await new Promise((r) => setTimeout(r, FPS_DURATION_MS));

    // Stop collector and compute FPS
    const result = await page.evaluate(() => {
        cancelAnimationFrame(window.__perfRaf);
        const ts = window.__perfTimestamps;
        if (ts.length < 2) {
            return { fps: 0, elapsed: FPS_DURATION_MS };
        }
        const elapsed = ts[ts.length - 1] - ts[0];
        const fps = (ts.length - 1) / (elapsed / 1000);
        return { fps, elapsed };
    });

    return result.fps;
}

// ---- main measurement loop -------------------------------------------------
async function runMeasurementIteration(browser) {
    const context = await browser.newContext({
        viewport: { width: 1280, height: 800 },
    });
    const page = await context.newPage();

    let ttfp;
    try {
        ttfp = await measureTTFP(page);
    } catch (err) {
        console.error(`[perf-cull] TTFP measurement FAILED: ${err.message}`);
        await context.close();
        return { error: "ttfp", message: err.message };
    }

    let fps;
    try {
        fps = await measureFPS(page);
    } catch (err) {
        console.error(`[perf-cull] FPS measurement FAILED: ${err.message}`);
        await context.close();
        return { error: "fps", message: err.message };
    }

    await context.close();
    return { ttfp, fps };
}

// ---- JSON output -----------------------------------------------------------
function emitResult(measurements, outputPath) {
    const ttfp_values = measurements.map((m) => m.ttfps).filter((v) => v != null);
    const fps_values = measurements.map((m) => m.fps_values).flat().filter((v) => v != null);

    const ttfp_ms = ttfp_values.length > 0 ? ttfp_values[ttfp_values.length - 1] : 0;
    const fps_avg = fps_values.length > 0 ? fps_values.reduce((a, b) => a + b, 0) / fps_values.length : 0;
    const fps_min = fps_values.length > 0 ? Math.min(...fps_values) : 0;

    const sample = {
        ttfp_ms,
        fps_avg: parseFloat(fps_avg.toFixed(2)),
        fps_min: parseFloat(fps_min.toFixed(2)),
        sample: "c4-stress-1k.json",
        runner: "archview-bench",
        timestamp: new Date().toISOString(),
        duration_ms: measurements.reduce((acc, m) => acc + (m.duration_ms || 0), 0),
        samples: measurements.map((m, i) => ({
            iteration: i + 1,
            ttfp_ms: m.ttfps,
            fps: m.fps_values.length > 0 ? parseFloat((m.fps_values.reduce((a, b) => a + b, 0) / m.fps_values.length).toFixed(2)) : 0,
        })),
    };

    const json = JSON.stringify(sample, null, 2);
    if (outputPath) {
        writeFileSync(outputPath, json);
        console.log(`[perf-cull] Result written to ${outputPath}`);
    } else {
        console.log(json);
    }

    return sample;
}

// ---- main entry point ------------------------------------------------------
async function main() {
    const totalIterations = warmup + 1; // warmup iterations + 1 measurement
    const measurements = [];
    let server = null;

    try {
        // Start the preview server
        try {
            server = await startServer(serverCmd);
        } catch (err) {
            console.error(`[perf-cull] Server failed to start: ${err.message}`);
            process.exit(2);
        }

        // Launch browser once for all iterations
        const browser = await chromium.launch({ headless: true });

        for (let iter = 0; iter < totalIterations; iter++) {
            const isWarmup = iter < warmup;
            const label = isWarmup ? `WARMUP ${iter + 1}/${warmup}` : `MEASUREMENT`;
            console.log(`[perf-cull] === ${label} ===`);

            const iterStart = Date.now();
            const result = await runMeasurementIteration(browser);
            const duration_ms = Date.now() - iterStart;

            if (result.error) {
                console.error(`[perf-cull] ${label} FAILED: ${result.error} — ${result.message}`);
                await browser.close();
                killServer(server);
                process.exit(2);
            }

            const { ttfp, fps } = result;

            if (isWarmup) {
                console.log(`[perf-cull] Warmup ${iter + 1}: TTFP=${Math.round(ttfp)}ms FPS=${fps.toFixed(1)} (discarded)`);
            } else {
                console.log(`[perf-cull] Measurement: TTFP=${Math.round(ttfp)}ms FPS=${fps.toFixed(1)}`);
                measurements.push({ ttfps: ttfp, fps_values: [fps], duration_ms });
            }
        }

        await browser.close();
        killServer(server);

        // Emit result (last measurement iteration is what we report)
        const sample = emitResult(measurements, outputPath);

        // Evaluate ACs
        const ttfpPass = sample.ttfp_ms <= TTFP_MAX_MS;
        const fpsPass = sample.fps_avg >= FPS_MIN;

        console.log("");
        console.log("=".repeat(60));
        console.log(`[perf-cull] TTFP: ${Math.round(sample.ttfp_ms)}ms (limit: ${TTFP_MAX_MS}ms) — ${ttfpPass ? "PASS" : "FAIL"}`);
        console.log(`[perf-cull] FPS:  ${sample.fps_avg.toFixed(1)} avg / ${sample.fps_min.toFixed(1)} min (minimum: ${FPS_MIN}) — ${fpsPass ? "PASS" : "FAIL"}`);

        if (ttfpPass && fpsPass) {
            console.log("[perf-cull] ALL ACCEPTANCE CRITERIA MET — safe to merge");
        } else {
            console.error("[perf-cull] PERFORMANCE GATE FAILED");
            if (!ttfpPass) console.error(`  AC-1 TTFP: ${Math.round(sample.ttfp_ms)}ms > ${TTFP_MAX_MS}ms`);
            if (!fpsPass) console.error(`  AC-2 FPS:  ${sample.fps_avg.toFixed(1)} < ${FPS_MIN}`);
        }
        console.log("=".repeat(60) + "\n");

        process.exit(ttfpPass && fpsPass ? 0 : 1);

    } catch (err) {
        console.error("[perf-cull] Unexpected error:", err);
        if (server) killServer(server);
        process.exit(2);
    }
}

main();
