/**
 * `<TabBar>` / `<TabPanel>` — ARIA APG tablist primitive (M22).
 *
 * Implements the W3C WAI-ARIA Authoring Practices Tabs Pattern with
 * **automatic activation** (focusing a tab activates it immediately).
 * Keyboard navigation: ArrowRight / ArrowLeft cycle, Home / End jump
 * to extremes, Space / Enter activate.
 *
 * Contract:
 *   - `<TabBar>` is a controlled component: it receives `items`,
 *     `value`, `onChange`, and `ariaLabel`. No internal signal.
 *   - `<TabPanel>` is a passive viewport: it renders `children`
 *     only when `value === activeValue`.
 *   - The primitive is agnostic to the meaning of `id` values — it
 *     treats them as opaque strings.
 *
 * Usage:
 *   ```tsx
 *   const [active, setActive] = createSignal("tab-a");
 *   <TabBar
 *     items={[
 *       { id: "tab-a", label: "Tab A", badge: 3 },
 *       { id: "tab-b", label: "Tab B" },
 *     ]}
 *     value={active()}
 *     onChange={setActive}
 *     ariaLabel="My panels"
 *   />
 *   <TabPanel value="tab-a" activeValue={active()}>
 *     <p>Content for tab A</p>
 *   </TabPanel>
 *   ```
 */
import { For, Show, type Component, type JSX } from "solid-js";

export interface TabItem {
  /** Opaque identifier — must match a corresponding `TabPanel value`. */
  id: string;
  /** User-facing label rendered inside the tab button. */
  label: string;
  /** Optional badge count. Rendered only when > 0. */
  badge?: number;
  /** If true the tab is non-interactive and non-focusable. */
  disabled?: boolean;
}

export interface TabBarProps {
  /**
   * Ordered list of tab descriptors. The order is the visual order.
   * `readonly` signals the caller should treat it as immutable.
   */
  items: readonly TabItem[];
  /**
   * The `id` of the currently active tab.
   * Must be present in `items`; no validation is performed.
   */
  value: string;
  /**
   * Called with the `id` of the tab to activate. The caller is
   * responsible for updating `value`.
   */
  onChange: (id: string) => void;
  /** Passed as `aria-label` on the `role="tablist"` container. */
  ariaLabel: string;
}

/** Returns the index of `id` in `items`, or -1 if not found. */
function indexOf(items: readonly TabItem[], id: string): number {
  return items.findIndex((t) => t.id === id);
}

/**
 * `<TabBar>` — accessible tab list (ARIA APG tablist).
 *
 * Renders a `role="tablist"` container with one `role="tab"` button
 * per `items` entry. Active tab has `aria-selected="true"` and
 * `tabindex="0"`; all others have `aria-selected="false"` and
 * `tabindex="-1"`.
 *
 * Keyboard handling (automatic activation):
 *   - ArrowRight → next tab (wraps)
 *   - ArrowLeft  → previous tab (wraps)
 *   - Home       → first tab
 *   - End        → last tab
 *   - Space / Enter → activate focused tab (calls `onChange`)
 */
export const TabBar: Component<TabBarProps> = (props) => {
  const handleKeyDown = (e: KeyboardEvent, tabId: string) => {
    if (tabId === undefined) return;
    const len = props.items.length;
    if (len === 0) return;

    const current = indexOf(props.items, tabId);

    switch (e.key) {
      case "ArrowRight": {
        e.preventDefault();
        const next = (current + 1) % len;
        const nextId = props.items[next]!.id;
        props.onChange(nextId);
        break;
      }
      case "ArrowLeft": {
        e.preventDefault();
        const prev = (current - 1 + len) % len;
        const prevId = props.items[prev]!.id;
        props.onChange(prevId);
        break;
      }
      case "Home": {
        e.preventDefault();
        props.onChange(props.items[0]!.id);
        break;
      }
      case "End": {
        e.preventDefault();
        props.onChange(props.items[len - 1]!.id);
        break;
      }
      case " ":
      case "Enter": {
        e.preventDefault();
        props.onChange(tabId);
        break;
      }
    }
  };

  return (
    <div role="tablist" aria-label={props.ariaLabel} class="tab-bar">
      <For each={props.items}>
        {(item) => {
          const isActive = () => item.id === props.value;
          const panelId = `panel-${item.id}`;
          const tabId = `tab-${item.id}`;

          return (
            <button
              type="button"
              id={tabId}
              role="tab"
              class={`tab${isActive() ? " is-active" : ""}`}
              aria-selected={isActive()}
              aria-controls={panelId}
              tabindex={isActive() ? 0 : -1}
              disabled={item.disabled ?? false}
              onClick={() => !item.disabled && props.onChange(item.id)}
              onKeyDown={(e) => handleKeyDown(e, item.id)}
            >
              {item.label}
              <Show when={item.badge !== undefined && item.badge > 0}>
                <span class="tab-badge" aria-label={`${item.badge} items`}>
                  {item.badge}
                </span>
              </Show>
            </button>
          );
        }}
      </For>
    </div>
  );
};

export interface TabPanelProps {
  /**
   * The `id` of this panel. Must match the `id` of the `TabItem`
   * that controls it.
   */
  value: string;
  /**
   * The `id` of the currently active tab. This panel renders its
   * `children` only when `value === activeValue`.
   */
  activeValue: string;
  /** Content to render inside the panel. */
  children: JSX.Element;
}

/**
 * `<TabPanel>` — accessible tab panel (ARIA APG tabpanel).
 *
 * Renders `children` only when `value === activeValue`. The panel
 * has `role="tabpanel"`, `aria-labelledby` pointing to the tab
 * button, `tabindex="0"` for focusability, and `hidden` when
 * inactive.
 *
 * Hidden panels use `display: none` so they are not in the accessibility
 * tree — this is the expected behavior for ARIA tabpanels with automatic
 * activation.
 */
export const TabPanel: Component<TabPanelProps> = (props) => {
  const isActive = () => props.value === props.activeValue;
  const panelId = () => `panel-${props.value}`;
  const tabId = () => `tab-${props.value}`;

  return (
    <div
      id={panelId()}
      role="tabpanel"
      aria-labelledby={tabId()}
      tabindex={0}
      class="tab-panel"
      hidden={!isActive()}
    >
      <Show when={isActive()}>{props.children}</Show>
    </div>
  );
};
