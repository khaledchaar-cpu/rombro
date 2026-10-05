import { createSignal, onCleanup } from "solid-js";
import { onImportProgress, type ImportProgress } from "../ipc";
import Segments from "./Segments";

/** Live scan progress of a running plan (mount only while planning). */
export default function ScanProgress() {
  const [p, setP] = createSignal<ImportProgress>({ phase: "library", done: 0, total: 0 });
  const unlisten = onImportProgress(setP);
  onCleanup(() => void unlisten.then((f) => f()));
  return (
    <>
      <Segments value={p().done} max={p().total} />
      <p class="mono dim small">
        {p().total ? `scanning ${p().phase} ${p().done}/${p().total}` : "scanning…"}
      </p>
    </>
  );
}
