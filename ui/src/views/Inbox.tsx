import { createSignal, For, onCleanup, Show } from "solid-js";
import DirField from "../components/DirField";
import Panel from "../components/Panel";
import Segments from "../components/Segments";
import { onImportProgress, type ImportProgress, type Mode } from "../ipc";
import { buildPlan, busy, inbox, library, mode, setInbox, setLibrary, setMode, status } from "../state/importStore";

const MODES: { id: Mode; label: string }[] = [
  { id: "move", label: "Move" },
  { id: "copy", label: "Copy" },
  { id: "hardlink", label: "Hardlink" },
];

export default function Inbox(props: { onPlanned: () => void }) {
  const [progress, setProgress] = createSignal<ImportProgress>({ phase: "library", done: 0, total: 0 });
  const unlisten = onImportProgress(setProgress);
  onCleanup(() => void unlisten.then((f) => f()));

  const run = async (withInbox: boolean) => {
    setProgress({ phase: "library", done: 0, total: 0 });
    if (await buildPlan(withInbox)) props.onPlanned();
  };

  return (
    <div class="grid">
      <Panel title="Import" class="wide">
        <DirField label="Inbox" value={inbox()} onChange={setInbox} />
        <DirField label="Library" value={library()} onChange={setLibrary} />
        <div class="row">
          <For each={MODES}>
            {(m) => (
              <button
                type="button"
                class="btn ghost"
                classList={{ active: mode() === m.id }}
                onClick={() => setMode(m.id)}
              >
                {m.label}
              </button>
            )}
          </For>
          <span class="spacer" />
          <button class="btn ghost" disabled={busy() || !library()} onClick={() => run(false)}>
            Audit library
          </button>
          <button class="btn" disabled={busy() || !library() || !inbox()} onClick={() => run(true)}>
            {busy() ? "Planning" : "Plan import"}
          </button>
        </div>
        <Show when={busy()}>
          <Segments value={progress().done} max={progress().total} />
          <p class="mono dim small">
            scanning {progress().phase} {progress().done}/{progress().total}
          </p>
        </Show>
        <Show when={status()}>{(s) => <p class={`mono ${s().ok ? "ok" : "err"}`}>{s().text}</p>}</Show>
      </Panel>
    </div>
  );
}
