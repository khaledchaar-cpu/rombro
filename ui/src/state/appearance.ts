// Per-device appearance preferences (theme, effects), persisted in localStorage.
import { createSignal } from "solid-js";

export type Theme = "dark" | "light";

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

const [theme, setThemeSignal] = createSignal<Theme>(load("theme") === "light" ? "light" : "dark");
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
