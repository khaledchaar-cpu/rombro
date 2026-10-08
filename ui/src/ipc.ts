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
  /** Bytes hashed so far (scan phases only). */
  bytes?: number;
  bytes_total?: number;
  /** Large file being hashed right now. */
  item?: string;
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

// Browser mock: an in-page event bus so progress UIs can be checked without the backend.
const mockBus = new Map<string, Set<(p: unknown) => void>>();
function mockListen<T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  const set = mockBus.get(event) ?? new Set();
  mockBus.set(event, set);
  const f = cb as (p: unknown) => void;
  set.add(f);
  return Promise.resolve(() => set.delete(f));
}
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
/** Emits `steps` progress events per phase over ~1.5 s each. */
async function mockProgress(
  event: string,
  phases: string[],
  payload: (phase: string, done: number, total: number, item: string) => unknown,
): Promise<void> {
  const total = 40;
  for (const phase of phases) {
    for (let done = 0; done <= total; done++) {
      const item = `/mnt/pool/Nostalgia/Romburak inbox/${phase}/A Very Long Folder Name For Testing/Game ${done} (Europe) (En,Fr,De,Es,It) (Rev 1).chd`;
      mockBus.get(event)?.forEach((cb) => cb(payload(phase, done, total, item)));
      await sleep(40);
    }
  }
}
const phased = (phase: string, done: number, total: number, item: string) => [phase, { done, total }, item];

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
  dats_updated: number;
  dat_warnings: string[];
}

export async function dbSync(dir: string | null): Promise<SyncSummary> {
  if (!inTauri) {
    await mockProgress("sync://progress", ["rdb", "dat"], phased);
    return {
      dir: dir ?? "(mock)",
      imported: 0,
      unchanged: 146,
      removed: 0,
      entries: 0,
      dats_updated: 0,
      dat_warnings: [],
    };
  }
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
  if (!inTauri) return mockListen("scan://progress", cb);
  return listen<ScanProgress>("scan://progress", (e) => cb(e.payload));
}

export type Mode = "move" | "copy";
export interface OpView {
  kind: "move" | "copy" | "link" | "clone" | "extract" | "write";
  from: string | null;
  to: string;
  rule: string;
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
  /** Inbox files no operation touches (path relative to the inbox). */
  leftovers: { path: string; size: number }[];
}
export interface ExecResult {
  done: number;
  journal: number | null;
  error: string | null;
}
export interface ImportProgress extends ScanProgress {
  phase: "library" | "inbox" | "planning";
}

