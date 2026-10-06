import { createMemo, For, Show } from "solid-js";
import Panel from "./Panel";
import { busy, clearInbox, leftovers } from "../state/importStore";

const fmtSize = (n: number) =>
  n < 1024 ? `${n} B` : n < 1 << 20 ? `${(n / 1024).toFixed(0)} KB` : `${(n / (1 << 20)).toFixed(1)} MB`;

/** Inbox files the plan leaves alone, grouped by top folder, with a "move to trash" action. */
export default function InboxLeftovers() {
  const groups = createMemo(() => {
    const m = new Map<string, { files: string[]; size: number }>();
    for (const f of leftovers()) {
      const i = f.path.indexOf("/");
      const top = i < 0 ? "(inbox root)" : f.path.slice(0, i);
      const g = m.get(top) ?? { files: [], size: 0 };
      g.files.push(i < 0 ? f.path : f.path.slice(i + 1));
      g.size += f.size;
      m.set(top, g);
    }
    return [...m.entries()];
  });
  const total = () => leftovers().reduce((a, f) => a + f.size, 0);
  return (
    <Show when={leftovers().length}>
      <Panel title="Left in inbox" class="wide">
        <p class="dim small">
          Files no operation touches: not in any RetroArch database (launcher stubs, frontend media, unknown files in
          folders without a match). Clearing moves them to the library trash; undo brings them back, only emptying
          the trash deletes them.
        </p>
        <For each={groups()}>
          {([top, g]) => (
            <details>
              <summary>
                <span class="mono">{top}/</span>{" "}
                <span class="dim small">
                  {g.files.length} files · {fmtSize(g.size)}
                </span>
              </summary>
              <For each={g.files}>{(f) => <div class="mono dim small">{f}</div>}</For>
            </details>
          )}
        </For>
        <div class="row">
          <span class="dim small">
            {leftovers().length} files · {fmtSize(total())}
          </span>
          <span class="spacer" />
          <button class="btn ghost" disabled={busy()} onClick={() => void clearInbox()}>
            Move to trash
          </button>
        </div>
      </Panel>
    </Show>
  );
}
