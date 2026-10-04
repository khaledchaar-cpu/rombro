import { createSignal, Match, onCleanup, onMount, Switch } from "solid-js";
import Sidebar from "./components/Sidebar";
import Topbar from "./components/Topbar";
import CommandPalette, { type Command } from "./components/CommandPalette";
import Dashboard from "./views/Dashboard";
import Placeholder from "./views/Placeholder";
import { VIEWS, type ViewId } from "./views";

export default function App() {
  const [view, setView] = createSignal<ViewId>("dashboard");
  const [palette, setPalette] = createSignal(false);
  const [effects, setEffects] = createSignal(true);

  const toggleEffects = () => {
    setEffects(!effects());
    document.documentElement.dataset.effects = effects() ? "on" : "off";
  };

  const commands = (): Command[] => [
    ...VIEWS.map((v) => ({
      id: `go:${v.id}`,
      label: `Go to ${v.label}`,
      hint: `Ctrl+${v.key}`,
      run: () => setView(v.id),
    })),
    { id: "fx", label: `Effects: ${effects() ? "off" : "on"}`, run: toggleEffects },
  ];

  const onKey = (e: KeyboardEvent) => {
    if (!(e.ctrlKey || e.metaKey)) return;
    if (e.key === "k") {
      e.preventDefault();
      setPalette(!palette());
      return;
    }
    const v = VIEWS.find((v) => v.key === e.key);
    if (v) {
      e.preventDefault();
      setView(v.id);
    }
  };
  onMount(() => window.addEventListener("keydown", onKey));
  onCleanup(() => window.removeEventListener("keydown", onKey));

  const title = () => VIEWS.find((v) => v.id === view())?.label ?? "";

  return (
    <div class="shell">
      <Sidebar view={view()} onSelect={setView} />
      <Topbar title={title()} onPalette={() => setPalette(true)} />
      <main class="content">
        <Switch fallback={<Placeholder title={title()} />}>
          <Match when={view() === "dashboard"}>
            <Dashboard />
          </Match>
        </Switch>
      </main>
      <CommandPalette open={palette()} commands={commands()} onClose={() => setPalette(false)} />
    </div>
  );
}
