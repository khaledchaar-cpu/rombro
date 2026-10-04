import type { JSX } from "solid-js";

/** HUD frame with beveled corners and corner markers. */
export default function Panel(props: { title?: string; class?: string; children: JSX.Element }) {
  return (
    <section class={`panel ${props.class ?? ""}`}>
      {props.title && <h3 class="panel-title">{props.title}</h3>}
      {props.children}
    </section>
  );
}
