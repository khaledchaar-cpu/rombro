import { createResource } from "solid-js";
import Panel from "../components/Panel";
import { thumbsOnlineGet, thumbsOnlineSet } from "../ipc";
import { gamifyEnabled, setGamifyEnabled } from "../state/gamify";
import { effects, setEffects, setTheme, theme } from "../state/appearance";

export default function Settings() {
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
