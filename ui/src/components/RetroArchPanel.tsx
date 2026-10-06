import { createSignal, For, Show } from "solid-js";
import Panel from "./Panel";
import { retroarchExport, type RetroArchExport } from "../ipc";

/** Preview, then export library playlists (with core) and identified BIOS files to RetroArch. */
export default function RetroArchPanel() {
  const [res, setRes] = createSignal<RetroArchExport>();
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal("");
  const run = async (dryRun: boolean) => {
    setBusy(true);
    setError("");
    try {
      setRes(await retroarchExport(dryRun));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const pending = () => {
    const r = res();
    return r && r.executed === null ? r.playlists.length + r.bios_copied.length : 0;
  };
  return (
    <Panel title="RetroArch">
      <p class="dim small">
        Writes the library playlists (with the matching installed core) and copies identified BIOS files into
        RetroArch's folders. Existing playlists are replaced; undo restores them. BIOS files already there are never
        overwritten.
      </p>
      <div class="row">
        <button class="btn ghost" disabled={busy()} onClick={() => void run(true)}>Preview</button>
        <button class="btn" disabled={busy() || !pending()} onClick={() => void run(false)}>
          Export {pending() ? `(${pending()})` : ""}
        </button>
      </div>
      <Show when={error()}>
        <p class="small err">{error()}</p>
      </Show>
      <Show when={res()}>
        {(r) => (
          <div class="ra-result small">
            <p class="dim">
              <span class="mono">{r().playlist_dir}</span> · <span class="mono">{r().system_dir}</span> · {r().cores} cores
              installed
            </p>
            <Show when={r().executed !== null}>
              <p>Done: {r().executed} operations. Undo from the dashboard reverts them.</p>
            </Show>
            <h4>
              Playlists: {r().playlists.length} to write, {r().playlists_unchanged} up to date
            </h4>
            <For each={r().playlists}>
              {(p) => (
                <div class="row ex-row">
                  <span class="ellipsis">{p.system}</span>
                  <span class={p.core ? "tag mono" : "dim"}>{p.core ?? "no core installed"}</span>
                </div>
              )}
            </For>
            <h4>
              BIOS: {r().bios_copied.length} to copy, {r().bios_present} present, {r().bios_missing.length} missing
            </h4>
            <For each={r().bios_copied}>{(p) => <div class="mono">+ {p}</div>}</For>
            <For each={r().bios_conflicts}>{(p) => <div class="mono dim">kept (different file exists): {p}</div>}</For>
            <Show when={r().bios_missing.length}>
              <details>
                <summary class="dim">Missing for systems in your library</summary>
                <For each={r().bios_missing}>
                  {(m) => (
                    <div class="mono dim">
                      {m.path} <span class="small">· {m.system}</span>
                    </div>
                  )}
                </For>
              </details>
            </Show>
          </div>
        )}
      </Show>
    </Panel>
  );
}
