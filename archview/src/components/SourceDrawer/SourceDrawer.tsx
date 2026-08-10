/**
 * SourceDrawer — read-only source preview + editor handoff (H1, ADR-041 §5–§6).
 *
 * Behaviour:
 * - Shows `file:line` header + windowed source preview (~5 lines around
 *   the requested line) fetched via `GET /api/source`.
 * - Read-only text: no editing, no syntax highlighting. Per scope
 *   minimalism in design.md §9.
 * - "Open in editor" button posts `POST /api/open-editor`. The backend
 *   resolves `$EDITOR`/`$VISUAL`/`xdg-open` and spawns without shell.
 * - Accessible: `role="region"`, `aria-label`, focus management on
 *   button activation.
 */

import {
  For,
  Match,
  Show,
  Switch,
  createResource,
  type Component,
} from "solid-js";
import type { SourcePreview } from "../../lib/workspace";

export interface SourceDrawerProps {
  file: string;
  line: number;
  fetchSource: (file: string, line: number) => Promise<SourcePreview>;
  openInEditor: (file: string, line: number) => Promise<boolean>;
}

export const SourceDrawer: Component<SourceDrawerProps> = (props) => {
  const [resource] = createResource(
    () => ({ file: props.file, line: props.line }),
    ({ file, line }) => props.fetchSource(file, line),
  );

  return (
    <section
      class="source-drawer"
      role="region"
      aria-label={`Source preview for ${props.file} line ${props.line}`}
    >
      <header class="source-drawer-header">
        <code class="source-drawer-path">
          {props.file}:{props.line}
        </code>
        <button
          class="open-in-editor"
          type="button"
          onClick={async () => {
            await props.openInEditor(props.file, props.line);
          }}
          aria-label={`Open ${props.file} at line ${props.line} in editor`}
        >
          Open in editor
        </button>
      </header>

      <Show
        when={!resource.loading}
        fallback={<p class="muted">Loading source preview…</p>}
      >
        <Switch>
          <Match when={resource.error}>
            <p class="error">
              Failed to load source: {(resource.error as Error).message}
            </p>
          </Match>
          <Match when={resource()}>
            {(preview) => (
              <ol class="source-drawer-lines" aria-label="Source code preview">
                <For each={preview().content}>
                  {(line, idx) => (
                    <li
                      class={
                        preview().start_line + idx() === props.line
                          ? "source-line current"
                          : "source-line"
                      }
                    >
                      <span class="line-number">
                        {preview().start_line + idx()}
                      </span>
                      <code class="line-content">{line || "\u00A0"}</code>
                    </li>
                  )}
                </For>
                <Show when={preview().truncated}>
                  <li class="source-truncated muted">
                    … truncated at {preview().total_lines} total lines …
                  </li>
                </Show>
              </ol>
            )}
          </Match>
        </Switch>
      </Show>
    </section>
  );
};

export default SourceDrawer;
