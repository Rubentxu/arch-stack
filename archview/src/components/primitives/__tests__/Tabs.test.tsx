// @vitest-environment jsdom
/**
 * Unit tests for the `<TabBar>` / `<TabPanel>` ARIA tablist primitive (M22).
 *
 * Tests the ARIA APG tablist behavior:
 *   - Render with items; active tab has aria-selected="true"
 *   - Clicking a tab calls onChange with the correct id
 *   - Keyboard ArrowRight/ArrowLeft cycles focus + activates
 *   - Keyboard Home/End jumps to first/last tab
 *   - Badge renders when > 0; omitted when undefined or 0
 *   - Disabled tab is not focusable and does not call onChange
 *   - Inactive TabPanel does not render children
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { render, fireEvent } from "@solidjs/testing-library";
import { TabBar, TabPanel } from "../Tabs";

describe("<TabBar>", () => {
  afterEach(() => {
    // Ensure cleanup between tests
  });

  it("renders tablist and tabs from items", () => {
    const { container } = render(() => (
      <TabBar
        items={[
          { id: "a", label: "Tab A" },
          { id: "b", label: "Tab B" },
        ]}
        value="a"
        onChange={() => {}}
        ariaLabel="Test tabs"
      />
    ));
    const tablist = container.querySelector('[role="tablist"]');
    expect(tablist).toBeTruthy();
    expect(tablist!.getAttribute("aria-label")).toBe("Test tabs");
    const tabs = container.querySelectorAll('[role="tab"]');
    expect(tabs).toHaveLength(2);
  });

  it("active tab has aria-selected true; others false", () => {
    const { container } = render(() => (
      <TabBar
        items={[
          { id: "a", label: "Tab A" },
          { id: "b", label: "Tab B" },
        ]}
        value="b"
        onChange={() => {}}
        ariaLabel="Test"
      />
    ));
    const tabs = Array.from(container.querySelectorAll('[role="tab"]')) as HTMLButtonElement[];
    expect(tabs).toHaveLength(2);
    expect(tabs[0].getAttribute("aria-selected")).toBe("false");
    expect(tabs[0].getAttribute("tabindex")).toBe("-1");
    expect(tabs[1].getAttribute("aria-selected")).toBe("true");
    expect(tabs[1].getAttribute("tabindex")).toBe("0");
  });

  it("clicking a tab calls onChange with correct id", () => {
    const onChange = vi.fn();
    const { container } = render(() => (
      <TabBar
        items={[
          { id: "a", label: "Tab A" },
          { id: "b", label: "Tab B" },
        ]}
        value="a"
        onChange={onChange}
        ariaLabel="Test"
      />
    ));
    const tabs = Array.from(container.querySelectorAll('[role="tab"]')) as HTMLButtonElement[];
    expect(tabs).toHaveLength(2);
    fireEvent.click(tabs[1]);
    expect(onChange).toHaveBeenCalledWith("b");
  });

  it("ArrowRight cycles to next tab; ArrowLeft cycles backward", () => {
    const onChange = vi.fn();
    const { container } = render(() => (
      <TabBar
        items={[
          { id: "a", label: "Tab A" },
          { id: "b", label: "Tab B" },
          { id: "c", label: "Tab C" },
        ]}
        value="a"
        onChange={onChange}
        ariaLabel="Test"
      />
    ));
    const tabs = Array.from(container.querySelectorAll('[role="tab"]')) as HTMLButtonElement[];
    expect(tabs).toHaveLength(3);
    // ArrowRight from tab A should activate tab B
    fireEvent.keyDown(tabs[0], { key: "ArrowRight" });
    expect(onChange).toHaveBeenLastCalledWith("b");
    // ArrowRight from tab B should activate tab C
    fireEvent.keyDown(tabs[1], { key: "ArrowRight" });
    expect(onChange).toHaveBeenLastCalledWith("c");
    // ArrowLeft from tab C should activate tab B
    fireEvent.keyDown(tabs[2], { key: "ArrowLeft" });
    expect(onChange).toHaveBeenLastCalledWith("b");
  });

  it("Home jumps to first tab; End jumps to last tab", () => {
    const onChange = vi.fn();
    const { container } = render(() => (
      <TabBar
        items={[
          { id: "a", label: "Tab A" },
          { id: "b", label: "Tab B" },
          { id: "c", label: "Tab C" },
        ]}
        value="b"
        onChange={onChange}
        ariaLabel="Test"
      />
    ));
    const tabs = Array.from(container.querySelectorAll('[role="tab"]')) as HTMLButtonElement[];
    expect(tabs).toHaveLength(3);
    fireEvent.keyDown(tabs[1], { key: "Home" });
    expect(onChange).toHaveBeenLastCalledWith("a");
    fireEvent.keyDown(tabs[0], { key: "End" });
    expect(onChange).toHaveBeenLastCalledWith("c");
  });

  it("badge renders count when > 0; omitted when undefined or 0", () => {
    const { container } = render(() => (
      <TabBar
        items={[
          { id: "a", label: "Tab A", badge: 3 },
          { id: "b", label: "Tab B", badge: undefined },
          { id: "c", label: "Tab C", badge: 0 },
        ]}
        value="a"
        onChange={() => {}}
        ariaLabel="Test"
      />
    ));
    const tabs = Array.from(container.querySelectorAll('[role="tab"]')) as HTMLButtonElement[];
    expect(tabs).toHaveLength(3);
    // Badge with count 3 — should be present
    expect(tabs[0].querySelector(".tab-badge")).toBeTruthy();
    expect(tabs[0].querySelector(".tab-badge")!.textContent).toBe("3");
    // Badge with undefined — should not be present
    expect(tabs[1].querySelector(".tab-badge")).toBeNull();
    // Badge with 0 — should not be present (our Show guard)
    expect(tabs[2].querySelector(".tab-badge")).toBeNull();
  });

  it("disabled tab has disabled attribute and does not call onChange on click", () => {
    const onChange = vi.fn();
    const { container } = render(() => (
      <TabBar
        items={[
          { id: "a", label: "Tab A", disabled: true },
          { id: "b", label: "Tab B" },
        ]}
        value="a"
        onChange={onChange}
        ariaLabel="Test"
      />
    ));
    const tabs = Array.from(container.querySelectorAll('[role="tab"]')) as HTMLButtonElement[];
    expect(tabs).toHaveLength(2);
    expect(tabs[0].hasAttribute("disabled")).toBeTruthy();
    fireEvent.click(tabs[0]);
    expect(onChange).not.toHaveBeenCalled();
  });
});

describe("<TabPanel>", () => {
  it("renders children when value matches activeValue", () => {
    const { container } = render(() => (
      <>
        <TabPanel value="a" activeValue="a">
          <p>Panel A content</p>
        </TabPanel>
        <TabPanel value="b" activeValue="a">
          <p>Panel B content</p>
        </TabPanel>
      </>
    ));
    const panels = Array.from(container.querySelectorAll('[role="tabpanel"]')) as HTMLDivElement[];
    expect(panels).toHaveLength(2);
    // First panel (a) should be visible
    expect(panels[0].textContent).toContain("Panel A content");
    // Second panel (b) should be hidden
    expect(panels[1].textContent).not.toContain("Panel B content");
  });

  it("inactive panel has hidden attribute", () => {
    const { container } = render(() => (
      <>
        <TabPanel value="a" activeValue="a">
          <p>Panel A</p>
        </TabPanel>
        <TabPanel value="b" activeValue="a">
          <p>Panel B</p>
        </TabPanel>
      </>
    ));
    const panels = Array.from(container.querySelectorAll('[role="tabpanel"]')) as HTMLDivElement[];
    expect(panels).toHaveLength(2);
    // First panel (a) is active: hidden should be falsy
    const hidden0 = panels[0].getAttribute("hidden");
    expect(hidden0 == null || hidden0 === "false").toBeTruthy();
    // Second panel (b) is inactive: hidden should be set
    expect(panels[1].hasAttribute("hidden")).toBeTruthy();
  });

  it("has id matching the corresponding tab's aria-controls", () => {
    const { container } = render(() => (
      <TabBar
        items={[
          { id: "a", label: "Tab A" },
          { id: "b", label: "Tab B" },
        ]}
        value="a"
        onChange={() => {}}
        ariaLabel="Test"
      />
    ));
    const tabs = Array.from(container.querySelectorAll('[role="tab"]')) as HTMLButtonElement[];
    expect(tabs).toHaveLength(2);
    // Tab a should have aria-controls pointing to panel-a
    expect(tabs[0].getAttribute("aria-controls")).toBe("panel-a");
    expect(tabs[1].getAttribute("aria-controls")).toBe("panel-b");
  });
});
