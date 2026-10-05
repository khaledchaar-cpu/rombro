import { For, Show, createResource, createSignal } from "solid-js";
import Panel from "../components/Panel";
import PriorityList from "../components/PriorityList";
import { FLAG_KEYS, type FlagKey, type Rules, rulesGet, rulesSet, thumbsOnlineGet, thumbsOnlineSet } from "../ipc";
import { gamifyEnabled, setGamifyEnabled } from "../state/gamify";
import { effects, setEffects, setTheme, theme } from "../state/appearance";

const FLAG_LABELS: Record<FlagKey, string> = {
  beta: "Beta", proto: "Prototype", demo: "Demo", kiosk: "Kiosk", sample: "Sample",
  unlicensed: "Unlicensed", pirate: "Pirate", bios: "BIOS", aftermarket: "Aftermarket",
  virtual_console: "Virtual Console", rerelease: "Digital re-release", hack: "Hack",
  translation: "Translation", bad_dump: "Bad dump", alt: "Alt dump",
};

export default function Settings() {
  const [saved, { mutate }] = createResource(rulesGet);
  const [draft, setDraft] = createSignal<Rules | null>(null);
  const [msg, setMsg] = createSignal("");
  const rules = () => draft() ?? saved();
  const [online, { mutate: setOnline }] = createResource(thumbsOnlineGet);
  const toggleOnline = async (on: boolean) => (await thumbsOnlineSet(on), setOnline(on));
  const dirty = () => draft() !== null && JSON.stringify(draft()) !== JSON.stringify(saved());

  const edit = (patch: Partial<Rules>) => {
    const r = rules();
    if (r) setDraft({ ...r, ...patch });
  };
  const save = async (r: Rules | null) => {
    try {
      mutate(await rulesSet(r));
      setDraft(null);
      setMsg(r ? "Saved – applies to the next plan." : "Reset to defaults.");
    } catch (e) {
      setMsg(`Error: ${e}`);
    }
  };

  return (
    <Show when={rules()} fallback={<p class="dim mono">loading…</p>}>
      {(r) => (
        <div class="settings">
          <Panel title="1G1R · Region priority">
            <p class="dim">Best first. Unlisted regions rank after all listed ones.</p>
            <PriorityList items={r().regions} onChange={(regions) => edit({ regions })} placeholder="Add region, e.g. Australia" />
          </Panel>
          <Panel title="1G1R · Language priority">
            <p class="dim">Tie-breaker after region.</p>
            <PriorityList items={r().languages} onChange={(languages) => edit({ languages })} placeholder="Add language code, e.g. Fr" />
          </Panel>
          <Panel title="1G1R · Never pick">
            <div class="flag-grid">
              <For each={FLAG_KEYS}>
                {(k) => (
                  <label class="check">
                    <input
                      type="checkbox"
                      checked={r().exclude[k]}
                      onChange={(e) => edit({ exclude: { ...r().exclude, [k]: e.currentTarget.checked } })}
                    />
                    {FLAG_LABELS[k]}
                  </label>
                )}
              </For>
            </div>
          </Panel>
          <Panel title="Appearance">
            <div class="row">
              <label class="check">
                <input type="checkbox" checked={theme() === "light"} onChange={(e) => setTheme(e.currentTarget.checked ? "light" : "dark")} />
                Light theme
              </label>
              <label class="check">
                <input type="checkbox" checked={effects()} onChange={(e) => setEffects(e.currentTarget.checked)} />
                Effects (glow, grid, chromatic edges)
              </label>
            </div>
          </Panel>
          <Panel title="Gamification">
            <label class="check">
              <input type="checkbox" checked={gamifyEnabled()} onChange={(e) => void setGamifyEnabled(e.currentTarget.checked)} />
              Show KPIs, completeness and achievements on the dashboard
            </label>
          </Panel>
          <Panel title="Thumbnails">
            <label class="check">
              <input type="checkbox" checked={online() ?? true} onChange={(e) => void toggleOnline(e.currentTarget.checked)} />
              Download missing thumbnails online
            </label>
            <p class="dim small">
              Images: <span class="mono">libretro-thumbnails</span> (thumbnails.libretro.com) · © respective owners.
              Already downloaded images stay available offline.
            </p>
          </Panel>
          <div class="row">
            <button class="btn" disabled={!dirty()} onClick={() => save(rules()!)}>Save</button>
            <button class="btn ghost" disabled={!dirty()} onClick={() => setDraft(null)}>Discard</button>
            <button class="btn ghost" onClick={() => save(null)}>Reset to defaults</button>
            <span class="dim mono">{msg()}</span>
          </div>
        </div>
      )}
    </Show>
  );
}
