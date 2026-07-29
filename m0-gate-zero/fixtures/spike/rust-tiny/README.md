# rust-tiny-fixture

In-tree Rust workspace used by archctl's M1 spike (task 1.11). It is
intentionally tiny and self-contained so that:

- The fast profile (`ast-grep` + `ctags` + `cargo metadata`) can ingest it
  in seconds.
- The IR produced from the deterministic runner matches a hand-labelled
  gold set with Jaccard ≥ 0.95.

## Layout

```
rust-tiny/
  Cargo.toml              # MIT, edition 2021, ≤5 kLoC
  LICENSE.spdx.json       # machine-readable SPDX license declaration
  README.md               # this file
  gold.json               # hand-labelled gold set (containers + relationships)
  src/
    lib.rs                # public API surface
    service.rs            # Container candidate — exports Service type
    repo.rs               # Container candidate — exports Repo type
    store.rs              # Container candidate — exports Store type
    main.rs               # entry point — wires the three together
```

## License

This fixture is **MIT** — written in-tree specifically for archctl, so
there is no third-party license ambiguity. See `LICENSE.spdx.json` for
the SPDX-2.3 declaration.
