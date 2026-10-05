// Thin typed wrappers around Tauri commands. Outside Tauri (plain `vite dev`)
// they return mock data so the UI can be developed in a browser.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface SystemCount {
  system: string;
  count: number;
}
export interface DbStats {
  db_path: string;
  entries: number;
  systems: SystemCount[];
}
export interface ScanProgress {
  done: number;
  total: number;
}
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

export interface SyncSummary {
  dir: string;
  imported: number;
  unchanged: number;
  removed: number;
  entries: number;
}

export async function dbSync(dir: string | null): Promise<SyncSummary> {
  if (!inTauri)
    return {
      dir: dir ?? "(mock)",
      imported: 0,
      unchanged: 146,
      removed: 0,
      entries: 0,
    };
  return invoke<SyncSummary>("db_sync", { dir });
}

export async function scan(dir: string): Promise<ScanSummary> {
  if (!inTauri)
    return {
      roms: 0,
      discs: 0,
      playlists: 0,
      failures: 0,
      bytes: 0,
      millis: 0,
    };
  return invoke<ScanSummary>("scan", { dir });
}

export function onScanProgress(
  cb: (p: ScanProgress) => void,
): Promise<UnlistenFn> {
  if (!inTauri) return Promise.resolve(() => {});
  return listen<ScanProgress>("scan://progress", (e) => cb(e.payload));
}

export type Mode = "move" | "copy" | "hardlink" | "reflink";
export interface OpView {
  kind: "move" | "copy" | "link" | "clone" | "extract" | "write";
  from: string | null;
  to: string;
  why: string;
}
export interface DecisionView {
  kind: "ambiguous" | "tie" | "rejected" | "skipped" | "conflict";
  path: string;
  /** rejected: core reason in a few words */
  headline: string;
  /** system the decision is about ("" if unknown) */
  system: string;
  detail: string;
  /** ambiguous: candidates · tie: releases · rejected: the release itself */
  options: Choice[];
  /** rejected only: false for duplicates of the pick */
  can_keep: boolean;
}
export interface Choice {
  system: string;
  name: string;
}
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
export interface ExecResult {
  done: number;
  journal: number | null;
  error: string | null;
}
export interface ImportProgress {
  phase: "library" | "inbox" | "planning";
  done: number;
  total: number;
}

function mockPlan(library: string): PlanView {
  const ops: OpView[] = Array.from({ length: 2000 }, (_, i) => ({
    kind: i % 50 === 0 ? "write" : "move",
    from: i % 50 === 0 ? null : `/inbox/rom_${i}.zip`,
    to: `${library}/Nintendo - Game Boy/Game ${i} (Europe).zip`,
    why: i % 50 === 0 ? "RetroArch playlist" : "1G1R pick",
  }));
  return {
    items: 2100,
    placed: 1960,
    unchanged: 100,
    quarantined: 12,
    discarded: 0,
    ops,
    decisions: [
      {
        kind: "ambiguous",
        path: "/inbox/x.bin",
        headline: "",
        system: "",
        detail: "",
        options: [
          { system: "Sega - Saturn", name: "A" },
          { system: "Sega - Saturn", name: "B" },
        ],
        can_keep: false,
      },
      {
        kind: "rejected",
        path: "/inbox/Tetris (Japan).gb",
        headline: "Kept release has a preferred region",
        system: "Nintendo - Game Boy",
        detail: "Tetris (Japan)  →  kept: Tetris (World)",
        options: [{ system: "Nintendo - Game Boy", name: "Tetris (Japan)" }],
        can_keep: true,
      },
      {
        kind: "conflict",
        path: "/inbox/y.gb",
        headline: "",
        system: "",
        detail: "target exists: /lib/y.gb",
        options: [],
        can_keep: false,
      },
    ],
  };
}

export async function pickDir(title: string): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ directory: true, title });
  return typeof r === "string" ? r : null;
}

