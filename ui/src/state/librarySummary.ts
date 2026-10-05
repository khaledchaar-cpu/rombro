// Summary of the last library scan, remembered for the Dashboard (a full scan is too slow to run there).
import { createSignal } from "solid-js";
import type { LibraryRow } from "../ipc";

export interface LibrarySummary {
  ts: number;
  library: string;
  total: number;
  states: Record<string, number>;
  systems: [string, number][];
}

const KEY = "rombro.librarySummary";
const load = (): LibrarySummary | undefined => {
  try { return JSON.parse(localStorage.getItem(KEY) ?? "") as LibrarySummary; } catch { return undefined; }
};

export const [librarySummary, setLibrarySummary] = createSignal<LibrarySummary | undefined>(load());

export function rememberLibrary(library: string, rows: LibraryRow[]) {
  const states: Record<string, number> = {};
  const systems = new Map<string, number>();
  for (const r of rows) {
    states[r.state] = (states[r.state] ?? 0) + 1;
    if (r.system) systems.set(r.system, (systems.get(r.system) ?? 0) + 1);
  }
  const s: LibrarySummary = {
    ts: Date.now(),
    library,
    total: rows.length,
    states,
    systems: [...systems].sort((a, b) => b[1] - a[1]),
  };
  setLibrarySummary(s);
  try { localStorage.setItem(KEY, JSON.stringify(s)); } catch { /* storage unavailable */ }
}
