// Achievement toasts: shows achievements unlocked by the latest stats computation.
import { createEffect, createSignal, For } from "solid-js";
import { stats } from "../state/gamify";

interface Toast { id: number; title: string; xp: number }
const TTL_MS = 6000;

export default function Toasts() {
  const [toasts, setToasts] = createSignal<Toast[]>([]);
  let seq = 0;
  const dismiss = (id: number) => setToasts((t) => t.filter((x) => x.id !== id));

  createEffect(() => {
    const s = stats();
    if (!s?.new.length) return;
    const fresh = s.achievements
      .filter(([a]) => s.new.includes(a.id))
      .map(([a]) => ({ id: ++seq, title: a.title, xp: a.xp }));
    setToasts((t) => [...t, ...fresh]);
    for (const f of fresh) setTimeout(() => dismiss(f.id), TTL_MS);
  });

  return (
    <div class="toasts" role="status" aria-live="polite">
      <For each={toasts()}>
        {(t) => (
          <button class="toast" onClick={() => dismiss(t.id)}>
            <span class="dim small">Achievement unlocked</span>
            <strong>{t.title}</strong>
            <span class="small">+{t.xp} XP</span>
          </button>
        )}
      </For>
    </div>
  );
}
