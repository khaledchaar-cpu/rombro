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

export type Mode = "move" | "copy" | "hardlink";
export interface OpView { kind: "move" | "copy" | "link" | "write"; from: string | null; to: string }
export interface DecisionView {
  kind: "ambiguous" | "tie" | "rejected" | "skipped" | "conflict";
  path: string;
  detail: string;
  /** ambiguous: candidates · tie: releases · rejected: the release itself */
  options: Choice[];
  /** rejected only: false for duplicates of the pick */
  can_keep: boolean;
}
export interface Choice { system: string; name: string }
export type Verdict = "keep" | "discard" | "prefer";
export interface PlanView {
  items: number;
  placed: number;
  unchanged: number;
  quarantined: number;
  discarded: number;
  ops: OpView[];
  decisions: DecisionView[];
}
export interface ExecResult { done: number; journal: number | null; error: string | null }
export interface ImportProgress { phase: "library" | "inbox"; done: number; total: number }

function mockPlan(library: string): PlanView {
  const ops: OpView[] = Array.from({ length: 2000 }, (_, i) => ({
    kind: i % 50 === 0 ? "write" : "move",
    from: i % 50 === 0 ? null : `/inbox/rom_${i}.zip`,
    to: `${library}/Nintendo - Game Boy/Game ${i} (Europe).zip`,
  }));
  return {
    items: 2100, placed: 1960, unchanged: 100, quarantined: 12, discarded: 0, ops,
    decisions: [
      { kind: "ambiguous", path: "/inbox/x.bin", detail: "", options: [{ system: "Sega - Saturn", name: "A" }, { system: "Sega - Saturn", name: "B" }], can_keep: false },
      { kind: "rejected", path: "/inbox/Tetris (Japan).gb", detail: "Tetris (Japan): region; kept Tetris (World)", options: [{ system: "Nintendo - Game Boy", name: "Tetris (Japan)" }], can_keep: true },
      { kind: "conflict", path: "/inbox/y.gb", detail: "target exists: /lib/y.gb", options: [], can_keep: false },
    ],
  };
}

export async function pickDir(title: string): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ directory: true, title });
  return typeof r === "string" ? r : null;
}

export async function planImport(inbox: string | null, library: string, mode: Mode): Promise<PlanView> {
  if (!inTauri) return mockPlan(library);
  return invoke<PlanView>("plan_import", { inbox, library, mode });
}

export async function executePlan(): Promise<ExecResult> {
  if (!inTauri) return { done: 0, journal: null, error: null };
  return invoke<ExecResult>("execute_plan");
}

export async function undoLast(): Promise<number> {
  if (!inTauri) return 0;
  return invoke<number>("undo_last");
}

export async function resolveAmbiguous(path: string, c: Choice): Promise<void> {
  if (!inTauri) return;
  return invoke("resolve_ambiguous", { path, system: c.system, name: c.name });
}

/** `null` clears a stored verdict. */
export async function setVerdict(c: Choice, verdict: Verdict | null): Promise<void> {
  if (!inTauri) return;
  return invoke("set_verdict", { system: c.system, name: c.name, verdict });
}

export function onImportProgress(cb: (p: ImportProgress) => void): Promise<UnlistenFn> {
  if (!inTauri) return Promise.resolve(() => {});
  return listen<[ImportProgress["phase"], ScanProgress]>("import://progress", (e) =>
    cb({ phase: e.payload[0], ...e.payload[1] }),
  );
}

export interface LibraryRow {
  path: string;
  system: string;
  name: string;
  state: "known" | "ambiguous" | "unknown" | "skip";
  files: number;
}

export async function libraryList(library: string): Promise<LibraryRow[]> {
  if (!inTauri) {
    return Array.from({ length: 5000 }, (_, i) => ({
      path: `Nintendo - Game Boy/Game ${i} (Europe).zip`,
      system: i % 7 ? "Nintendo - Game Boy" : "Sega - Mega Drive - Genesis",
      name: `Game ${i} (Europe)`,
      state: i % 97 ? "known" : "unknown",
      files: 1,
    }));
  }
  return invoke<LibraryRow[]>("library_list", { library });
}

export interface TrashFile { path: string; bytes: number }

export async function trashList(library: string): Promise<TrashFile[]> {
  if (!inTauri) return [{ path: "Tetris (Japan).gb", bytes: 65536 }, { path: "Mario (Beta).sfc", bytes: 1048576 }];
  return invoke<TrashFile[]>("trash_list", { library });
}

/** Permanently deletes the trash folder; returns the number of deleted files. */
export async function trashEmpty(library: string): Promise<number> {
  if (!inTauri) return 0;
  return invoke<number>("trash_empty", { library });
}

export interface JournalView { id: number; ts: number; library: string; state: "done" | "undone"; ops: number }

export async function journalList(): Promise<JournalView[]> {
  if (!inTauri) return [{ id: 2, ts: Date.now() / 1000, library: "/lib", state: "done", ops: 82 }];
  return invoke<JournalView[]>("journal_list");
}
