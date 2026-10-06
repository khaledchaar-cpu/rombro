import { createSignal, onCleanup } from "solid-js";
import { onPhaseProgress, type PhaseProgress as P } from "../ipc";
import Segments from "./Segments";

/** Live progress of a phased backend task (mount only while it runs). */
export default function PhaseProgress(props: { event: string; labels: Record<string, string> }) {
  const [p, setP] = createSignal<P>({ phase: "", done: 0, total: 0 });
  const [secs, setSecs] = createSignal(0);
  const unlisten = onPhaseProgress(props.event, setP);
  const started = Date.now();
  const timer = setInterval(() => setSecs(Math.floor((Date.now() - started) / 1000)), 1000);
  onCleanup(() => {
    clearInterval(timer);
    void unlisten.then((f) => f());
  });
  const count = () => (p().total ? ` ${p().done}/${p().total}` : "…");
  return (
    <>
      <Segments value={p().done} max={p().total} />
      <p class="mono dim small">
        {props.labels[p().phase] ?? "starting"}
        {count()}
        {p().item ? ` · ${p().item}` : ""} · {secs()}s
      </p>
    </>
  );
}
