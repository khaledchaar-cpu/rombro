import { createVirtualizer } from "@tanstack/solid-virtual";
import { createMemo, createSignal, For, Show } from "solid-js";
import Panel from "../components/Panel";
import type { DecisionView } from "../ipc";
import { busy, execute, library, plan, status, undo } from "../state/importStore";

const ROW_H = 26;

function rel(p: string) {
  const lib = library();
  return lib && p.startsWith(lib) ? p.slice(lib.length).replace(/^\/+/, "") : p;
}

function OpList() {
  let scroller!: HTMLDivElement;
  const ops = () => plan()?.ops ?? [];
  const v = createVirtualizer({
    get count() { return ops().length; },
    getScrollElement: () => scroller,
    estimateSize: () => ROW_H,
    overscan: 12,
  });
  return (
    <div class="vlist" ref={scroller}>
      <div style={{ height: `${v.getTotalSize()}px`, position: "relative" }}>
        <For each={v.getVirtualItems()}>
          {(row) => {
            const op = ops()[row.index];
            return (
              <div class="vrow mono small" style={{ transform: `translateY(${row.start}px)`, height: `${ROW_H}px` }}>
                <span class={`tag tag-${op.kind}`}>{op.kind}</span>
                <span class="dim ellipsis" title={op.from ?? ""}>{op.from ?? ""}</span>
                <span class="arrow">→</span>
                <span class="ellipsis" title={op.to}>{rel(op.to)}</span>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}

const KINDS: DecisionView["kind"][] = ["ambiguous", "tie", "conflict", "rejected", "skipped"];

function Decisions() {
  const [kind, setKind] = createSignal<DecisionView["kind"] | "all">("all");
  const all = () => plan()?.decisions ?? [];
  const counts = createMemo(() => {
    const c = new Map<string, number>();
    for (const d of all()) c.set(d.kind, (c.get(d.kind) ?? 0) + 1);
    return c;
  });
  const shown = () => (kind() === "all" ? all() : all().filter((d) => d.kind === kind())).slice(0, 500);
  return (
    <>
      <div class="row wrap">
        <button class="btn ghost" classList={{ active: kind() === "all" }} onClick={() => setKind("all")}>
          all {all().length}
        </button>
        <For each={KINDS.filter((k) => counts().get(k))}>
          {(k) => (
            <button class="btn ghost" classList={{ active: kind() === k }} onClick={() => setKind(k)}>
              {k} {counts().get(k)}
            </button>
          )}
        </For>
      </div>
      <ul class="decisions">
        <For each={shown()}>
          {(d) => (
            <li>
              <span class={`tag tag-${d.kind}`}>{d.kind}</span>
              <span class="mono small wrap-any">{d.path}</span>
              <Show when={d.detail}>
                <span class="dim small">{d.detail}</span>
              </Show>
              <For each={d.options}>{(o) => <span class="dim small mono">? {o}</span>}</For>
            </li>
          )}
        </For>
      </ul>
    </>
  );
}

export default function Plan() {
  return (
    <div class="grid">
      <Show
        when={plan()}
        fallback={
          <Panel title="Plan" class="wide">
            <p class="dim">No plan yet – build one in the Inbox view.</p>
            <Show when={status()}>{(s) => <p class={`mono ${s().ok ? "ok" : "err"}`}>{s().text}</p>}</Show>
            <button class="btn ghost" disabled={busy()} onClick={undo}>
              Undo last run
            </button>
          </Panel>
        }
      >
        {(p) => (
          <>
            <Panel title="Summary" class="wide">
              <div class="kpis">
                <div><div class="kpi">{p().ops.length}</div><span class="dim">operations</span></div>
                <div><div class="kpi">{p().placed}</div><span class="dim">to place</span></div>
                <div><div class="kpi">{p().unchanged}</div><span class="dim">unchanged</span></div>
                <div><div class="kpi">{p().quarantined}</div><span class="dim">quarantine</span></div>
                <div><div class="kpi">{p().decisions.length}</div><span class="dim">need attention</span></div>
              </div>
              <div class="row">
                <span class="dim small">dry run · {p().items} items scanned</span>
                <span class="spacer" />
                <button class="btn" disabled={busy() || !p().ops.length} onClick={execute}>
                  {busy() ? "Executing" : "Execute"}
                </button>
              </div>
            </Panel>
            <Panel title="Operations" class="wide">
              <OpList />
            </Panel>
            <Show when={p().decisions.length}>
              <Panel title="Needs attention" class="wide">
                <Decisions />
              </Panel>
            </Show>
          </>
        )}
      </Show>
    </div>
  );
}
