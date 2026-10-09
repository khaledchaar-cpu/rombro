import { createSignal, For, Match, Show, Switch } from "solid-js";
import Cover from "./Cover";
import Panel from "./Panel";
import GamePlay from "./GamePlay";
import SaveStates from "./SaveStates";
import CheevosList from "./CheevosList";
import { type LibraryRow, type ThumbKind } from "../ipc";
import { CHEEVOS_PBP_HINT, cheevosUnhashable, formatPlayTime, toggleFavorite } from "../state/libraryStore";
import { rankLabel, rankTitle } from "../state/popularity";

const KINDS: { kind: ThumbKind; label: string }[] = [
  { kind: "boxart", label: "Boxart" },
  { kind: "title", label: "Title" },
  { kind: "snap", label: "Snap" },
];

function Thumb(props: { row: LibraryRow; kind: ThumbKind; label: string }) {
  return (
    <figure class="thumb">
      <Cover system={props.row.system} name={props.row.name} kind={props.kind} />
      <figcaption class="dim small">{props.label}</figcaption>
    </figure>
  );
}

type Tab = "info" | "states" | "cheevos";
// kept across games: browsing the list keeps the chosen tab
const [tab, setTab] = createSignal<Tab>("info");

export default function GameDetail(props: { row: LibraryRow }) {
  const tabs = (): { id: Tab; label: string }[] => [
    { id: "info", label: "Info" },
    { id: "states", label: "Savestates" },
    ...(props.row.cheevos_game ? [{ id: "cheevos" as const, label: "Achievements" }] : []),
  ];
  // a game without achievements falls back to its info
  const shown = (): Tab => (tab() === "cheevos" && !props.row.cheevos_game ? "info" : tab());
  const at = (t: number) => (t ? new Date(t * 1000).toLocaleString() : "–");
  return (
    <Panel title="Details" class="wide">
      <div class="row">
        <button class="btn ghost" classList={{ "star on": props.row.favorite }} onClick={() => void toggleFavorite(props.row)}>
          {props.row.favorite ? "★ Favorite" : "☆ Add to favorites"}
        </button>
      </div>
      <GamePlay path={props.row.path} />
      <div class="row detail-tabs" role="tablist">
        <For each={tabs()}>
          {(t) => (
            <button role="tab" class="btn ghost small" classList={{ active: shown() === t.id }} aria-selected={shown() === t.id} onClick={() => setTab(t.id)}>
              {t.label}
            </button>
          )}
        </For>
      </div>
      <Switch>
      <Match when={shown() === "states"}>
        <SaveStates path={props.row.path} />
      </Match>
      <Match when={shown() === "cheevos" && props.row.cheevos_game}>
        {(g) => <CheevosList game={g()} other={props.row.cheevos ? null : props.row.cheevos_other} unlocked={props.row.cheevos_progress?.awarded} />}
      </Match>
      <Match when={shown() === "info"}>
      <div class="detail">
        <div class="thumbs">
          <For each={KINDS}>{(k) => <Thumb row={props.row} kind={k.kind} label={k.label} />}</For>
          <p class="dim small credit">Images: libretro-thumbnails</p>
        </div>
        <dl class="meta small">
          <dt>Name</dt><dd>{props.row.name || "–"}</dd>
          <dt>System</dt><dd>{props.row.system || "–"}</dd>
          <dt>State</dt><dd class={`tag-${props.row.state}`}>{props.row.state}</dd>
          <dt>Released</dt><dd>{props.row.year ?? "–"}</dd>
          <dt>Regions</dt><dd>{props.row.regions.join(", ") || "–"}</dd>
          <dt>Files</dt><dd>{props.row.files}</dd>
          <dt>Added</dt><dd>{at(props.row.added)}</dd>
          <dt>Played</dt>
          <dd>{props.row.plays ? `${formatPlayTime(props.row.seconds)} · ${props.row.plays} runs` : "never"}</dd>
          <dt>Last played</dt><dd>{at(props.row.last_played)}</dd>
          <dt>Achievements</dt>
          <dd>
            {props.row.cheevos_progress
              ? `${props.row.cheevos_progress.awarded} of ${props.row.cheevos_progress.total} unlocked${props.row.cheevos_progress.award ? ` · ${props.row.cheevos_progress.award}` : ""}`
              : props.row.cheevos
              ? `${props.row.cheevos} (RetroAchievements)`
              : cheevosUnhashable(props.row)
                ? CHEEVOS_PBP_HINT
                : props.row.cheevos_other
                ? `none for this version – supported: ${props.row.cheevos_other}`
                : "–"}
          </dd>
          <Show when={rankLabel(props.row)}>
            <dt>Popularity</dt><dd title={rankTitle(props.row)}>{rankLabel(props.row)}</dd>
          </Show>
          <dt>Path</dt><dd class="mono">{props.row.path}</dd>
        </dl>
      </div>
      </Match>
      </Switch>
    </Panel>
  );
}
