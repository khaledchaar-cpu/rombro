// Stats → "Most popular": top 10 RetroAchievements games of the library by players, per system.
import { createMemo, createSignal, For, Show } from "solid-js";
import Cover from "../components/Cover";
import Panel from "../components/Panel";
import Select from "../components/Select";
import { formatPlayTime, libraryRows } from "../state/libraryStore";
import { rankLabel, rankTitle, shortSystem } from "../state/popularity";
import { popularList } from "./DashboardPopular";

const TOP = 10;

export function PopularStats() {
  const [system, setSystem] = createSignal("");
  const systems = createMemo(() => {
    const s = new Set(libraryRows().filter((r) => r.cheevos_players != null).map((r) => r.system));
    return [{ value: "", label: "All systems" }, ...[...s].sort().map((v) => ({ value: v, label: shortSystem(v) }))];
  });
  const top = createMemo(() =>
    popularList(libraryRows().filter((r) => !system() || r.system === system()), TOP),
  );
  return (
    <Panel title="Most popular" class="wide">
      <Show
        when={systems().length > 1}
        fallback={<p class="dim small">No player counts yet (Settings → RetroAchievements → Sync).</p>}
      >
        <div class="row">
          <Select value={system()} onChange={setSystem} options={systems()} />
        </div>
        <ol class="rank">
          <For each={top()}>
            {(r) => (
              <li>
                <Cover system={r.system} name={r.name} class="cover-mini" />
                <span class="rank-name">
                  <span class="ellipsis small" title={rankTitle(r)}>{r.name}</span>
                  <span class="dim small">{rankLabel(r)}</span>
                </span>
                <span class="dim small">{r.seconds ? formatPlayTime(r.seconds) : "not played"}</span>
              </li>
            )}
          </For>
        </ol>
      </Show>
    </Panel>
  );
}
