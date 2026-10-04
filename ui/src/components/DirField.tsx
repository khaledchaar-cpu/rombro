import { pickDir } from "../ipc";

/** Path input with a native folder picker. */
export default function DirField(props: { label: string; value: string; onChange: (v: string) => void }) {
  const browse = async () => {
    const d = await pickDir(props.label);
    if (d) props.onChange(d);
  };
  return (
    <label class="dirfield">
      <span class="dim small">{props.label}</span>
      <div class="row">
        <input
          class="field mono"
          placeholder="/path/to/dir"
          value={props.value}
          onInput={(e) => props.onChange(e.currentTarget.value)}
        />
        <button type="button" class="btn ghost" onClick={browse}>
          Browse
        </button>
      </div>
    </label>
  );
}
