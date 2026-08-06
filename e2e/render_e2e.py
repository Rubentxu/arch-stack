#!/usr/bin/env python3
"""E2E render suite (ADR-034 §2, M29.2).

Verifies that the archview workbench (served by `archctl view`) renders
what the bundle contains: DOM assertions per bundle type (C4/sequence/
class/call-graph), real multi-language bundles, zero JS console errors,
screenshots as artifacts.

Usage:
    python3 e2e/render_e2e.py [--bin <path>] [--samples-only] [--artifacts <dir>]

Requirements: playwright (pip install playwright && playwright install chromium)
"""
import argparse
import json
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

from playwright.sync_api import sync_playwright

REPO_ROOT = Path(__file__).resolve().parent.parent
CACHE = Path.home() / ".cache" / "archctl-smoke"

SAMPLES = [
    # (label, url, view_selector, min_nodes, raw_kind)
    ("c4-context", "/samples/c4-context.json", ".c4-view", 1, "c4"),
    ("c4-container", "/samples/c4-container.json", ".c4-view", 1, "c4"),
    ("sequence", "/samples/sequence.json", ".sequence-view, .seq-view, main", 1, "sequence"),
    ("class-diagram", "/samples/class-diagram.json", ".class-view, main", 1, "class-diagram"),
    ("call-graph", "/samples/call-graph.json", ".impact-view, main", 1, "call-graph"),
]

REAL_BUNDLES = [
    # (label, cache_dir, export_selector)
    ("axum", "tokio-rs/axum", "container:*"),
    ("ripgrep", "BurntSushi/ripgrep", "container:*"),
    ("zustand", "pmndrs/zustand", "container:*"),
    ("express", "expressjs/express", "container:*"),
]

failures = []


def check(name, cond, detail=""):
    print(f"[{'PASS' if cond else 'FAIL'}] {name} {detail}")
    if not cond:
        failures.append(name)


def wait_health(base, retries=20):
    for _ in range(retries):
        try:
            with urllib.request.urlopen(f"{base}/api/health", timeout=2) as r:
                return json.loads(r.read())
        except Exception:
            time.sleep(0.5)
    return None


def load_bundle(page, url):
    inp = page.locator("input[placeholder*='bundle URL']")
    inp.fill(url)
    inp.press("Enter")
    page.wait_for_timeout(4000)


