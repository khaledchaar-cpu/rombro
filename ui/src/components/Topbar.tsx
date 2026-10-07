import { Show } from "solid-js";
import { getCurrentWindow } from "@tauri-apps/api/window";

const inTauri = "__TAURI_INTERNALS__" in window;

type Props = { title: string; onPalette: () => void };

/** Title bar of the content area: view title, command palette, window controls
 * (the native title bar is disabled). */
export default function Topbar(props: Props) {
  const win = () => getCurrentWindow();
  return (
    <header class="topbar" data-tauri-drag-region>
      <h1 class="view-title" data-tauri-drag-region>{props.title}</h1>
      <div class="drag-fill" data-tauri-drag-region />
      <button class="palette-trigger" onClick={props.onPalette}>
        <span class="dim">Command…</span> <kbd>Ctrl K</kbd>
      </button>
      <Show when={inTauri}>
        <div class="win-controls">
          <button class="min" title="Minimize" onClick={() => void win().minimize()}>
            <svg viewBox="0 0 12 12"><path d="M2 9h8" /></svg>
          </button>
          <button class="max" title="Maximize" onClick={() => void win().toggleMaximize()}>
            <svg viewBox="0 0 12 12"><path d="M2 4V2h2M8 2h2v2M10 8v2H8M4 10H2V8" /></svg>
          </button>
          <button class="close" title="Close" onClick={() => void win().close()}>
            <svg viewBox="0 0 12 12"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" /></svg>
          </button>
        </div>
      </Show>
    </header>
  );
}
