import { createResource, For, Match, Switch } from "solid-js";
import CheevosPanel from "../components/CheevosPanel";
import DatabasePanel from "../components/DatabasePanel";
import Panel from "../components/Panel";
import { thumbsOnlineGet, thumbsOnlineSet } from "../ipc";
import { gamifyEnabled, setGamifyEnabled } from "../state/gamify";
import { effects, setEffects, setTheme, theme, THEMES } from "../state/appearance";
import { SECTIONS, setSystemSection, systemSection } from "../state/settingsNav";

function Appearance() {
  const [online, { mutate: setOnline }] = createResource(thumbsOnlineGet);
  const toggleOnline = async (on: boolean) => (await thumbsOnlineSet(on), setOnline(on));
  return (
    <div class="settings">
      <Panel title="Appearance">
        <div class="row">
          <For each={THEMES}>
            {(t) => (
              <button type="button" class="btn ghost" classList={{ active: theme() === t.id }} onClick={() => setTheme(t.id)}>
                {t.label}
              </button>
            )}
          </For>
        </div>
        <div class="row">
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
    </div>
  );
}

/** Maintenance and preferences: game databases, appearance. */
export default function System() {
  return (
    <div class="settings-shell">
      <nav class="settings-nav">
        <For each={SECTIONS}>
          {(s) => (
            <button
              type="button"
              class="settings-tab"
              classList={{ active: systemSection() === s.id }}
              onClick={() => setSystemSection(s.id)}
            >
              {s.label}
            </button>
          )}
        </For>
      </nav>
      <div class="settings-body">
        <Switch>
          <Match when={systemSection() === "databases"}>
            <div class="settings">
              <DatabasePanel />
              <CheevosPanel />
            </div>
          </Match>
          <Match when={systemSection() === "appearance"}>
            <Appearance />
          </Match>
        </Switch>
      </div>
    </div>
  );
}
