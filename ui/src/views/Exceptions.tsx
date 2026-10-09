import { For, Show, createResource, createSignal } from "solid-js";
import Pager, { createPaged, wheelPage } from "../components/Pager";
import Panel from "../components/Panel";
import DirField from "../components/DirField";
import { exceptionsGet, ignoreSet, resolutionClear, setVerdict, type Exceptions as Ex } from "../ipc";

type Verdict = Ex["verdicts"][number];

/** Past keep/discard/prefer decisions on releases, filterable, each removable. */
export function VerdictList(props: { verdicts: Verdict[]; onChange: () => void }) {
  const [q, setQ] = createSignal("");
  const shown = () => {
    const words = q().toLowerCase().split(/\s+/).filter(Boolean);
    return props.verdicts.filter((v) => words.every((w) => `${v.verdict} ${v.name} ${v.system}`.toLowerCase().includes(w)));
  };
  const paged = createPaged(shown, () => 8);
  return (
    <>
      <p class="dim small">keep = always place in addition · discard = move to _trash · prefer = wins a 1G1R tie. Remove a decision to be asked again on the next plan.</p>
      <Show when={props.verdicts.length} fallback={<p class="dim small">none yet – decisions are made in the import review</p>}>
        <Show when={props.verdicts.length > 8}>
          <input class="field preset-search" placeholder={`Filter ${props.verdicts.length} decisions…`} value={q()} onInput={(e) => setQ(e.currentTarget.value)} />
        </Show>
        <div {...wheelPage(paged)}>
        <For each={paged.items()}>
          {(v) => (
            <div class="row ex-row">
              <span class="tag mono">{v.verdict}</span>
              <div class="ex-main">
                <span class="ellipsis" title={v.name}>{v.name}</span>
                <span class="dim small ellipsis">
                  {[v.system, v.reason || "reason not recorded", v.decided && new Date(v.decided * 1000).toLocaleDateString()]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
              </div>
              <button class="btn ghost" onClick={async () => (await setVerdict(v, null), props.onChange())}>Remove</button>
            </div>
          )}
        </For>
        </div>
        <Pager paged={paged} />
      </Show>
    </>
  );
}

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
          <VerdictList verdicts={e().verdicts} onChange={refetch} />

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
