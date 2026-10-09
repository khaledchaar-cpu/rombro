import { createSignal, Show } from "solid-js";
import { thumbUrl, type ThumbKind } from "../ipc";

/** Lazy libretro thumbnail; placeholder tile (initials) if missing or offline. */
export default function Cover(props: { system: string; name: string; kind?: ThumbKind; class?: string }) {
  const [failed, setFailed] = createSignal<string | null>(null);
  const src = () => thumbUrl(props.system, props.name, props.kind ?? "boxart");
  const initials = () =>
    props.name
      .replace(/[([].*$/, "")
      .split(/\s+/)
      .filter((w) => /^[\p{L}\p{N}]/u.test(w))
      .slice(0, 2)
      .map((w) => w[0].toUpperCase())
      .join("");
  return (
    <Show
      when={src() && failed() !== src() && src()}
      fallback={
        <div class={`cover cover-ph ${props.class ?? ""}`} title={props.name}>
          <span>{initials()}</span>
        </div>
      }
    >
      {(s) => (
        <img
          class={`cover ${props.class ?? ""}`}
          src={s()}
          alt=""
          loading="lazy"
          decoding="async"
          onError={() => setFailed(s())}
        />
      )}
    </Show>
  );
}
