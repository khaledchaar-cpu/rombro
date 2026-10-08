import { createResource, createSignal, Show } from "solid-js";
import Select from "./Select";
import PhaseProgress from "./PhaseProgress";
import { gameCores, play, setGameCore, type CoreOption } from "../ipc";

const label = (o: CoreOption) =>
  `${o.name}${o.recommended ? " · recommended" : ""}${o.installed ? "" : " · installs on play"}`;

const LABELS = {
  download: "downloading RetroArch (MB)",
  verify: "verifying RetroArch",
  unpack: "unpacking RetroArch (MB)",
  info: "loading core list",
  core: "installing core",
  assets: "downloading system files",
};

/** Play button and core choice for one game (managed RetroArch). */
export default function GamePlay(props: { path: string }) {
  const [cores, { refetch }] = createResource(() => props.path, gameCores);
  const [busy, setBusy] = createSignal(false);
  const [msg, setMsg] = createSignal("");
  const start = async () => {
    setBusy(true);
    setMsg("");
    try {
      const core = await play(props.path);
      setMsg(`Started with ${core}`);
    } catch (e) {
      setMsg(String(e));
    } finally {
      setBusy(false);
      void refetch();
    }
  };
  const choose = async (id: string) => {
    await setGameCore(props.path, id);
    void refetch();
  };
  return (
    <div class="game-play">
      <div class="row">
        <button class="btn" disabled={busy() || !cores()?.chosen} onClick={() => void start()}>
          ▶ Play
        </button>
        <Show when={cores()?.options.length} fallback={<span class="dim small">{cores.loading ? "…" : "no core known"}</span>}>
          <Select
            value={cores()?.chosen ?? ""}
            options={cores()!.options.map((o) => ({ value: o.id, label: label(o) }))}
            onChange={(id) => void choose(id)}
          />
          <Show when={cores()?.overridden}>
            <button class="btn ghost small" onClick={() => void setGameCore(props.path, null).then(() => refetch())}>
              Reset to system core
            </button>
          </Show>
        </Show>
      </div>
      <Show when={busy()} fallback={<Show when={msg()}><p class="dim small">{msg()}</p></Show>}>
        <PhaseProgress event="play://progress" labels={LABELS} />
      </Show>
    </div>
  );
}
