// Custom dropdown (native <select> popups can't be themed). Popup is portalled with fixed
// positioning so clipped panels don't cut it off. Keyboard: ↑/↓, Home/End, Enter, Esc.
import { createSignal, For, onCleanup, Show } from "solid-js";
import { Portal } from "solid-js/web";

export interface Option<T> { value: T; label: string; hint?: string }

export default function Select<T>(props: {
  value: T;
  options: Option<T>[];
  onChange: (v: T) => void;
  placeholder?: string;
}) {
  const [open, setOpen] = createSignal(false);
  const [hi, setHi] = createSignal(0);
  const [pos, setPos] = createSignal({ left: 0, top: 0, width: 0, maxH: 300 });
  let button!: HTMLButtonElement;
  let list: HTMLUListElement | undefined;

  const current = () => props.options.find((o) => o.value === props.value);
  const isDefault = () => props.options[0]?.value === props.value;

  const place = () => {
    const r = button.getBoundingClientRect();
    const below = window.innerHeight - r.bottom - 12;
    setPos({ left: r.left, top: r.bottom + 4, width: r.width, maxH: Math.max(160, Math.min(360, below)) });
  };
  const show = () => {
    place();
    setHi(Math.max(0, props.options.findIndex((o) => o.value === props.value)));
    setOpen(true);
    queueMicrotask(scrollHi);
  };
  const close = () => setOpen(false);
  const pick = (o: Option<T>) => (props.onChange(o.value), close(), button.focus());
  const scrollHi = () => list?.children[hi()]?.scrollIntoView({ block: "nearest" });
  const move = (d: number) => {
    const n = props.options.length;
    setHi((h) => Math.min(n - 1, Math.max(0, h + d)));
    scrollHi();
  };

  const onKey = (e: KeyboardEvent) => {
    if (!open()) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) (e.preventDefault(), show());
      return;
    }
    const keys: Record<string, () => void> = {
      ArrowDown: () => move(1),
      ArrowUp: () => move(-1),
      PageDown: () => move(8),
      PageUp: () => move(-8),
      Home: () => move(-1e6),
      End: () => move(1e6),
      Enter: () => pick(props.options[hi()]),
      " ": () => pick(props.options[hi()]),
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
  // the popup is fixed-positioned: close when anything but the list itself scrolls
  const scrolled = (e: Event) => open() && e.target !== list && close();
  window.addEventListener("mousedown", outside);
  window.addEventListener("resize", close);
  window.addEventListener("scroll", scrolled, true);
  onCleanup(() => {
    window.removeEventListener("scroll", scrolled, true);
    window.removeEventListener("mousedown", outside);
    window.removeEventListener("resize", close);
  });

  return (
    <>
      <button
        ref={button}
        type="button"
        class="xsel"
        classList={{ open: open(), set: !isDefault() }}
        aria-haspopup="listbox"
        aria-expanded={open()}
        onClick={() => (open() ? close() : show())}
        onKeyDown={onKey}
      >
        <span class="ellipsis">{current()?.label ?? props.placeholder ?? ""}</span>
        <i class="xsel-chev" aria-hidden="true" />
      </button>
      <Show when={open()}>
        <Portal>
          <ul
            ref={list}
            class="xsel-pop"
            role="listbox"
            style={{
              left: `${pos().left}px`,
              top: `${pos().top}px`,
              "min-width": `${pos().width}px`,
              "max-height": `${pos().maxH}px`,
            }}
          >
            <For each={props.options}>
              {(o, i) => (
                <li
                  role="option"
                  aria-selected={o.value === props.value}
                  classList={{ hi: i() === hi(), sel: o.value === props.value }}
                  onMouseEnter={() => setHi(i())}
                  onMouseDown={(e) => (e.preventDefault(), pick(o))}
                >
                  <span class="ellipsis">{o.label}</span>
                  <Show when={o.hint}><span class="xsel-hint">{o.hint}</span></Show>
                </li>
              )}
            </For>
          </ul>
        </Portal>
      </Show>
    </>
  );
}
