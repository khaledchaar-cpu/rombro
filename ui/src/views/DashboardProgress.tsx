// Dashboard gamification panels (SPEC F6): level/XP, KPIs, completeness per system, achievements.
import { jumpProps, type LibraryJump } from "../state/jump";
import { createMemo, For, Show } from "solid-js";
import Pager, { createPaged } from "../components/Pager";
import Panel from "../components/Panel";
import Segments from "../components/Segments";
import { stats } from "../state/gamify";

const fmt = new Intl.NumberFormat("en-US");
const pct = (a: number, b: number) => (b > 0 ? `${Math.floor((a / b) * 100)}%` : "–");
const bytes = (n: number) => {
  const u = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  while (n >= 1024 && i < u.length - 1) { n /= 1024; i++; }
  return `${n.toFixed(i ? 1 : 0)} ${u[i]}`;
};
const top = (m: Record<string, number>, n: number) => Object.entries(m).sort((a, b) => b[1] - a[1]).slice(0, n);

export function ProgressPanel() {
  return (
    <Panel title="Progress" class="wide">
      <Show when={stats()} fallback={<p class="dim">{stats.error ? String(stats.error) : "computing…"}</p>}>
        {(s) => {
          const k = () => s().kpis;
          const l = () => s().level;
          const verified = () => k().games + k().unknown + k().ambiguous;
          return (
            <>
              <div class="kpis">
                <div><div class="kpi">LV {l().level}</div><p class="dim small">{fmt.format(l().xp)} / {fmt.format(l().next)} XP</p></div>
                <div {...jumpProps({ states: ["known"] })}><div class="kpi">{fmt.format(k().games)}</div><p class="dim small">verified games</p></div>
                <div><div class="kpi">{pct(k().games, verified())}</div><p class="dim small">verified quota</p></div>
                <div><div class="kpi">{k().streak}</div><p class="dim small">day streak</p></div>
                <div><div class="kpi">{fmt.format(k().trashed)}</div><p class="dim small">files trashed</p></div>
                <div><div class="kpi">{bytes(k().trashed_bytes)}</div><p class="dim small">space in trash</p></div>
              </div>
              <Segments value={l().xp - l().floor} max={l().next - l().floor} />
              <div class="mix">
                <Mix title="Regions" items={top(k().regions, 5)} jump={(r) => ({ regions: [r], states: ["known"] })} />
                <Mix title="Genres" items={top(k().genres, 5)} />
                <Mix title="Decades" items={Object.entries(k().decades).map(([d, n]) => [`${d}s`, n] as [string, number])} jump={(d) => ({ decades: [d], states: ["known"] })} />
              </div>
            </>
          );
        }}
      </Show>
    </Panel>
  );
}

function Mix(props: { title: string; items: [string, number][]; jump?: (k: string) => LibraryJump }) {
  return (
    <div>
      <h3 class="dim small">{props.title}</h3>
      <ul class="rows small">
        <For each={props.items} fallback={<li class="dim">–</li>}>
          {([k, n]) => (<li {...(props.jump ? jumpProps(props.jump(k)) : {})}><span class="ellipsis">{k}</span><span class="mono">{fmt.format(n)}</span></li>)}
        </For>
      </ul>
    </div>
  );
}

export function CompletenessPanel() {
  const systems = createMemo(() =>
    [...(stats()?.kpis.systems ?? [])].sort((a, b) => b.owned / b.total - a.owned / a.total || b.owned - a.owned),
  );
  const paged = createPaged(systems, () => 8);
  return (
    <Panel title="Completeness (1G1R)">
      <ul class="complete">
        <For each={paged.items()} fallback={<li class="dim">No identified games yet.</li>}>
          {(s) => (
            <li title="Distinct games, one per 1G1R group; the library lists every release" {...(s.owned ? jumpProps({ systems: [s.system], states: ["known"] }) : {})}>
              <div class="row">
                <span class="ellipsis">{s.system}</span>
                <span class="spacer" />
                <span class="mono small">{fmt.format(s.owned)}/{fmt.format(s.total)} · {pct(s.owned, s.total)}</span>
              </div>
              <Segments value={s.owned} max={s.total} count={20} />
            </li>
          )}
        </For>
      </ul>
      <Pager paged={paged} />
    </Panel>
  );
}

/** Franchise goals: started franchises with at least 3 games on the owned systems, closest first. */
export function FranchisePanel() {
  const goals = () => stats()?.kpis.franchises ?? [];
  const paged = createPaged(goals, () => 8);
  return (
    <Panel title="Franchise goals">
      <ul class="complete">
        <For each={paged.items()} fallback={<li class="dim">No franchise started yet.</li>}>
          {(f) => (
            <li title="Distinct games, one per 1G1R group; the library lists every release" {...jumpProps({ franchises: [f.franchise], states: ["known"] })}>
              <div class="row">
                <span class="ellipsis">{f.franchise}</span>
                <span class="spacer" />
                <span class="mono small">{f.owned}/{f.total}</span>
              </div>
              <Segments value={f.owned} max={f.total} count={Math.min(f.total, 20)} />
            </li>
          )}
        </For>
      </ul>
      <Pager paged={paged} />
    </Panel>
  );
}

export function AchievementsPanel() {
  const list = createMemo(() =>
    [...(stats()?.achievements ?? [])].sort((a, b) => Number(b[0].unlocked) - Number(a[0].unlocked) || (b[1] ?? 0) - (a[1] ?? 0)),
  );
  const isNew = (id: string) => stats()?.new.includes(id) ?? false;
  const unlocked = () => list().filter(([a]) => a.unlocked).length;
  const paged = createPaged(list, () => 8);
  return (
    <Panel title={`Achievements ${unlocked()}/${list().length}`}>
      <ul class="achs">
        <For each={paged.items()}>
          {([a, at]) => (
            <li classList={{ locked: !a.unlocked }} title={at ? `unlocked ${new Date(at * 1000).toLocaleString()}` : "locked"}>
              <div class="row">
                <span class="ellipsis">{a.title}</span>
                <Show when={isNew(a.id)}><span class="tag tag-new">new</span></Show>
                <span class="spacer" />
                <span class="mono small">+{a.xp} XP</span>
              </div>
              <span class="dim small">{a.description}</span>
            </li>
          )}
        </For>
      </ul>
      <Pager paged={paged} />
    </Panel>
  );
}
