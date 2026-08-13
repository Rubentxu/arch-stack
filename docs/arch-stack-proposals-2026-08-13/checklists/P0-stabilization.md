# Checklist P0 — Stabilization

## Build / storage
- [ ] Reproducir fallo Ladybug en clean runner.
- [ ] Determinar versión exacta crate/native.
- [ ] Eliminar `latest` mutable.
- [ ] Definir compiler/C++ stdlib mínimos.
- [ ] `doctor --scope storage`.
- [ ] Linux x86_64 verde.
- [ ] Linux aarch64 verde.
- [ ] macOS x86_64 verde en runner macOS.
- [ ] macOS arm64 verde en runner macOS.

## Plugins
- [ ] `~/.local/share/archctl/plugins`.
- [ ] `create_dir_all` antes de staging.
- [ ] identity value objects.
- [ ] checksum remote obligatorio.
- [ ] safe tar extraction.
- [ ] malicious fixtures.
- [ ] first-install E2E.

## Governance
- [ ] Resolver duplicate ADR-040.
- [ ] Resolver duplicate ADR-041.
- [ ] ADR integrity gate.
- [ ] License decision + files.
- [ ] License coherence gate.
- [ ] PR CI fast gate.
- [ ] branch protection.

## Contracts
- [ ] Filesystem contract documented.
- [ ] SystemFS contract suite.
- [ ] MemoryFS contract suite.
- [ ] Stale capability comments corrected.
