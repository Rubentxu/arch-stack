// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@solidjs/testing-library";
import { ImpactView } from "../views/ImpactView";
import type { SidebarStats } from "../components/Sidebar";

/**
 * R4 — Impact statistics notification must be a reactive side effect
 * while derived impact calculations stay pure. The observable count,
 * depth, and direction MUST remain unchanged.
 *
 * These are approval tests for the refactor of ImpactView's
 * side-effect `createMemo` into a `createEffect`: they capture the
 * observer contract before the refactor and must still pass after it.
 */

// a → b → c: changing the focus/direction must re-emit matching stats.
const nodes = [
  { id: "a", label: "alpha", kind: "function", file: "src/a.rs", line: 1 },
  { id: "b", label: "beta", kind: "function", file: "src/b.rs", line: 1 },
  { id: "c", label: "gamma", kind: "function", file: "src/c.rs", line: 1 },
];
const edges = [
  { id: "e1", source: "a", target: "b", kind: "SyncCall" },
  { id: "e2", source: "b", target: "c", kind: "SyncCall" },
];

afterEach(cleanup);

describe("ImpactView onStats observer (R4)", () => {
  it("emits statistics matching the rendered impact result on render", async () => {
    const onStats = vi.fn();
    render(() => (
      <ImpactView
        nodes={nodes}
        edges={edges}
        initialFocusId="a"
        onSelect={vi.fn()}
        onStats={onStats}
      />
    ));

    await waitFor(() => expect(onStats).toHaveBeenCalled());
    const last = onStats.mock.calls.at(-1)?.[0] as SidebarStats;
    // a → b → c downstream: 2 impacted functions, depth 2, default "both".
    expect(last.blastRadius).toBe(2);
    expect(last.depth).toBe(2);
    expect(last.direction).toBe("both");
    // The rendered stats text agrees with the emitted values.
    expect(document.querySelector(".impact-stats")?.textContent).toContain("2");
  });

  it("re-emits statistics when the direction control changes", async () => {
    const onStats = vi.fn();
    render(() => (
      <ImpactView
        nodes={nodes}
        edges={edges}
        initialFocusId="a"
        onSelect={vi.fn()}
        onStats={onStats}
      />
    ));
    await waitFor(() => expect(onStats).toHaveBeenCalled());

    const [, directionSelect] = screen.getAllByRole(
      "combobox",
    ) as HTMLSelectElement[];
    fireEvent.change(directionSelect, { target: { value: "upstream" } });

    await waitFor(() => expect(onStats.mock.calls.length).toBeGreaterThan(1));
    const last = onStats.mock.calls.at(-1)?.[0] as SidebarStats;
    // alpha has no upstream callers → empty impact, direction mapped to "callers".
    expect(last.blastRadius).toBe(0);
    expect(last.depth).toBe(0);
    expect(last.direction).toBe("callers");
    expect(document.querySelector(".impact-stats")?.textContent).toContain(
      "upstream",
    );
  });

  it("re-emits statistics when the focus changes", async () => {
    const onStats = vi.fn();
    render(() => (
      <ImpactView
        nodes={nodes}
        edges={edges}
        initialFocusId="a"
        onSelect={vi.fn()}
        onStats={onStats}
      />
    ));
    await waitFor(() => expect(onStats).toHaveBeenCalled());

    const [focusCombo] = screen.getAllByRole("combobox") as HTMLSelectElement[];
    fireEvent.change(focusCombo, { target: { value: "b" } });

    await waitFor(() => expect(onStats.mock.calls.length).toBeGreaterThan(1));
    const last = onStats.mock.calls.at(-1)?.[0] as SidebarStats;
    // Focus beta: upstream alpha + downstream gamma → 2 impacted, depth 1.
    expect(last.blastRadius).toBe(2);
    expect(last.depth).toBe(1);
  });

  it("updates normally without throwing when no observer is provided", async () => {
    render(() => (
      <ImpactView
        nodes={nodes}
        edges={edges}
        initialFocusId="a"
        onSelect={vi.fn()}
      />
    ));
    expect(document.querySelector(".impact-view")).not.toBeNull();

    const [, directionSelect] = screen.getAllByRole(
      "combobox",
    ) as HTMLSelectElement[];
    fireEvent.change(directionSelect, { target: { value: "downstream" } });
    expect(document.querySelector(".impact-stats")?.textContent).toContain(
      "downstream",
    );
    expect(document.querySelector(".impact-view")).not.toBeNull();
  });
});
