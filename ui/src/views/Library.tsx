import { createVirtualizer } from "@tanstack/solid-virtual";
import { createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import DirField from "../components/DirField";
import GameDetail from "../components/GameDetail";
import Panel from "../components/Panel";
import Segments from "../components/Segments";
import { createLibraryFilter } from "./LibraryFilters";
import { onScanProgress, type LibraryRow, type ScanProgress } from "../ipc";
import {
  formatPlayTime, library, libraryBusy as busy, libraryError as error, libraryRows as rows, refreshLibrary,
  setLibrary, toggleFavorite,
} from "../state/libraryStore";

const ROW_H = 26;
const EMPTY: LibraryRow = {
  path: "", system: "", name: "", state: "known", files: 0, regions: [], added: 0,
  favorite: false, plays: 0, seconds: 0, last_played: 0, cheevos: 0, cheevos_other: null,
};
type Key = "favorite" | "system" | "name" | "path" | "state" | "seconds" | "cheevos";
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

  let scroller!: HTMLDivElement;
  const v = createVirtualizer({
    get count() { return view().length; },
    getScrollElement: () => scroller,
    estimateSize: () => ROW_H,
    overscan: 12,
  });

  return (
    <div class="grid">
      <Panel title="Library" class="wide">
        <DirField label="Library" value={library()} onChange={(v) => (setLibrary(v), void refreshLibrary())} />
        <div class="row">
          <button
            class="btn"
            disabled={busy() || !library()}
            title="Checks every file on disk – only needed after changes made outside RomBro"
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
        <p class="dim small">
          {view().length} / {rows().length} items
        </p>
        <div class="ltable">
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
                      <span class="ellipsis" title={r().name}>{r().name}</span>
                      <span class="ellipsis mono dim" title={r().path}>{r().path}</span>
                      <span class="dim">{r().seconds ? formatPlayTime(r().seconds) : ""}</span>
                      <span
                        class="dim"
                        title={r().cheevos ? `${r().cheevos} achievements` : r().cheevos_other ? `Another version has achievements: ${r().cheevos_other}` : ""}
                      >
                        {r().cheevos ? r().cheevos : r().cheevos_other ? "◌" : ""}
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
