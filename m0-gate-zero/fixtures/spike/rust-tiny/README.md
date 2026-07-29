# Rust tiny fixture — planned for M1

This fixture is a *placeholder* for the M1 Rust tiny repo (≤5k LoC). It is
not yet materialised because the Spike Report (1.12) operates on synthetic
IRs against the gold set; the real Rust repo will land when M1 starts.

What the fixture will provide:
- A small Rust workspace (`cargo metadata` ingestable) with at most 5k LoC.
- Hand-labelled gold set (`gold.json`) covering the discovered workspace
  members + their relationship graph.
- SPDX license declaration in `LICENSE.spdx`.

The placeholder exists so the file path is reserved and the manifest of the
spike suite is stable.
