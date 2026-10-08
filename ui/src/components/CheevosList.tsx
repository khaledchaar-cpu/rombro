import { createResource, For, Show } from "solid-js";
import { cheevosAchievements, cheevosStatus } from "../ipc";

const KIND: Record<string, string> = { progression: "progression", win_condition: "win", missable: "missable" };

/** The achievements of a RetroAchievements game (details view); `other` = they belong to another version. */
export default function CheevosList(props: { game: number; other: string | null }) {
  const [list] = createResource(() => props.game, cheevosAchievements);
  const [status] = createResource(cheevosStatus);
  const total = () => (list() ?? []).reduce((s, a) => s + a.points, 0);
  const unlocked = () => (list() ?? []).filter((a) => a.earned).length;
  // locked achievements are dimmed only when there is a user to unlock them
  const locked = (earned: string | null) => !!status()?.user && !props.other && !earned;
  return (
    <div class="cheevos">
      <h3 class="small">
        Achievements
        <Show when={list()}> · {list()!.length} · {total()} points</Show>
        <Show when={status()?.user && !props.other && list()}> · {unlocked()} unlocked</Show>
        <Show when={props.other}> · for “{props.other}” (another version)</Show>
      </h3>
      <Show when={!list.loading} fallback={<p class="dim small">loading…</p>}>
        <Show when={!list.error} fallback={<p class="err small">{String(list.error)}</p>}>
          <ul class="cheevo-list">
            <For each={list()}>
              {(a) => (
                <li class="cheevo" classList={{ locked: locked(a.earned) }}>
                  <Show when={a.badge} fallback={<div class="cheevo-badge" />}>
                    <img class="cheevo-badge" loading="lazy" alt=""
                      src={`https://media.retroachievements.org/Badge/${a.badge}.png`} />
                  </Show>
                  <div class="cheevo-text">
                    <span class="cheevo-title">{a.title}</span>
                    <span class="dim small">{a.description}</span>
                  </div>
                  <span class="dim small mono cheevo-meta">
                    <Show when={a.earned}>
                      <span title={`unlocked ${a.earned}`}>{a.earned_hardcore ? "✔ hardcore" : "✔"} · </span>
                    </Show>
                    {a.points} pts · {Math.round(a.rarity * 100)}%
                    <Show when={KIND[a.kind]}> · {KIND[a.kind]}</Show>
                  </span>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </Show>
    </div>
  );
}
