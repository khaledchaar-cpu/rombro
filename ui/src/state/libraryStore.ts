// Library rows from the persistent file index; loaded at app start and refreshed after execute/undo.
import { createMemo, createRoot, createSignal } from "solid-js";
import { libraryList, sessionGet, type LibraryRow, type Mode } from "../ipc";

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

/** Loads the library from its snapshot (rebuilt only after changes); `rescan` checks
 * every file on disk, for changes made outside RomBro. */
export async function refreshLibrary(rescan = false) {
  if (libraryBusy() || !library()) return;
  setLibraryBusy(true);
  setLibraryError("");
  try {
    setLibraryRows(await libraryList(library(), rescan));
    setLoadedAt(Date.now());
  } catch (e) {
    setLibraryError(String(e));
  } finally {
    setLibraryBusy(false);
  }
}

export const [inbox, setInbox] = createSignal("");
export const [mode, setMode] = createSignal<Mode>("move");

/** Restores the last session (library, inbox, mode) and loads the library. */
export async function initLibrary() {
  let session;
  try {
    session = await sessionGet();
  } catch (e) {
    // never fall back to a stale path: the next load would store it as the library
    setLibraryError(`Could not read the last session: ${e}`);
    return;
  }
  if (session.inbox && !inbox()) setInbox(session.inbox);
  // sessions from older versions may hold a removed mode (hardlink, reflink)
  if (session.mode === "move" || session.mode === "copy") setMode(session.mode);
  let stored = session.library ?? null;
  try {
    // one-time migration: the path used to live in localStorage
    if (!stored) stored = localStorage.getItem("rombro.library");
    localStorage.removeItem("rombro.library");
  } catch {
    /* storage unavailable */
  }
  if (stored && !library()) setLibrary(stored);
  await refreshLibrary();
}

export const librarySummary = createRoot(() =>
  createMemo<LibrarySummary | undefined>(() => {
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
  }),
);
