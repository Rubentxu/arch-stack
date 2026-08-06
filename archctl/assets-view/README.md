# assets-view

Carpeta de origen para el workbench `archview` embebido en el binario
`archctl` (ADR-033).

El contenido se genera con `scripts/embed-view.sh` (copia `archview/dist`
excluyendo sourcemaps). Esta carpeta está gitignored — el dist se reconstruye
en CI durante el release.

Sin assets copiados, `archctl view` compila pero responde con un error claro:

```
view assets not embedded — run: pnpm build (archview) && scripts/embed-view.sh
```
