// Statistics: play time totals, history (week/month/year), top games, time per system, achievements.
import { createMemo, createResource, createSignal, For, onCleanup, Show } from "solid-js";
import Cover from "../components/Cover";
import Pager, { createPaged } from "../components/Pager";
import Panel from "../components/Panel";
import { onPlayEnded, playSessions, type LibraryRow } from "../ipc";
import { formatPlayTime, libraryRows } from "../state/libraryStore";

type Range = "week" | "month" | "year";
const RANGES: { id: Range; label: string }[] = [
  { id: "week", label: "7 days" },
  { id: "month", label: "30 days" },
  { id: "year", label: "12 months" },
];
const TOP = 10;

/** Buckets (oldest first) with label and start (local time) for a range. */
function buckets(range: Range): { label: string; start: number; end: number }[] {
  const out = [];
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  if (range === "year") {
    for (let i = 11; i >= 0; i--) {
      const a = new Date(today.getFullYear(), today.getMonth() - i, 1);
      const b = new Date(today.getFullYear(), today.getMonth() - i + 1, 1);
      out.push({ label: a.toLocaleDateString(undefined, { month: "short" }), start: a.getTime() / 1000, end: b.getTime() / 1000 });
    }
    return out;
  }
  const days = range === "week" ? 7 : 30;
  for (let i = days - 1; i >= 0; i--) {
    const a = new Date(today.getFullYear(), today.getMonth(), today.getDate() - i);
    const b = new Date(today.getFullYear(), today.getMonth(), today.getDate() - i + 1);
    const label = range === "week" ? a.toLocaleDateString(undefined, { weekday: "short" }) : String(a.getDate());
    out.push({ label, start: a.getTime() / 1000, end: b.getTime() / 1000 });
  }
  return out;
}

function Bars(props: { items: { label: string; value: number; title: string }[]; compact?: boolean }) {
  const max = () => Math.max(1, ...props.items.map((i) => i.value));
  return (
    <div class="hbars" classList={{ compact: props.compact }}>
      <For each={props.items}>
        {(i) => (
          <div class="hbar" title={i.title}>
            <span class="hbar-fill" style={{ height: `${(i.value / max()) * 100}%` }} />
            <span class="hbar-label dim">{i.label}</span>
          </div>
        )}
      </For>
    </div>
  );
}

function Meter(props: { value: number; max: number }) {
  return (
    <span class="meter">
      <span style={{ width: `${props.max ? Math.min(100, (props.value / props.max) * 100) : 0}%` }} />
    </span>
  );
}

