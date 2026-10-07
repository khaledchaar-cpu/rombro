import { For, type JSX, Match, Show, Switch, createResource, createSignal } from "solid-js";
import Panel from "../components/Panel";
import PriorityList from "../components/PriorityList";
import { FLAG_KEYS, type FlagKey, type RuleInfo, type Rules as R, rulesCatalog, rulesDefaults, rulesGet, rulesSet } from "../ipc";
import Exceptions from "./Exceptions";
import NameFolders from "./NameFolders";
import SystemOverrides from "./SystemOverrides";

const FLAG_LABELS: Record<FlagKey, string> = {
  beta: "Beta", proto: "Prototype", demo: "Demo", kiosk: "Kiosk", sample: "Sample",
  unlicensed: "Unlicensed", pirate: "Pirate", bios: "BIOS", aftermarket: "Aftermarket",
  virtual_console: "Virtual Console", rerelease: "Digital re-release", hack: "Hack",
  translation: "Translation", bad_dump: "Bad dump", alt: "Alt dump",
};

/** Settings fields each rule owns (reset restores exactly these). */
const FIELDS: Record<string, (keyof R)[]> = {
  "g1r-pick": ["regions", "languages", "exclude", "systems"],
  "arcade-set": ["arcade_order", "arcade_g1r"],
  "game-folder": ["folder_systems"],
  "arcade-dat": ["arcade_working_only"],
  "name-only": ["name_folders"],
  quarantine: ["quarantine", "unknown_to_trash"],
  "frontend-meta": ["frontend_trash"],
};

/** Every planner rule: what it does, how often it fired in the last plan, its settings. */
export default function Rules() {
  const [catalog] = createResource(rulesCatalog);
  const [defaults] = createResource(rulesDefaults);
  const [saved, { mutate }] = createResource(rulesGet);
  const [draft, setDraft] = createSignal<R | null>(null);
  const [msg, setMsg] = createSignal("");
  const rules = () => draft() ?? saved();
  const dirty = () => draft() !== null && JSON.stringify(draft()) !== JSON.stringify(saved());
  const edit = (patch: Partial<R>) => {
    const r = rules();
    if (r) setDraft({ ...r, ...patch });
  };
  const reset = (id: string) => {
    const d = defaults();
    if (d) edit(Object.fromEntries(FIELDS[id].map((k) => [k, d[k]])) as Partial<R>);
  };
  const save = async () => {
    try {
      mutate(await rulesSet(rules()!));
      setDraft(null);
      setMsg("Saved – run an audit to see what changes in the library.");
    } catch (e) {
      setMsg(`Error: ${e}`);
    }
  };

  const editor = (info: RuleInfo, r: R): JSX.Element => (
    <Switch>
      <Match when={info.id === "g1r-pick"}>
        <h4>Region priority</h4>
        <PriorityList items={r.regions} onChange={(regions) => edit({ regions })} placeholder="Add region, e.g. Australia" />
        <h4>Language priority</h4>
        <PriorityList items={r.languages} onChange={(languages) => edit({ languages })} placeholder="Add language code, e.g. Fr" />
        <h4>Never pick</h4>
        <div class="flag-grid">
          <For each={FLAG_KEYS}>
            {(k) => (
              <label class="check">
                <input type="checkbox" checked={r.exclude[k]} onChange={(e) => edit({ exclude: { ...r.exclude, [k]: e.currentTarget.checked } })} />
                {FLAG_LABELS[k]}
              </label>
            )}
          </For>
        </div>
        <h4>Per system</h4>
        <SystemOverrides rules={r} onChange={(systems) => edit({ systems })} />
      </Match>
      <Match when={info.id === "arcade-set"}>
        <label class="check">
          <input type="checkbox" checked={r.arcade_g1r} onChange={(e) => edit({ arcade_g1r: e.currentTarget.checked })} />
          1G1R for arcade (off: place every matching set)
        </label>
        <h4>Database priority</h4>
        <PriorityList items={r.arcade_order} onChange={(arcade_order) => edit({ arcade_order })} placeholder="Add database" />
      </Match>
      <Match when={info.id === "game-folder"}>
        <h4>Folder systems</h4>
        <PriorityList items={r.folder_systems} onChange={(folder_systems) => edit({ folder_systems })} placeholder="Add system" />
      </Match>
      <Match when={info.id === "name-only"}>
        <h4>Name folders</h4>
        <NameFolders folders={r.name_folders} onChange={(name_folders) => edit({ name_folders })} />
      </Match>
      <Match when={info.id === "arcade-dat"}>
        <label class="check">
          <input
            type="checkbox"
            checked={r.arcade_working_only}
            onChange={(e) => edit({ arcade_working_only: e.currentTarget.checked })}
          />
          Skip sets the core marks as not working (off: place them anyway)
        </label>
      </Match>
      <Match when={info.id === "quarantine"}>
        <label class="check">
          <input type="checkbox" checked={r.quarantine} onChange={(e) => edit({ quarantine: e.currentTarget.checked })} />
          Move unknown files out of the inbox and system folders (off: leave them where they are)
        </label>
        <label class="check">
          <input
            type="checkbox"
            checked={r.unknown_to_trash}
            onChange={(e) => edit({ unknown_to_trash: e.currentTarget.checked })}
          />
          …into _trash/unknown, and empty the old _quarantine there (off: keep them in _quarantine)
        </label>
      </Match>
      <Match when={info.id === "frontend-meta"}>
        <label class="check">
          <input
            type="checkbox"
            checked={r.frontend_trash}
            onChange={(e) => edit({ frontend_trash: e.currentTarget.checked })}
          />
          Move frontend metadata to _trash/frontend (off: treat it like any unknown file)
        </label>
      </Match>
    </Switch>
  );

  // one rule at a time: the sub-navigation lists every rule plus the exceptions
  const [section, setSection] = createSignal("g1r-pick");
  return (
    <Show when={rules() && catalog()} fallback={<p class="dim mono">loading…</p>}>
      <div class="settings-shell">
        <nav class="settings-nav">
          <For each={catalog()}>
            {(info) => (
              <button
                type="button"
                class="settings-tab"
                classList={{ active: section() === info.id }}
                onClick={() => setSection(info.id)}
              >
                {info.title}
                <span class="settings-tab-hits">{info.hits}</span>
              </button>
            )}
          </For>
          <button
            type="button"
            class="settings-tab"
            classList={{ active: section() === "exceptions" }}
            onClick={() => setSection("exceptions")}
          >
            Exceptions
          </button>
        </nav>
        <div class="settings">
          <div class="row save-bar">
            <button class="btn" disabled={!dirty()} onClick={save}>Save</button>
            <button class="btn ghost" disabled={!dirty()} onClick={() => setDraft(null)}>Discard</button>
            <span class="dim mono">{msg()}</span>
          </div>
          <For each={catalog()?.filter((info) => info.id === section())}>
            {(info) => (
              <Panel title={info.title}>
                <div class="row rule-head">
                  <span class="tag mono">{info.id}</span>
                  <span class="dim mono">{info.hits} ops in last plan</span>
                  <Show when={FIELDS[info.id]}>
                    <button class="btn ghost" onClick={() => reset(info.id)}>Reset</button>
                  </Show>
                </div>
                <p class="dim">{info.explain}</p>
                {editor(info, rules()!)}
              </Panel>
            )}
          </For>
          <Show when={section() === "exceptions"}>
            <Exceptions />
          </Show>
        </div>
      </div>
    </Show>
  );
}
