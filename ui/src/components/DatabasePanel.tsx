import { createResource, createSignal, For, Show } from "solid-js";
import Panel from "./Panel";
import PhaseProgress from "./PhaseProgress";
import { dbStats, dbSync, pickDir } from "../ipc";

const fmt = new Intl.NumberFormat("en-US");

/** Game databases: size, sync from RetroArch's RDBs (+ arcade DATs), entries per system. */
export default function DatabasePanel() {
  const [stats, { refetch }] = createResource(dbStats);
  const [msg, setMsg] = createSignal("");
  const [syncing, setSyncing] = createSignal(false);

  const sync = async (manual: boolean) => {
    const d = manual ? await pickDir("RetroArch database/rdb folder") : null;
    if (manual && !d) return;
    setSyncing(true);
    setMsg("");
    try {
      const r = await dbSync(d);
      const rdbs =
        r.imported + r.removed === 0
          ? `Already up to date · ${r.unchanged} RDBs unchanged`
          : `${r.imported} imported · ${r.unchanged} unchanged · ${r.removed} removed`;
      const dats = r.dats_updated > 0 ? ` · ${r.dats_updated} arcade DATs updated` : "";
      const warn = r.dat_warnings.length > 0 ? ` · DAT warning: ${r.dat_warnings.join("; ")}` : "";
      setMsg((r.downloaded ? "Databases downloaded · " : "") + rdbs + dats + warn);
      void refetch();
    } catch (e) {
      setMsg(String(e));
    } finally {
      setSyncing(false);
    }
  };
  const systems = () => [...(stats()?.systems ?? [])].sort((a, b) => b.count - a.count);

  return (
    <>
      <Panel title="Databases">
        <Show when={stats()} fallback={<p class="dim">{stats.error ? String(stats.error) : "loading…"}</p>}>
          {(s) => (
            <>
              <div class="kpi">{fmt.format(s().entries)}</div>
              <p class="dim">entries · {s().systems.length} systems</p>
              <p class="dim mono small">{s().db_path}</p>
            </>
          )}
        </Show>
        <p class="dim small">RetroArch databases and arcade DATs, downloaded from libretro. The managed RetroArch uses the same databases.</p>
        <div class="row">
          <button class="btn" disabled={syncing()} onClick={() => void sync(false)}>
            Sync databases
          </button>
          <button class="btn ghost" disabled={syncing()} onClick={() => void sync(true)}>
            Use local RDB folder…
          </button>
        </div>
        <Show when={syncing()} fallback={<Show when={msg()}><p class="dim small">{msg()}</p></Show>}>
          <PhaseProgress
            event="sync://progress"
            labels={{ download: "downloading RetroArch databases (MB)", rdb: "reading RetroArch databases", dat: "downloading arcade DATs" }}
          />
        </Show>
      </Panel>
      <Panel title="Entries per system">
        <ul class="rows">
          <For each={systems()}>
            {(s) => (
              <li>
                <span>{s.system}</span>
                <span class="mono">{fmt.format(s.count)}</span>
              </li>
            )}
          </For>
        </ul>
      </Panel>
    </>
  );
}
