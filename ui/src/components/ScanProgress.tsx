import { createSignal, onCleanup } from "solid-js";
import { onImportProgress, type ImportProgress } from "../ipc";
import Segments from "./Segments";

const LABELS: Record<ImportProgress["phase"], string> = {
  library: "scanning library",
  inbox: "scanning inbox",
  planning: "building plan",
};

/** Live scan progress of a running plan (mount only while planning). */
export default function ScanProgress() {
  const [p, setP] = createSignal<ImportProgress>({
    phase: "library",
    done: 0,
    total: 0,
  });
  const [secs, setSecs] = createSignal(0);
  const unlisten = onImportProgress(setP);
  const started = Date.now();
  const timer = setInterval(
    () => setSecs(Math.floor((Date.now() - started) / 1000)),
    1000,
  );
  onCleanup(() => {
    clearInterval(timer);
    void unlisten.then((f) => f());
  });
  const count = () => (p().total ? ` ${p().done}/${p().total}` : "…");
  return (
    <>
      <Segments value={p().done} max={p().total} />
      <p class="mono dim small">
        {LABELS[p().phase]}
        {count()} · {secs()}s
      </p>
    </>
  );
}
