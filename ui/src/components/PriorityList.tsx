import { For, createSignal } from "solid-js";

/**
 * Ordered list, best first. Reorder by dragging the handle (pointer events, so it
 * works inside the Tauri webview where native HTML5 DnD is taken by file drops).
 */
export default function PriorityList(props: {
  items: string[];
  onChange: (items: string[]) => void;
  placeholder: string;
}) {
  const [dragging, setDragging] = createSignal<number | null>(null);
  const [draft, setDraft] = createSignal("");
  let listEl!: HTMLOListElement;

  const move = (from: number, to: number) => {
    if (from === to || to < 0 || to >= props.items.length) return;
    const next = [...props.items];
    const [it] = next.splice(from, 1);
    next.splice(to, 0, it);
    props.onChange(next);
  };

  const onPointerDown = (i: number, e: PointerEvent) => {
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    setDragging(i);
  };
  const onPointerMove = (e: PointerEvent) => {
    const from = dragging();
    if (from === null) return;
    const rows = [...listEl.children] as HTMLElement[];
    const to = rows.findIndex((r) => {
      const b = r.getBoundingClientRect();
      return e.clientY >= b.top && e.clientY < b.bottom;
    });
    if (to >= 0 && to !== from) {
      move(from, to);
      setDragging(to);
    }
  };

  const add = () => {
    const v = draft().trim();
    if (v && !props.items.some((x) => x.toLowerCase() === v.toLowerCase())) props.onChange([...props.items, v]);
    setDraft("");
  };

  return (
    <div class="prio">
      <ol class="prio-list" ref={listEl}>
        <For each={props.items}>
          {(item, i) => (
            <li class="prio-item" classList={{ dragging: dragging() === i() }}>
              <span
                class="prio-handle"
                title="Drag to reorder"
                onPointerDown={(e) => onPointerDown(i(), e)}
                onPointerMove={onPointerMove}
                onPointerUp={() => setDragging(null)}
                onPointerCancel={() => setDragging(null)}
              >
                ⋮⋮
              </span>
              <span class="mono dim">{i() + 1}</span>
              <span class="prio-name">{item}</span>
              <button class="btn ghost sm" disabled={i() === 0} onClick={() => move(i(), i() - 1)} title="Up">↑</button>
              <button class="btn ghost sm" disabled={i() === props.items.length - 1} onClick={() => move(i(), i() + 1)} title="Down">↓</button>
              <button class="btn ghost sm" onClick={() => props.onChange(props.items.filter((_, j) => j !== i()))} title="Remove">✕</button>
            </li>
          )}
        </For>
      </ol>
      <div class="row">
        <input
          class="field mono"
          placeholder={props.placeholder}
          value={draft()}
          onInput={(e) => setDraft(e.currentTarget.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
        />
        <button class="btn ghost" disabled={!draft().trim()} onClick={add}>Add</button>
      </div>
    </div>
  );
}
