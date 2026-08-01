#!/usr/bin/env python3
"""Deterministic dataset generator for archctl benches.

Emits 3 JSON fixtures at benchmarks/datasets/ matching the row shape
that `archctl graph query` returns for `MATCH (e:Element) ...`:

- small-100.json:  100 elements,  250 relations
- medium-1k.json: 1000 elements, 2500 relations
- large-10k.json: 10000 elements, 25000 relations

Deterministic via `random.seed(0xC0DE0001)`. Re-running produces
byte-identical fixtures.

Usage:
  python3 scripts/generate_bench_datasets.py [--out benchmarks/datasets]
"""

import argparse
import json
import random
import sys
from pathlib import Path


# Pools — chosen for variety without needing semantic precision.
META_TYPES = [
    "mt.system", "mt.container", "mt.component", "mt.code",
    "mt.deployment", "mt.person", "mt.datastore", "mt.external",
    "mt.queue", "mt.endpoint",
]

CATEGORIES = ["container", "component", "code", "c4_dynamic", "deployment"]

# 50 canonical keys (mostly services-style identifiers).
CANONICAL_KEYS = [
    "orders", "payments", "inventory", "shipping", "users", "auth",
    "gateway", "api", "admin", "reporting", "billing", "search",
    "catalog", "checkout", "cart", "wishlist", "review", "ratings",
    "notifications", "email", "sms", "push", "audit", "analytics",
    "metrics", "logs", "tracing", "scheduler", "worker", "queue",
    "cache", "session", "token", "config", "feature_flag", "secret",
    "profile", "address", "payment_method", "subscription", "invoice",
    "refund", "tax", "fx", "promo", "coupon", "loyalty", "reward",
    "media", "asset",
]

PREDICATES = [
    "p.calls", "p.depends_on", "p.uses", "p.owns", "p.contains",
    "p.flows_to", "p.publishes", "p.subscribes",
]

# Stable status distribution: 90% accepted, 8% drafted, 2% superseded.
STATUS_DIST = [
    ("accepted", 0.90),
    ("drafted", 0.08),
    ("superseded", 0.02),
]


def pick_status(rng: random.Random) -> str:
    r = rng.random()
    acc = 0.0
    for s, p in STATUS_DIST:
        acc += p
        if r <= acc:
            return s
    return "accepted"


def generate(seed: int, n_elements: int, relation_ratio: float) -> dict:
    """Generate a single dataset dict."""
    rng = random.Random(seed)

    elements = []
    for i in range(n_elements):
        e_id = f"el:{i + 1}"
        kind_id = rng.choice(META_TYPES)
        category = rng.choice(CATEGORIES)
        canonical_key = rng.choice(CANONICAL_KEYS)
        current_name = canonical_key.replace("_", " ").title() + f" {i + 1}"
        current_status = pick_status(rng)
        current_confidence = round(rng.uniform(0.70, 0.99), 3)
        current_version_id = f"v:{i + 1}"
        elements.append({
            "e.id": e_id,
            "e.kind_id": kind_id,
            "e.category": category,
            "e.canonical_key": canonical_key,
            "e.current_name": current_name,
            "e.current_status": current_status,
            "e.current_confidence": current_confidence,
            "e.current_version_id": current_version_id,
        })

    n_relations = int(n_elements * relation_ratio)
    relations = []
    for i in range(n_relations):
        src_i = rng.randint(0, n_elements - 1)
        tgt_i = rng.randint(0, n_elements - 1)
        # Avoid self-loops in 80% of cases; allow loops for cyclic graphs.
        if tgt_i == src_i and rng.random() < 0.8:
            tgt_i = (tgt_i + 1) % n_elements
        rid = f"rel:{i + 1}"
        predicate_id = rng.choice(PREDICATES)
        order_key = str(i + 1)
        relations.append({
            "edge.relation_id": rid,
            "edge.predicate_id": predicate_id,
            "src.id": f"el:{src_i + 1}",
            "tgt.id": f"el:{tgt_i + 1}",
            "edge.order_key": order_key,
            "edge.props": "{}",
        })

    # One version per element for completeness; not currently consumed
    # by benches (kept for forward-compat).
    versions = []
    for el in elements:
        versions.append({
            "v.id": el["e.current_version_id"],
            "v.name": el["e.current_name"],
            "v.description": f"Auto-generated fixture for {el['e.id']}",
            "v.props": "{}",
        })

    return {
        "elements": elements,
        "relations": relations,
        "versions": versions,
    }


SIZES = [
    ("small-100", 100, 2.5),
    ("medium-1k", 1_000, 2.5),
    ("large-10k", 10_000, 2.5),
]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--out",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "benchmarks" / "datasets",
        help="Output directory for fixtures",
    )
    args = ap.parse_args()

    args.out.mkdir(parents=True, exist_ok=True)

    for label, n_elements, rel_ratio in SIZES:
        path = args.out / f"{label}.json"
        ds = generate(0xC0DE0001, n_elements, rel_ratio)
        path.write_text(json.dumps(ds, separators=(",", ":")))
        size_kb = path.stat().st_size / 1024
        print(
            f"  wrote {path}  ({n_elements} elements, {len(ds['relations'])} relations, {size_kb:.1f} KB)",
            file=sys.stderr,
        )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())