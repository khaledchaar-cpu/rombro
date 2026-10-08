import { createResource, createSignal, Show } from "solid-js";
import Panel from "./Panel";
import PhaseProgress from "./PhaseProgress";
import { cheevosSetKey, cheevosStatus, cheevosSync } from "../ipc";
import { refreshLibrary } from "../state/libraryStore";

/** RetroAchievements: Web API key and game lists, so the library shows which games have achievements. */
export default function CheevosPanel() {
  const [status, { refetch }] = createResource(cheevosStatus);
  const [key, setKey] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [msg, setMsg] = createSignal("");

  const saveKey = async () => {
    await cheevosSetKey(key());
    setKey("");
    void refetch();
  };
  const sync = async () => {
    setBusy(true);
    setMsg("");
    try {
      const n = await cheevosSync();
      setMsg(`${n} library games have achievements`);
      void refetch();
      void refreshLibrary();
    } catch (e) {
      setMsg(String(e));
    } finally {
      setBusy(false);
    }
  };
  const synced = () => {
    const t = status()?.synced;
    return t ? new Date(t * 1000).toLocaleString() : "never";
  };

  return (
    <Panel title="RetroAchievements">
      <p class="dim small">
        Shows which library games have achievements. Needs your Web API key from retroachievements.org → Settings → Keys
        {status()?.has_key ? " (stored)" : ""}.
      </p>
      <div class="row">
        <input
          class="field"
          type="password"
          placeholder={status()?.has_key ? "Replace Web API key" : "Web API key"}
          value={key()}
          onInput={(e) => setKey(e.currentTarget.value)}
        />
        <button class="btn ghost" disabled={!key().trim()} onClick={() => void saveKey()}>
          Save key
        </button>
      </div>
      <div class="row">
        <button class="btn" disabled={busy() || !status()?.has_key} onClick={() => void sync()}>
          Sync achievements
        </button>
        <span class="dim small">last sync: {synced()}</span>
      </div>
      <Show when={busy()} fallback={<Show when={msg()}><p class="dim small">{msg()}</p></Show>}>
        <PhaseProgress
          event="cheevos://progress"
          labels={{ sync: "downloading game lists", hash: "hashing library" }}
        />
      </Show>
    </Panel>
  );
}
