import { createResource, Show } from "solid-js";
import Panel from "../components/Panel";
import { LibraryPanel, RunsPanel } from "./DashboardHistory";
import { OpenDecisionsPanel, TrashPanel } from "./DashboardQueue";
import {
  AchievementsPanel,
  CompletenessPanel,
  FranchisePanel,
  ProgressPanel,
} from "./DashboardProgress";
import { gamifyEnabled } from "../state/gamify";
import { dbStats } from "../ipc";

export default function Dashboard(props: {
  onReview: () => void;
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
      <Show when={gamifyEnabled()}>
        <ProgressPanel />
        <CompletenessPanel />
        <FranchisePanel />
        <AchievementsPanel />
      </Show>
      <LibraryPanel onOpen={props.onLibrary} />
      <OpenDecisionsPanel onReview={props.onReview} />
      <TrashPanel />
      <RunsPanel />
    </div>
  );
}
