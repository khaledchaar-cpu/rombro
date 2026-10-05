import { createVirtualizer } from "@tanstack/solid-virtual";
import { createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import DirField from "../components/DirField";
import Panel from "../components/Panel";
import Segments from "../components/Segments";
import { onScanProgress } from "../ipc";
import {
  library, libraryBusy as busy, libraryError as error, libraryRows as rows, refreshLibrary, setLibrary,
} from "../state/libraryStore";

const ROW_H = 26;
type Key = "system" | "name" | "path" | "state";
const COLS: { key: Key; label: string }[] = [
  { key: "state", label: "State" },
  { key: "system", label: "System" },
  { key: "name", label: "Name" },
  { key: "path", label: "Path" },
];

export default function Library() {
  const [query, setQuery] = createSignal("");
  const [sort, setSort] = createSignal<{ key: Key; asc: boolean }>({ key: "system", asc: true });
  const [progress, setProgress] = createSignal({ done: 0, total: 0 });
  const unlisten = onScanProgress(setProgress);
  onCleanup(() => void unlisten.then((f) => f()));

  const view = createMemo(() => {
    const q = query().toLowerCase();
    const { key, asc } = sort();
    const f = q
      ? rows().filter((r) => r.name.toLowerCase().includes(q) || r.path.toLowerCase().includes(q) || r.system.toLowerCase().includes(q))
      : [...rows()];
    return f.sort((a, b) => (asc ? 1 : -1) * (a[key].localeCompare(b[key]) || a.name.localeCompare(b.name)));
  });

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
          <input class="field" placeholder="Filter…" value={query()} onInput={(e) => setQuery(e.currentTarget.value)} />
          <button class="btn" disabled={busy() || !library()} onClick={refreshLibrary}>
            {busy() ? "Scanning" : "Rescan"}
          </button>
        </div>
        <Show when={busy()}>
          <Segments value={progress().done} max={progress().total} />
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
                  const r = view()[it.index];
                  return (
                    <div class="vrow lrow small" style={{ transform: `translateY(${it.start}px)`, height: `${ROW_H}px` }}>
                      <span class={`tag tag-${r.state}`}>{r.state}</span>
                      <span class="ellipsis dim" title={r.system}>{r.system}</span>
                      <span class="ellipsis" title={r.name}>{r.name}</span>
                      <span class="ellipsis mono dim" title={r.path}>{r.path}</span>
                    </div>
                  );
                }}
              </For>
            </div>
          </div>
        </div>
      </Panel>
    </div>
  );
}
