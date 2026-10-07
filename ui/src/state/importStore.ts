// Shared import state: the Import view builds, reviews and executes the plan.
import { createSignal } from "solid-js";
import { inbox, library, mode, refreshLibrary } from "./libraryStore";
import {
  executePlan, inboxClear, planImport, resolveAmbiguous, setVerdict, undoLast,
  type Choice, type DecisionView, type ExecResult, type PlanView, type Verdict,
} from "../ipc";

const load = (k: string) => {
  try { return localStorage.getItem(k) ?? ""; } catch { return ""; }
};
const save = (k: string, v: string) => {
  try { localStorage.setItem(k, v); } catch { /* storage unavailable */ }
};

export { inbox, library, mode, setInbox, setLibrary, setMode } from "./libraryStore";
export const [plan, setPlan] = createSignal<PlanView>();
/** Inbox files the last plan leaves alone; kept after executing so they can be cleared. */
export const [leftovers, setLeftovers] = createSignal<PlanView["leftovers"]>([]);
export const [busy, setBusy] = createSignal(false);
export const [executing, setExecuting] = createSignal(false);
/** When the current plan was built and whether it is a library-only audit. */
export const [planned, setPlanned] = createSignal<{ at: Date; audit: boolean }>();
/** Unique per decision (a tie's `path` is its system, shared by all ties of that system). */
export const decisionKey = (d: DecisionView) =>
  d.kind === "tie" ? `tie:${d.path}:${d.options.map((o) => o.name).join("|")}` : `${d.kind}:${d.path}`;

/** Decisions taken in the current plan (key → label); applied on the next plan. */
export const [decided, setDecided] = createSignal<Map<string, string>>(new Map());
let lastWithInbox = true;

/** Decisions left open by the last plan (kind → count), remembered for the Dashboard. */
export interface OpenDecisions { ts: number; kinds: Record<string, number> }
const loadOpen = (): OpenDecisions | undefined => {
  try { return JSON.parse(load("rombro.open")) as OpenDecisions; } catch { return undefined; }
};
export const [openDecisions, setOpenDecisions] = createSignal<OpenDecisions | undefined>(loadOpen());
function rememberOpen(p: PlanView) {
  const kinds: Record<string, number> = {};
  for (const d of p.decisions) kinds[d.kind] = (kinds[d.kind] ?? 0) + 1;
  const o = { ts: Date.now(), kinds };
  setOpenDecisions(o);
  save("rombro.open", JSON.stringify(o));
}
export const [status, setStatus] = createSignal<{ ok: boolean; text: string }>();
/** The last executed plan: the operations that ran (in order) and how it ended. */
export interface LastRun { ops: PlanView["ops"]; journal: number | null; error: string | null }
export const [lastRun, setLastRun] = createSignal<LastRun>();

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
    setPlanned({ at: new Date(), audit: !withInbox });
    setLeftovers(p.leftovers);
    rememberOpen(p);
    return p;
  });

export const execute = () =>
  guard(async () => {
    const ops = plan()?.ops ?? [];
    setExecuting(true);
    const r: ExecResult = await executePlan().finally(() => setExecuting(false));
    setLastRun({ ops: ops.slice(0, r.done), journal: r.journal, error: r.error });
    setPlan(undefined);
    void refreshLibrary();
    // the "Last run" report shows the outcome (and the error)
  });

/** Moves the inbox leftovers to the library trash (undo brings them back). */
export const clearInbox = () =>
  guard(async () => {
    const r = await inboxClear();
    setLeftovers([]);
    setStatus(
      r.error
        ? { ok: false, text: `stopped after ${r.done} files: ${r.error}` }
        : { ok: true, text: `moved ${r.done} inbox files to the trash${r.journal ? ` (journal #${r.journal})` : ""}` },
    );
  });

export const undo = () =>
  guard(async () => {
    const n = await undoLast();
    void refreshLibrary();
    setStatus({ ok: true, text: n ? `reverted ${n} operations` : "nothing to undo" });
  });

/** Re-plans with the same inputs so recorded decisions take effect. */
export const replan = () => buildPlan(lastWithInbox);

const mark = (d: DecisionView, label: string) => setDecided((m) => new Map(m).set(decisionKey(d), label));

/** Decisions whose answer is being saved. Per item, not the global `busy`: answering one
 * must not disable the others or re-layout the page. */
export const [saving, setSaving] = createSignal<Set<string>>(new Set());

async function decide(d: DecisionView, f: () => Promise<void>) {
  const key = decisionKey(d);
  if (busy() || saving().has(key)) return;
  setSaving((s) => new Set(s).add(key));
  try {
    await f();
  } catch (e) {
    setStatus({ ok: false, text: String(e) });
  } finally {
    setSaving((s) => {
      const n = new Set(s);
      n.delete(key);
      return n;
    });
  }
}

/** Picks a candidate for an ambiguous match. */
export const pick = (d: DecisionView, c: Choice) =>
  decide(d, async () => {
    await resolveAmbiguous(d.path, c);
    mark(d, `→ ${c.name}`);
  });

/** Keeps or discards a release 1G1R rejected. */
export const judge = (d: DecisionView, v: Verdict) =>
  decide(d, async () => {
    const kept = d.detail.split("→")[1]?.trim();
    await setVerdict(d.options[0], v, kept ? `${d.headline} · ${kept}` : d.headline);
    mark(d, `→ ${v}`);
  });

/** Resolves a 1G1R tie in favour of release `c`. */
export const prefer = (d: DecisionView, c: Choice) =>
  decide(d, async () => {
    const others = d.options.filter((o) => o.name !== c.name).map((o) => o.name);
    await setVerdict(c, "prefer", `1G1R tie with ${others.join(", ")}`);
    mark(d, `→ ${c.name}`);
  });
