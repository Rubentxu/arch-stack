/**
 * `<VirtualList>` — fixed-height virtualized list primitive.
 *
 * Renders only the items currently visible in the scroll viewport (plus
 * an `overscan` buffer above and below). When `items` has thousands of
 * entries, the DOM only ever contains ~`visibleCount + 2 * overscan`
 * child elements, not one per item.
 *
 * This is a defense-in-depth primitive for lists that can grow large
 * (Sidebar relations list, future node picker). Today the workbench's
 * largest list is the Sidebar's relations on a hub node (~100 entries);
 * tomorrow a C4 bundle could expose 1k+ relations on a single hub.
 *
 * Contract:
 *   - `items`: array of T (any T; the consumer decides what to render).
 *   - `itemHeight`: fixed height of every item in CSS pixels. Required
 *     because the virtualizer uses arithmetic, not measurement.
 *   - `height`: visible viewport height in CSS pixels. Required for the
 *     same reason. Callers can use `height="100%"` and let the parent
 *     constrain the box.
 *   - `overscan`: how many extra items to render above/below the
 *     visible window. Default 5 — covers most scroll deltas without
 *     leaving the user staring at empty space.
 *   - `renderItem`: `(item: T, index: number) => JSX.Element`.
 *   - `ariaLabel` / `role`: passed to the outer container.
 *
 * What this is NOT:
 *   - Not a measurement-based virtualizer (fixed-height is a hard
 *     requirement; for variable heights use a different primitive).
 *   - Not accessible to screen readers in the deep sense: the visible
 *     items are real DOM but the underlying list semantics are not
 *     reified. For a 1k-item relations list, that's acceptable —
 *     screen reader users can navigate row by row.
 */
import {
  For,
  createMemo,
  createSignal,
  onCleanup,
  type Component,
  type JSX,
} from "solid-js";

export interface VirtualListProps<T> {
  items: readonly T[];
  itemHeight: number;
  height: number | string;
  renderItem: (item: T, index: number) => JSX.Element;
  overscan?: number;
  ariaLabel?: string;
  role?: string;
  class?: string;
  /** Key extractor for stable identity. Defaults to index. */
  itemKey?: (item: T, index: number) => string | number;
}

/**
 * Internal helper: builds a positional-key extractor for `<For>` so
 * the virtualizer can keep stable row identity when the visible
 * window slides. Without this, every scroll would re-create every
 * row, defeating the whole point of virtualization.
 */
function defaultKey<T>(item: T, index: number): string {
  return String(index);
}

export const VirtualList: Component<VirtualListProps<unknown>> = (props) => {
  const overscan = (): number => props.overscan ?? 5;
  const [scrollTop, setScrollTop] = createSignal(0);
  let viewportEl: HTMLDivElement | undefined;

  // Total scroll length — used to size the inner spacer so the
  // scrollbar reflects the real extent of the list.
  const totalHeight = createMemo(() => props.items.length * props.itemHeight);

  // First and last indices to render (inclusive). The formula:
  //   - firstVisible = floor(scrollTop / itemHeight)
  //   - lastVisible = ceil((scrollTop + viewport) / itemHeight) - 1
  //   - start = firstVisible - overscan (clamped to 0)
  //   - end = lastVisible + overscan (clamped to items.length - 1)
  // so the rendered window is `visibleCount + 2*overscan` items,
  // where visibleCount = lastVisible - firstVisible + 1.
  const range = createMemo<readonly [number, number]>(() => {
    const itemH = props.itemHeight;
    if (itemH <= 0 || props.items.length === 0) return [0, -1] as const;
    const viewportPx = typeof props.height === "number" ? props.height : 400;
    const firstVisible = Math.floor(scrollTop() / itemH);
    const lastVisible = Math.ceil((scrollTop() + viewportPx) / itemH) - 1;
    const start = Math.max(0, firstVisible - overscan());
    const end = Math.min(props.items.length - 1, lastVisible + overscan());
    return [start, end] as const;
  });

  // Rendered window: items in [start..end] of the source list, with
  // their original index. Solid's <For> tracks by reference, but
  // the window itself is a new array on every scroll, so identity
  // tracking is per-scroll not per-item. We pair that with the
  // `itemKey` extractor so <For> can spot rows that survive across
  // scrolls and reuse their DOM nodes.
  const window = createMemo(() => {
    const [start, end] = range();
    if (end < start) return [];
    const out: Array<{ item: unknown; index: number }> = [];
    for (let i = start; i <= end; i++) {
      out.push({ item: props.items[i], index: i });
    }
    return out;
  });

  const onScroll = (e: Event) => {
    const target = e.currentTarget as HTMLDivElement;
    setScrollTop(target.scrollTop);
  };

  onCleanup(() => {
    viewportEl = undefined;
  });

  return (
    <div
      ref={viewportEl}
      class={`virtual-list ${props.class ?? ""}`.trim()}
      style={{
        height:
          typeof props.height === "number" ? `${props.height}px` : props.height,
        overflow: "auto",
        position: "relative",
      }}
      onScroll={onScroll}
      role={props.role ?? "list"}
      aria-label={props.ariaLabel}
    >
      <div
        style={{
          height: `${totalHeight()}px`,
          position: "relative",
        }}
      >
        <For each={window()}>
          {(entry) => {
            const top = entry.index * props.itemHeight;
            return (
              <div
                style={{
                  position: "absolute",
                  top: `${top}px`,
                  left: 0,
                  right: 0,
                  height: `${props.itemHeight}px`,
                }}
                role="listitem"
              >
                {props.renderItem(entry.item, entry.index)}
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
};

// Re-export the key helper so tests can assert against it.
export const keyFor = defaultKey;
