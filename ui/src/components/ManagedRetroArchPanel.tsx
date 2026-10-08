import { createResource, createSignal, Show } from "solid-js";
import Panel from "./Panel";
import PhaseProgress from "./PhaseProgress";
import { raInstall, raStatus } from "../ipc";

/** The RetroArch RomBro downloads and runs itself: install, version, update to newest stable. */
export default function ManagedRetroArchPanel() {
  const [check, setCheck] = createSignal(false);
  // Wrapped: a falsy source (`false`) would keep the resource from ever loading.
  const [status, { refetch }] = createResource(() => ({ check: check() }), (s) => raStatus(s.check));
  const [busy, setBusy] = createSignal(false);
  const [msg, setMsg] = createSignal("");

  const install = async (version: string | null) => {
    setBusy(true);
    setMsg("");
    try {
      const v = await raInstall(version);
      setMsg(`RetroArch ${v} installed`);
      void refetch();
    } catch (e) {
      setMsg(String(e));
    } finally {
      setBusy(false);
    }
  };
  const newer = () => {
    const s = status();
    return s?.latest && s.latest !== s.installed ? s.latest : null;
  };

  return (
    <Panel title="Managed RetroArch">
      <Show when={status()} fallback={<p class="dim">{status.error ? String(status.error) : "loading…"}</p>}>
        {(s) => (
          <Show when={s().supported} fallback={<p class="dim">No RetroArch build for this platform.</p>}>
            <div class="kpi">{s().installed ?? "not installed"}</div>
            <p class="dim small">
              RomBro downloads the official stable build (pinned: {s().pinned}, ~200 MB) and keeps cores, saves and
              states in its own folder. Your ROM folders stay untouched.
            </p>
            <p class="dim mono small">{s().folder}</p>
            <div class="row">
              <Show when={!s().installed}>
                <button class="btn" disabled={busy()} onClick={() => void install(null)}>
                  Install RetroArch {s().pinned}
                </button>
              </Show>
              <Show when={newer()}>
                {(v) => (
                  <button class="btn" disabled={busy()} onClick={() => void install(v())}>
                    Update to {v()}
                  </button>
                )}
              </Show>
              <button
                class="btn ghost"
                disabled={busy() || status.loading}
                onClick={() => (check() ? void refetch() : setCheck(true))}
              >
                Check for updates
              </button>
            </div>
            <Show when={check() && !status.loading && !newer() && s().installed}>
              <p class="dim small">
                {s().latest ? `Up to date (newest stable: ${s().latest})` : "Buildbot not reachable"}
              </p>
            </Show>
          </Show>
        )}
      </Show>
      <Show when={busy()} fallback={<Show when={msg()}><p class="dim small">{msg()}</p></Show>}>
        <PhaseProgress
          event="ra://progress"
          labels={{ download: "downloading (MB)", verify: "verifying", unpack: "unpacking (MB)" }}
        />
      </Show>
    </Panel>
  );
}