export async function planImport(
  inbox: string | null,
  library: string,
  mode: Mode,
): Promise<PlanView> {
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
export async function setVerdict(
  c: Choice,
  verdict: Verdict | null,
): Promise<void> {
  if (!inTauri) return;
  return invoke("set_verdict", { system: c.system, name: c.name, verdict });
}

export function onImportProgress(
  cb: (p: ImportProgress) => void,
): Promise<UnlistenFn> {
  if (!inTauri) return Promise.resolve(() => {});
  return listen<[ImportProgress["phase"], ScanProgress]>(
    "import://progress",
    (e) => cb({ phase: e.payload[0], ...e.payload[1] }),
  );
}

export interface LibraryRow {
  regions: string[];
  /** unix seconds */
  added: number;
  path: string;
  system: string;
  name: string;
  state: "known" | "ambiguous" | "unknown" | "skip";
  files: number;
}

export interface Session {
  library: string | null;
  inbox: string | null;
  mode: Mode | null;
}

/** Last used library, inbox and import mode (stored in the database). */
export async function sessionGet(): Promise<Session> {
  if (!inTauri)
    return { library: "/mock/library", inbox: "/mock/inbox", mode: "move" };
  return invoke<Session>("session_get");
}

export async function libraryList(library?: string): Promise<LibraryRow[]> {
  if (!inTauri) {
    return Array.from({ length: 5000 }, (_, i) => ({
      path: `Nintendo - Game Boy/Game ${i} (Europe).zip`,
      system: i % 7 ? "Nintendo - Game Boy" : "Sega - Mega Drive - Genesis",
      name: `Game ${i} (Europe)`,
      state: i % 97 ? "known" : "unknown",
      files: 1,
      regions: i % 3 ? ["Europe"] : ["USA", "Japan"],
      added: Date.now() / 1000 - i * 3600,
    }));
  }
  return invoke<LibraryRow[]>("library_list", { library });
}

export interface TrashFile {
  path: string;
  bytes: number;
}

export async function trashList(library: string): Promise<TrashFile[]> {
  if (!inTauri)
    return [
      { path: "Tetris (Japan).gb", bytes: 65536 },
      { path: "Mario (Beta).sfc", bytes: 1048576 },
    ];
  return invoke<TrashFile[]>("trash_list", { library });
}

/** Permanently deletes the trash folder; returns the number of deleted files. */
export async function trashEmpty(library: string): Promise<number> {
  if (!inTauri) return 0;
  return invoke<number>("trash_empty", { library });
}

export interface JournalView {
  id: number;
  ts: number;
  library: string;
  state: "done" | "undone";
  ops: number;
}

export async function journalList(): Promise<JournalView[]> {
  if (!inTauri)
    return [
      { id: 2, ts: Date.now() / 1000, library: "/lib", state: "done", ops: 82 },
    ];
  return invoke<JournalView[]>("journal_list");
}

export const FLAG_KEYS = [
  "beta",
  "proto",
  "demo",
  "kiosk",
  "sample",
  "unlicensed",
  "pirate",
  "bios",
  "aftermarket",
  "virtual_console",
  "rerelease",
  "hack",
  "translation",
  "bad_dump",
  "alt",
] as const;
export type FlagKey = (typeof FLAG_KEYS)[number];
export interface Rules {
  regions: string[];
  languages: string[];
  exclude: Record<FlagKey, boolean>;
}

const mockRules = (): Rules => ({
  regions: ["Europe", "World", "USA", "Germany", "Japan"],
  languages: ["En", "De"],
  exclude: Object.fromEntries(
    FLAG_KEYS.map((k) => [
      k,
      !["aftermarket", "virtual_console", "rerelease", "alt"].includes(k),
    ]),
  ) as Rules["exclude"],
});

export async function rulesGet(): Promise<Rules> {
  if (!inTauri) return mockRules();
  return invoke<Rules>("rules_get");
}

/** Saves the rules; `null` resets to defaults. Returns the effective rules. */
export async function rulesSet(rules: Rules | null): Promise<Rules> {
  if (!inTauri) return rules ?? mockRules();
  return invoke<Rules>("rules_set", { rules });
}

export type ThumbKind = "boxart" | "title" | "snap";

/** Object URL of a libretro thumbnail, or null if the server has none. Cached on disk by the backend. */
export async function thumbnail(
  system: string,
  name: string,
  kind: ThumbKind,
): Promise<string | null> {
  if (!inTauri) return null;
  const buf = await invoke<ArrayBuffer>("thumbnail", { system, name, kind });
  return buf.byteLength
    ? URL.createObjectURL(new Blob([buf], { type: "image/png" }))
    : null;
}

/** Whether missing thumbnails are downloaded from libretro (default on). */
export async function thumbsOnlineGet(): Promise<boolean> {
  if (!inTauri) return true;
  return invoke<boolean>("thumbs_online_get");
}

export async function thumbsOnlineSet(on: boolean): Promise<void> {
  if (!inTauri) return;
  return invoke("thumbs_online_set", { on });
}

export interface FranchiseProgress {
  franchise: string;
  owned: number;
  total: number;
}
export interface SystemProgress {
  system: string;
  owned: number;
  total: number;
}
export interface Kpis {
  games: number;
  unknown: number;
  ambiguous: number;
  trashed: number;
  trashed_bytes: number;
  runs: number;
  streak: number;
  systems: SystemProgress[];
  franchises: FranchiseProgress[];
  regions: Record<string, number>;
  genres: Record<string, number>;
  decades: Record<string, number>;
}
export interface Achievement {
  id: string;
  title: string;
  description: string;
  xp: number;
  unlocked: boolean;
}
export interface Level {
  level: number;
  xp: number;
  floor: number;
  next: number;
}
/** Achievements come as `[achievement, unlocked_at | null]`. */
export interface Stats {
  kpis: Kpis;
  level: Level;
  achievements: [Achievement, number | null][];
  new: string[];
}

/** KPIs, completeness and achievements for the identified library games; stores new unlocks. */
export async function gamifyStats(
  games: [string, string][],
  unknown: number,
  ambiguous: number,
): Promise<Stats> {
  if (!inTauri) {
    return {
      kpis: {
        games: games.length,
        unknown,
        ambiguous,
        trashed: 12,
        trashed_bytes: 734_003_200,
        runs: 3,
        streak: 2,
        systems: [
          { system: "Nintendo - Game Boy", owned: 40, total: 520 },
          { system: "Nintendo - Virtual Boy", owned: 22, total: 22 },
        ],
        franchises: [
          { franchise: "Mario", owned: 7, total: 9 },
          { franchise: "Zelda", owned: 2, total: 5 },
        ],
        regions: { Europe: 30, USA: 25, Japan: 7 },
        genres: { Action: 20, Puzzle: 8 },
        decades: { 1990: 50, 2000: 12 },
      },
      level: { level: 3, xp: 1234, floor: 900, next: 1600 },
      achievements: [
        [
          {
            id: "games-1",
            title: "First Blood",
            description: "Own 1 verified games",
            xp: 20,
            unlocked: true,
          },
          Date.now() / 1000,
        ],
        [
          {
            id: "games-100",
            title: "Collector",
            description: "Own 100 verified games",
            xp: 200,
            unlocked: false,
          },
          null,
        ],
        [
          {
            id: "full-set:Nintendo - Virtual Boy",
            title: "Full Set: Nintendo - Virtual Boy",
            description: "Own every game of the 1G1R set (22)",
            xp: 522,
            unlocked: true,
          },
          Date.now() / 1000,
        ],
      ],
      new: [],
    };
  }
  return invoke<Stats>("gamify_stats", { games, unknown, ambiguous });
}

export async function gamifyEnabledGet(): Promise<boolean> {
  if (!inTauri) return true;
  return invoke<boolean>("gamify_enabled_get");
}

export async function gamifyEnabledSet(on: boolean): Promise<void> {
  if (!inTauri) return;
  return invoke("gamify_enabled_set", { on });
}
