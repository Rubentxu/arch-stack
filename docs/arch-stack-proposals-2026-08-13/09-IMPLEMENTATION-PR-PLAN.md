# Plan de PRs pequeños

## Wave 0 — remediation
1. plugin root + first-install tests;
2. plugin identifier validation + malicious tar;
3. ADR duplicate resolution + integrity gate;
4. license coherence;
5. Ladybug toolchain/native pin;
6. PR CI fast gates;
7. native release runners.

## Wave 1 — architecture scaffolding
8. dependency fitness baseline report-only;
9. composition root skeleton;
10. migrate `doctor`;
11. repository ports delegating to current store;
12. migrate `diagram` reads;
13. RawGraphQuery boundary;
14. filesystem contracts;
15. CapabilityRegistry current-state;
16. generated capability docs.

## Wave 2 — intelligence
17. Snapshot metadata MVP;
18. Architecture Diff schema/pure diff;
19. CLI diff + cognitive/delta adapter;
20. DriftView contract;
21. Explain v1;
22. coverage;
23. policy evaluator;
24. SARIF;
25. Task Context deterministic core;
26. MCP context tool;
27. Observation/Claim dual-write experiment.

## Wave 3 — platform
28. strict ArchBundle;
29. archview read-only bundle;
30. session token;
31. cross-view NavigationTarget;
32. action palette;
33. semantic zoom;
34. lens recommendation experiment.

## PR policy
- PR estructural ≠ feature no relacionada;
- actualizar ADR/spec/manifest si cambia contrato;
- golden antes/después;
- rollback explícito;
- benchmark si toca hot path.
