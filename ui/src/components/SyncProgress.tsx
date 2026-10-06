import { createSignal, onCleanup } from "solid-js";
import { onSyncProgress, type SyncProgress as P } from "../ipc";
import Segments from "./Segments";

const LABELS: Record<P["phase"], string> = {
  rdb: "reading RetroArch databases",
  dat: "downloading arcade DATs",
};

/** Live progress of a database sync (mount only while syncing). */
export default function SyncProgress() {
  const [p, setP] = createSignal<P>({ phase: "rdb", done: 0, total: 0 });
  const unlisten = onSyncProgress(setP);
  onCleanup(() => void unlisten.then((f) => f()));
  const count = () => (p().total ? ` ${p().done}/${p().total}` : "…");
  return (
    <>
      <Segments value={p().done} max={p().total} />
      <p class="mono dim small">
        {LABELS[p().phase]}
        {count()}
      </p>
    </>
  );
}
