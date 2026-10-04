import { createResource, createSignal, For, onCleanup, Show } from "solid-js";
import Panel from "../components/Panel";
import Segments from "../components/Segments";
import { dbStats, onScanProgress, scan, type ScanSummary } from "../ipc";

const fmt = new Intl.NumberFormat("en-US");

export default function Dashboard() {
  const [stats] = createResource(dbStats);
  const [dir, setDir] = createSignal("");
  const [progress, setProgress] = createSignal({ done: 0, total: 0 });
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

  const top = () => [...(stats()?.systems ?? [])].sort((a, b) => b.count - a.count).slice(0, 8);

  return (
    <div class="grid">
      <Panel title="Database">
        <Show when={stats()} fallback={<p class="dim">{stats.error ? String(stats.error) : "loading…"}</p>}>
          {(s) => (
            <>
              <div class="kpi">{fmt.format(s().entries)}</div>
              <p class="dim">entries · {s().systems.length} systems</p>
              <p class="dim mono small">{s().db_path}</p>
            </>
          )}
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
        <Segments value={progress().done} max={progress().total} />
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
                {v.roms} roms · {v.discs} discs · {v.playlists} m3u · {v.failures} errors ·{" "}
                {(v.bytes / 1e6).toFixed(1)} MB in {v.millis} ms
              </p>
            );
          }}
        </Show>
      </Panel>
    </div>
  );
}
