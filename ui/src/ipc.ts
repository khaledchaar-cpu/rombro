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
  if (!inTauri)
    return {
      dir: dir ?? "(mock)",
      imported: 0,
      unchanged: 146,
      removed: 0,
      entries: 0,
      dats_updated: 0,
      dat_warnings: [],
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
    why: i % 50 === 0 ? "RetroArch playlist" : "1G1R pick",
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
  if (!inTauri) return mockPlan(library);
  return invoke<PlanView>("plan_import", { inbox, library, mode });
}

export async function executePlan(): Promise<ExecResult> {
  if (!inTauri) return { done: 0, journal: null, error: null };
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
  if (!inTauri) return Promise.resolve(() => {});
  return listen<[string, ScanProgress, string?]>(event, (e) =>
    cb({ phase: e.payload[0], ...e.payload[1], item: e.payload[2] }),
  );
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
  state: "known" | "named" | "ambiguous" | "unknown" | "skip";
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
  name_folders: { n64dd: "Nintendo - Nintendo 64", solarus: "Solarus" },
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

export interface RetroArchExport {
  playlist_dir: string;
  system_dir: string;
  cores: number;
  /** Cores to install (id). */
  cores_install: string[];
  /** Wanted cores that are missing and not to be installed (path = core id). */
  cores_missing: { system: string; path: string }[];
  /** RetroArch's config names a buildbot to download cores from. */
  can_install: boolean;
  /** core = core set as default (installed, or installed by this export); null = RetroArch asks. */
  playlists: { system: string; core: string | null }[];
  playlists_unchanged: number;
  /** Exported playlists of systems without games left, moved to the library trash. */
  playlists_removed: string[];
  /** Systems no RetroArch core runs: no playlist exported. */
  playlists_no_core: string[];
  bios_copied: string[];
  bios_present: number;
  bios_conflicts: string[];
  bios_missing: { system: string; path: string }[];
  /** ScummVM targets written to scummvm.ini so games start without asking. */
  scummvm_targets: string[];
  /** Operations executed; null for a preview. */
  executed: number | null;
}

export interface CoreOption {
  id: string;
  name: string;
  installed: boolean;
  recommended: boolean;
}
export interface SystemCores {
  system: string;
  /** Core the playlist gets; null = none known. */
  chosen: string | null;
  /** Chosen by the user (stored in the rules) rather than recommended. */
  picked: boolean;
  options: CoreOption[];
}

/** The cores to choose from for each playlist of the library. */
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

/** Plans (dryRun) or executes the export of playlists, BIOS files and (installCores) missing cores. */
export async function retroarchExport(dryRun: boolean, installCores: boolean): Promise<RetroArchExport> {
  if (!inTauri)
    return {
      playlist_dir: "~/.config/retroarch/playlists",
      system_dir: "~/.config/retroarch/system",
      cores: 4,
      cores_install: installCores ? ["snes9x"] : [],
      cores_missing: installCores ? [] : [{ system: "Nintendo - Super Nintendo Entertainment System", path: "snes9x" }],
      can_install: true,
      playlists: [
        { system: "Nintendo - Super Nintendo Entertainment System", core: "Snes9x" },
        { system: "Nintendo - Sufami Turbo", core: null },
      ],
      playlists_unchanged: 3,
      playlists_removed: ["MAME.lpl"],
      playlists_no_core: ["Solarus"],
      bios_copied: ["STBIOS.bin", "fbneo/neogeo.zip"],
      bios_present: 2,
      bios_conflicts: [],
      bios_missing: [{ system: "Sony - PlayStation", path: "scph5501.bin" }],
      scummvm_targets: ["monkey2", "atlantis"],
      executed: dryRun ? null : 4,
    };
  return invoke<RetroArchExport>("retroarch_export", { dryRun, installCores });
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
      { id: "quarantine", title: "Quarantine", explain: "Files without database match go to _quarantine.", hits: 5270 },
      { id: "playlist", title: "RetroArch playlist", explain: "One .lpl per system.", hits: 30 },
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
