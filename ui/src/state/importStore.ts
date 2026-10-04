// Shared import state: Inbox view builds the plan, Plan view reviews and executes it.
import { createSignal } from "solid-js";
import { executePlan, planImport, undoLast, type ExecResult, type Mode, type PlanView } from "../ipc";

const load = (k: string) => {
  try { return localStorage.getItem(k) ?? ""; } catch { return ""; }
};
const save = (k: string, v: string) => {
  try { localStorage.setItem(k, v); } catch { /* storage unavailable */ }
};

export const [library, setLibraryRaw] = createSignal(load("rombro.library"));
export const setLibrary = (v: string) => (setLibraryRaw(v), save("rombro.library", v));
export const [inbox, setInbox] = createSignal("");
export const [mode, setMode] = createSignal<Mode>("move");
export const [plan, setPlan] = createSignal<PlanView>();
export const [busy, setBusy] = createSignal(false);
export const [status, setStatus] = createSignal<{ ok: boolean; text: string }>();

async function guard<T>(f: () => Promise<T>): Promise<T | undefined> {
  if (busy()) return;
  setBusy(true);
  setStatus(undefined);
  try {
    return await f();
  } catch (e) {
    setStatus({ ok: false, text: String(e) });
  } finally {
    setBusy(false);
  }
}

/** Builds a plan; `withInbox = false` audits the library only. */
export const buildPlan = (withInbox: boolean) =>
  guard(async () => {
    setPlan(undefined);
    const p = await planImport(withInbox ? inbox() || null : null, library(), mode());
    setPlan(p);
    return p;
  });

export const execute = () =>
  guard(async () => {
    const r: ExecResult = await executePlan();
    setPlan(undefined);
    setStatus(
      r.error
        ? { ok: false, text: `stopped after ${r.done} ops: ${r.error}` }
        : { ok: true, text: `executed ${r.done} operations${r.journal ? ` (journal #${r.journal})` : ""}` },
    );
  });

export const undo = () =>
  guard(async () => {
    const n = await undoLast();
    setStatus({ ok: true, text: n ? `reverted ${n} operations` : "nothing to undo" });
  });
