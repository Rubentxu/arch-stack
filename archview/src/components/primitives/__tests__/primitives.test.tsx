// @vitest-environment jsdom
/**
 * M17.B design-system primitives.
 *
 * The three primitives (Button, EmptyState, Tag) are the
 * foundation of the visual language going forward. Their
 * behaviour contracts are:
 *   - Button: forwards every prop to the underlying <button>,
 *     defaults type="button", and applies the right class for
 *     variant + size.
 *   - EmptyState: renders the title, body, and optional icon +
 *     action. Skips empty sections.
 *   - Tag: applies the right tone class.
 *
 * Visual styles are tested via screenshots, not unit tests.
 */
import { describe, expect, it } from "vitest";
import { render, screen } from "@solidjs/testing-library";
import { Button, EmptyState, Tag } from "../index";

describe("Button primitive", () => {
  it("renders a <button> with the right variant + size class", () => {
    render(() => (
      <Button variant="primary" size="md">
        Save
      </Button>
    ));
    const btn = screen.getByRole("button", { name: "Save" });
    expect(btn.tagName).toBe("BUTTON");
    expect(btn.className).toContain("btn-primary");
    expect(btn.className).toContain("btn-md");
    // type="button" is the default; this prevents accidental form
    // submits when a button lives inside a future <form>.
    expect(btn.getAttribute("type")).toBe("button");
  });

  it("defaults to variant=secondary and size=md", () => {
    render(() => <Button>Default</Button>);
    const btn = screen.getByRole("button", { name: "Default" });
    expect(btn.className).toContain("btn-secondary");
    expect(btn.className).toContain("btn-md");
  });

  it("respects disabled", () => {
    render(() => <Button disabled>Off</Button>);
    const btn = screen.getByRole("button", { name: "Off" });
    expect((btn as HTMLButtonElement).disabled).toBe(true);
  });
});

describe("EmptyState primitive", () => {
  it("renders title and body when provided", () => {
    render(() => (
      <EmptyState
        title="Nothing here yet"
        body="Pick a sample from the top bar to start exploring."
      />
    ));
    expect(
      screen.getByRole("heading", { name: "Nothing here yet" }),
    ).toBeTruthy();
    expect(screen.getByText(/Pick a sample from the top bar/)).toBeTruthy();
  });

  it("renders an icon when provided", () => {
    render(() => (
      <EmptyState title="With icon" icon={<svg data-testid="icon" />} />
    ));
    expect(screen.getByTestId("icon")).toBeTruthy();
  });
});

describe("Tag primitive", () => {
  it("applies the tone class", () => {
    render(() => <Tag tone="context">Context</Tag>);
    const tag = screen.getByText("Context");
    expect(tag.className).toContain("tag-context");
  });

  it("defaults to tone=default", () => {
    render(() => <Tag>plain</Tag>);
    expect(screen.getByText("plain").className).toContain("tag-default");
  });
});
