/**
 * `<EmptyState>` — empty-canvas placeholder (M17.B).
 *
 * The old `archview` empty state was a single line of muted
 * text. Sprint C's "onboarding" goal is to turn this into a
 * real landing card: icon, headline, body copy, optional
 * action button. Sprint B ships the primitive so the canvas
 * (and any other empty surfaces) can use it consistently.
 */
import { Show, type Component, type JSX } from "solid-js";

export interface EmptyStateProps {
  /** Optional icon — text-only fallback if absent. */
  icon?: JSX.Element;
  /** Headline. Required. */
  title: string;
  /** One paragraph of body copy. */
  body?: string;
  /** Optional action button (rendered below the body). */
  action?: JSX.Element;
  /** Optional extra class on the root. */
  class?: string;
}

export const EmptyState: Component<EmptyStateProps> = (props) => {
  return (
    <div class={`empty-state ${props.class ?? ""}`.trim()}>
      <Show when={props.icon}>
        <div class="empty-state-icon" aria-hidden="true">
          {props.icon}
        </div>
      </Show>
      <h2 class="empty-state-title">{props.title}</h2>
      <Show when={props.body}>
        <p class="empty-state-body">{props.body}</p>
      </Show>
      <Show when={props.action}>
        <div class="empty-state-action">{props.action}</div>
      </Show>
    </div>
  );
};
