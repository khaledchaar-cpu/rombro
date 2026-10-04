import { For } from "solid-js";

/** Progress bar drawn as discrete HUD segments. */
export default function Segments(props: { value: number; max: number; count?: number }) {
  const n = () => props.count ?? 32;
  const lit = () => (props.max > 0 ? Math.round((props.value / props.max) * n()) : 0);
  return (
    <div class="segments" role="progressbar" aria-valuenow={props.value} aria-valuemax={props.max}>
      <For each={Array.from({ length: n() })}>{(_, i) => <i classList={{ on: i() < lit() }} />}</For>
    </div>
  );
}
