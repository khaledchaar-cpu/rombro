import { For } from "solid-js";
import DirField from "../components/DirField";
import Panel from "../components/Panel";
import ImportSteps from "../components/ImportSteps";
import type { Mode } from "../ipc";
import { buildPlan, busy, executing, inbox, library, mode, setInbox, setLibrary, setMode } from "../state/importStore";

const MODES: { id: Mode; label: string }[] = [
  { id: "move", label: "Move" },
  { id: "copy", label: "Copy" },
];

/** Folders and mode of the import; plans with or without the inbox. Results show below it. */
export default function ImportSetup() {
  const run = (withInbox: boolean) => void buildPlan(withInbox);

  return (
    <Panel title="Import" class="wide">
      <ImportSteps />
      <div class="dir-pair">
        <DirField label="Inbox" value={inbox()} onChange={setInbox} />
        <DirField label="Library" value={library()} onChange={setLibrary} />
      </div>
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
          {busy() && !executing() ? "Planning" : "Plan import"}
        </button>
      </div>
    </Panel>
  );
}
