/**
 * useWorkspaceState — durable workspace state hook (H1, ADR-041).
 *
 * The hook talks to four backend endpoints exposed by `archctl view`:
 *   - GET    /api/workspace          → restore viewport on mount
 *   - PUT    /api/workspace          → debounced save on state change
 *   - GET    /api/source?file&line  → fetch source preview for the drawer
 *   - POST   /api/open-editor       → hand off to $EDITOR / $VISUAL
 *
 * State survives `archctl view` restarts (incl. port changes) because it is
 * persisted to `~/.local/share/archctl/projects/<hash>/workspace.json` (XDG,
 * never in the repo per ADR-038).
 *
 * Debounce: 500 ms after the last change before firing PUT. Bounded to one
 * in-flight PUT at a time; the latest state always wins.
 */

import {
  createEffect,
  createResource,
  createSignal,
  onCleanup,
} from "solid-js";
import type {
  Workspace,
  HttpsArchctlLocalSchemasWorkspaceStateSchemaJson,
} from "./workspace.types";

const DEFAULT_WORKSPACE: Workspace = {
  camera: { x: 0, y: 0 },
  zoom: 50,
  filters: [],
  selection: null,
};

const DEBOUNCE_MS = 500;

type SaveStatus = "idle" | "saving" | "saved" | "error";

export interface SourcePreview {
  file: string;
  start_line: number;
  total_lines: number;
  content: string[];
  truncated: boolean;
}

export interface UseWorkspaceStateResult {
  /** Current viewport (camera + zoom). Reactive: updates on remote PUT. */
  workspace: () => Workspace;
  /** Debounced save status. */
  saveStatus: () => SaveStatus;
  /** Last error from a save attempt. */
  saveError: () => string | null;
  /** Update the local viewport (PUT will fire after debounce). */
  setWorkspace: (next: Workspace) => void;
  /** Read source preview for a file:line. Returns a resource. */
  fetchSource: (file: string, line: number) => Promise<SourcePreview>;
  /** Ask the backend to spawn the user's editor for file:line. */
  openInEditor: (file: string, line: number) => Promise<boolean>;
}

interface FetchLike {
  (input: string, init?: RequestInit): Promise<Response>;
}

/** Test seam: lets the test suite inject a fetch stub. */
let fetchImpl: FetchLike = (...args) => fetch(...args);

export function __setFetchForTests(impl: FetchLike | null): void {
  fetchImpl = impl ?? ((...args) => fetch(...args));
}

// ---- Explain action (ADR-062, /api/explain) ------------------------------

/** Minimal explain subject shape for the action palette. */
export interface ExplainSubjectLite {
  kind: string;
  id: string;
  statement: string;
  versionId?: string;
}

/** Shape of the explain-report/1 carrier returned by `archctl view`. */
export interface ExplainResult {
  schemaVersion: string;
  capability: string;
  subject: ExplainSubjectLite;
  provenance: {
    evidence: Array<Record<string, unknown>>;
    unsubstantiated: boolean;
  };
  fusedClaims?: Array<Record<string, unknown>>;
  warnings: string[];
}

/**
 * Ask the backend to explain the evidence chain backing a graph subject.
 * Only meaningful when `archctl view` runs with a configured project_dir
 * (the receiver of a strict bundle has no store — App hides the action).
 */
export async function explainElement(id: string): Promise<ExplainResult> {
  const params = new URLSearchParams({ id });
  const res = await fetchImpl(`/api/explain?${params.toString()}`);
  if (!res.ok) {
    const detail = await res.text();
    throw new Error(`GET /api/explain → ${res.status}: ${detail}`);
  }
  return (await res.json()) as ExplainResult;
}

export function useWorkspaceState(): UseWorkspaceStateResult {
  const [workspace, setLocal] = createSignal<Workspace>(DEFAULT_WORKSPACE);
  const [saveStatus, setSaveStatus] = createSignal<SaveStatus>("idle");
  const [saveError, setSaveError] = createSignal<string | null>(null);

  // ---- Restore on mount --------------------------------------------------
  const [restored] = createResource(async () => {
    try {
      const res = await fetchImpl("/api/workspace");
      if (!res.ok) {
        // 404 (no workspace yet) is normal; any other status is real but
        // we still degrade to defaults rather than crash the UI.
        return null;
      }
      const body = (await res.json()) as {
        workspace:
          HttpsArchctlLocalSchemasWorkspaceStateSchemaJson["workspace"] | null;
        version: string;
      };
      return body.workspace;
    } catch {
      // Network error / endpoint unavailable — same: defaults win.
      return null;
    }
  });

  createEffect(() => {
    const r = restored();
    if (r) {
      setLocal(r);
    }
  });

  // ---- Debounced PUT -----------------------------------------------------
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let inflight: AbortController | null = null;
  let pending: Workspace | null = null;

  const fire = async (state: Workspace) => {
    inflight?.abort();
    const ctrl = new AbortController();
    inflight = ctrl;
    setSaveStatus("saving");
    setSaveError(null);
    try {
      const payload: HttpsArchctlLocalSchemasWorkspaceStateSchemaJson = {
        version: "1.0",
        project_hash: "0".repeat(64), // backend recomputes; placeholder satisfies schema
        workspace: state,
        updated_at: new Date().toISOString(),
      };
      const res = await fetchImpl("/api/workspace", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
        signal: ctrl.signal,
      });
      if (!res.ok) {
        const detail = await res.text();
        throw new Error(`PUT /api/workspace → ${res.status}: ${detail}`);
      }
      setSaveStatus("saved");
    } catch (e) {
      if ((e as { name?: string }).name === "AbortError") return;
      setSaveStatus("error");
      setSaveError(e instanceof Error ? e.message : String(e));
    } finally {
      if (inflight === ctrl) inflight = null;
    }
  };

  const flush = () => {
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
    if (pending) {
      const p = pending;
      pending = null;
      void fire(p);
    }
  };

  const setWorkspace = (next: Workspace) => {
    setLocal(next);
    pending = next;
    setSaveStatus("idle");
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(flush, DEBOUNCE_MS);
  };

  // Flush on unmount so we don't lose pending edits when the user navigates.
  onCleanup(() => {
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
    flush();
  });

  // ---- Source preview ----------------------------------------------------
  const fetchSource = async (
    file: string,
    line: number,
  ): Promise<SourcePreview> => {
    const params = new URLSearchParams({ file, line: String(line) });
    const res = await fetchImpl(`/api/source?${params.toString()}`);
    if (!res.ok) {
      throw new Error(`GET /api/source → ${res.status}`);
    }
    return (await res.json()) as SourcePreview;
  };

  // ---- Editor handoff ----------------------------------------------------
  const openInEditor = async (file: string, line: number): Promise<boolean> => {
    const res = await fetchImpl("/api/open-editor", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ file, line }),
    });
    // 204 = spawned; 503 = no editor configured; anything else = error.
    return res.status === 204;
  };

  return {
    workspace,
    saveStatus,
    saveError,
    setWorkspace,
    fetchSource,
    openInEditor,
  };
}
