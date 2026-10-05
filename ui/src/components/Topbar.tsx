import { Show } from "solid-js";
import { getCurrentWindow } from "@tauri-apps/api/window";

const inTauri = "__TAURI_INTERNALS__" in window;

/** Top bar; doubles as window title bar (the native one is disabled). */
export default function Topbar(props: { title: string; onPalette: () => void }) {
  const win = () => getCurrentWindow();
  return (
    <header class="topbar" data-tauri-drag-region>
      <h2 data-tauri-drag-region>{props.title}</h2>
      <button class="palette-trigger" onClick={props.onPalette}>
        <span class="dim">Command…</span> <kbd>Ctrl K</kbd>
      </button>
      <Show when={inTauri}>
        <div class="win-controls">
          <button title="Minimize" onClick={() => void win().minimize()}>─</button>
          <button title="Maximize" onClick={() => void win().toggleMaximize()}>□</button>
          <button class="close" title="Close" onClick={() => void win().close()}>✕</button>
        </div>
      </Show>
    </header>
  );
}
