// Dashboard panel: library summary (from the file index).
import { jumpProps } from "../state/jump";
import { For, Show } from "solid-js";
import Panel from "../components/Panel";
import { librarySummary } from "../state/libraryStore";

const fmt = new Intl.NumberFormat("en-US");

export function LibraryPanel(props: { onOpen: () => void }) {
  return (
    <Panel title="Library">
      <Show when={librarySummary()} fallback={<p class="dim">No library yet – choose one in the Library view.</p>}>
        {(s) => (
          <>
            <div class="kpi" {...jumpProps({})}>{fmt.format(s().total)}</div>
            <p class="dim small">
              items · {s().systems.length} systems · scanned {new Date(s().ts).toLocaleString()}
            </p>
            <ul class="rows">
              <For each={Object.entries(s().states)}>
                {([k, n]) => (
                  <li {...jumpProps({ states: [k] })}>
                    <span>{k}</span>
                    <span class="mono">{fmt.format(n)}</span>
                  </li>
                )}
              </For>
            </ul>
            <ul class="rows small">
              <For each={s().systems.slice(0, 5)}>
                {([sys, n]) => (
                  <li {...jumpProps({ systems: [sys] })}>
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
