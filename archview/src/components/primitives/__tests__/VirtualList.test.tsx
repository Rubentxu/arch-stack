// @vitest-environment jsdom
/**
 * VirtualList primitive tests (M20).
 *
 * Contract:
 *   - Renders only items visible in the viewport + overscan, regardless
 *     of the total items length. This is the whole point: DOM count
 *     stays bounded.
 *   - Fixed item height: the virtualizer uses arithmetic, not
 *     measurement, so it requires a `itemHeight` prop.
 *   - Scrolling updates the window — items leaving the visible range
 *     unmount, items entering mount.
 *   - Empty input produces no rows.
 *   - List semantics: outer is role="list", each row role="listitem".
 */
import { describe, expect, it } from "vitest";
import { render, fireEvent, screen } from "@solidjs/testing-library";
import { VirtualList } from "../index";

describe("VirtualList primitive", () => {
  it("renders only the items visible in the viewport (plus overscan)", () => {
    const items = Array.from({ length: 1000 }, (_, i) => i);
    render(() => (
      <VirtualList
        items={items}
        itemHeight={32}
        height={320}
        overscan={2}
        renderItem={(n) => <span data-testid="row">{n}</span>}
      />
    ));
    // height 320 / itemHeight 32 = 10 visible rows. At scrollTop=0
    // there's no overscan above (clamped to 0), so we render 10
    // visible + 2 overscan below = 12 items.
    const rows = screen.queryAllByTestId("row");
    expect(rows.length).toBe(12);
    // First row is item 0 (no scroll yet).
    expect(rows[0].textContent).toBe("0");
  });

  it("updates the rendered window when the user scrolls", () => {
    const items = Array.from({ length: 1000 }, (_, i) => `item-${i}`);
    const { container } = render(() => (
      <VirtualList
        items={items}
        itemHeight={32}
        height={320}
        overscan={2}
        renderItem={(n) => <span data-testid="row">{n as string}</span>}
      />
    ));
    const viewport = container.querySelector(".virtual-list") as HTMLDivElement;
    // Scroll down by 1000px → firstVisible = 1000/32 = 31. lastVisible
    // = ceil((1000+320)/32) - 1 = 41. start = 29, end = 43 → 15 items.
    fireEvent.scroll(viewport, { target: { scrollTop: 1000 } });
    const rows = screen.queryAllByTestId("row");
    // After scroll, first row should be item-29.
    expect(rows[0].textContent).toBe("item-29");
    expect(rows.length).toBe(15);
  });

  it("renders no rows when items is empty", () => {
    render(() => (
      <VirtualList
        items={[]}
        itemHeight={32}
        height={320}
        renderItem={(n: unknown) => <span>{String(n)}</span>}
      />
    ));
    expect(screen.queryAllByRole("listitem").length).toBe(0);
  });

  it("exposes list + listitem roles for assistive tech", () => {
    const items = [1, 2, 3];
    render(() => (
      <VirtualList
        items={items}
        itemHeight={32}
        height={320}
        renderItem={(n) => <span>{n as number}</span>}
        ariaLabel="Test list"
      />
    ));
    const list = screen.getByRole("list", { name: "Test list" });
    expect(list).toBeTruthy();
    expect(list.querySelectorAll('[role="listitem"]').length).toBe(3);
  });

  it("respects a custom itemKey extractor", () => {
    const items = [{ id: "a" }, { id: "b" }, { id: "c" }];
    const { container } = render(() => (
      <VirtualList
        items={items}
        itemHeight={32}
        height={320}
        renderItem={(it) => <span data-testid="row">{it.id}</span>}
        itemKey={(it) => it.id}
      />
    ));
    const rows = container.querySelectorAll('[role="listitem"]');
    expect(rows.length).toBe(3);
    // Each rendered item is wrapped in a role="listitem" container.
    expect(rows[0].textContent).toBe("a");
  });

  it("scales to 10k items without exploding the DOM", () => {
    const items = Array.from({ length: 10_000 }, (_, i) => i);
    const { container } = render(() => (
      <VirtualList
        items={items}
        itemHeight={32}
        height={320}
        overscan={3}
        renderItem={(n) => <span data-testid="row">{n as number}</span>}
      />
    ));
    // Total spacer height is 10_000 * 32 = 320_000px (scroll bar reflects it).
    const spacer = container.querySelector(
      ".virtual-list > div",
    ) as HTMLDivElement;
    expect(spacer.style.height).toBe("320000px");
    // But the actual rendered row count is bounded by the viewport.
    const rows = screen.queryAllByTestId("row");
    expect(rows.length).toBeLessThan(20);
  });
});
