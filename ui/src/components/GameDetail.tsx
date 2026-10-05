import { createResource, For, onCleanup, Show } from "solid-js";
import Panel from "./Panel";
import { thumbnail, type LibraryRow, type ThumbKind } from "../ipc";

const KINDS: { kind: ThumbKind; label: string }[] = [
  { kind: "boxart", label: "Boxart" },
  { kind: "title", label: "Title" },
  { kind: "snap", label: "Snap" },
];

function Thumb(props: { row: LibraryRow; kind: ThumbKind; label: string }) {
  const [src] = createResource(
    () => [props.row.system, props.row.name] as const,
    async ([system, name]) => (system ? thumbnail(system, name, props.kind) : null),
  );
  onCleanup(() => {
    const s = src.latest;
    if (s) URL.revokeObjectURL(s);
  });
  return (
    <figure class="thumb">
      <Show when={!src.loading} fallback={<div class="thumb-ph dim small">loading…</div>}>
        <Show when={src()} fallback={<div class="thumb-ph dim small">{src.error ? "offline" : "no image"}</div>}>
          {(s) => <img src={s()} alt={props.label} />}
        </Show>
      </Show>
      <figcaption class="dim small">{props.label}</figcaption>
    </figure>
  );
}

export default function GameDetail(props: { row: LibraryRow }) {
  const date = () => (props.row.added ? new Date(props.row.added * 1000).toLocaleString() : "–");
  return (
    <Panel title="Details" class="wide">
      <div class="detail">
        <div class="thumbs">
          <For each={KINDS}>{(k) => <Thumb row={props.row} kind={k.kind} label={k.label} />}</For>
        </div>
        <dl class="meta small">
          <dt>Name</dt><dd>{props.row.name || "–"}</dd>
          <dt>System</dt><dd>{props.row.system || "–"}</dd>
          <dt>State</dt><dd class={`tag-${props.row.state}`}>{props.row.state}</dd>
          <dt>Regions</dt><dd>{props.row.regions.join(", ") || "–"}</dd>
          <dt>Files</dt><dd>{props.row.files}</dd>
          <dt>Added</dt><dd>{date()}</dd>
          <dt>Path</dt><dd class="mono">{props.row.path}</dd>
        </dl>
      </div>
    </Panel>
  );
}
