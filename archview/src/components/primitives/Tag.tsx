/**
 * `<Tag>` — small inline chip for state and category labels.
 *
 * Used by:
 *   - C4 levels ("Context (1)", "Container (3)")
 *   - Drift categories ("+ added", "− removed")
 *   - Edge kinds ("calls", "implements")
 *
 * The color is driven by CSS classes (`.tag-context`,
 * `.tag-container`, ...) so the design system stays in CSS,
 * not in the TS code.
 */
import { splitProps, type Component, type JSX } from "solid-js";

export type TagTone =
  | "default"
  | "context"
  | "container"
  | "component"
  | "ok"
  | "warn"
  | "err"
  | "info";

export interface TagProps extends JSX.HTMLAttributes<HTMLSpanElement> {
  tone?: TagTone;
}

export const Tag: Component<TagProps> = (props) => {
  const [local, rest] = splitProps(props, ["tone", "class", "children"]);
  const tone = (): TagTone => local.tone ?? "default";
  return (
    <span class={`tag tag-${tone()} ${local.class ?? ""}`.trim()} {...rest}>
      {local.children}
    </span>
  );
};
