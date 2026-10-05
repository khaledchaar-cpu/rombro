import { createVirtualizer } from "@tanstack/solid-virtual";
import { createMemo, createSignal, For, Show } from "solid-js";
import Panel from "../components/Panel";
import ScanProgress from "../components/ScanProgress";
import type { DecisionView } from "../ipc";
import {
  busy,
  decided,
  decisionKey,
  execute,
  judge,
  library,
  pick,
  plan,
  prefer,
  replan,
  status,
  undo,
} from "../state/importStore";

const ROW_H = 26;

function rel(p: string) {
  const lib = library();
  return lib && p.startsWith(lib) ? p.slice(lib.length).replace(/^\/+/, "") : p;
}

function OpList() {
  let scroller!: HTMLDivElement;
  const ops = () => plan()?.ops ?? [];
  const v = createVirtualizer({
    get count() {
      return ops().length;
    },
    getScrollElement: () => scroller,
    estimateSize: () => ROW_H,
    overscan: 12,
  });
  return (
    <div class="vlist" ref={scroller}>
      <div style={{ height: `${v.getTotalSize()}px`, position: "relative" }}>
        <For each={v.getVirtualItems()}>
          {(row) => {
            const op = () =>
              ops()[row.index] ?? { kind: "move", from: null, to: "", why: "" };
            return (
              <div
                class="vrow oprow mono small"
                style={{
                  transform: `translateY(${row.start}px)`,
                  height: `${ROW_H}px`,
                }}
              >
                <span class={`tag tag-${op().kind}`}>{op().kind}</span>
                <span class="dim ellipsis" title={op().from ?? ""}>
                  {op().from ?? ""}
                </span>
                <span class="arrow">→</span>
                <span class="ellipsis" title={op().to}>
                  {rel(op().to)}
                </span>
                <span class="ellipsis why" title={op().why}>
                  {op().why}
                </span>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}

function Actions(props: { d: DecisionView }) {
  const d = props.d;
  if (d.kind === "ambiguous")
    return (
      <div class="row wrap">
        <For each={d.options}>
          {(c) => (
            <button
              class="btn ghost small"
              disabled={busy()}
              onClick={() => pick(d, c)}
              title={c.system}
            >
              {c.name}
            </button>
          )}
        </For>
      </div>
    );
  if (d.kind === "rejected" && d.options.length)
    return (
      <div class="row">
        <Show when={d.can_keep}>
          <button
            class="btn ghost small"
            disabled={busy()}
            onClick={() => judge(d, "keep")}
          >
            Keep
          </button>
        </Show>
        <button
          class="btn ghost small"
          disabled={busy()}
          onClick={() => judge(d, "discard")}
        >
          Trash
        </button>
      </div>
    );
  if (d.kind === "tie")
    return (
      <div class="row wrap">
        <For each={d.options}>
          {(c) => (
            <button
              class="btn ghost small"
              disabled={busy()}
              onClick={() => prefer(d, c)}
            >
              {c.name}
            </button>
          )}
        </For>
      </div>
    );
  return null;
}

const KINDS: DecisionView["kind"][] = [
  "ambiguous",
  "tie",
  "conflict",
  "rejected",
  "skipped",
];

function Decisions() {
  const [kind, setKind] = createSignal<DecisionView["kind"] | "all">("all");
  // Decided items drop out so the next one moves up under the cursor.
  const all = () =>
    (plan()?.decisions ?? []).filter((d) => !decided().has(decisionKey(d)));
  const counts = createMemo(() => {
    const c = new Map<string, number>();
    for (const d of all()) c.set(d.kind, (c.get(d.kind) ?? 0) + 1);
    return c;
  });
  const shown = () =>
    (kind() === "all" ? all() : all().filter((d) => d.kind === kind())).slice(
      0,
      500,
    );
  return (
    <>
      <div class="row wrap">
        <button
          class="btn ghost"
          classList={{ active: kind() === "all" }}
          onClick={() => setKind("all")}
        >
          all {all().length}
        </button>
        <For each={KINDS.filter((k) => counts().get(k))}>
          {(k) => (
            <button
              class="btn ghost"
              classList={{ active: kind() === k }}
              onClick={() => setKind(k)}
            >
              {k} {counts().get(k)}
            </button>
          )}
        </For>
      </div>
      <ul class="decisions">
        <For each={shown()}>
          {(d) => (
            <li>
              <div class="row dhead">
                <span class={`tag tag-${d.kind}`}>{d.kind}</span>
                <Show when={d.headline}>
                  <strong class="small">{d.headline}</strong>
                </Show>
                <Show when={d.system}>
                  <span class="dim small">· {d.system}</span>
                </Show>
              </div>
              <Show when={d.detail}>
                <span class="small">{d.detail}</span>
              </Show>
              <span class="mono small dim wrap-any">{d.path}</span>
              <Actions d={d} />
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
            <Show
              when={busy()}
              fallback={
                <p class="dim">No plan yet – build one in the Inbox view.</p>
              }
            >
              <p>Planning – scanning library and inbox…</p>
              <ScanProgress />
            </Show>
            <Show when={status()}>
              {(s) => <p class={`mono ${s().ok ? "ok" : "err"}`}>{s().text}</p>}
            </Show>
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
                <div>
                  <div class="kpi">{p().ops.length}</div>
                  <span class="dim">operations</span>
                </div>
                <div>
                  <div class="kpi">{p().placed}</div>
                  <span class="dim">to place</span>
                </div>
                <div>
                  <div class="kpi">{p().unchanged}</div>
                  <span class="dim">unchanged</span>
                </div>
                <div>
                  <div class="kpi">{p().quarantined}</div>
                  <span class="dim">quarantine</span>
                </div>
                <Show when={p().discarded}>
                  <div>
                    <div class="kpi">{p().discarded}</div>
                    <span class="dim">to trash</span>
                  </div>
                </Show>
                <div>
                  <div class="kpi">{p().decisions.length}</div>
                  <span class="dim">need attention</span>
                </div>
              </div>
              <div class="row">
                <span class="dim small">
                  dry run · {p().items} items scanned
                </span>
                <span class="spacer" />
                <Show when={decided().size}>
                  <button class="btn ghost" disabled={busy()} onClick={replan}>
                    Re-plan ({decided().size} decided)
                  </button>
                </Show>
                <button
                  class="btn"
                  disabled={busy() || !p().ops.length}
                  onClick={execute}
                >
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
