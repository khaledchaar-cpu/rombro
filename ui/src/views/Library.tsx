import { createEffect, createMemo, createSignal, For, on, onCleanup, Show } from "solid-js";
import Cover from "../components/Cover";
import GameDetail from "../components/GameDetail";
import Pager, { createPaged, fitCount } from "../components/Pager";
import Panel from "../components/Panel";
import Segments from "../components/Segments";
import Select from "../components/Select";
import { createLibraryFilter } from "./LibraryFilters";
import { onScanProgress, type LibraryRow, type ScanProgress } from "../ipc";
import {
  cheevosCell, cheevosTitle, formatPlayTime, library, libraryBusy as busy, libraryError as error, libraryRows as rows, toggleFavorite,
} from "../state/libraryStore";
import { libraryJump, setLibraryJump } from "../state/jump";
import { popularityRank, rankLabel, rankTitle } from "../state/popularity";

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
type Key = "favorite" | "system" | "name" | "state" | "seconds" | "cheevos" | "cheevos_players" | "last_played" | "added";
const SORTS: { value: Key; label: string }[] = [
  { value: "system", label: "Sort: system" },
  { value: "name", label: "Sort: name" },
  { value: "last_played", label: "Sort: last played" },
  { value: "seconds", label: "Sort: play time" },
  { value: "added", label: "Sort: recently added" },
  { value: "favorite", label: "Sort: favorites first" },
  { value: "cheevos_players", label: "Sort: popularity" },
];
const COLS: { key: Key; label: string }[] = [
  { key: "favorite", label: "★" },
  { key: "state", label: "State" },
  { key: "system", label: "System" },
  { key: "name", label: "Name" },
  { key: "seconds", label: "Played" },
  { key: "cheevos", label: "🏆" },
  { key: "cheevos_players", label: "Rank" },
];

const cmp = (a: LibraryRow, b: LibraryRow, key: Key) => {
  const x = a[key], y = b[key];
  return typeof x === "string" ? x.localeCompare(y as string) : Number(y) - Number(x);
};

export default function Library(props: { onSettings: () => void }) {
  const filter = createLibraryFilter(rows);
  const [sort, setSort] = createSignal<{ key: Key; asc: boolean }>({ key: "system", asc: true });
  const [progress, setProgress] = createSignal<ScanProgress>({ done: 0, total: 0 });
  const unlisten = onScanProgress(setProgress);
  onCleanup(() => void unlisten.then((f) => f()));

  const view = createMemo(() => {
    const { key, asc } = sort();
    const f = [...filter.filtered()];
    // rows without a system (unknown files) stay at the end in both directions
    return f.sort((a, b) => +!a.system - +!b.system || (asc ? 1 : -1) * cmp(a, b, key) || a.name.localeCompare(b.name));
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

  const [listEl, setListEl] = createSignal<HTMLDivElement>();
  const [gridEl, setGridEl] = createSignal<HTMLDivElement>();
  // real row height (themes differ in font size), read from a rendered row
  const rowH = () => listEl()?.querySelector(".vrow")?.getBoundingClientRect().height || ROW_H;
  const listRows = fitCount(listEl, rowH, 76);
  const gridRows = fitCount(gridEl, TILE_H, 76, 1);
  const [cols, setCols] = createSignal(4);
  const observer = new ResizeObserver(([e]) => setCols(Math.max(1, Math.floor(e.contentRect.width / TILE_W))));
  onCleanup(() => observer.disconnect());
  const paged = createPaged(view, () => (mode() === "grid" ? cols() * gridRows() : listRows()));
  // a jump from the dashboard: apply its filter, select its game and page to it once the
  // rows are there (the first visit loads them after the jump arrives)
  const [pending, setPending] = createSignal<string | null>(null);
  createEffect(on(libraryJump, (j) => {
    if (!j) return;
    filter.apply(j);
    setSelPath(j.path ?? null);
    setPending(j.path ?? null);
    setLibraryJump(null);
  }));
  createEffect(() => {
    const p = pending();
    if (!p) return;
    const r = view().find((r) => r.path === p);
    if (r) (paged.show(r), setPending(null));
  });

  // keep the selected game in sight when switching list ↔ grid
  createEffect(on(mode, () => {
    const r = view().find((r) => r.path === selPath());
    if (r) paged.show(r);
  }, { defer: true }));

  return (
    <div class="libview" classList={{ "with-detail": !!selected() }}>
      <Panel title="Library" class="lib-main">
        <Show when={!library()}>
          <p class="dim">
            No library folder set –{" "}
            <button class="btn ghost small" onClick={() => props.onSettings()}>choose one in Settings</button>
          </p>
        </Show>
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
          <div class="cgrid" ref={(el) => (setGridEl(el), observer.observe(el))}>
            <div class="cgrid-page" style={{ "grid-template-columns": `repeat(${cols()}, minmax(0, 1fr))`, "grid-auto-rows": `${TILE_H - 8}px` }}>
                    <For each={paged.items()}>
                      {(r) => (
                        <button class="tile" classList={{ sel: selPath() === r.path }} title={`${r.name}\n${r.system}`} onClick={() => setSelPath(r.path)}>
                          <Cover system={r.system} name={r.name} class="tile-cover" />
                          <span class="tile-name ellipsis small">{r.favorite ? "★ " : ""}{r.name}</span>
                          <span class="tile-sys ellipsis dim small">{r.system}</span>
                        </button>
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
          <div class="lpage" ref={setListEl}>
              <For each={paged.items()}>
                {(row) => {
                  const r = () => row;
                  return (
                    <div
                      class="vrow lrow small"
                      classList={{ sel: selPath() === r().path }}
                      onClick={() => setSelPath(r().path)}
>
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
                      <span class="dim">{r().seconds ? formatPlayTime(r().seconds) : ""}</span>
                      <span
                        class="dim"
                        title={cheevosTitle(r())}
                      >
                        {cheevosCell(r())}
                      </span>
                      <span class="dim" title={`${rankLabel(r())} · ${rankTitle(r())}`}>
                        {popularityRank(r()) ? `#${popularityRank(r())}` : ""}
                      </span>
                    </div>
                  );
                }}
              </For>
          </div>
        </div>
        <Pager paged={paged} />
      </Panel>
      <Show when={selected()}>{(row) => <GameDetail row={row()} />}</Show>
    </div>
  );
}
