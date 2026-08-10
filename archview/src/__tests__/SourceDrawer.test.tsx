// @vitest-environment jsdom
/**
 * SourceDrawer component tests.
 */

import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@solidjs/testing-library";
import { SourceDrawer } from "../components/SourceDrawer";
import type { SourcePreview } from "../lib/workspace";

const PREVIEW: SourcePreview = {
  file: "src/main.rs",
  start_line: 8,
  total_lines: 20,
  content: ["fn main() {", "    do_thing();", "}"],
  truncated: false,
};

describe("<SourceDrawer>", () => {
  it("renders the file:line header", () => {
    const fetchSource = vi.fn(async () => PREVIEW);
    const openInEditor = vi.fn(async () => true);
    const { getByText } = render(() => (
      <SourceDrawer
        file="src/main.rs"
        line={10}
        fetchSource={fetchSource}
        openInEditor={openInEditor}
      />
    ));
    expect(getByText("src/main.rs:10")).toBeDefined();
  });

  it("renders the source lines after the resource resolves", async () => {
    const fetchSource = vi.fn(async () => PREVIEW);
    const openInEditor = vi.fn(async () => true);
    const { findByText, findAllByRole } = render(() => (
      <SourceDrawer
        file="src/main.rs"
        line={9}
        fetchSource={fetchSource}
        openInEditor={openInEditor}
      />
    ));
    // The current line (start_line + idx === line) gets the .current class.
    const line9 = await findByText("fn main() {");
    expect(line9).toBeDefined();
    const items = await findAllByRole("listitem");
    // 3 content lines + no truncation footer = 3 items.
    expect(items.length).toBe(3);
  });

  it("shows truncation notice when truncated=true", async () => {
    const truncated: SourcePreview = { ...PREVIEW, truncated: true };
    const fetchSource = vi.fn(async () => truncated);
    const openInEditor = vi.fn(async () => true);
    const { findByText } = render(() => (
      <SourceDrawer
        file="src/main.rs"
        line={10}
        fetchSource={fetchSource}
        openInEditor={openInEditor}
      />
    ));
    expect(await findByText(/truncated at 20 total lines/)).toBeDefined();
  });

  it("clicking 'Open in editor' calls openInEditor with file+line", async () => {
    const fetchSource = vi.fn(async () => PREVIEW);
    const openInEditor = vi.fn(async () => true);
    const { findByRole } = render(() => (
      <SourceDrawer
        file="src/main.rs"
        line={42}
        fetchSource={fetchSource}
        openInEditor={openInEditor}
      />
    ));
    const btn = await findByRole("button", {
      name: /open .* line 42 in editor/i,
    });
    await fireEvent.click(btn);
    expect(openInEditor).toHaveBeenCalledWith("src/main.rs", 42);
  });

  it("renders error message when fetchSource rejects", async () => {
    const fetchSource = vi.fn(async () => {
      throw new Error("404 file_not_found");
    });
    const openInEditor = vi.fn(async () => true);
    const { findByText } = render(() => (
      <SourceDrawer
        file="missing.rs"
        line={1}
        fetchSource={fetchSource}
        openInEditor={openInEditor}
      />
    ));
    expect(await findByText(/Failed to load source: 404/)).toBeDefined();
  });
});
