// Filter bar for the Library table: text, system, state, region and added-date filters.
import { createMemo, createSignal, For, Show, type Accessor } from "solid-js";
import Select, { type Option } from "../components/Select";
import type { LibraryRow } from "../ipc";

const opts = (all: string, list: [string, number][]): Option<string>[] => [
  { value: "", label: all },
  ...list.map(([s, n]) => ({ value: s, label: s, hint: String(n) })),
];

const STATES = ["known", "ambiguous", "unknown", "skip"] as const;
const AGES = [
  { label: "Any time", secs: 0 },
  { label: "Last 24 h", secs: 86400 },
  { label: "Last 7 days", secs: 7 * 86400 },
  { label: "Last 30 days", secs: 30 * 86400 },
  { label: "Last 90 days", secs: 90 * 86400 },
];

export function createLibraryFilter(rows: Accessor<LibraryRow[]>) {
  const [query, setQuery] = createSignal("");
  const [system, setSystem] = createSignal("");
  const [region, setRegion] = createSignal("");
  const [age, setAge] = createSignal(0);
  const [states, setStates] = createSignal<Set<string>>(new Set());

  const count = (key: (r: LibraryRow) => string[]) => {
    const m = new Map<string, number>();
    for (const r of rows()) for (const k of key(r)) if (k) m.set(k, (m.get(k) ?? 0) + 1);
    return [...m].sort((a, b) => a[0].localeCompare(b[0]));
  };
  const systems = createMemo(() => count((r) => [r.system]));
  const regions = createMemo(() => count((r) => r.regions));
  const stateCounts = createMemo(() => new Map(count((r) => [r.state])));

  const filtered = createMemo(() => {
    const q = query().toLowerCase();
    const [sys, reg, st] = [system(), region(), states()];
    const since = age() ? Date.now() / 1000 - age() : 0;
    return rows().filter(
      (r) =>
        (!sys || r.system === sys) &&
        (!reg || r.regions.includes(reg)) &&
        (!st.size || st.has(r.state)) &&
        r.added >= since &&
        (!q || r.name.toLowerCase().includes(q) || r.path.toLowerCase().includes(q) || r.system.toLowerCase().includes(q)),
    );
  });

  const active = () => !!(query() || system() || region() || age() || states().size);
  const reset = () => (setQuery(""), setSystem(""), setRegion(""), setAge(0), setStates(new Set<string>()));
  const toggleState = (s: string) =>
    setStates((cur) => {
      const n = new Set(cur);
      if (n.has(s)) n.delete(s);
      else n.add(s);
      return n;
    });

  function Bar() {
    return (
      <div class="filters">
        <input class="field" placeholder="Filter…" value={query()} onInput={(e) => setQuery(e.currentTarget.value)} />
        <Select value={system()} onChange={setSystem} options={opts("All systems", systems())} />
        <Select value={region()} onChange={setRegion} options={opts("All regions", regions())} />
        <Select value={age()} onChange={setAge} options={AGES.map((a) => ({ value: a.secs, label: a.label }))} />
        <div class="chips">
          <For each={STATES}>
            {(s) => (
              <button
                class={`chip tag tag-${s}`}
                classList={{ on: states().has(s) }}
                disabled={!stateCounts().get(s)}
                onClick={() => toggleState(s)}
              >
                {s} {stateCounts().get(s) ?? 0}
              </button>
            )}
          </For>
          <Show when={active()}>
            <button class="btn" onClick={reset}>Reset</button>
          </Show>
        </div>
      </div>
    );
  }

  return { filtered, Bar };
}
