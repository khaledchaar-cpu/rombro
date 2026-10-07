import { createSignal, For } from "solid-js";
import { VIEWS, type ViewId } from "../views";

const KEY = "sidebar";
function loadCollapsed(): boolean {
  try {
    return localStorage.getItem(KEY) === "collapsed";
  } catch {
    return false;
  }
}

type Props = { view: ViewId; onSelect: (v: ViewId) => void };

/** Vertical main navigation (collapsible to icons); keyboard: Ctrl+1…6. */
export default function Sidebar(props: Props) {
  const [collapsed, setCollapsed] = createSignal(loadCollapsed());
  const toggle = () => {
    setCollapsed(!collapsed());
    try {
      localStorage.setItem(KEY, collapsed() ? "collapsed" : "open");
    } catch {
      // preference just won't persist
    }
  };
  const item = (v: (typeof VIEWS)[number]) => (
    <button
      type="button"
      class="nav-item"
      classList={{ active: props.view === v.id }}
      title={`${v.label} · Ctrl+${v.key}`}
      aria-current={props.view === v.id ? "page" : undefined}
      onClick={() => props.onSelect(v.id)}
    >
      <svg class="nav-icon" viewBox="0 0 24 24" aria-hidden="true">
        <path d={v.icon} />
      </svg>
      <span class="nav-label">{v.label}</span>
      <kbd class="nav-key">{v.key}</kbd>
    </button>
  );
  return (
    <aside class="sidebar" classList={{ collapsed: collapsed() }} data-tauri-drag-region>
      <div class="logo" data-text="ROMBRO" data-tauri-drag-region>
        <span class="logo-full">ROMBRO</span>
        <span class="logo-short">RB</span>
      </div>
      <nav class="nav">
        <For each={VIEWS.filter((v) => !v.group)}>{item}</For>
      </nav>
      <nav class="nav nav-foot">
        <For each={VIEWS.filter((v) => v.group === "foot")}>{item}</For>
        <button type="button" class="nav-item nav-collapse" title={collapsed() ? "Expand" : "Collapse"} onClick={toggle}>
          <svg class="nav-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d={collapsed() ? "M9 6l6 6-6 6" : "M15 6l-6 6 6 6"} />
          </svg>
          <span class="nav-label">Collapse</span>
        </button>
      </nav>
    </aside>
  );
}
