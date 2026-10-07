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
    <div class="overrides">
      <For each={Object.entries(props.folders)}>
        {([f, s]) => (
          <div class="row">
            <span class="mono">{f}</span>
            <span class="dim">→</span>
            <span class="mono">{s}</span>
            <button class="btn ghost" onClick={() => remove(f)}>Remove</button>
          </div>
        )}
      </For>
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
