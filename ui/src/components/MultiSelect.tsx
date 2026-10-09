// Multiple-choice dropdown in the style of Select. The popup lays its options out in as many
// columns as needed to fit below the button, so it never scrolls. Empty selection = all.
// Keyboard: arrows, Home/End, Space/Enter toggles, Esc closes.
import { createSignal, For, onCleanup, Show } from "solid-js";
import { Portal } from "solid-js/web";
import type { Option } from "./Select";

const ROW_H = 27;

export default function MultiSelect(props: {
  values: Set<string>;
  options: Option<string>[];
  onChange: (v: Set<string>) => void;
  /** Label when nothing is chosen, e.g. "All systems". */
  all: string;
  /** Plural noun for several choices, e.g. "systems". */
  noun: string;
}) {
  const [open, setOpen] = createSignal(false);
  const [hi, setHi] = createSignal(0);
  const [pos, setPos] = createSignal({ left: 0, top: 0, width: 0, rows: 10 });
  let button!: HTMLButtonElement;
  let list: HTMLUListElement | undefined;

  const label = () => {
    const v = [...props.values];
    if (!v.length) return props.all;
    if (v.length === 1) return props.options.find((o) => o.value === v[0])?.label ?? v[0];
    return `${v.length} ${props.noun}`;
  };

  const place = () => {
    const r = button.getBoundingClientRect();
    const rows = Math.max(4, Math.floor((window.innerHeight - r.bottom - 24) / ROW_H));
    const n = props.options.length + 1;
    const cols = Math.ceil(n / rows);
    const left = Math.max(8, Math.min(r.left, window.innerWidth - cols * 240 - 16));
    setPos({ left, top: r.bottom + 4, width: r.width, rows: Math.ceil(n / cols) });
  };
  const show = () => (place(), setHi(0), setOpen(true));
  const close = () => setOpen(false);
  // index 0 = "all" (clears), i > 0 = options[i - 1]
  const toggle = (i: number) => {
    if (i === 0) return props.onChange(new Set());
    const v = props.options[i - 1].value;
    const n = new Set(props.values);
    if (n.has(v)) n.delete(v);
    else n.add(v);
    props.onChange(n);
  };
  const move = (d: number) => setHi((h) => Math.min(props.options.length, Math.max(0, h + d)));

  const onKey = (e: KeyboardEvent) => {
    if (!open()) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) (e.preventDefault(), show());
      return;
    }
    const rows = pos().rows;
    const keys: Record<string, () => void> = {
      ArrowDown: () => move(1),
      ArrowUp: () => move(-1),
      ArrowRight: () => move(rows),
      ArrowLeft: () => move(-rows),
      Home: () => move(-1e6),
      End: () => move(1e6),
      Enter: () => toggle(hi()),
      " ": () => toggle(hi()),
      Escape: close,
      Tab: close,
    };
    const f = keys[e.key];
    if (f) {
      if (e.key !== "Tab") e.preventDefault();
      e.stopPropagation();
      f();
    }
  };

  const outside = (e: MouseEvent) => {
    const t = e.target as Node;
    if (open() && !button.contains(t) && !list?.contains(t)) close();
  };
  window.addEventListener("mousedown", outside);
  window.addEventListener("resize", close);
  onCleanup(() => {
    window.removeEventListener("mousedown", outside);
    window.removeEventListener("resize", close);
  });

  const items = () => [{ value: "", label: props.all }, ...props.options];

  return (
    <>
      <button
        ref={button}
        type="button"
        class="xsel"
        classList={{ open: open(), set: props.values.size > 0 }}
        aria-haspopup="listbox"
        aria-expanded={open()}
        onClick={() => (open() ? close() : show())}
        onKeyDown={onKey}
      >
        <span class="ellipsis">{label()}</span>
        <i class="xsel-chev" aria-hidden="true" />
      </button>
      <Show when={open()}>
        <Portal>
          <ul
            ref={list}
            class="xsel-pop xsel-multi"
            role="listbox"
            aria-multiselectable="true"
            style={{
              left: `${pos().left}px`,
              top: `${pos().top}px`,
              "min-width": `${pos().width}px`,
              "grid-template-rows": `repeat(${pos().rows}, auto)`,
            }}
          >
            <For each={items()}>
              {(o, i) => {
                const on = () => (i() === 0 ? props.values.size === 0 : props.values.has(o.value));
                return (
                  <li
                    role="option"
                    aria-selected={on()}
                    classList={{ hi: i() === hi(), on: on() }}
                    onMouseEnter={() => setHi(i())}
                    onMouseDown={(e) => (e.preventDefault(), toggle(i()))}
                  >
                    <input type="checkbox" tabIndex={-1} checked={on()} />
                    <span class="ellipsis">{o.label}</span>
                    <Show when={o.hint}><span class="xsel-hint">{o.hint}</span></Show>
                  </li>
                );
              }}
            </For>
          </ul>
        </Portal>
      </Show>
    </>
  );
}
