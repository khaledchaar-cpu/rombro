// Gamification switch (stored in the database) and stats derived from the loaded library rows.
import { createResource, createRoot, createSignal } from "solid-js";
import { gamifyEnabledGet, gamifyEnabledSet, gamifyStats } from "../ipc";
import { libraryRows } from "./libraryStore";

const [enabled, setEnabledSig] = createSignal(true);
void gamifyEnabledGet().then(setEnabledSig, () => {});
export const gamifyEnabled = enabled;

export async function setGamifyEnabled(on: boolean) {
  await gamifyEnabledSet(on);
  setEnabledSig(on);
}

/** Recomputed whenever the library rows change (only while enabled). */
export const [stats] = createRoot(() =>
  createResource(
    () => (enabled() ? libraryRows() : false),
    (rows) => {
      const games: [string, string][] = [];
      let unknown = 0;
      let ambiguous = 0;
      for (const r of rows) {
        if (r.state === "known") games.push([r.system, r.name]);
        else if (r.state === "unknown") unknown++;
        else if (r.state === "ambiguous") ambiguous++;
      }
      return gamifyStats(games, unknown, ambiguous);
    },
  ),
);
