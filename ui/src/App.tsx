import { createEffect, createSignal, Match, onCleanup, onMount, Switch } from "solid-js";
import Topbar from "./components/Topbar";
import Toasts from "./components/Toasts";
import CommandPalette, { type Command } from "./components/CommandPalette";
import Dashboard from "./views/Dashboard";
import Placeholder from "./views/Placeholder";
import System from "./views/System";
import RetroArch from "./views/RetroArch";
import Rules from "./views/Rules";
import Sidebar from "./components/Sidebar";
import { libraryJump } from "./state/jump";
import Import from "./views/Import";
import Library from "./views/Library";
import { execute, plan, undo } from "./state/importStore";
import { initLibrary } from "./state/libraryStore";
import { refreshPlayers } from "./state/popularity";
import { effects, setEffects, setTheme, THEMES } from "./state/appearance";
import { VIEWS, type ViewId } from "./views";
import { SECTIONS, setSystemSection } from "./state/settingsNav";

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
    ...SECTIONS.map((s) => ({
      id: `settings:${s.id}`,
      label: `System: ${s.label}`,
      run: () => (setSystemSection(s.id), setView("system")),
    })),
    ...(plan()?.ops.length ? [{ id: "exec", label: "Execute plan", run: () => void execute() }] : []),
    { id: "undo", label: "Undo last run", run: () => void undo() },
    { id: "fx", label: `Effects: ${effects() ? "off" : "on"}`, run: () => setEffects(!effects()) },
    ...THEMES.map((t) => ({ id: `theme:${t.id}`, label: `Theme: ${t.label}`, run: () => setTheme(t.id) })),
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
    void initLibrary().then(refreshPlayers);
  });
  onCleanup(() => window.removeEventListener("keydown", onKey));

  // dashboard items jump into the Library (it applies the jump itself)
  createEffect(() => libraryJump() && setView("library"));

  const title = () => VIEWS.find((v) => v.id === view())?.label ?? "";

  return (
    <div class="shell">
      <Sidebar view={view()} onSelect={setView} />
      <Topbar title={title()} onPalette={() => setPalette(true)} />
      <Toasts />
      <main class="content">
        <Switch fallback={<Placeholder title={title()} />}>
          <Match when={view() === "dashboard"}>
            <Dashboard
              onLibrary={() => setView("library")}
              onDatabases={() => (setSystemSection("databases"), setView("system"))}
            />
          </Match>
          <Match when={view() === "import"}>
            <Import />
          </Match>
          <Match when={view() === "library"}>
            <Library onSettings={() => (setSystemSection("library"), setView("system"))} />
          </Match>
          <Match when={view() === "retroarch"}>
            <RetroArch />
          </Match>
          <Match when={view() === "rules"}>
            <Rules />
          </Match>
          <Match when={view() === "system"}>
            <System onReview={() => setView("import")} />
          </Match>
        </Switch>
      </main>
      <CommandPalette open={palette()} commands={commands()} onClose={() => setPalette(false)} />
    </div>
  );
}
