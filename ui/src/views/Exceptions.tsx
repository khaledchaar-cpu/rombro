import { For, Show, createResource, createSignal } from "solid-js";
import Panel from "../components/Panel";
import DirField from "../components/DirField";
import { exceptionsGet, ignoreSet, resolutionClear, setVerdict } from "../ipc";

/** Your exceptions: ignored paths, verdicts on 1G1R rejects, resolved ambiguous matches. */
export default function Exceptions() {
  const [ex, { refetch }] = createResource(exceptionsGet);
  const [path, setPath] = createSignal("");
  const ignore = async (paths: string[]) => (await ignoreSet(paths), refetch());

  return (
    <Show when={ex()}>
      {(e) => (
        <Panel title="Exceptions">
          <h4>Never touch</h4>
          <p class="dim small">Nothing at or below these paths is planned – no move, no quarantine.</p>
          <For each={e().ignored}>
            {(p) => (
              <div class="row ex-row">
                <span class="mono ellipsis">{p}</span>
                <button class="btn ghost" onClick={() => ignore(e().ignored.filter((x) => x !== p))}>Remove</button>
              </div>
            )}
          </For>
          <DirField label="Add path" value={path()} onChange={setPath} />
          <button
            class="btn ghost"
            disabled={!path().trim()}
            onClick={async () => (await ignore([...e().ignored, path().trim()]), setPath(""))}
          >
            Add
          </button>

          <h4>Your decisions on releases</h4>
          <p class="dim small">keep = always place in addition · discard = move to _trash · prefer = wins a 1G1R tie.</p>
          <Show when={e().verdicts.length} fallback={<p class="dim small">none</p>}>
            <For each={e().verdicts}>
              {(v) => (
                <div class="row ex-row">
                  <span class="tag mono">{v.verdict}</span>
                  <span class="ellipsis" title={v.system}>{v.name}</span>
                  <button class="btn ghost" onClick={async () => (await setVerdict(v, null), refetch())}>Remove</button>
                </div>
              )}
            </For>
          </Show>

          <h4>Resolved ambiguous matches</h4>
          <Show when={e().resolutions.length} fallback={<p class="dim small">none</p>}>
            <For each={e().resolutions}>
              {(r) => (
                <div class="row ex-row">
                  <span class="ellipsis" title={r.system}>{r.name}</span>
                  <span class="dim small mono">{r.sha1.slice(0, 8)}</span>
                  <button class="btn ghost" onClick={async () => (await resolutionClear(r.sha1), refetch())}>Remove</button>
                </div>
              )}
            </For>
          </Show>
        </Panel>
      )}
    </Show>
  );
}
