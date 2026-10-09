// Settings → RetroArch → Picture: global shader preset (searchable) and aspect ratio.
import { createMemo, createResource, createSignal, For, onCleanup, Show } from "solid-js";
import Pager, { createPaged } from "./Pager";
import Panel from "./Panel";
import Select from "./Select";
import { onPlayEnded, raRunning, raSetAspect, raSetShader, raShaders, raVideo } from "../ipc";

const ASPECTS = [
  { value: "", label: "RetroArch decides", hint: "its own menu setting" },
  { value: "core", label: "Core provided" },
  { value: "4:3", label: "4:3" },
  { value: "16:9", label: "16:9" },
  { value: "square", label: "Square pixels", hint: "1:1 PAR" },
  { value: "full", label: "Stretch to window" },
];

export default function PicturePanel() {
  const [video, { refetch }] = createResource(raVideo);
  const [load, setLoad] = createSignal(false);
  const [presets] = createResource(load, () => raShaders());
  const [running, { refetch: refetchRunning }] = createResource(raRunning);
  const poll = setInterval(() => void refetchRunning(), 3000);
  const unlisten = onPlayEnded(() => void refetchRunning());
  onCleanup(() => (clearInterval(poll), void unlisten.then((f) => f())));
  const [query, setQuery] = createSignal("");
  const [msg, setMsg] = createSignal("");
  const matches = createMemo(() => {
    const words = query().toLowerCase().split(/\s+/).filter(Boolean);
    return (presets() ?? []).filter((p) => words.every((w) => p.toLowerCase().includes(w)));
  });
  const paged = createPaged(matches, () => 12);
  const run = async (f: () => Promise<unknown>, ok: string) => {
    try {
      const live = await f();
      setMsg(live === true ? `${ok} · applied to the running game` : ok);
    } catch (e) {
      setMsg(String(e));
    }
    void refetch();
  };
  const shader = () => video()?.shader ?? null;
  const shaderLabel = () =>
    shader() === null ? "RetroArch decides" : shader() === "off" ? "Off" : shader()!.replace(/\.slangp$/, "");
  return (
    <Panel title="Picture" class="wide">
      <p class="dim small">Applies to every game started from Romburak. Aspect ratio: next start.</p>
      <Show when={running()}>
        {(p) => <p class="small live-note">● Running: <span class="mono">{p()}</span> – shader changes apply live.</p>}
      </Show>
      <div class="row wrap">
        <label class="dim small">Aspect ratio</label>
        <Select
          value={video()?.aspect ?? ""}
          options={ASPECTS}
          onChange={(v) => void run(() => raSetAspect(v || null), "Aspect ratio saved")}
        />
      </div>
      <div class="row wrap">
        <label class="dim small">Shader</label>
        <strong class="mono small">{shaderLabel()}</strong>
        <button class="btn ghost small" disabled={shader() === "off"} onClick={() => void run(() => raSetShader("off"), "Shader off")}>
          Off
        </button>
        <button class="btn ghost small" disabled={shader() === null} onClick={() => void run(() => raSetShader(null), "Shader left to RetroArch")}>
          RetroArch decides
        </button>
        <Show when={!load()}>
          <button class="btn small" onClick={() => setLoad(true)}>Choose preset…</button>
        </Show>
      </div>
      <Show when={load()}>
        <Show when={!presets.loading} fallback={<p class="dim small">Loading shaders (first time: downloading the slang shader package, ~55 MB)…</p>}>
          <Show when={!presets.error} fallback={<p class="err small">{String(presets.error)}</p>}>
            <input
              class="field preset-search"
              type="search"
              placeholder={`Search ${presets()?.length ?? 0} presets, e.g. "crt royale"`}
              value={query()}
              onInput={(e) => setQuery(e.currentTarget.value)}
            />
            <ul class="preset-list">
              <For each={paged.items()}>
                {(p) => (
                  <li>
                    <button
                      class="preset mono small"
                      classList={{ sel: shader() === p }}
                      onClick={() => void run(() => raSetShader(p), `Shader: ${p}`)}
                    >
                      {p.replace(/\.slangp$/, "")}
                    </button>
                  </li>
                )}
              </For>
            </ul>
            <Pager paged={paged} />
          </Show>
        </Show>
      </Show>
      <Show when={msg()}><p class="dim small">{msg()}</p></Show>
    </Panel>
  );
}
