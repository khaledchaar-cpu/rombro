import { createResource, For, Show } from "solid-js";
import Select from "./Select";
import { retroarchCores, rulesGet, rulesSet, type CoreOption } from "../ipc";

const label = (o: CoreOption) =>
  `${o.name}${o.recommended ? " · recommended" : ""}${o.installed ? "" : " · installs on play"}`;

/** Core per system: recommended (Batocera defaults) preselected, the user's pick stored in the rules. */
export default function CorePicker() {
  const [systems, { refetch }] = createResource(retroarchCores);
  const choose = async (system: string, id: string, recommended: boolean) => {
    const rules = await rulesGet();
    const cores = { ...rules.cores };
    if (recommended) delete cores[system];
    else cores[system] = id;
    await rulesSet({ ...rules, cores });
    await refetch();
  };
  return (
    <Show when={systems()?.length} fallback={<p class="dim small">{systems.loading ? "loading cores…" : systems.error ? String(systems.error) : "no systems in the library"}</p>}>
      <div class="ra-cores">
        <p class="dim small">Core a game starts with, unless chosen per game in its details.</p>
        <For each={systems()}>
          {(s) => (
            <div class="row ex-row">
              <span class="ellipsis">{s.system}</span>
              <Show when={s.options.length} fallback={<span class="dim">no core known</span>}>
                <Select
                  value={s.chosen ?? ""}
                  options={s.options.map((o) => ({ value: o.id, label: label(o) }))}
                  onChange={(id) =>
                    void choose(s.system, id, s.options.find((o) => o.id === id)?.recommended ?? false)
                  }
                />
              </Show>
            </div>
          )}
        </For>
      </div>
    </Show>
  );
}
