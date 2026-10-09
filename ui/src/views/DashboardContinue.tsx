// "Continue playing" row: recently played games first, then unplayed favorites.
import { createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import Cover from "../components/Cover";
import Pager, { createPaged } from "../components/Pager";
import Panel from "../components/Panel";
import { play, type LibraryRow } from "../ipc";
import { formatPlayTime, libraryRows } from "../state/libraryStore";

const MAX = 12;

function ago(t: number): string {
  const d = Math.floor((Date.now() / 1000 - t) / 86400);
  return d <= 0 ? "today" : d === 1 ? "yesterday" : d < 30 ? `${d} days ago` : new Date(t * 1000).toLocaleDateString();
}

export function continueList(rows: LibraryRow[]): LibraryRow[] {
  const played = rows.filter((r) => r.last_played > 0).sort((a, b) => b.last_played - a.last_played);
  const favs = rows.filter((r) => r.favorite && !r.last_played).sort((a, b) => a.name.localeCompare(b.name));
  return [...played, ...favs].slice(0, MAX);
}

export function ContinuePanel(props: { onLibrary: () => void }) {
  const list = createMemo(() => continueList(libraryRows()));
  return (
    <Shelf
      title="Continue playing"
      list={list()}
      sub={(r) => (r.last_played ? `${ago(r.last_played)} · ${formatPlayTime(r.seconds)}` : "favorite · not played yet")}
      onLibrary={props.onLibrary}
    />
  );
}

/** A row of game tiles; a click starts the game. */
export function Shelf(props: {
  title: string;
  list: LibraryRow[];
  sub: (r: LibraryRow) => string;
  onLibrary: () => void;
}) {
  const [busy, setBusy] = createSignal<string | null>(null);
  const [msg, setMsg] = createSignal("");
  const start = async (r: LibraryRow) => {
    setBusy(r.path);
    setMsg(`Starting ${r.name}…`);
    try {
      setMsg(`${r.name} started with ${await play(r.path)}`);
    } catch (e) {
      setMsg(String(e));
    } finally {
      setBusy(null);
    }
  };
  // as many tiles as fit the width, the rest on further pages
  const [width, setWidth] = createSignal(0);
  const ro = new ResizeObserver(([e]) => setWidth(e.contentRect.width));
  onCleanup(() => ro.disconnect());
  const paged = createPaged(() => props.list, () => Math.max(1, Math.floor((width() + 8) / 148)));
  return (
    <Show when={props.list.length}>
      <Panel title={props.title} class="wide">
        <div class="shelf" ref={(el) => ro.observe(el)}>
          <For each={paged.items()}>
            {(r) => (
              <button
                class="tile shelf-tile"
                disabled={!!busy()}
                title={`Play ${r.name}\n${r.system}`}
                onClick={() => void start(r)}
              >
                <Cover system={r.system} name={r.name} class="tile-cover" />
                <span class="tile-name ellipsis small">{r.favorite ? "★ " : ""}{r.name}</span>
                <span class="tile-sys ellipsis dim small">{props.sub(r)}</span>
              </button>
            )}
          </For>
        </div>
        <Pager paged={paged} />
        <div class="row spread">
          <p class="dim small">{msg()}</p>
          <button class="btn ghost small" onClick={() => props.onLibrary()}>Open library</button>
        </div>
      </Panel>
    </Show>
  );
}
