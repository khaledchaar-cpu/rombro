import { createVirtualizer } from "@tanstack/solid-virtual";
import { createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import DirField from "../components/DirField";
import Cover from "../components/Cover";
import GameDetail from "../components/GameDetail";
import Panel from "../components/Panel";
import Segments from "../components/Segments";
import Select from "../components/Select";
import { createLibraryFilter } from "./LibraryFilters";
import { onScanProgress, type LibraryRow, type ScanProgress } from "../ipc";
import {
  cheevosCell, cheevosTitle, formatPlayTime, library, libraryBusy as busy, libraryError as error, libraryRows as rows, refreshLibrary,
  setLibrary, toggleFavorite,
} from "../state/libraryStore";

const ROW_H = 26;
const TILE_W = 148;
const TILE_H = 228;
const VIEW_KEY = "rombro.libraryView";
const savedMode = (): "list" | "grid" => {
  try {
    return localStorage.getItem(VIEW_KEY) === "grid" ? "grid" : "list";
  } catch {
    return "list";
  }
};
const EMPTY: LibraryRow = {
  path: "", system: "", name: "", state: "known", files: 0, regions: [], added: 0,
  favorite: false, plays: 0, seconds: 0, last_played: 0, cheevos: 0, cheevos_other: null, cheevos_game: null, cheevos_progress: null,
};
type Key = "favorite" | "system" | "name" | "path" | "state" | "seconds" | "cheevos" | "last_played" | "added";
const SORTS: { value: Key; label: string }[] = [
  { value: "system", label: "Sort: system" },
  { value: "name", label: "Sort: name" },
  { value: "last_played", label: "Sort: last played" },
  { value: "seconds", label: "Sort: play time" },
  { value: "added", label: "Sort: recently added" },
  { value: "favorite", label: "Sort: favorites first" },
];
const COLS: { key: Key; label: string }[] = [
  { key: "favorite", label: "★" },
  { key: "state", label: "State" },
  { key: "system", label: "System" },
  { key: "name", label: "Name" },
  { key: "path", label: "Path" },
  { key: "seconds", label: "Played" },
  { key: "cheevos", label: "🏆" },
];

const cmp = (a: LibraryRow, b: LibraryRow, key: Key) => {
  const x = a[key], y = b[key];
  return typeof x === "string" ? x.localeCompare(y as string) : Number(y) - Number(x);
};

export default function Library() {
  const filter = createLibraryFilter(rows);
  const [sort, setSort] = createSignal<{ key: Key; asc: boolean }>({ key: "system", asc: true });
  const [progress, setProgress] = createSignal<ScanProgress>({ done: 0, total: 0 });
  const unlisten = onScanProgress(setProgress);
  onCleanup(() => void unlisten.then((f) => f()));

  const view = createMemo(() => {
    const { key, asc } = sort();
    const f = [...filter.filtered()];
    return f.sort((a, b) => (asc ? 1 : -1) * cmp(a, b, key) || a.name.localeCompare(b.name));
  });

  // by path: rows are replaced when a favorite or play time changes
  const [selPath, setSelPath] = createSignal<string | null>(null);
  const selected = createMemo(() => rows().find((r) => r.path === selPath()) ?? null);
  const toggle = (key: Key) => setSort((s) => ({ key, asc: s.key === key ? !s.asc : true }));

  const [mode, setModeRaw] = createSignal(savedMode());
  const setMode = (m: "list" | "grid") => {
    setModeRaw(m);
    try {
      localStorage.setItem(VIEW_KEY, m);
    } catch { /* per-device convenience only */ }
  };

  let scroller!: HTMLDivElement;
  const v = createVirtualizer({
    get count() { return view().length; },
    getScrollElement: () => scroller,
    estimateSize: () => ROW_H,
    overscan: 12,
  });

  let gridScroller!: HTMLDivElement;
  const [cols, setCols] = createSignal(4);
  const observer = new ResizeObserver(([e]) => setCols(Math.max(1, Math.floor(e.contentRect.width / TILE_W))));
  onCleanup(() => observer.disconnect());
  const g = createVirtualizer({
    get count() { return Math.ceil(view().length / cols()); },
    getScrollElement: () => gridScroller,
    estimateSize: () => TILE_H,
    overscan: 3,
  });

  return (
    <div class="grid">
      <Panel title="Library" class="wide">
        <DirField label="Library" value={library()} onChange={(v) => (setLibrary(v), void refreshLibrary())} />
        <div class="row">
          <button
            class="btn"
            disabled={busy() || !library()}
            title="Checks every file on disk – only needed after changes made outside Romburak"
            onClick={() => void refreshLibrary(true)}
          >
            {busy() ? "Scanning" : "Rescan library"}
          </button>
        </div>
        <filter.Bar />
        <Show when={busy()}>
          <Segments
          value={progress().bytes_total ? (progress().bytes ?? 0) : progress().done}
          max={progress().bytes_total || progress().total}
        />
        </Show>
        <Show when={error()}>
          <p class="err mono">{error()}</p>
        </Show>
        <div class="row spread">
          <p class="dim small">
            {view().length} / {rows().length} items
          </p>
          <div class="seg-toggle" role="group" aria-label="View">
            <Select value={sort().key} onChange={(key) => setSort({ key, asc: true })} options={SORTS} placeholder="Sort: column" />
            <button class="btn ghost small" classList={{ on: mode() === "list" }} onClick={() => setMode("list")}>☰ List</button>
            <button class="btn ghost small" classList={{ on: mode() === "grid" }} onClick={() => setMode("grid")}>▦ Grid</button>
          </div>
        </div>
        <Show when={mode() === "grid"}>
          <div class="vlist cgrid" ref={(el) => ((gridScroller = el), observer.observe(el))}>
            <div style={{ height: `${g.getTotalSize()}px`, position: "relative" }}>
              <For each={g.getVirtualItems()}>
                {(line) => (
                  <div class="cgrid-row" style={{ transform: `translateY(${line.start}px)`, height: `${TILE_H}px`, "grid-template-columns": `repeat(${cols()}, minmax(0, 1fr))` }}>
                    <For each={view().slice(line.index * cols(), line.index * cols() + cols())}>
                      {(r) => (
                        <button class="tile" classList={{ sel: selPath() === r.path }} title={`${r.name}\n${r.system}`} onClick={() => setSelPath(r.path)}>
                          <Cover system={r.system} name={r.name} class="tile-cover" />
                          <span class="tile-name ellipsis small">{r.favorite ? "★ " : ""}{r.name}</span>
                          <span class="tile-sys ellipsis dim small">{r.system}</span>
                        </button>
                      )}
                    </For>
                  </div>
                )}
              </For>
            </div>
          </div>
        </Show>
        <div class="ltable" classList={{ hidden: mode() === "grid" }}>
          <div class="lrow lhead">
            <For each={COLS}>
              {(c) => (
                <button class="lsort" onClick={() => toggle(c.key)}>
                  {c.label}
                  {sort().key === c.key ? (sort().asc ? " ▲" : " ▼") : ""}
                </button>
              )}
            </For>
          </div>
          <div class="vlist" ref={scroller}>
            <div style={{ height: `${v.getTotalSize()}px`, position: "relative" }}>
              <For each={v.getVirtualItems()}>
                {(it) => {
                  const r = () => view()[it.index] ?? EMPTY;
                  return (
                    <div
                      class="vrow lrow small"
                      classList={{ sel: selPath() === r().path }}
                      onClick={() => setSelPath(r().path)}
                      style={{ transform: `translateY(${it.start}px)`, height: `${ROW_H}px` }}>
                      <button
                        class="star"
                        classList={{ on: r().favorite }}
                        title={r().favorite ? "Remove from favorites" : "Add to favorites"}
                        onClick={(e) => (e.stopPropagation(), void toggleFavorite(r()))}
                      >
                        {r().favorite ? "★" : "☆"}
                      </button>
                      <span class={`tag tag-${r().state}`}>{r().state}</span>
                      <span class="ellipsis dim" title={r().system}>{r().system}</span>
                      <span class="lname" title={r().name}>
                        <Cover system={r().system} name={r().name} class="cover-mini" />
                        <span class="ellipsis">{r().name}</span>
                      </span>
                      <span class="ellipsis mono dim" title={r().path}>{r().path}</span>
                      <span class="dim">{r().seconds ? formatPlayTime(r().seconds) : ""}</span>
                      <span
                        class="dim"
                        title={cheevosTitle(r())}
                      >
                        {cheevosCell(r())}
                      </span>
                    </div>
                  );
                }}
              </For>
            </div>
          </div>
        </div>
      </Panel>
      <Show when={selected()}>{(row) => <GameDetail row={row()} />}</Show>
    </div>
  );
}
