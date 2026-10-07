import { createResource, For, Match, Switch } from "solid-js";
import DatabasePanel from "../components/DatabasePanel";
import Panel from "../components/Panel";
import RetroArchPanel from "../components/RetroArchPanel";
import { thumbsOnlineGet, thumbsOnlineSet } from "../ipc";
import { gamifyEnabled, setGamifyEnabled } from "../state/gamify";
import { effects, setEffects, setTheme, theme } from "../state/appearance";
import { SECTIONS, setSettingsSection, settingsSection } from "../state/settingsNav";
import Rules from "./Rules";

function Appearance() {
  const [online, { mutate: setOnline }] = createResource(thumbsOnlineGet);
  const toggleOnline = async (on: boolean) => (await thumbsOnlineSet(on), setOnline(on));
  return (
    <div class="settings">
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
    </div>
  );
}

/** Everything that configures RomBro: planner rules, databases, RetroArch, appearance. */
export default function Settings() {
  return (
    <div class="settings-shell">
      <nav class="settings-nav">
        <For each={SECTIONS}>
          {(s) => (
            <button
              type="button"
              class="settings-tab"
              classList={{ active: settingsSection() === s.id }}
              onClick={() => setSettingsSection(s.id)}
            >
              {s.label}
            </button>
          )}
        </For>
      </nav>
      <div class="settings-body">
        <Switch>
          <Match when={settingsSection() === "rules"}>
            <Rules />
          </Match>
          <Match when={settingsSection() === "databases"}>
            <div class="settings">
              <DatabasePanel />
            </div>
          </Match>
          <Match when={settingsSection() === "retroarch"}>
            <div class="settings">
              <RetroArchPanel />
            </div>
          </Match>
          <Match when={settingsSection() === "appearance"}>
            <Appearance />
          </Match>
        </Switch>
      </div>
    </div>
  );
}
