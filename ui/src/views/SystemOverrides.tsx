import { For, Show, createSignal } from "solid-js";
import PriorityList from "../components/PriorityList";
import type { Rules, SystemRules } from "../ipc";

/** Per-system region/language overrides; unset lists fall back to the global ones. */
export default function SystemOverrides(props: {
  rules: Rules;
  onChange: (systems: Rules["systems"]) => void;
}) {
  const [name, setName] = createSignal("");
  const set = (sys: string, o: SystemRules | null) => {
    const next = { ...props.rules.systems };
    if (o) next[sys] = o;
    else delete next[sys];
    props.onChange(next);
  };
  const add = () => {
    const sys = name().trim();
    if (!sys || props.rules.systems[sys]) return;
    set(sys, { regions: [...props.rules.regions], languages: null, exclude: null });
    setName("");
  };
  const list = (sys: string, o: SystemRules, key: "regions" | "languages") => (
    <Show
      when={o[key]}
      fallback={
        <button class="btn ghost" onClick={() => set(sys, { ...o, [key]: [...props.rules[key]] })}>
          Override {key}
        </button>
      }
    >
      {(items) => (
        <div>
          <span class="dim small">{key} </span>
          <button class="btn ghost" onClick={() => set(sys, { ...o, [key]: null })}>use global</button>
          <PriorityList items={items()} onChange={(v) => set(sys, { ...o, [key]: v })} placeholder={`Add to ${key}`} />
        </div>
      )}
    </Show>
  );
  return (
    <div class="overrides">
      <For each={Object.entries(props.rules.systems)}>
        {([sys, o]) => (
          <div class="override">
            <div class="row">
              <strong class="mono">{sys}</strong>
              <button class="btn ghost" onClick={() => set(sys, null)}>Remove</button>
            </div>
            {list(sys, o, "regions")}
            {list(sys, o, "languages")}
          </div>
        )}
      </For>
      <div class="row">
        <input
          class="field mono"
          placeholder="System, e.g. NEC - PC Engine - TurboGrafx 16"
          value={name()}
          onInput={(e) => setName(e.currentTarget.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
        />
        <button class="btn ghost" onClick={add}>Add override</button>
      </div>
    </div>
  );
}
