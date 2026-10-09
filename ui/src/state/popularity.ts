// RetroAchievements popularity: background fetch of player counts and the rank of each game
// among the library's games of its system ("#3 on SNES").
import { createMemo, createRoot, createSignal } from "solid-js";
import { cheevosPlayers, cheevosPlayersCancel, onPlayersProgress, type LibraryRow, type ScanProgress } from "../ipc";
import { libraryRows, setLibraryRows } from "./libraryStore";

/** Progress of the running fetch, null when idle. */
export const [playersProgress, setPlayersProgress] = createSignal<ScanProgress | null>(null);
export const [playersError, setPlayersError] = createSignal("");

void onPlayersProgress((p) => {
  if (playersProgress()) setPlayersProgress(p.done < p.total ? p : null);
});

/** Fetches missing or outdated player counts (after a sync or library load) and patches the rows. */
export async function refreshPlayers() {
  if (playersProgress()) return;
  setPlayersError("");
  setPlayersProgress({ done: 0, total: 0 });
  try {
    const players = await cheevosPlayers();
    if (players) {
      setLibraryRows((all) =>
        all.map((r) => {
          const n = r.cheevos && r.cheevos_game != null ? (players[r.cheevos_game] ?? null) : null;
          return n === r.cheevos_players ? r : { ...r, cheevos_players: n };
        }),
      );
    }
  } catch (e) {
    setPlayersError(String(e));
  } finally {
    setPlayersProgress(null);
  }
}

export const cancelPlayers = () => void cheevosPlayersCancel();

/** Rank per library path within its system (1 = most players); one entry per RA game, so
 * several versions of the same game share a rank. */
const ranks = createRoot(() =>
  createMemo(() => {
    const bySystem = new Map<string, Map<number, number>>();
    for (const r of libraryRows()) {
      if (r.cheevos_players == null || r.cheevos_game == null) continue;
      let games = bySystem.get(r.system);
      if (!games) bySystem.set(r.system, (games = new Map()));
      games.set(r.cheevos_game, r.cheevos_players);
    }
    const rank = new Map<string, Map<number, number>>();
    for (const [system, games] of bySystem) {
      const sorted = [...games].sort((a, b) => b[1] - a[1]);
      rank.set(system, new Map(sorted.map(([g], i) => [g, i + 1])));
    }
    return rank;
  }),
);

export function popularityRank(r: LibraryRow): number | null {
  if (r.cheevos_players == null || r.cheevos_game == null) return null;
  return ranks().get(r.system)?.get(r.cheevos_game) ?? null;
}

/** Short system label for "#3 on SNES": the part after the vendor ("Nintendo - SNES"). */
export function shortSystem(system: string): string {
  const i = system.indexOf(" - ");
  return i < 0 ? system : system.slice(i + 3);
}

export function rankLabel(r: LibraryRow): string {
  const n = popularityRank(r);
  return n == null ? "" : `#${n} on ${shortSystem(r.system)}`;
}

export function rankTitle(r: LibraryRow): string {
  return r.cheevos_players == null ? "" : `${r.cheevos_players.toLocaleString()} RetroAchievements players`;
}
