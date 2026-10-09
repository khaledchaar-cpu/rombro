// Filter bar for the Library table: text (name + system), systems, regions, release decades (all
// multiple choice), favorite/played/achievement and state chips.
import { createMemo, createSignal, For, Show, type Accessor } from "solid-js";
import MultiSelect from "../components/MultiSelect";
import type { Option } from "../components/Select";
import type { LibraryRow } from "../ipc";

const opts = (list: [string, number][]): Option<string>[] => list.map(([s, n]) => ({ value: s, label: s, hint: String(n) }));

export const UNKNOWN_YEAR = "unknown";
/** Release decade of a row, e.g. `1990s`, or `unknown`. */
export const decade = (r: LibraryRow) => (r.year ? `${Math.floor(r.year / 10) * 10}s` : UNKNOWN_YEAR);

/** Search text of a system: its name plus acronyms of multi-word parts ("Nintendo Entertainment System" → nes). */
const sysWords = new Map<string, string>();
function systemText(system: string): string {
  let t = sysWords.get(system);
  if (t == null) {
    const acr = system
      .split(" - ")
      .map((part) => part.split(/\s+/).filter((w) => /^[A-Za-z]/.test(w)))
      .filter((w) => w.length > 1)
      .map((w) => w.map((x) => x[0]).join(""));
    t = `${system} ${acr.join(" ")}`.toLowerCase();
    sysWords.set(system, t);
  }
  return t;
}

const toggled = (cur: Set<string>, s: string) => {
  const n = new Set(cur);
  if (n.has(s)) n.delete(s);
  else n.add(s);
  return n;
};

const STATES = ["known", "named", "ambiguous", "unknown", "skip"] as const;

export function createLibraryFilter(rows: Accessor<LibraryRow[]>) {
  const [query, setQuery] = createSignal("");
  const [systems, setSystems] = createSignal<Set<string>>(new Set());
  const [regions, setRegions] = createSignal<Set<string>>(new Set());
  const [decades, setDecades] = createSignal<Set<string>>(new Set());
  const [states, setStates] = createSignal<Set<string>>(new Set());
  const [favOnly, setFavOnly] = createSignal(false);
  const [playedOnly, setPlayedOnly] = createSignal(false);
  const [cheevosOnly, setCheevosOnly] = createSignal(false);

  const count = (key: (r: LibraryRow) => string[]) => {
    const m = new Map<string, number>();
    for (const r of rows()) for (const k of key(r)) if (k) m.set(k, (m.get(k) ?? 0) + 1);
    return [...m].sort((a, b) => a[0].localeCompare(b[0]));
  };
  const systemOpts = createMemo(() => opts(count((r) => [r.system])));
  const regionOpts = createMemo(() => opts(count((r) => r.regions)));
  // decades oldest first, unknown last
  const decadeOpts = createMemo(() =>
    opts(count((r) => [decade(r)]).sort((a, b) => +(a[0] === UNKNOWN_YEAR) - +(b[0] === UNKNOWN_YEAR) || a[0].localeCompare(b[0]))),
  );
  const stateCounts = createMemo(() => new Map(count((r) => [r.state])));
  const favCount = createMemo(() => rows().filter((r) => r.favorite).length);
  const playedCount = createMemo(() => rows().filter((r) => r.plays > 0).length);
  const cheevosCount = createMemo(() => rows().filter((r) => r.cheevos > 0).length);

  const filtered = createMemo(() => {
    // every word must occur in name or system ("arkanoid cpc")
    const words = query().toLowerCase().split(/\s+/).filter(Boolean);
    const [sys, reg, dec, st, fav, played, ach] = [systems(), regions(), decades(), states(), favOnly(), playedOnly(), cheevosOnly()];
    return rows().filter(
      (r) =>
        (!sys.size || sys.has(r.system)) &&
        (!reg.size || r.regions.some((x) => reg.has(x))) &&
        (!dec.size || dec.has(decade(r))) &&
        (!st.size || st.has(r.state)) &&
        (!fav || r.favorite) &&
        (!played || r.plays > 0) &&
        (!ach || r.cheevos > 0) &&
        (!words.length || ((h) => words.every((w) => h.includes(w)))(`${r.name.toLowerCase()} ${systemText(r.system)}`)),
    );
  });

  const active = () => !!(query() || systems().size || regions().size || decades().size || states().size || favOnly() || playedOnly() || cheevosOnly());
  const reset = () => (
    setQuery(""), setSystems(new Set<string>()), setRegions(new Set<string>()), setDecades(new Set<string>()), setStates(new Set<string>()), setFavOnly(false), setPlayedOnly(false), setCheevosOnly(false)
  );
  const toggleState = (s: string) => setStates((cur) => toggled(cur, s));

  function Bar() {
    return (
      <div class="filters">
        <input class="field" placeholder="Filter name + system…" value={query()} onInput={(e) => setQuery(e.currentTarget.value)} />
        <MultiSelect values={systems()} onChange={setSystems} options={systemOpts()} all="All systems" noun="systems" />
        <MultiSelect values={regions()} onChange={setRegions} options={regionOpts()} all="All regions" noun="regions" />
        <MultiSelect values={decades()} onChange={setDecades} options={decadeOpts()} all="Any release year" noun="decades" />
        <div class="chips">
          <button class="chip" classList={{ on: favOnly() }} disabled={!favCount()} onClick={() => setFavOnly((v) => !v)}>
            ★ favorites {favCount()}
          </button>
          <button class="chip" classList={{ on: playedOnly() }} disabled={!playedCount()} onClick={() => setPlayedOnly((v) => !v)}>
            played {playedCount()}
          </button>
          <button class="chip" classList={{ on: cheevosOnly() }} disabled={!cheevosCount()} onClick={() => setCheevosOnly((v) => !v)}>
            🏆 achievements {cheevosCount()}
          </button>
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
