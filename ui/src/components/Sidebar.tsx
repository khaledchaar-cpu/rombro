import { For } from "solid-js";
import { VIEWS, type ViewId } from "../views";

export default function Sidebar(props: { view: ViewId; onSelect: (v: ViewId) => void }) {
  return (
    <nav class="sidebar">
      <div class="logo">ROMBRO</div>
      <For each={VIEWS}>
        {(v) => (
          <button
            class="nav-item"
            classList={{ active: props.view === v.id }}
            onClick={() => props.onSelect(v.id)}
          >
            <span>{v.label}</span>
            <kbd>^{v.key}</kbd>
          </button>
        )}
      </For>
    </nav>
  );
}
