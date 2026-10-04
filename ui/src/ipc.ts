// Thin typed wrappers around Tauri commands. Outside Tauri (plain `vite dev`)
// they return mock data so the UI can be developed in a browser.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface SystemCount { system: string; count: number }
export interface DbStats { db_path: string; entries: number; systems: SystemCount[] }
export interface ScanProgress { done: number; total: number }
export interface ScanSummary {
  roms: number;
  discs: number;
  playlists: number;
  failures: number;
  bytes: number;
  millis: number;
}

const inTauri = "__TAURI_INTERNALS__" in window;

export async function dbStats(): Promise<DbStats> {
  if (!inTauri) {
    return {
      db_path: "(browser mock)",
      entries: 412_380,
      systems: [
        { system: "Nintendo - Game Boy Advance", count: 3_102 },
        { system: "Sony - PlayStation", count: 10_544 },
        { system: "Sega - Mega Drive - Genesis", count: 2_701 },
      ],
    };
  }
  return invoke<DbStats>("db_stats");
}

export async function scan(dir: string): Promise<ScanSummary> {
  if (!inTauri) return { roms: 0, discs: 0, playlists: 0, failures: 0, bytes: 0, millis: 0 };
  return invoke<ScanSummary>("scan", { dir });
}

export function onScanProgress(cb: (p: ScanProgress) => void): Promise<UnlistenFn> {
  if (!inTauri) return Promise.resolve(() => {});
  return listen<ScanProgress>("scan://progress", (e) => cb(e.payload));
}
