// Shared import state: Inbox view builds the plan, Plan view reviews and executes it.
import { createSignal } from "solid-js";
import {
  executePlan, planImport, resolveAmbiguous, setVerdict, undoLast,
  type Choice, type DecisionView, type ExecResult, type Mode, type PlanView, type Verdict,
} from "../ipc";

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
/** Decisions taken in the current plan (path → label); applied on the next plan. */
export const [decided, setDecided] = createSignal<Map<string, string>>(new Map());
let lastWithInbox = true;
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
    setDecided(new Map());
    lastWithInbox = withInbox;
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

/** Re-plans with the same inputs so recorded decisions take effect. */
export const replan = () => buildPlan(lastWithInbox);

const mark = (path: string, label: string) => setDecided((m) => new Map(m).set(path, label));

/** Picks a candidate for an ambiguous match. */
export const pick = (d: DecisionView, c: Choice) =>
  guard(async () => {
    await resolveAmbiguous(d.path, c);
    mark(d.path, `→ ${c.name}`);
  });

/** Keeps or discards a release 1G1R rejected. */
export const judge = (d: DecisionView, v: Verdict) =>
  guard(async () => {
    await setVerdict(d.options[0], v);
    mark(d.path, v === "keep" ? "→ keep" : "→ trash");
  });
