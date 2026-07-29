# XDG layout created on Phase 0 bootstrap. Idempotent.
default:
  data: ${XDG_DATA_HOME:-$HOME/.local/share}/archctl
  state: ${XDG_STATE_HOME:-$HOME/.local/state}/archctl
  cache: ${XDG_CACHE_HOME:-$HOME/.cache}/archctl
  config: ${XDG_CONFIG_HOME:-$HOME/.config}/archctl
created:
  - ${data}/projects/
  - ${state}/runs/
  - ${cache}/archctl/
  - ${config}/archctl/
note: archctl never writes to the analyzed repository. See ADR-0003.
