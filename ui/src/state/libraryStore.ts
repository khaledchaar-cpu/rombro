// Library rows from the persistent file index; loaded at app start and refreshed after execute/undo.
import { createMemo, createSignal } from "solid-js";
import { libraryGet, libraryList, type LibraryRow } from "../ipc";

export interface LibrarySummary {
  ts: number;
  total: number;
  states: Record<string, number>;
  systems: [string, number][];
}

export const [library, setLibrary] = createSignal("");
export const [libraryRows, setLibraryRows] = createSignal<LibraryRow[]>([]);
export const [libraryBusy, setLibraryBusy] = createSignal(false);
export const [libraryError, setLibraryError] = createSignal("");
const [loadedAt, setLoadedAt] = createSignal(0);

/** Rescans the library incrementally (only new or changed files are hashed). */
export async function refreshLibrary() {
  if (libraryBusy() || !library()) return;
  setLibraryBusy(true);
  setLibraryError("");
  try {
    setLibraryRows(await libraryList(library()));
    setLoadedAt(Date.now());
  } catch (e) {
    setLibraryError(String(e));
  } finally {
    setLibraryBusy(false);
  }
}

/** Restores the stored library path and loads it. */
export async function initLibrary() {
  let stored = await libraryGet().catch(() => null);
  if (!stored) {
    // one-time migration: the path used to live in localStorage
    try { stored = localStorage.getItem("rombro.library"); } catch { /* storage unavailable */ }
  }
  if (stored && !library()) setLibrary(stored);
  await refreshLibrary();
}

export const librarySummary = createMemo<LibrarySummary | undefined>(() => {
  const rows = libraryRows();
  if (!loadedAt()) return undefined;
  const states: Record<string, number> = {};
  const systems = new Map<string, number>();
  for (const r of rows) {
    states[r.state] = (states[r.state] ?? 0) + 1;
    if (r.system) systems.set(r.system, (systems.get(r.system) ?? 0) + 1);
  }
  return {
    ts: loadedAt(),
    total: rows.length,
    states,
    systems: [...systems].sort((a, b) => b[1] - a[1]),
  };
});
