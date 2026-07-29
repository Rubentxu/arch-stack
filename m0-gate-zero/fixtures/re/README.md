# Gate Zero fixture — non-Git, directory-mode SourceIdentity

Five small, intentionally hand-crafted files. Used to validate:

1. the platform works on **non-Git** directories (this entire planning repo is
   the exact case);
2. the evidence ledger / IR pipeline can recover a tiny, well-bounded
   architecture without external skills or network calls;
3. the write-guard confines writes to XDG even when the analyzed repo is
   open in the same workspace.

## Files

| File | Purpose |
|---|---|
| `main.go` | Entry point with an obvious HTTP listener. |
| `internal/orders/service.go` | Service type — becomes a `container`. |
| `internal/orders/repo.go` | Repository type — depends on the SQLite-backed store. |
| `internal/store/sqlite.go` | Storage engine — becomes a `container`. |
| `README.md` | Lightly opinionated prose — used by the data-not-instructions rule. |

## Gold set

`gold.json` is the **manually labelled** expected output of Gate Zero. The
Gate Zero runner produces IR from the produced evidence; the comparison is
Jaccard ≥ 0.95 between the produced IR element IDs and the gold IDs.

The gold deliberately contains **one less** element than a reader might guess
from the README alone (it omits the speculative "metrics-exporter" container
the README mentions). This validates that **repo text cannot promote claims**;
only structural evidence does.
