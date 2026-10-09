// "Popular in your library": the games with the most RetroAchievements players.
import { createMemo } from "solid-js";
import type { LibraryRow } from "../ipc";
import { libraryRows } from "../state/libraryStore";
import { rankLabel } from "../state/popularity";
import { Shelf } from "./DashboardContinue";

const MAX = 12;

/** Most played RA games first; one file per RA game (versions share the count). */
export function popularList(rows: LibraryRow[], max = MAX): LibraryRow[] {
  const seen = new Set<number>();
  return rows
    .filter((r) => r.cheevos_players != null && r.cheevos_game != null)
    .sort((a, b) => (b.cheevos_players ?? 0) - (a.cheevos_players ?? 0))
    .filter((r) => !seen.has(r.cheevos_game as number) && (seen.add(r.cheevos_game as number), true))
    .slice(0, max);
}

export function PopularPanel(props: { onLibrary: () => void }) {
  const list = createMemo(() => popularList(libraryRows()));
  return <Shelf title="Popular in your library" list={list()} sub={rankLabel} onLibrary={props.onLibrary} />;
}
