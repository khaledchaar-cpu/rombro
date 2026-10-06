import { createResource, createSignal, For, onCleanup, Show } from "solid-js";
import Panel from "../components/Panel";
import Segments from "../components/Segments";
import PhaseProgress from "../components/PhaseProgress";
import { LibraryPanel, RunsPanel } from "./DashboardHistory";
import { OpenDecisionsPanel, TrashPanel } from "./DashboardQueue";
import {
  AchievementsPanel,
  CompletenessPanel,
  FranchisePanel,
  ProgressPanel,
} from "./DashboardProgress";
import { gamifyEnabled } from "../state/gamify";
import {
  dbStats,
  dbSync,
  pickDir,
  onScanProgress,
  type ScanProgress,
  scan,
  type ScanSummary,
} from "../ipc";

const fmt = new Intl.NumberFormat("en-US");

export default function Dashboard(props: {
  onReview: () => void;
  onLibrary: () => void;
}) {
  const [stats, { refetch }] = createResource(dbStats);
  const [syncMsg, setSyncMsg] = createSignal("");
  const [syncing, setSyncing] = createSignal(false);

  const sync = async (manual: boolean) => {
    const d = manual ? await pickDir("RetroArch database/rdb folder") : null;
    if (manual && !d) return;
    setSyncing(true);
    setSyncMsg("syncing…");
    try {
      const r = await dbSync(d);
      const rdbs =
        r.imported + r.removed === 0
          ? `Already up to date · ${r.unchanged} RDBs unchanged`
          : `${r.imported} imported · ${r.unchanged} unchanged · ${r.removed} removed`;
      const dats = r.dats_updated > 0 ? ` · ${r.dats_updated} arcade DATs updated` : "";
      const warn = r.dat_warnings.length > 0 ? ` · DAT warning: ${r.dat_warnings.join("; ")}` : "";
      setSyncMsg(rdbs + dats + warn);
      void refetch();
    } catch (e) {
      setSyncMsg(String(e));
    } finally {
      setSyncing(false);
    }
  };
  const [dir, setDir] = createSignal("");
  const [progress, setProgress] = createSignal<ScanProgress>({ done: 0, total: 0 });
  const [busy, setBusy] = createSignal(false);
  const [result, setResult] = createSignal<ScanSummary | string>();

  const unlisten = onScanProgress(setProgress);
  onCleanup(() => void unlisten.then((f) => f()));

  const run = async () => {
    if (!dir() || busy()) return;
    setBusy(true);
    setResult(undefined);
    setProgress({ done: 0, total: 0 });
    try {
      setResult(await scan(dir()));
    } catch (e) {
      setResult(String(e));
    } finally {
      setBusy(false);
    }
  };

  const top = () =>
    [...(stats()?.systems ?? [])].sort((a, b) => b.count - a.count).slice(0, 8);

  return (
    <div class="grid">
      <Show when={gamifyEnabled()}>
        <ProgressPanel />
        <CompletenessPanel />
        <FranchisePanel />
        <AchievementsPanel />
      </Show>
      <Panel title="Database">
        <Show
          when={stats()}
          fallback={
            <p class="dim">{stats.error ? String(stats.error) : "loading…"}</p>
          }
        >
          {(s) => (
            <>
              <div class="kpi">{fmt.format(s().entries)}</div>
              <p class="dim">entries · {s().systems.length} systems</p>
              <p class="dim mono small">{s().db_path}</p>
            </>
          )}
        </Show>
        <div class="row">
          <button
            class="btn"
            disabled={syncing()}
            onClick={() => void sync(false)}
          >
            Sync databases
          </button>
          <button
            class="btn ghost"
            disabled={syncing()}
            onClick={() => void sync(true)}
          >
            Choose RDB folder…
          </button>
        </div>
        <p class="dim small">RetroArch databases (local) and arcade DATs (downloaded).</p>
        <Show when={syncing()} fallback={<Show when={syncMsg()}><p class="dim small">{syncMsg()}</p></Show>}>
          <PhaseProgress
            event="sync://progress"
            labels={{ rdb: "reading RetroArch databases", dat: "downloading arcade DATs" }}
          />
        </Show>
      </Panel>

      <Panel title="Top systems">
        <ul class="rows">
          <For each={top()}>
            {(s) => (
              <li>
                <span>{s.system}</span>
                <span class="mono">{fmt.format(s.count)}</span>
              </li>
            )}
          </For>
        </ul>
      </Panel>

      <LibraryPanel onOpen={props.onLibrary} />
      <OpenDecisionsPanel onReview={props.onReview} />
      <TrashPanel />
      <RunsPanel />

      <Panel title="Quick scan" class="wide">
        <form class="row" onSubmit={(e) => (e.preventDefault(), run())}>
          <input
            class="field mono"
            placeholder="/path/to/roms"
            value={dir()}
            onInput={(e) => setDir(e.currentTarget.value)}
          />
          <button class="btn" disabled={busy() || !dir()}>
            {busy() ? "Scanning" : "Scan"}
          </button>
        </form>
        <Segments
          value={progress().bytes_total ? (progress().bytes ?? 0) : progress().done}
          max={progress().bytes_total || progress().total}
        />
        <p class="mono dim small">
          {progress().done}/{progress().total}
        </p>
        <Show when={result()}>
          {(r) => {
            const v = r();
            return typeof v === "string" ? (
              <p class="err mono">{v}</p>
            ) : (
              <p class="mono">
                {v.roms} roms · {v.discs} discs · {v.playlists} m3u ·{" "}
                {v.failures} errors · {(v.bytes / 1e6).toFixed(1)} MB in{" "}
                {v.millis} ms
              </p>
            );
          }}
        </Show>
      </Panel>
    </div>
  );
}
