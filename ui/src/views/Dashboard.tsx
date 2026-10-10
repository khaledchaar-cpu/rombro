import { createResource, createSignal, For, Match, Show, Switch } from "solid-js";
import Panel from "../components/Panel";
import LibraryProgress from "../components/LibraryProgress";
import { ContinuePanel } from "./DashboardContinue";
import { PopularPanel } from "./DashboardPopular";
import { LibraryPanel } from "./DashboardHistory";
import PlayStats from "./Stats";
import { GrowthPanel } from "./DashboardGrowth";
import {
  AchievementsPanel,
  CompletenessPanel,
  FranchisePanel,
  ProgressPanel,
} from "./DashboardProgress";
import { gamifyEnabled } from "../state/gamify";
import { dbStats } from "../ipc";
import { libraryBusy, libraryRows } from "../state/libraryStore";

type Page = "play" | "stats" | "collection" | "goals";
// kept while switching views
const [page, setPage] = createSignal<Page>("play");

export default function Dashboard(props: {
  onLibrary: () => void;
  onDatabases: () => void;
}) {
  // databases live in Settings; the dashboard only asks for a first sync
  const [stats] = createResource(dbStats);
  const pages = (): { id: Page; label: string }[] => [
    { id: "play", label: "Play" },
    { id: "stats", label: "Stats" },
    { id: "collection", label: "Collection" },
    ...(gamifyEnabled() ? [{ id: "goals" as const, label: "Goals" }] : []),
  ];
  const shown = (): Page => (page() === "goals" && !gamifyEnabled() ? "play" : page());
  return (
    <div class="grid">
      <Show when={stats()?.entries === 0}>
        <Panel title="No databases yet" class="wide">
          <p>Games are identified with RetroArch's databases. Sync them once before importing.</p>
          <button class="btn" onClick={() => props.onDatabases()}>
            Open Settings → Databases
          </button>
        </Panel>
      </Show>
      {/* pages instead of one long scroll: play, how you play, the collection, then gamification */}
      <div class="row detail-tabs wide" role="tablist">
        <For each={pages()}>
          {(p) => (
            <button role="tab" class="btn ghost small" classList={{ active: shown() === p.id }} aria-selected={shown() === p.id} onClick={() => setPage(p.id)}>
              {p.label}
            </button>
          )}
        </For>
      </div>
      <Switch>
        <Match when={shown() === "play"}>
          <Show when={libraryBusy() && !libraryRows().length}>
            <LibraryProgress />
          </Show>
          <ContinuePanel onLibrary={props.onLibrary} />
          <PopularPanel onLibrary={props.onLibrary} />
        </Match>
        <Match when={shown() === "stats"}>
          <PlayStats />
        </Match>
        <Match when={shown() === "collection"}>
          <LibraryPanel onOpen={props.onLibrary} />
          <GrowthPanel />
          <Show when={gamifyEnabled()}>
            <CompletenessPanel />
          </Show>
        </Match>
        <Match when={shown() === "goals"}>
          <ProgressPanel />
          <FranchisePanel />
          <AchievementsPanel />
        </Match>
      </Switch>
    </div>
  );
}
