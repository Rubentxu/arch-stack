# Quality Gates

## Pull Request — prevention
1. fmt;
2. clippy `-D warnings`;
3. cargo check/test;
4. contract tests;
5. archview test/build;
6. JSON schema validation;
7. ADR integrity;
8. specs/index integrity;
9. architecture dependency rules;
10. license coherence;
11. plugin security tests;
12. bundle size cap.

Benchmarks largos solo con etiqueta/perf-sensitive o post-merge.

## Post-merge — evidence
- full integration/E2E;
- benchmark smoke;
- regression compare;
- real-project corpus;
- vulnerability/license scan;
- nightly matrix opcional.

## Release
- build nativo por Tier-1;
- Ladybug compatibility smoke;
- SHA256 manifest;
- artifact provenance;
- self-update/install/uninstall E2E;
- migration dry run;
- no publicar si falta Tier-1.

## Architectural fitness

```text
application -> domain/ports
domain -> std + pure approved crates
adapters -> application/domain/ports
cli -> application + formatting
archview -> projection contracts
```

## Debt ratchet
- nuevo archivo >20 KB exige justificación;
- archivo legacy grande no crece >5% sin excepción;
- `cli.rs`/`store.rs` target decreciente;
- excepción incluye issue de extracción.
