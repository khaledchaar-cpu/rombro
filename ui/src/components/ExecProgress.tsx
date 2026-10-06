import { createSignal, onCleanup } from "solid-js";
import { onPhaseProgress, type PhaseProgress } from "../ipc";
import Segments from "./Segments";

/** Live progress of a running plan execution (mount only while executing). */
export default function ExecProgress() {
  const [p, setP] = createSignal<PhaseProgress>({ phase: "move", done: 0, total: 0 });
  const unlisten = onPhaseProgress("execute://progress", setP);
  onCleanup(() => void unlisten.then((f) => f()));
  const finishing = () => p().phase === "finish";
  return (
    <>
      <Segments value={finishing() ? 1 : p().done} max={finishing() ? 1 : p().total} />
      <p class="mono dim small">
        {finishing()
          ? "updating index and journal…"
          : p().total
            ? `moving files ${p().done}/${p().total}`
            : "starting…"}
      </p>
    </>
  );
}
