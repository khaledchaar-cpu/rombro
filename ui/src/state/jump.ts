// Jumps from anywhere (dashboard) into the Library: a game to select, or a filter to apply.
import { createSignal } from "solid-js";

export interface LibraryJump {
  /** Game to select (and page to). */
  path?: string;
  systems?: string[];
  regions?: string[];
  /** Release decades, e.g. `1990s`. */
  decades?: string[];
  states?: string[];
  favorite?: boolean;
  played?: boolean;
  cheevos?: boolean;
  franchises?: string[];
  query?: string;
}

/** Pending jump; the Library applies and clears it, App switches to the Library view. */
export const [libraryJump, setLibraryJump] = createSignal<LibraryJump | null>(null);

/** Props that make any element a keyboard-accessible jump target (`[data-jump]` styles it). */
export function jumpProps(j: LibraryJump) {
  const go = () => setLibraryJump({ ...j });
  return {
    role: "button",
    tabIndex: 0,
    "data-jump": "",
    onClick: go,
    onKeyDown: (e: KeyboardEvent) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), go()),
  } as const;
}
