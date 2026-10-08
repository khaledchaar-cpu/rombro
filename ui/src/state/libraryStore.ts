// Library rows from the persistent file index; loaded at app start and refreshed after execute/undo.
import { createMemo, createRoot, createSignal } from "solid-js";
import { cheevosProgress, libraryList, onPlayEnded, sessionGet, setFavorite, type LibraryRow, type Mode } from "../ipc";

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

/** Replaces the row of `path` with `f(row)` (local update after favorite/play). */
function patchRow(path: string, f: (r: LibraryRow) => LibraryRow) {
  setLibraryRows((rows) => rows.map((r) => (r.path === path ? f(r) : r)));
}

export async function toggleFavorite(row: LibraryRow) {
  const on = !row.favorite;
  await setFavorite(row.path, on);
  patchRow(row.path, (r) => ({ ...r, favorite: on }));
}

/** Minimum counted run, as in the backend (`MIN_PLAY_SECS`). */
const MIN_PLAY_SECS = 30;
void onPlayEnded((path, secs) => {
  if (secs < MIN_PLAY_SECS) return;
  patchRow(path, (r) => ({ ...r, plays: r.plays + 1, seconds: r.seconds + secs, last_played: Date.now() / 1000 }));
  const game = libraryRows().find((r) => r.path === path);
  if (game?.cheevos) void refreshProgress();
});

/** Fetches the user's RetroAchievements progress and patches the rows of games with achievements. */
export async function refreshProgress() {
  let progress;
  try {
    progress = await cheevosProgress();
  } catch {
    return; // offline: keep the last known progress
  }
  setLibraryRows((all) =>
    all.map((r) => {
      const p = r.cheevos && r.cheevos_game != null ? (progress[r.cheevos_game] ?? null) : null;
      return p || r.cheevos_progress ? { ...r, cheevos_progress: p } : r;
    }),
  );
}

/** Library cell: unlocked/total when logged in, else the count; ◌ = another version has some. */
export function cheevosCell(r: LibraryRow): string {
  const p = r.cheevos_progress;
  if (p?.award === "mastered" || p?.award === "completed") return `★ ${p.total}`;
  if (p?.awarded) return `${p.awarded}/${p.total}`;
  return r.cheevos ? String(r.cheevos) : r.cheevos_other ? "◌" : "";
}

export function cheevosTitle(r: LibraryRow): string {
  const p = r.cheevos_progress;
  if (p) return `${p.awarded} of ${p.total} achievements unlocked${p.hardcore ? ` (${p.hardcore} hardcore)` : ""}${p.award ? ` – ${p.award}` : ""}`;
  if (r.cheevos) return `${r.cheevos} achievements`;
  return r.cheevos_other ? `Another version has achievements: ${r.cheevos_other}` : "";
}

/** Play time as `1 h 05 min` / `12 min`. */
export function formatPlayTime(secs: number): string {
  const min = Math.round(secs / 60);
  return min >= 60 ? `${Math.floor(min / 60)} h ${String(min % 60).padStart(2, "0")} min` : `${min} min`;
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
