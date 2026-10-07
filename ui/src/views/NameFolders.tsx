import { For, createSignal } from "solid-js";

/** Folder → system map for the `name-only` rule; matching is case-insensitive on the folder name. */
export default function NameFolders(props: {
  folders: Record<string, string>;
  onChange: (folders: Record<string, string>) => void;
}) {
  const [folder, setFolder] = createSignal("");
  const [system, setSystem] = createSignal("");
  const remove = (f: string) => {
    const next = { ...props.folders };
    delete next[f];
    props.onChange(next);
  };
  const add = () => {
    const f = folder().trim().toLowerCase();
    const s = system().trim();
    if (!f || !s) return;
    props.onChange({ ...props.folders, [f]: s });
    setFolder("");
    setSystem("");
  };
  return (
    <div class="prio">
      <ol class="prio-list">
        <For each={Object.entries(props.folders)}>
          {([f, s]) => (
            <li class="prio-item">
              <span class="prio-name">
                {f} <span class="dim">→</span> {s}
              </span>
              <button class="btn ghost sm" onClick={() => remove(f)} title="Remove">✕</button>
            </li>
          )}
        </For>
      </ol>
      <div class="row">
        <input class="field mono" placeholder="Folder, e.g. openbor" value={folder()} onInput={(e) => setFolder(e.currentTarget.value)} />
        <input
          class="field mono"
          placeholder="System, e.g. OpenBOR"
          value={system()}
          onInput={(e) => setSystem(e.currentTarget.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
        />
        <button class="btn ghost" onClick={add}>Add</button>
      </div>
    </div>
  );
}
