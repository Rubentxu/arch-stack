# archctl-bench — M27 Sandbox

> Reference: [`docs/adr/ADR-032-bench-methodology.md`](../../docs/adr/ADR-032-bench-methodology.md)

## What

The `archctl-bench` Quadlet sandbox runs the C4 vertical and complementary
extractors against 10+ multi-language repositories to produce a pre-v1.0
release-gate report. Manual invocation, not a CI job.

## Components

| File | Purpose |
|---|---|
| `bench/Containerfile` | ubuntu:24.04 + rustup 1.97.1 |
| `bench/build.sh` | `podman build` helper |
| `bench/entrypoint.sh` | Container entrypoint (prints toolchain, execs $@) |
| `bench/quadlets/archctl-bench.container` | Quadlet unit (Type=oneshot, rootless) |
| `bench/datasets.toml` | 10+ pinned repos (Phase 2) |
| `bench/run-bench.sh` | Orchestrator + metrics + report (Phase 3) |
| `bench/reports/<date>.md` | Generated report (Phase 3) |

## Build

```bash
bench/build.sh
podman run --rm archctl-bench:latest rustc --version
# Expected: rustc 1.97.1 (...)
```

## Run via Quadlet (preferred)

```bash
# Install the unit
mkdir -p ~/.config/containers/systemd
cp bench/quadlets/archctl-bench.container ~/.config/containers/systemd/
systemctl --user daemon-reload
systemctl --user start archctl-bench.service
```

## Run via direct invocation (fallback)

If Quadlet is unworkable (no systemd user session, no subuid mapping):

```bash
podman run --rm -it \
  -v ~/.local/share/archctl:/xdg/data:rw \
  -v ./bench/reports:/reports:rw \
  archctl-bench:latest
```

## Notes

- archctl binary is mounted pre-built from host `target/release/archctl` (not built inside container — ~10min first-run cost).
- Dataset cache at `~/.cache/archctl-smoke/` (gitignored).
- See `docs/specs/bench-harness.md` and `docs/specs/bench-methodology.md` for the behavioral contract.
