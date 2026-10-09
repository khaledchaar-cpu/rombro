import { createResource, Show } from "solid-js";
import Panel from "../components/Panel";
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

export default function Dashboard(props: {
  onLibrary: () => void;
  onDatabases: () => void;
}) {
  // databases live in Settings; the dashboard only asks for a first sync
  const [stats] = createResource(dbStats);
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
      {/* play first: what to start, then how you play, then the collection, then gamification */}
      <ContinuePanel onLibrary={props.onLibrary} />
      <PopularPanel onLibrary={props.onLibrary} />
      <PlayStats />
      <LibraryPanel onOpen={props.onLibrary} />
      <GrowthPanel />
      <Show when={gamifyEnabled()}>
        <CompletenessPanel />
        <FranchisePanel />
        <ProgressPanel />
        <AchievementsPanel />
      </Show>
    </div>
  );
}