function mockPlan(library: string): PlanView {
  const ops: OpView[] = Array.from({ length: 2000 }, (_, i) => ({
    kind: i % 50 === 0 ? "write" : "move",
    from: i % 50 === 0 ? null : `/inbox/rom_${i}.zip`,
    to: `${library}/Nintendo - Game Boy/Game ${i} (Europe).zip`,
    rule: i % 50 === 0 ? "playlist" : "g1r-pick",
    why: i % 50 === 0 ? "Old RetroArch playlist" : "1G1R pick",
  }));
  return {
    items: 2100,
    placed: 1960,
    unchanged: 100,
    quarantined: 12,
    discarded: 0,
    ops,
    leftovers: [
      { path: "mrboom/MrBoom.libretro", size: 25 },
      { path: "mrboom/gamelist.xml", size: 1005 },
      { path: "mrboom/images/MrBoom-image.png", size: 696200 },
      { path: "sdlpop/PrinceOfPersia.sdlpop", size: 0 },
    ],
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
  if (!inTauri) {
    await mockProgress("import://progress", ["library", "inbox", "planning"], (phase, done, total, item) => [
      phase,
      { done, total, bytes: done * 1e8, bytes_total: total * 1e8, item },
    ]);
    return mockPlan(library);
  }
  return invoke<PlanView>("plan_import", { inbox, library, mode });
}

export async function executePlan(): Promise<ExecResult> {
  if (!inTauri) {
    await mockProgress("execute://progress", ["execute"], phased);
    return { done: 40, journal: 1, error: null };
  }
  return invoke<ExecResult>("execute_plan");
}

/** Moves the last plan's inbox leftovers to the library trash (undoable). */
export async function inboxClear(): Promise<ExecResult> {
  if (!inTauri) return { done: 0, journal: null, error: null };
  return invoke<ExecResult>("inbox_clear");
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
  reason = "",
): Promise<void> {
  if (!inTauri) return;
  return invoke("set_verdict", { system: c.system, name: c.name, verdict, reason });
}

export interface PhaseProgress {
  phase: string;
  done: number;
  total: number;
  /** Current item (e.g. the core being downloaded), if the event names one. */
  item?: string;
}

/** Phased progress events: `sync://progress` (rdb, dat), `retroarch://progress` (scan, download, write). */
export function onPhaseProgress(event: string, cb: (p: PhaseProgress) => void): Promise<UnlistenFn> {
  if (!inTauri)
    return mockListen<[string, ScanProgress, string?]>(event, (p) => cb({ phase: p[0], ...p[1], item: p[2] }));
  return listen<[string, ScanProgress, string?]>(event, (e) =>
    cb({ phase: e.payload[0], ...e.payload[1], item: e.payload[2] }),
  );
}

export function onImportProgress(
  cb: (p: ImportProgress) => void,
): Promise<UnlistenFn> {
  if (!inTauri)
    return mockListen<[ImportProgress["phase"], ScanProgress]>("import://progress", (p) => cb({ phase: p[0], ...p[1] }));
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
  state: "known" | "named" | "ambiguous" | "unknown" | "skip";
  files: number;
  favorite: boolean;
  /** Counted runs (≥ 30 s), total play time in seconds, last run (unix seconds, 0 = never). */
  plays: number;
  seconds: number;
  last_played: number;
  /** RetroAchievements of this exact file (0 = none, or game lists not synced). */
  cheevos: number;
  /** RA title of another version with achievements, when this one has none. */
  cheevos_other: string | null;
  /** RA game id (this file's, else the other version's). */
  cheevos_game: number | null;
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

/** The library from its stored snapshot; `rescan` checks every file on disk (maintenance). */
export async function libraryList(library?: string, rescan = false): Promise<LibraryRow[]> {
  if (!inTauri) {
    return Array.from({ length: 5000 }, (_, i) => ({
      path: `Nintendo - Game Boy/Game ${i} (Europe).zip`,
      system: i % 7 ? "Nintendo - Game Boy" : "Sega - Mega Drive - Genesis",
      name: `Game ${i} (Europe)`,
      state: i % 97 ? "known" : "unknown",
      files: 1,
      regions: i % 3 ? ["Europe"] : ["USA", "Japan"],
      added: Date.now() / 1000 - i * 3600,
      favorite: i % 41 === 0,
      plays: i % 13 ? 0 : 3,
      seconds: i % 13 ? 0 : 5400 + i,
      last_played: i % 13 ? 0 : Date.now() / 1000 - i * 600,
      cheevos: i % 3 ? 0 : 20 + (i % 40),
      cheevos_other: i % 3 === 1 && i % 5 === 0 ? `Game ${i}` : null,
      cheevos_game: i % 3 === 0 || i % 5 === 0 ? i : null,
    }));
  }
  return invoke<LibraryRow[]>("library_list", { library, rescan });
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
  arcade_order: string[];
  arcade_g1r: boolean;
  /** Sets a core marks as not working count as incomplete for it. */
  arcade_working_only: boolean;
  folder_systems: string[];
  /** Folder name → system for files identified only by where they lie (rule `name-only`). */
  name_folders: Record<string, string>;
  quarantine: boolean;
  /** Unknown files go to _trash/unknown instead of _quarantine. */
  unknown_to_trash: boolean;
  /** Move frontend metadata (gamelist.xml, scraped media) to _trash/frontend. */
  frontend_trash: boolean;
  systems: Record<string, SystemRules>;
  /** RetroArch core per system (core id); unlisted systems use the recommendation. */
  cores: Record<string, string>;
}

export interface SystemRules {
  regions: string[] | null;
  languages: string[] | null;
  exclude: Record<FlagKey, boolean> | null;
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
  arcade_order: ["FBNeo - Arcade Games", "MAME", "MAME 2016", "MAME 2015", "MAME 2010", "MAME 2003-Plus", "MAME 2003", "MAME 2000", "HBMAME"],
  arcade_g1r: true,
  arcade_working_only: true,
  folder_systems: ["DOS", "ScummVM", "DOOM", "Quake"],
  name_folders: { solarus: "Solarus", openbor: "OpenBOR" },
  quarantine: true,
  unknown_to_trash: true,
  frontend_trash: true,
  systems: {},
  cores: {},
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

export interface CoreOption {
  id: string;
  name: string;
  installed: boolean;
  recommended: boolean;
}
export interface SystemCores {
  system: string;
  /** Core games of the system start with; null = none known. */
  chosen: string | null;
  /** Chosen by the user (stored in the rules) rather than recommended. */
  picked: boolean;
  options: CoreOption[];
}

/** The cores to choose from for each system of the library. */
export async function retroarchCores(): Promise<SystemCores[]> {
  if (!inTauri) {
    const o = (id: string, installed: boolean, recommended = false) => ({ id, name: id, installed, recommended });
    return [
      {
        system: "Nintendo - Super Nintendo Entertainment System",
        chosen: "snes9x",
        picked: false,
        options: [o("snes9x", false, true), o("bsnes", true), o("mesen-s", false)],
      },
      {
        system: "Sony - PlayStation",
        chosen: "swanstation",
        picked: true,
        options: [o("pcsx_rearmed", false, true), o("swanstation", true), o("mednafen_psx_hw", false)],
      },
    ];
  }
  return invoke<SystemCores[]>("retroarch_cores");
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

export interface RuleInfo {
  id: string;
  title: string;
  explain: string;
  hits: number;
}

export async function rulesCatalog(): Promise<RuleInfo[]> {
  if (!inTauri)
    return [
      { id: "g1r-pick", title: "1G1R pick", explain: "One release per game: region order, language, fewest variant flags.", hits: 1234 },
      { id: "arcade-set", title: "Arcade romset", explain: "Arcade zips matched as a whole, grouped by title.", hits: 87 },
      { id: "arcade-dat", title: "Arcade DAT check", explain: "Uncertain arcade matches are checked against each core's DAT.", hits: 3 },
      { id: "game-folder", title: "Game folder", explain: "DOS/ScummVM/ports moved as whole folders.", hits: 12 },
      { id: "name-only", title: "Name folder", explain: "Files without hash match whose folder names a system (e.g. solarus, openbor).", hits: 4 },
      { id: "quarantine", title: "Quarantine", explain: "Files without database match go to _quarantine.", hits: 5270 },
      { id: "playlist", title: "Old RetroArch playlist", explain: "Old .lpl files go to the trash.", hits: 30 },
    ];
  return invoke<RuleInfo[]>("rules_catalog");
}

export async function rulesDefaults(): Promise<Rules> {
  if (!inTauri) return mockRules();
  return invoke<Rules>("rules_defaults");
}

export interface Exceptions {
  verdicts: {
    system: string;
    name: string;
    verdict: "keep" | "discard" | "prefer";
    /** Why the release was up for decision; empty for old verdicts. */
    reason: string;
    /** Unix seconds, null if unknown. */
    decided: number | null;
  }[];
  resolutions: { sha1: string; system: string; name: string }[];
  ignored: string[];
}

export async function exceptionsGet(): Promise<Exceptions> {
  if (!inTauri)
    return {
      verdicts: [{ system: "Nintendo - SNES", name: "Mario (USA)", verdict: "keep", reason: "Kept release has a preferred region · kept: Mario (Europe)", decided: 1791244800 }],
      resolutions: [{ sha1: "00ab", system: "Sony - PlayStation", name: "Final Fantasy VII (Europe) (Disc 1)" }],
      ignored: ["/roms/frontend-media"],
    };
  return invoke<Exceptions>("exceptions_get");
}

export async function ignoreSet(paths: string[]): Promise<void> {
  if (inTauri) await invoke("ignore_set", { paths });
}

export async function resolutionClear(sha1: string): Promise<void> {
  if (inTauri) await invoke("resolution_clear", { sha1 });
}

export interface ManagedRetroArch {
  supported: boolean;
  folder: string | null;
  installed: string | null;
  pinned: string;
  latest: string | null;
}

/** State of the RetroArch rombro manages; `checkLatest` asks the buildbot for the newest stable. */
export async function raStatus(checkLatest: boolean): Promise<ManagedRetroArch> {
  if (!inTauri) {
    return {
      supported: true,
      folder: "~/.local/share/rombro/retroarch",
      installed: mockRa,
      pinned: "1.22.2",
      latest: checkLatest ? "1.22.3" : null,
    };
  }
  return invoke<ManagedRetroArch>("ra_status", { checkLatest });
}
let mockRa: string | null = null;

/** Downloads, verifies and installs RetroArch (`version` null = pinned); progress on `ra://progress`. */
export async function raInstall(version: string | null): Promise<string> {
  if (!inTauri) {
    await mockProgress("ra://progress", ["download", "verify", "unpack"], phased);
    mockRa = version ?? "1.22.2";
    return mockRa;
  }
  return invoke<string>("ra_install", { version });
}

/** Display mode of the managed RetroArch: `fullscreen`, `window:<1-6>`, null = RetroArch's own. */
export async function raDisplay(): Promise<string | null> {
  if (!inTauri) return mockDisplay;
  return invoke<string | null>("ra_display");
}
let mockDisplay: string | null = null;

export async function raSetDisplay(mode: string | null): Promise<void> {
  if (!inTauri) {
    mockDisplay = mode;
    return;
  }
  return invoke<void>("ra_set_display", { mode });
}

export interface GameCores {
  system: string | null;
  chosen: string | null;
  /** Chosen for this game only. */
  overridden: boolean;
  options: CoreOption[];
}

/** Cores that run a library game (`path` as listed, relative to the library). */
export async function gameCores(path: string): Promise<GameCores> {
  if (!inTauri) {
    const chosen = mockGameCore.get(path) ?? "snes9x";
    return {
      system: "Nintendo - Super Nintendo Entertainment System",
      chosen,
      overridden: mockGameCore.has(path),
      options: [
        { id: "snes9x", name: "Snes9x", installed: true, recommended: true },
        { id: "bsnes", name: "bsnes", installed: false, recommended: false },
      ],
    };
  }
  return invoke<GameCores>("game_cores", { path });
}
const mockGameCore = new Map<string, string>();

/** Sets (or with null clears) the core of one game. */
export async function setGameCore(path: string, core: string | null): Promise<void> {
  if (!inTauri) {
    if (core) mockGameCore.set(path, core);
    else mockGameCore.delete(path);
    return;
  }
  return invoke<void>("set_game_core", { path, core });
}

/** Marks or unmarks a library game as favorite. */
export async function setFavorite(path: string, on: boolean): Promise<void> {
  if (!inTauri) return;
  return invoke<void>("set_favorite", { path, on });
}

/** RetroArch closed: game path and run time in seconds (counted when ≥ 30 s). */
export function onPlayEnded(cb: (path: string, secs: number) => void): Promise<UnlistenFn> {
  if (!inTauri) return mockListen<[string, number]>("play://ended", (p) => cb(p[0], p[1]));
  return listen<[string, number]>("play://ended", (e) => cb(e.payload[0], e.payload[1]));
}

/** Installs what is missing (RetroArch, core) and starts the game; progress on `play://progress`. */
export async function play(path: string): Promise<string> {
  if (!inTauri) {
    if (!mockRa) await mockProgress("play://progress", ["download", "verify", "unpack"], phased);
    mockRa = mockRa ?? "1.22.2";
    await mockProgress("play://progress", ["core"], (p, d, t) => [p, { done: d, total: t }, "snes9x"]);
    setTimeout(() => mockBus.get("play://ended")?.forEach((cb) => cb([path, 95])), 3000);
    return mockGameCore.get(path) ?? "snes9x";
  }
  return invoke<string>("play", { path });
}

export interface CheevosStatus {
  has_key: boolean;
  /** unix seconds of the last game list sync */
  synced: number | null;
}

/** Whether a RetroAchievements Web API key is stored and when the game lists were synced. */
export async function cheevosStatus(): Promise<CheevosStatus> {
  if (!inTauri) return { has_key: true, synced: Date.now() / 1000 - 3600 };
  return invoke<CheevosStatus>("cheevos_status");
}

export async function cheevosSetKey(key: string): Promise<void> {
  if (!inTauri) return;
  return invoke<void>("cheevos_set_key", { key });
}

/** Downloads RA game lists, then hashes the library (`cheevos://progress`, phases sync/hash).
 * Returns the number of library games with achievements. */
export async function cheevosSync(): Promise<number> {
  if (!inTauri) {
    await mockProgress("cheevos://progress", ["sync", "hash"], phased);
    return 2685;
  }
  return invoke<number>("cheevos_sync");
}

export interface RaAchievement {
  id: number;
  title: string;
  description: string;
  points: number;
  /** badge image id: https://media.retroachievements.org/Badge/<badge>.png */
  badge: string;
  /** progression, win_condition, missable or "" */
  kind: string;
  /** share of players who unlocked it (0–1) */
  rarity: number;
}

/** Achievements of a RetroAchievements game (one Web API request). */
export async function cheevosAchievements(game: number): Promise<RaAchievement[]> {
  if (!inTauri)
    return Array.from({ length: 12 }, (_, i) => ({
      id: i,
      title: `Achievement ${i + 1}`,
      description: "Complete the first act of Green Hill Zone.",
      points: [1, 3, 5, 10, 25][i % 5],
      badge: "",
      kind: i === 11 ? "win_condition" : i % 4 ? "" : "progression",
      rarity: 1 / (i + 1.5),
    }));
  return invoke<RaAchievement[]>("cheevos_achievements", { game });
}
