// Dashboard panels: library summary (from the last Library scan) and recent import runs.
import { createResource, For, Show } from "solid-js";
import Panel from "../components/Panel";
import { journalList } from "../ipc";
import { busy, status, undo } from "../state/importStore";
import { librarySummary } from "../state/librarySummary";

const fmt = new Intl.NumberFormat("en-US");

export function LibraryPanel(props: { onOpen: () => void }) {
  return (
    <Panel title="Library">
      <Show when={librarySummary()} fallback={<p class="dim">Not scanned yet – open the Library view.</p>}>
        {(s) => (
          <>
            <div class="kpi">{fmt.format(s().total)}</div>
            <p class="dim small">
              items · {s().systems.length} systems · scanned {new Date(s().ts).toLocaleString()}
            </p>
            <ul class="rows">
              <For each={Object.entries(s().states)}>
                {([k, n]) => (
                  <li>
                    <span>{k}</span>
                    <span class="mono">{fmt.format(n)}</span>
                  </li>
                )}
              </For>
            </ul>
            <ul class="rows small">
              <For each={s().systems.slice(0, 5)}>
                {([sys, n]) => (
                  <li>
                    <span class="ellipsis">{sys}</span>
                    <span class="mono">{fmt.format(n)}</span>
                  </li>
                )}
              </For>
            </ul>
            <button class="btn ghost" onClick={props.onOpen}>Open library</button>
          </>
        )}
      </Show>
    </Panel>
  );
}

export function RunsPanel() {
  const [runs, { refetch }] = createResource(journalList);
  const undoLatest = async () => {
    await undo();
    refetch();
  };
  const latestDone = () => runs()?.find((r) => r.state === "done");
  return (
    <Panel title="Recent runs" class="wide">
      <Show when={runs()?.length} fallback={<p class="dim">{runs.error ? String(runs.error) : "No runs yet."}</p>}>
        <ul class="rows mono small">
          <For each={runs()}>
            {(r) => (
              <li classList={{ dim: r.state === "undone" }}>
                <span>#{r.id} · {new Date(r.ts * 1000).toLocaleString()}</span>
                <span class="ellipsis" title={r.library}>{r.library}</span>
                <span>{r.ops} ops · {r.state}</span>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <div class="row">
        <button class="btn ghost" onClick={refetch}>Refresh</button>
        <span class="spacer" />
        <button class="btn ghost" disabled={busy() || !latestDone()} onClick={undoLatest}>
          Undo #{latestDone()?.id ?? "–"}
        </button>
      </div>
      <Show when={status()}>{(s) => <p class={`mono small ${s().ok ? "ok" : "err"}`}>{s().text}</p>}</Show>
    </Panel>
  );
}
