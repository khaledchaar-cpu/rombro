// Dashboard → "Library growth": items in the library over time (cumulative, from each row's
// `added`), one point per day, week or month depending on the span. Hover shows the value.
import { createMemo, createSignal, onCleanup, Show } from "solid-js";
import Panel from "../components/Panel";
import { libraryRows } from "../state/libraryStore";

const H = 180;
const PAD = { l: 52, r: 12, t: 12, b: 24 };
const DAY = 86400;
const fmt = new Intl.NumberFormat("en-US");

interface Point { t: number; total: number; added: number }

/** Bucket start (unix seconds, local time) for `t` at the given step. */
function floorTo(t: number, step: "day" | "week" | "month"): number {
  const d = new Date(t * 1000);
  d.setHours(0, 0, 0, 0);
  if (step === "week") d.setDate(d.getDate() - ((d.getDay() + 6) % 7));
  if (step === "month") d.setDate(1);
  return d.getTime() / 1000;
}
function next(t: number, step: "day" | "week" | "month"): number {
  const d = new Date(t * 1000);
  if (step === "day") d.setDate(d.getDate() + 1);
  else if (step === "week") d.setDate(d.getDate() + 7);
  else d.setMonth(d.getMonth() + 1);
  return d.getTime() / 1000;
}

export function GrowthPanel() {
  // drawn at the panel's real width so text and markers keep their size
  const [W, setW] = createSignal(800);
  const ro = new ResizeObserver(([e]) => e.contentRect.width && setW(e.contentRect.width));
  onCleanup(() => ro.disconnect());
  const series = createMemo(() => {
    const times = libraryRows().map((r) => r.added).filter((t) => t > 0).sort((a, b) => a - b);
    if (!times.length) return { step: "day" as const, points: [] as Point[] };
    const now = Date.now() / 1000;
    const span = now - times[0];
    const step = span < 60 * DAY ? "day" : span < 2 * 365 * DAY ? "week" : "month";
    const points: Point[] = [];
    let i = 0;
    for (let b = floorTo(times[0], step); b <= now; b = next(b, step)) {
      const end = next(b, step);
      let added = 0;
      while (i < times.length && times[i] < end) (i++, added++);
      points.push({ t: b, total: i, added });
    }
    return { step, points };
  });

  const max = () => Math.max(1, ...series().points.map((p) => p.total));
  const x = (i: number) => PAD.l + (series().points.length < 2 ? 0 : (i / (series().points.length - 1)) * (W() - PAD.l - PAD.r));
  const y = (v: number) => PAD.t + (1 - v / max()) * (H - PAD.t - PAD.b);
  const line = () => series().points.map((p, i) => `${i ? "L" : "M"}${x(i).toFixed(1)},${y(p.total).toFixed(1)}`).join("");
  const area = () => {
    const n = series().points.length;
    return n ? `${line()}L${x(n - 1).toFixed(1)},${y(0)}L${x(0).toFixed(1)},${y(0)}Z` : "";
  };
  const label = (t: number) => {
    const d = new Date(t * 1000);
    return series().step === "month"
      ? d.toLocaleDateString(undefined, { month: "short", year: "numeric" })
      : d.toLocaleDateString(undefined, { day: "numeric", month: "short", year: "2-digit" });
  };
  const ticks = () => [0, 0.5, 1].map((f) => Math.round(max() * f));

  const [hover, setHover] = createSignal<number | null>(null);
  let svg!: SVGSVGElement;
  const onMove = (e: PointerEvent) => {
    const r = svg.getBoundingClientRect();
    const px = ((e.clientX - r.left) / r.width) * W();
    const n = series().points.length;
    if (n < 2) return setHover(n ? 0 : null);
    const i = Math.round(((px - PAD.l) / (W() - PAD.l - PAD.r)) * (n - 1));
    setHover(Math.max(0, Math.min(n - 1, i)));
  };
  const hp = () => (hover() == null ? null : series().points[hover()!]);

  return (
    <Panel title="Library growth" class="wide">
      <Show when={series().points.length > 1} fallback={<p class="dim small">Grows as you import – nothing to chart yet.</p>}>
        <div class="growth" ref={(el) => ro.observe(el)}>
          <svg
            ref={svg}
            viewBox={`0 0 ${W()} ${H}`}
            role="img"
            aria-label={`Library growth: ${fmt.format(max())} items now`}
            onPointerMove={onMove}
            onPointerLeave={() => setHover(null)}
          >
            {ticks().map((v) => (
              <>
                <line class="growth-grid" x1={PAD.l} x2={W() - PAD.r} y1={y(v)} y2={y(v)} />
                <text class="growth-axis" x={PAD.l - 6} y={y(v) + 4} text-anchor="end">{fmt.format(v)}</text>
              </>
            ))}
            <path class="growth-area" d={area()} />
            <path class="growth-line" d={line()} />
            <text class="growth-axis" x={PAD.l} y={H - 6}>{label(series().points[0].t)}</text>
            <text class="growth-axis" x={W() - PAD.r} y={H - 6} text-anchor="end">{label(series().points.at(-1)!.t)}</text>
            <Show when={hp()}>
              {(p) => (
                <>
                  <line class="growth-cross" x1={x(hover()!)} x2={x(hover()!)} y1={PAD.t} y2={y(0)} />
                  <circle class="growth-dot" cx={x(hover()!)} cy={y(p().total)} r="5" />
                </>
              )}
            </Show>
          </svg>
          <Show when={hp()}>
            {(p) => (
              <div class="growth-tip" style={{ left: `${(x(hover()!) / W()) * 100}%` }}>
                <strong>{fmt.format(p().total)}</strong> items
                <span class="dim"> · {label(p().t)} · +{fmt.format(p().added)}</span>
              </div>
            )}
          </Show>
        </div>
      </Show>
    </Panel>
  );
}
