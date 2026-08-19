/**
 * `<Button>` — primary interactive primitive (M17.B).
 *
 * Variants:
 *   - primary: solid accent background, accent-on text.
 *   - secondary: subtle background, fg text, border.
 *   - ghost: transparent, fg text, no border (used in toolbars).
 *
 * Sizes: sm | md (default) | lg.
 *
 * The element is a real `<button>` so screen readers, focus
 * rings, and keyboard activation work out of the box. The
 * visual styling is the only thing this component owns.
 */
import { splitProps, type Component, type JSX } from "solid-js";

export type ButtonVariant = "primary" | "secondary" | "ghost";
export type ButtonSize = "sm" | "md" | "lg";

export interface ButtonProps extends Omit<
  JSX.ButtonHTMLAttributes<HTMLButtonElement>,
  "type"
> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** Forwarded to the underlying <button> for non-button submit
   *  behaviour. Defaults to "button" so a form submit is never
   *  accidental. */
  type?: "button" | "submit" | "reset";
}

export const Button: Component<ButtonProps> = (props) => {
  const [local, rest] = splitProps(props, [
    "variant",
    "size",
    "type",
    "class",
    "children",
  ]);
  const variant = (): ButtonVariant => local.variant ?? "secondary";
  const size = (): ButtonSize => local.size ?? "md";
  return (
    <button
      type={local.type ?? "button"}
      class={`btn btn-${variant()} btn-${size()} ${local.class ?? ""}`.trim()}
      {...rest}
    >
      {local.children}
    </button>
  );
};
