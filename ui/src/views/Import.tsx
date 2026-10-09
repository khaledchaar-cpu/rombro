import { createMemo, createSignal, For, Show } from "solid-js";
import Pager, { createPaged, fitCount } from "../components/Pager";
import Panel from "../components/Panel";
import ScanProgress from "../components/ScanProgress";
import InboxLeftovers from "../components/InboxLeftovers";
import ImportSetup from "./ImportSetup";
import RunReport from "../components/RunReport";
import ExecProgress from "../components/ExecProgress";
import type { DecisionView } from "../ipc";
import {
  busy,
  decided,
  trashAllRejected,
  decisionKey,
  execute,
  executing,
  judge,
  library,
  pick,
  plan,
  planned,
  prefer,
  replan,
  saving,
  status,
  undo,
} from "../state/importStore";

const ROW_H = 26;

function rel(p: string) {
  const lib = library();
  return lib && p.startsWith(lib) ? p.slice(lib.length).replace(/^\/+/, "") : p;
}

const isBios = (o: { rule: string }) => o.rule === "bios";

function OpList() {
  const [el, setEl] = createSignal<HTMLDivElement>();
  const paged = createPaged(() => plan()?.ops ?? [], fitCount(el, ROW_H, 72, 8));
  return (
    <>
      <div class="lpage" ref={setEl}>
        <For each={paged.items()}>
          {(op) => (
            <div class={`vrow oprow mono small${isBios(op) ? " bios" : ""}`} style={{ height: `${ROW_H}px` }}>
              <span class={`tag tag-${op.kind}`}>{op.kind}</span>
              <span class="dim ellipsis" title={op.from ?? ""}>
                {op.from ?? ""}
              </span>
              <span class="arrow">→</span>
              <span class="ellipsis" title={op.to}>
                {rel(op.to)}
              </span>
              <span class="ellipsis why" title={`rule: ${op.rule}`}>
                <Show when={isBios(op)}>
                  <span class="tag tag-bios">BIOS</span>{" "}
                </Show>
                {op.why}
              </span>
            </div>
          )}
        </For>
      </div>
      <Pager paged={paged} />
    </>
  );
}

function Actions(props: { d: DecisionView }) {
  const d = props.d;
  const off = () => busy() || saving().has(decisionKey(d));
  if (d.kind === "ambiguous")
    return (
      <div class="row wrap">
        <For each={d.options}>
          {(c) => (
            <button
              class="btn ghost small"
              disabled={off()}
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
            disabled={off()}
            onClick={() => judge(d, "keep")}
          >
            Keep
          </button>
        </Show>
        <button
          class="btn ghost small"
          disabled={off()}
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
              disabled={off()}
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
  const shown = () => (kind() === "all" ? all() : all().filter((d) => d.kind === kind()));
  // decided items drop out: stay on the page, only a new filter starts over
  const paged = createPaged(shown, () => 6, kind);
  const rejected = () => all().filter((d) => d.kind === "rejected" && d.options.length);
  const [confirmAll, setConfirmAll] = createSignal(false);
  const [trashing, setTrashing] = createSignal(false);
  const trashAll = async () => {
    if (!confirmAll()) return setConfirmAll(true);
    setConfirmAll(false);
    setTrashing(true);
    try {
      await trashAllRejected(rejected());
    } finally {
      setTrashing(false);
    }
  };
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
        <Show when={rejected().length}>
          <button
            class="btn ghost"
            disabled={trashing()}
            onClick={trashAll}
            onBlur={() => setConfirmAll(false)}
            title="Trash every open rejected release (undo via Settings → Rules → exceptions)"
          >
            {trashing()
              ? "Trashing…"
              : confirmAll()
                ? `Really trash ${rejected().length}?`
                : `Trash all rejected (${rejected().length})`}
          </button>
        </Show>
      </div>
      <ul class="decisions">
        <For each={paged.items()}>
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
              <Show
                when={!saving().has(decisionKey(d))}
                fallback={<span class="dim small">saving…</span>}
              >
                <Actions d={d} />
              </Show>
            </li>
          )}
        </For>
      </ul>
      <Pager paged={paged} />
    </>
  );
}

/** Import: folders and mode on top, then the plan (or its progress) and the inbox leftovers. */
export default function Import() {
  return (
    <div class="grid">
      <ImportSetup />
      <Show
        when={plan()}
        fallback={
          <>
          <RunReport rel={rel} />
          <Panel title="Plan" class="wide">
            <Show
              when={busy()}
              fallback={
                <p class="dim">No plan yet – choose the folders above and plan the import.</p>
              }
            >
              <p>Planning – loading the library, scanning the inbox…</p>
              <ScanProgress />
            </Show>
            <Show when={status()}>
              {(s) => <p class={`mono ${s().ok ? "ok" : "err"}`}>{s().text}</p>}
            </Show>
            <button class="btn ghost" disabled={busy()} onClick={undo}>
              Undo last run
            </button>
          </Panel>
          </>
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
                  <span class="dim">unknown</span>
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
                  {planned()?.audit ? "audit" : "dry run"} · {p().items} items scanned
                  {planned() ? ` · ${planned()!.at.toLocaleTimeString()}` : ""}
                </span>
                <span class="spacer" />
                <Show when={decided().size}>
                  <button class="btn ghost" disabled={busy() || saving().size > 0} onClick={replan}>
                    Re-plan ({decided().size} decided)
                  </button>
                </Show>
                <button
                  class="btn"
                  disabled={busy() || saving().size > 0 || !p().ops.length}
                  onClick={execute}
                >
                  {executing() ? "Executing" : "Execute"}
                </button>
              </div>
              <Show when={executing()}>
                <ExecProgress />
              </Show>
            </Panel>
            <Show
              when={p().ops.length || p().decisions.length}
              fallback={
                <Panel title="Operations" class="wide">
                  <p class="ok">
                    {planned()?.audit
                      ? "Library is clean – nothing to move, rename or trash."
                      : "Nothing to do – the inbox adds nothing to the library."}
                  </p>
                </Panel>
              }
            >
              <Panel title="Operations" class="wide">
                <OpList />
              </Panel>
            </Show>
            <Show when={p().decisions.length}>
              <Panel title="Needs attention" class="wide">
                <Decisions />
              </Panel>
            </Show>
          </>
        )}
      </Show>
      <Show when={!busy()}>
        <InboxLeftovers />
      </Show>
    </div>
  );
}
