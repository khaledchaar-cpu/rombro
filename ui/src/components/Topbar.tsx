export default function Topbar(props: { title: string; onPalette: () => void }) {
  return (
    <header class="topbar">
      <h2>{props.title}</h2>
      <button class="palette-trigger" onClick={props.onPalette}>
        <span class="dim">Command…</span> <kbd>Ctrl K</kbd>
      </button>
    </header>
  );
}
