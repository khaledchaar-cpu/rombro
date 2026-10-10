import { createSignal, onCleanup } from "solid-js";
import { onScanProgress, type ScanProgress } from "../ipc";
import Segments from "./Segments";

/** Live progress of loading the library (mount only while it loads). */
export default function LibraryProgress() {
  const [p, setP] = createSignal<ScanProgress>({ done: 0, total: 0 });
  const unlisten = onScanProgress(setP);
  onCleanup(() => void unlisten.then((f) => f()));
  return (
    <>
      <p class="dim">Loading the library…</p>
      <Segments value={p().done} max={p().total} />
      <p class="mono dim small">{p().total ? `${p().done}/${p().total} files` : "starting"}</p>
    </>
  );
}
