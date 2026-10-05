// Dashboard panels for what a run left behind: open decisions and the trash folder.
import { createResource, createSignal, For, Show } from "solid-js";
import Panel from "../components/Panel";
import { trashEmpty, trashList } from "../ipc";
import { buildPlan, busy, inbox, library, openDecisions } from "../state/importStore";

const mb = (b: number) => `${(b / 1e6).toFixed(1)} MB`;

export function OpenDecisionsPanel(props: { onReview: () => void }) {
  const total = () => Object.values(openDecisions()?.kinds ?? {}).reduce((a, b) => a + b, 0);
  const review = async () => {
    props.onReview();
    await buildPlan(!!inbox());
  };
  return (
    <Panel title="Needs attention">
      <Show when={openDecisions()} fallback={<p class="dim">No plan built yet.</p>}>
        {(o) => (
          <>
            <div class="kpi">{total()}</div>
            <p class="dim small">open after last plan · {new Date(o().ts).toLocaleString()}</p>
            <ul class="rows">
              <For each={Object.entries(o().kinds)}>
                {([k, n]) => (
                  <li>
                    <span class={`tag tag-${k}`}>{k}</span>
                    <span class="mono">{n}</span>
                  </li>
                )}
              </For>
            </ul>
            <p class="dim small">Undecided items stay where they are (inbox or current library path).</p>
            <button class="btn" disabled={busy() || !library() || !total()} onClick={review}>
              Review (re-plan)
            </button>
          </>
        )}
      </Show>
    </Panel>
  );
}

export function TrashPanel() {
  const [files, { refetch }] = createResource(library, (l) => (l ? trashList(l) : Promise.resolve([])));
  const [armed, setArmed] = createSignal(false);
  const [msg, setMsg] = createSignal<{ ok: boolean; text: string }>();
  const bytes = () => (files() ?? []).reduce((a, f) => a + f.bytes, 0);

  const empty = async () => {
    if (!armed()) return setArmed(true);
    setArmed(false);
    try {
      const n = await trashEmpty(library());
      setMsg({ ok: true, text: `deleted ${n} files` });
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    }
    refetch();
  };

  return (
    <Panel title="Trash">
      <Show when={library()} fallback={<p class="dim">Choose a library in the Inbox view.</p>}>
        <div class="kpi">{files()?.length ?? 0}</div>
        <p class="dim small">files · {mb(bytes())} in {library()}/_trash</p>
        <ul class="rows mono small">
          <For each={(files() ?? []).slice(0, 8)}>
            {(f) => (
              <li>
                <span class="ellipsis" title={f.path}>{f.path}</span>
                <span>{mb(f.bytes)}</span>
              </li>
            )}
          </For>
        </ul>
        <Show when={(files()?.length ?? 0) > 8}>
          <p class="dim small">… and {(files()?.length ?? 0) - 8} more</p>
        </Show>
        <div class="row">
          <button class="btn ghost" onClick={() => (setArmed(false), refetch())}>Refresh</button>
          <span class="spacer" />
          <button
            class="btn"
            classList={{ danger: armed() }}
            disabled={!files()?.length}
            onClick={empty}
            onMouseLeave={() => setArmed(false)}
          >
            {armed() ? "Click again: delete permanently" : "Empty trash"}
          </button>
        </div>
        <Show when={msg()}>{(m) => <p class={`mono small ${m().ok ? "ok" : "err"}`}>{m().text}</p>}</Show>
      </Show>
    </Panel>
  );
}
