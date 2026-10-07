// Per-device appearance preferences (theme, effects), persisted in localStorage.
import { createSignal } from "solid-js";

export type Theme = "dark" | "light" | "pinup";

export const THEMES: { id: Theme; label: string }[] = [
  { id: "dark", label: "Neon" },
  { id: "light", label: "Neon light" },
  { id: "pinup", label: "Pin-up '40s" },
];

function load(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function store(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // preference just won't persist
  }
}

const [theme, setThemeSignal] = createSignal<Theme>(THEMES.find((t) => t.id === load("theme"))?.id ?? "dark");
const [effects, setEffectsSignal] = createSignal(load("effects") !== "off");

function apply() {
  document.documentElement.dataset.theme = theme();
  document.documentElement.dataset.effects = effects() ? "on" : "off";
}
apply();

export { theme, effects };

export function setTheme(t: Theme) {
  setThemeSignal(t);
  store("theme", t);
  apply();
}

export function setEffects(on: boolean) {
  setEffectsSignal(on);
  store("effects", on ? "on" : "off");
  apply();
}
