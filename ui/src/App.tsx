import { createSignal, Match, onCleanup, onMount, Switch } from "solid-js";
import Sidebar from "./components/Sidebar";
import Topbar from "./components/Topbar";
import CommandPalette, { type Command } from "./components/CommandPalette";
import Dashboard from "./views/Dashboard";
import Placeholder from "./views/Placeholder";
import Settings from "./views/Settings";
import Inbox from "./views/Inbox";
import Plan from "./views/Plan";
import Library from "./views/Library";
import { execute, plan, undo } from "./state/importStore";
import { initLibrary } from "./state/libraryStore";
import { effects, setEffects, setTheme, theme } from "./state/appearance";
import { VIEWS, type ViewId } from "./views";

export default function App() {
  const [view, setView] = createSignal<ViewId>("dashboard");
  const [palette, setPalette] = createSignal(false);

  const commands = (): Command[] => [
    ...VIEWS.map((v) => ({
      id: `go:${v.id}`,
      label: `Go to ${v.label}`,
      hint: `Ctrl+${v.key}`,
      run: () => setView(v.id),
    })),
    ...(plan()?.ops.length ? [{ id: "exec", label: "Execute plan", run: () => void execute() }] : []),
    { id: "undo", label: "Undo last run", run: () => void undo() },
    { id: "fx", label: `Effects: ${effects() ? "off" : "on"}`, run: () => setEffects(!effects()) },
    { id: "theme", label: `Theme: ${theme() === "dark" ? "light" : "dark"}`, run: () => setTheme(theme() === "dark" ? "light" : "dark") },
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
  onMount(() => {
    window.addEventListener("keydown", onKey);
    void initLibrary();
  });
  onCleanup(() => window.removeEventListener("keydown", onKey));

  const title = () => VIEWS.find((v) => v.id === view())?.label ?? "";

  return (
    <div class="shell">
      <Sidebar view={view()} onSelect={setView} />
      <Topbar title={title()} onPalette={() => setPalette(true)} />
      <main class="content">
        <Switch fallback={<Placeholder title={title()} />}>
          <Match when={view() === "dashboard"}>
            <Dashboard onReview={() => setView("plan")} onLibrary={() => setView("library")} />
          </Match>
          <Match when={view() === "inbox"}>
            <Inbox onPlanned={() => setView("plan")} />
          </Match>
          <Match when={view() === "library"}>
            <Library />
          </Match>
          <Match when={view() === "plan"}>
            <Plan />
          </Match>
          <Match when={view() === "settings"}>
            <Settings />
          </Match>
        </Switch>
      </main>
      <CommandPalette open={palette()} commands={commands()} onClose={() => setPalette(false)} />
    </div>
  );
}
