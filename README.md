# archctl — Architecture Intelligence Planning Base

`archctl` is currently a **decision-grade research and planning repository**, not an implemented product.

The project evaluates an evidence-first architecture recovery system for OpenCode: source-grounded evidence is normalized into a renderer-neutral Architecture IR and projected to C4/UML views. The platform remains conditional on small, falsifiable experiments.

## Start here

1. [Executive summary](docs/EXECUTIVE-SUMMARY.md)
2. [Roadmap and kill gates](docs/ROADMAP.md)
3. [Proposed ADRs](docs/adr/README.md)
4. [Source design document](Skills-para-agentes-IA.md)

## Detailed planning artifacts

- [Exploration and feasibility review](sddk/architecture-intelligence-platform/explore-report.md)
- [Proposal](sddk/architecture-intelligence-platform/proposal.md)
- [Behavior specification](sddk/architecture-intelligence-platform/spec.md)
- [Technical design](sddk/architecture-intelligence-platform/design.md)
- [Implementation task plan](sddk/architecture-intelligence-platform/tasks.md)
- [Final verification](sddk/architecture-intelligence-platform/verification-report.md)
- [Domain glossary](CONTEXT.md)

## Current decision

- TypeScript-first for M0–M2; Rust is conditional and deferred.
- All eight ADRs are **Accepted** as of 2026-07-29, with operationalised decision rules.
- The next step is a 3–4 day Gate Zero experiment, not platform implementation.
- A high-confidence claim without evidence is a hard failure.
- Initial OpenCode pin is **1.18.x**.

This is the first commit of the planning repository (`main` branch). No product source code,
no remote, no tags, no releases — only decision-grade planning artifacts.