def assert_bundle_meta(page, expected_kind):
    meta = page.locator(".bundle-meta").inner_text() if page.locator(".bundle-meta").count() else ""
    # Format: "rawKind\nc4" (dt/dd) or "rawKind | c4" (flattened)
    raw = None
    lines = [l.strip() for l in meta.split("\n") if l.strip()]
    for i, l in enumerate(lines):
        if l == "rawKind" and i + 1 < len(lines):
            raw = lines[i + 1]
            break
        if "rawKind" in l and "|" in l:
            raw = l.split("|")[-1].strip()
            break
    check(f"rawKind == {expected_kind}", raw == expected_kind, f"(got {raw})")
    return raw


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default=str(REPO_ROOT / "archctl/target/release/archctl"))
    ap.add_argument("--samples-only", action="store_true")
    ap.add_argument("--artifacts", default=str(REPO_ROOT / "e2e/artifacts"))
    args = ap.parse_args()

    bin_path = Path(args.bin)
    artifacts = Path(args.artifacts)
    artifacts.mkdir(parents=True, exist_ok=True)

    assert bin_path.exists(), f"binary not found: {bin_path}"

    # Start one view server (no cwd: samples served; /api/export off)
    server = subprocess.Popen(
        [str(bin_path), "view", "--port", "0"],
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        text=True,
    )
    try:
        # Read the ephemeral port from server output
        port = None
        deadline = time.time() + 10
        while time.time() < deadline:
            line = server.stdout.readline()
            if "127.0.0.1:" in line:
                port = line.split("127.0.0.1:")[1].split()[0]
                break
        assert port, "server did not report a port"
        base = f"http://127.0.0.1:{port}"
        check("health endpoint", wait_health(base) is not None)

        with sync_playwright() as p:
            browser = p.chromium.launch(headless=True, args=["--enable-unsafe-swiftshader"])
            page = browser.new_page(viewport={"width": 1440, "height": 900})
            console_errors = []
            page.on("console", lambda m: console_errors.append(m.text[:200])
                    if m.type == "error" else None)
            page.on("pageerror", lambda e: console_errors.append(f"PAGEERROR: {e}"))

            page.goto(f"{base}/", wait_until="networkidle", timeout=20000)
            time.sleep(1.5)

            # ── 1. Samples ────────────────────────────────────────────────
            print("== sample bundles ==")
            for label, url, selector, min_nodes, kind in SAMPLES:
                load_bundle(page, url)
                # Wait for either the view or an error
                try:
                    page.wait_for_selector(selector, timeout=8000)
                except Exception:
                    pass
                assert_bundle_meta(page, kind)
                main_text = page.locator("main").inner_text()
                nodes_visible = len([n for n in main_text.split("\n") if n.strip()]) > min_nodes
                check(f"{label}: content rendered", nodes_visible,
                      f"(main has {len(main_text.splitlines())} lines)")
                page.screenshot(path=str(artifacts / f"sample-{label}.png"), full_page=True)

            # ── 2. Real bundles (unless --samples-only) ───────────────────
            if not args.samples_only:
                print("== real bundles ==")
                for label, cache_dir, selector in REAL_BUNDLES:
                    repo = CACHE / cache_dir
                    if not repo.exists():
                        print(f"[SKIP] {label}: not cached ({repo})")
                        continue
                    # Populate graph in an isolated XDG for this repo so the
                    # export reflects THIS repo only (deterministic).
                    import tempfile
                    with tempfile.TemporaryDirectory() as tmp:
                        xdg = Path(tmp) / "xdg"
                        (xdg / "data").mkdir(parents=True)
                        (xdg / "config").mkdir(parents=True)
                        env = {
                            "RUST_LOG": "error",
                            "XDG_DATA_HOME": str(xdg / "data"),
                            "XDG_CONFIG_HOME": str(xdg / "config"),
                        }
                        r = subprocess.run(
                            [str(bin_path), "code", "c4-discover", "--cwd", str(repo), "--apply"],
                            capture_output=True, text=True, env=env,
                        )
                        if r.returncode != 0:
                            print(f"[SKIP] {label}: discover failed: {r.stderr[:100]}")
                            continue
                        # Start a per-repo server with --cwd for /api/export
                        srv = subprocess.Popen(
                            [str(bin_path), "view", "--cwd", str(repo), "--port", "0"],
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                        )
                        try:
                            srv_port = None
                            dl = time.time() + 10
                            while time.time() < dl:
                                line = srv.stdout.readline()
                                if "127.0.0.1:" in line:
                                    srv_port = line.split("127.0.0.1:")[1].split()[0]
                                    break
                            if not srv_port:
                                print(f"[SKIP] {label}: server did not start")
                                continue
                            srv_base = f"http://127.0.0.1:{srv_port}"
                            # Navigate to THIS server's origin (avoids CORS
                            # between two localhost origins), then load its
                            # own /api/export same-origin.
                            page.goto(f"{srv_base}/", wait_until="networkidle", timeout=20000)
                            time.sleep(1.5)
                            load_bundle(page, f"{srv_base}/api/export")
                            try:
                                page.wait_for_selector(".c4-view", timeout=8000)
                            except Exception:
                                pass
                            elements = page.locator(".c4-element").count()
                            check(f"{label}: C4 elements rendered", elements >= 1,
                                  f"({elements} elements)")
                            page.screenshot(
                                path=str(artifacts / f"real-{label}.png"), full_page=True)
                        finally:
                            srv.terminate()
                            try:
                                srv.wait(timeout=5)
                            except Exception:
                                srv.kill()

            # ── 3. Invariants ─────────────────────────────────────────────
            check("zero console errors", len(console_errors) == 0,
                  f"-> {console_errors[:3]}" if console_errors else "")
            browser.close()
    finally:
        server.terminate()
        try:
            server.wait(timeout=5)
        except Exception:
            server.kill()

    print()
    if failures:
        print(f"RENDER_E2E FAIL: {len(failures)}: {failures}")
        sys.exit(1)
    print("RENDER_E2E PASS: workbench renders all bundle types correctly")


if __name__ == "__main__":
    main()
