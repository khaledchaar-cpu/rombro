// Paging instead of scrolling: `createPaged` slices a list into pages, `fitCount` sizes a page
// to the space left in the window, `Pager` is the ‹ 3/41 › control. Blocks take no focus and
// leave the mouse wheel to the page.
import { createEffect, createSignal, on, onCleanup, Show, type Accessor } from "solid-js";

export interface Paged<T> {
  items: Accessor<T[]>;
  page: Accessor<number>;
  pages: Accessor<number>;
  /** Index of the first item on the current page. */
  offset: Accessor<number>;
  go: (p: number) => void;
  step: (d: number) => void;
  /** Turns to the page holding item `i`. */
  show: (i: number) => void;
}

/** `reset`: back to page 1 when it changes (default: when the item count changes). */
export function createPaged<T>(all: Accessor<T[]>, size: Accessor<number>, reset?: Accessor<unknown>): Paged<T> {
  const [raw, setRaw] = createSignal(0);
  const per = () => Math.max(1, size());
  const pages = () => Math.max(1, Math.ceil(all().length / per()));
  const page = () => Math.min(raw(), pages() - 1);
  // a new filter result starts on page 1; replaced rows (same count) keep the page
  createEffect(on(reset ?? (() => all().length), () => setRaw(0), { defer: true }));
  const go = (p: number) => setRaw(Math.max(0, Math.min(pages() - 1, p)));
  return {
    items: () => all().slice(page() * per(), page() * per() + per()),
    page,
    pages,
    offset: () => page() * per(),
    go,
    step: (d) => go(page() + d),
    show: (i) => go(Math.floor(i / per())),
  };
}

/**
 * How many `itemH`-high lines fit between the top of `el` and the bottom of the window,
 * leaving `reserve` px (pager, padding). Re-measured on resize and layout changes.
 */
export function fitCount(el: Accessor<HTMLElement | undefined>, itemH: number | (() => number), reserve = 56, min = 3): Accessor<number> {
  // read at measure time (not tracked): a function can return the live height of a rendered item
  const h = typeof itemH === "number" ? () => itemH : itemH;
  const [n, setN] = createSignal(min);
  const measure = () => {
    const e = el();
    if (!e) return;
    const avail = window.innerHeight - e.getBoundingClientRect().top - reserve;
    setN(Math.max(min, Math.floor(avail / Math.max(1, h()))));
  };
  let frame = 0;
  const schedule = () => (cancelAnimationFrame(frame), (frame = requestAnimationFrame(measure)));
  const ro = new ResizeObserver(schedule);
  createEffect(() => {
    const e = el();
    if (!e) return;
    ro.observe(document.body);
    if (e.parentElement) ro.observe(e.parentElement);
    schedule();
  });
  window.addEventListener("resize", schedule);
  onCleanup(() => (ro.disconnect(), cancelAnimationFrame(frame), window.removeEventListener("resize", schedule)));
  return n;
}

export default function Pager(props: { paged: Paged<unknown>; class?: string }) {
  const p = props.paged;
  return (
    <Show when={p.pages() > 1}>
      <div class={`pager ${props.class ?? ""}`}>
        <button class="pager-btn" disabled={p.page() === 0} onClick={() => p.go(0)} title="First page">«</button>
        <button class="pager-btn" disabled={p.page() === 0} onClick={() => p.step(-1)} title="Previous page">‹</button>
        <span>{p.page() + 1} / {p.pages()}</span>
        <button class="pager-btn" disabled={p.page() >= p.pages() - 1} onClick={() => p.step(1)} title="Next page">›</button>
        <button class="pager-btn" disabled={p.page() >= p.pages() - 1} onClick={() => p.go(p.pages() - 1)} title="Last page">»</button>
      </div>
    </Show>
  );
}
