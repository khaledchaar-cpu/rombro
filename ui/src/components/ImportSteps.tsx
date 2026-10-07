import { For } from "solid-js";
import { busy, executing, lastRun, plan, saving } from "../state/importStore";

const STEPS = ["Folders", "Scan", "Review", "Execute", "Done"] as const;

/** Where the import stands: folders → scan/plan → review → execute → done. */
export default function ImportSteps() {
  const current = () => {
    if (executing()) return 3;
    const p = plan();
    if (p) return p.ops.length || p.decisions.length || saving().size ? 2 : 4;
    if (busy()) return 1;
    return lastRun() ? 4 : 0;
  };
  return (
    <ol class="steps" aria-label="Import progress">
      <For each={STEPS}>
        {(label, i) => (
          <li
            classList={{ done: i() < current(), active: i() === current(), live: i() === current() && busy() }}
            aria-current={i() === current() ? "step" : undefined}
          >
            <span class="step-no">{String(i() + 1).padStart(2, "0")}</span>
            {label}
          </li>
        )}
      </For>
    </ol>
  );
}
