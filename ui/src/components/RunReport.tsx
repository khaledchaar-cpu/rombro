import { For, Show } from "solid-js";
import Pager, { createPaged } from "./Pager";
import Panel from "./Panel";
import { lastRun } from "../state/importStore";
import type { OpView } from "../ipc";


/** What an operation did, in a word, for grouping the report. */
function outcome(op: OpView): string {
  if (op.kind === "write") return "written";
  if (/\/_trash\//.test(op.to)) return "to the trash";
  if (/\/_quarantine\//.test(op.to)) return "quarantined";
  return "placed";
}

/** Report of the last executed plan: counts per outcome and the files it touched. */
export default function RunReport(props: { rel: (p: string) => string }) {
  return (
    <Show when={lastRun()}>
      {(r) => {
        const counts = () => {
          const c = new Map<string, number>();
          for (const op of r().ops) c.set(outcome(op), (c.get(outcome(op)) ?? 0) + 1);
          return [...c];
        };
        const paged = createPaged(() => r().ops, () => 12);
        return (
          <Panel title="Last run" class="wide">
            <p class={r().error ? "err" : "ok"}>
              {r().error
                ? `Stopped after ${r().ops.length} operations: ${r().error}`
                : `Done: ${r().ops.length} operations executed`}
              {r().journal ? ` (journal #${r().journal}, undo below reverts it)` : ""}
            </p>
            <div class="kpis">
              <For each={counts()}>
                {([k, n]) => (
                  <div>
                    <div class="kpi">{n}</div>
                    <span class="dim">{k}</span>
                  </div>
                )}
              </For>
            </div>
            <div class="oplist">
            <For each={paged.items()}>
              {(op) => (
                <div class="row mono">
                  <span>{props.rel(op.from ?? op.to)}</span>
                  <span class="dim"> → {outcome(op)}{op.why ? ` – ${op.why}` : ""}</span>
                </div>
              )}
            </For>
            </div>
            <Pager paged={paged} />
          </Panel>
        );
      }}
    </Show>
  );
}
