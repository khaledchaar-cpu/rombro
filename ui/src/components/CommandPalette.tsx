import { createEffect, createSignal, For, Show } from "solid-js";
import Pager, { createPaged } from "./Pager";

export interface Command { id: string; label: string; hint?: string; run: () => void }

export default function CommandPalette(props: {
  open: boolean;
  commands: Command[];
  onClose: () => void;
}) {
  const [query, setQuery] = createSignal("");
  const [sel, setSel] = createSignal(0);
  let input: HTMLInputElement | undefined;

  const matches = () => {
    const q = query().toLowerCase();
    return props.commands.filter((c) => c.label.toLowerCase().includes(q));
  };

  const paged = createPaged(matches, () => 10, query);
  // the page follows the arrow-key selection
  createEffect(() => {
    const c = matches()[sel()];
    if (c) paged.show(c);
  });

  createEffect(() => {
    if (props.open) {
      setQuery("");
      setSel(0);
      queueMicrotask(() => input?.focus());
    }
  });

  const run = (c: Command | undefined) => {
    if (!c) return;
    props.onClose();
    c.run();
  };

  const onKey = (e: KeyboardEvent) => {
    const n = matches().length;
    if (e.key === "Escape") props.onClose();
    else if (e.key === "ArrowDown") setSel((sel() + 1) % Math.max(n, 1));
    else if (e.key === "ArrowUp") setSel((sel() - 1 + n) % Math.max(n, 1));
    else if (e.key === "Enter") run(matches()[sel()]);
    else return;
    e.preventDefault();
  };

  return (
    <Show when={props.open}>
      <div class="palette-backdrop" onClick={props.onClose}>
        <div class="palette panel" onClick={(e) => e.stopPropagation()}>
          <input
            ref={input}
            class="palette-input mono"
            placeholder="> type a command"
            value={query()}
            onInput={(e) => {
              setQuery(e.currentTarget.value);
              setSel(0);
            }}
            onKeyDown={onKey}
          />
          <ul class="palette-list">
            <For each={paged.items()}>
              {(c, i) => (
                <li
                  classList={{ selected: paged.offset() + i() === sel() }}
                  onMouseEnter={() => setSel(paged.offset() + i())}
                  onClick={() => run(c)}
                >
                  <span>{c.label}</span>
                  {c.hint && <kbd>{c.hint}</kbd>}
                </li>
              )}
            </For>
          </ul>
          <Pager paged={paged} />
        </div>
      </div>
    </Show>
  );
}
