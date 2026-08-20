# Security, Privacy & Local-first

## Defaults
Localhost only, source read-only, XDG fuera del repo, no network desde workbench por defecto, sanitized bundles read-only, agent capabilities declaradas y proposals governed.

## Indexing controls
Redaction configurable, límites de tamaño, exclusiones de path/file, secret scanner y nunca persistir credential payloads.

## Epistemic poisoning
Riesgo propio del producto: una inference agentic almacenada como fact contamina análisis futuros. Mitigación: AuthorityClass, candidate gate, provenance y UAT false-claim.