/** Play statistics panels (part of the dashboard): overview, history, most played, time per system, RA unlocks. */
export default function PlayStats() {
  const [range, setRange] = createSignal<Range>("week");
  const span = createMemo(() => buckets(range()));
  const [sessions, { refetch }] = createResource(() => span()[0].start, playSessions);
  const unlisten = onPlayEnded(() => void refetch());
  onCleanup(() => void unlisten.then((f) => f()));

  const played = createMemo(() => libraryRows().filter((r) => r.seconds > 0));
  const total = createMemo(() => played().reduce((s, r) => s + r.seconds, 0));
  const runs = createMemo(() => played().reduce((s, r) => s + r.plays, 0));
  const history = createMemo(() => {
    const list = sessions() ?? [];
    return span().map((b) => {
      const secs = list.filter(([s]) => s >= b.start && s < b.end).reduce((a, [, d]) => a + d, 0);
      return { label: b.label, value: secs, title: `${b.label}: ${secs ? formatPlayTime(secs) : "–"}` };
    });
  });
  const rangeTotal = createMemo(() => history().reduce((a, b) => a + b.value, 0));
  const top = createMemo(() => [...played()].sort((a, b) => b.seconds - a.seconds).slice(0, TOP));
  const perSystem = createMemo(() => {
    const m = new Map<string, number>();
    for (const r of played()) m.set(r.system, (m.get(r.system) ?? 0) + r.seconds);
    return [...m].sort((a, b) => b[1] - a[1]);
  });
  const cheevos = createMemo(() =>
    libraryRows()
      .filter((r): r is LibraryRow & { cheevos_progress: NonNullable<LibraryRow["cheevos_progress"]> } => !!r.cheevos_progress?.awarded)
      .sort((a, b) => b.cheevos_progress.awarded / b.cheevos_progress.total - a.cheevos_progress.awarded / a.cheevos_progress.total),
  );
  const unlocked = createMemo(() => cheevos().reduce((s, r) => s + r.cheevos_progress.awarded, 0));
  const mastered = createMemo(() => cheevos().filter((r) => r.cheevos_progress.award === "mastered").length);

  const pTop = createPaged(top, () => 10);
  const pSys = createPaged(perSystem, () => 10);
  const pAch = createPaged(cheevos, () => 10);
  return (
    <>
      <Panel title="Overview" class="wide">
        <div class="stat-tiles">
          <div class="stat"><strong>{total() ? formatPlayTime(total()) : "0 min"}</strong><span class="dim small">played in total</span></div>
          <div class="stat"><strong>{played().length}</strong><span class="dim small">games played</span></div>
          <div class="stat"><strong>{runs()}</strong><span class="dim small">sessions</span></div>
          <div class="stat"><strong>{unlocked()}</strong><span class="dim small">achievements · {mastered()} mastered</span></div>
        </div>
      </Panel>

      <Panel title="History" class="wide">
        <div class="row spread">
          <p class="dim small">{rangeTotal() ? formatPlayTime(rangeTotal()) : "Nothing played"} in the last {RANGES.find((r) => r.id === range())?.label}</p>
          <div class="seg-toggle">
            <For each={RANGES}>
              {(r) => (
                <button class="btn ghost small" classList={{ on: range() === r.id }} onClick={() => setRange(r.id)}>
                  {r.label}
                </button>
              )}
            </For>
          </div>
        </div>
        <Bars items={history()} compact={range() === "month"} />
      </Panel>

      <Panel title="Most played">
        <Show when={top().length} fallback={<p class="dim small">Play something – it shows up here.</p>}>
          <ol class="rank" start={pTop.offset() + 1}>
            <For each={pTop.items()}>
              {(r) => (
                <li>
                  <Cover system={r.system} name={r.name} class="cover-mini" />
                  <span class="rank-name">
                    <span class="ellipsis small" title={r.name}>{r.name}</span>
                    <Meter value={r.seconds} max={top()[0].seconds} />
                  </span>
                  <span class="dim small">{formatPlayTime(r.seconds)}</span>
                </li>
              )}
            </For>
          </ol>
          <Pager paged={pTop} />
        </Show>
      </Panel>

      <Panel title="Time per system">
        <Show when={perSystem().length} fallback={<p class="dim small">No play time yet.</p>}>
          <ol class="rank" start={pSys.offset() + 1}>
            <For each={pSys.items()}>
              {([sys, secs]) => (
                <li>
                  <span class="rank-name">
                    <span class="ellipsis small" title={sys}>{sys}</span>
                    <Meter value={secs} max={perSystem()[0][1]} />
                  </span>
                  <span class="dim small">{formatPlayTime(secs)}</span>
                </li>
              )}
            </For>
          </ol>
          <Pager paged={pSys} />
        </Show>
      </Panel>

      <Panel title="RetroAchievements" class="wide">
        <Show when={cheevos().length} fallback={<p class="dim small">No unlocks yet (log in under RetroArch → RetroAchievements).</p>}>
          <ol class="rank" start={pAch.offset() + 1}>
            <For each={pAch.items()}>
              {(r) => (
                <li>
                  <Cover system={r.system} name={r.name} class="cover-mini" />
                  <span class="rank-name">
                    <span class="ellipsis small" title={r.name}>
                      {r.name}
                      {r.cheevos_progress.award ? ` · ${r.cheevos_progress.award}` : ""}
                    </span>
                    <Meter value={r.cheevos_progress.awarded} max={r.cheevos_progress.total} />
                  </span>
                  <span class="dim small">{r.cheevos_progress.awarded} / {r.cheevos_progress.total}</span>
                </li>
              )}
            </For>
          </ol>
          <Pager paged={pAch} />
        </Show>
      </Panel>

    </>
  );
}
