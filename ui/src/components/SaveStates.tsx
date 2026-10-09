// Savestates of one game: screenshot, slot, date; start from a slot or delete (inline confirm).
import { createResource, createSignal, For, onCleanup, Show } from "solid-js";
import { deleteSaveState, onPlayEnded, play, saveStates, stateShotUrl, type SaveState } from "../ipc";

function Shot(props: { s: SaveState }) {
  const [failed, setFailed] = createSignal(false);
  const src = () => (props.s.screenshot ? stateShotUrl(props.s.screenshot) : null);
  return (
    <Show when={src() && !failed()} fallback={<div class="state-shot cover-ph">{props.s.slot ?? "A"}</div>}>
      <img class="state-shot" src={src()!} alt="" loading="lazy" onError={() => setFailed(true)} />
    </Show>
  );
}

export default function SaveStates(props: { path: string }) {
  const [list, { refetch }] = createResource(() => props.path, saveStates);
  const unlisten = onPlayEnded(() => void refetch());
  onCleanup(() => void unlisten.then((f) => f()));
  const [confirm, setConfirm] = createSignal<string | null>(null);
  const [msg, setMsg] = createSignal("");
  const start = async (s: SaveState) => {
    setMsg(`Starting from slot ${s.slot}…`);
    try {
      setMsg(`Started with ${await play(props.path, s.slot ?? undefined, s.core_id ?? undefined)} from slot ${s.slot}`);
    } catch (e) {
      setMsg(String(e));
    }
  };
  const remove = async (s: SaveState) => {
    setConfirm(null);
    try {
      await deleteSaveState(s.path);
    } catch (e) {
      setMsg(String(e));
    }
    void refetch();
  };
  return (
    <div class="states">
      <h4 class="dim small">Savestates</h4>
      <Show when={list()?.length} fallback={<p class="dim small">{list.loading ? "…" : "No savestates yet (F2 saves in RetroArch)."}</p>}>
        <ul class="state-list">
          <For each={list()}>
            {(s) => (
              <li class="state">
                <Shot s={s} />
                <div class="state-meta small">
                  <strong>{s.slot === null ? "Auto" : `Slot ${s.slot}`}</strong>
                  <span class="dim">{new Date(s.modified * 1000).toLocaleString()}</span>
                  <span class="dim ellipsis" title={s.path}>{s.core || "–"}</span>
                </div>
                <div class="state-actions">
                  <Show
                    when={s.slot !== null}
                    fallback={<span class="dim small" title="RetroArch loads it on start when “Auto load state” is on">auto-load</span>}
                  >
                    <button class="btn small" onClick={() => void start(s)}>▶ Start</button>
                  </Show>
                  <Show
                    when={confirm() === s.path}
                    fallback={<button class="btn ghost small" onClick={() => setConfirm(s.path)}>Delete</button>}
                  >
                    <button class="btn danger small" onClick={() => void remove(s)}>Really delete</button>
                    <button class="btn ghost small" onClick={() => setConfirm(null)}>Keep</button>
                  </Show>
                </div>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={msg()}><p class="dim small">{msg()}</p></Show>
    </div>
  );
}
