import { createResource, createSignal, Show } from "solid-js";
import Panel from "./Panel";
import { cheevosLogin, cheevosLogout, cheevosSetHardcore, cheevosStatus } from "../ipc";

/** RetroAchievements login for RetroArch (unlocks while playing) and hardcore mode. */
export default function CheevosAccountPanel() {
  const [status, { refetch, mutate }] = createResource(cheevosStatus);
  const [user, setUser] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [msg, setMsg] = createSignal("");

  const login = async () => {
    setBusy(true);
    setMsg("");
    try {
      await cheevosLogin(user().trim(), password());
      setUser("");
      void refetch();
    } catch (e) {
      setMsg(String(e));
    } finally {
      setPassword("");
      setBusy(false);
    }
  };
  const logout = async () => {
    await cheevosLogout();
    void refetch();
  };
  const hardcore = async (on: boolean) => {
    const s = status();
    if (s) mutate({ ...s, hardcore: on });
    try {
      await cheevosSetHardcore(on);
    } catch (e) {
      setMsg(String(e));
      void refetch();
    }
  };

  return (
    <Panel title="RetroAchievements">
      <Show
        when={status()?.user}
        fallback={
          <>
            <p class="dim small">
              Log in so RetroArch unlocks achievements while you play. Your password is sent once to
              retroachievements.org; only the login token is kept.
            </p>
            <form
              class="row"
              onSubmit={(e) => {
                e.preventDefault();
                void login();
              }}
            >
              <input
                class="field"
                placeholder="User name"
                autocomplete="username"
                value={user()}
                onInput={(e) => setUser(e.currentTarget.value)}
              />
              <input
                class="field"
                type="password"
                placeholder="Password"
                autocomplete="current-password"
                value={password()}
                onInput={(e) => setPassword(e.currentTarget.value)}
              />
              <button class="btn" type="submit" disabled={busy() || !user().trim() || !password()}>
                Log in
              </button>
            </form>
          </>
        }
      >
        <div class="row">
          <span>
            Logged in as <strong>{status()?.user}</strong>
          </span>
          <button class="btn ghost" onClick={() => void logout()}>
            Log out
          </button>
        </div>
      </Show>
      <label class="check">
        <input
          type="checkbox"
          checked={status()?.hardcore ?? false}
          onChange={(e) => void hardcore(e.currentTarget.checked)}
        />
        Hardcore mode – no savestates, rewind or cheats; unlocks count as hardcore on the site
      </label>
      <Show when={msg()}>
        <p class="dim small">{msg()}</p>
      </Show>
    </Panel>
  );
}
