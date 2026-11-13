"use client";

import { useEffect, useRef, useState } from "react";
import { formatNumber, longDay, weekdayShort } from "@/lib/format";

type Day = { day: string; sales_sats: number; count: number };

const PAD = { top: 12, right: 8, bottom: 28, left: 48 };

function niceMax(v: number) {
  if (v <= 0) return 1000;
  const mag = 10 ** Math.floor(Math.log10(v));
  const step = [1, 2, 2.5, 5, 10].find((s) => s * mag >= v / 1) ?? 10;
  return step * mag;
}

const compact = (n: number) =>
  n >= 1_000_000 ? `${+(n / 1_000_000).toFixed(2)}M` : n >= 1000 ? `${+(n / 1000).toFixed(2)}k` : String(Math.round(n));

/** Bar path with 4px rounded top corners and a flat baseline. */
function barPath(x: number, y: number, w: number, h: number) {
  const r = Math.min(4, w / 2, h);
  return `M${x},${y + h}V${y + r}Q${x},${y} ${x + r},${y}H${x + w - r}Q${x + w},${y} ${x + w},${y + r}V${y + h}Z`;
}

/** Single-series daily sales: one validated hue, title names it, hover tooltip, text table for screen readers. */
export function SalesChart({ days }: { days: Day[] }) {
  const [hover, setHover] = useState<number | null>(null);
  // Draw at the real pixel width so text stays 11-12px on every screen.
  const ref = useRef<HTMLDivElement>(null);
  const [W, setW] = useState(640);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setW(Math.max(280, Math.round(e.contentRect.width))));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const H = W < 480 ? 180 : 220;
  const max = niceMax(Math.max(...days.map((d) => d.sales_sats)));
  const plotW = W - PAD.left - PAD.right;
  const plotH = H - PAD.top - PAD.bottom;
  const band = plotW / days.length;
  const barW = Math.min(36, band * 0.55);
  const y = (v: number) => PAD.top + plotH - (v / max) * plotH;
  const ticks = [0, max / 2, max];
  const active = hover !== null ? days[hover] : null;

  return (
    <div className="relative" ref={ref}>
      <svg width={W} height={H} viewBox={`0 0 ${W} ${H}`} className="block max-w-full" role="img" aria-label="Paid sales per day for the last 7 days">
        {ticks.map((t) => (
          <g key={t}>
            <line x1={PAD.left} x2={W - PAD.right} y1={y(t)} y2={y(t)} stroke={t === 0 ? "#d6d2c4" : "#efece3"} strokeWidth={1} />
            <text x={PAD.left - 8} y={y(t)} dy="0.32em" textAnchor="end" fontSize={11} fill="#8a948f">
              {compact(t)}
            </text>
          </g>
        ))}
        {days.map((d, i) => {
          const cx = PAD.left + band * i + band / 2;
          const h = Math.max(0, plotH - (y(d.sales_sats) - PAD.top));
          const isToday = i === days.length - 1;
          return (
            <g key={d.day}>
              {h > 0 && (
                <path
                  d={barPath(cx - barW / 2, y(d.sales_sats), barW, h)}
                  fill="var(--color-chart)"
                  opacity={hover === null || hover === i ? 1 : 0.45}
                />
              )}
              <text
                x={cx}
                y={H - 8}
                textAnchor="middle"
                fontSize={11}
                fontWeight={isToday ? 700 : 400}
                fill={isToday ? "#26332f" : "#8a948f"}
              >
                {isToday ? "Today" : weekdayShort(d.day)}
              </text>
              {/* Hit target: the whole column, larger than the bar. */}
              <rect
                x={PAD.left + band * i}
                y={PAD.top}
                width={band}
                height={plotH}
                fill="transparent"
                onMouseEnter={() => setHover(i)}
                onMouseLeave={() => setHover(null)}
                onFocus={() => setHover(i)}
                onBlur={() => setHover(null)}
                tabIndex={0}
                aria-label={`${longDay(d.day)}: ${formatNumber(d.sales_sats)} sats from ${d.count} payments`}
              />
            </g>
          );
        })}
      </svg>
      {active && hover !== null && (
        <div
          className="pointer-events-none absolute top-0 z-10 -translate-x-1/2 rounded-lg bg-forest px-3 py-2 text-xs text-ivory shadow-lg"
          style={{ left: `${((PAD.left + band * hover + band / 2) / W) * 100}%` }}
        >
          <p className="font-semibold">{longDay(active.day)}</p>
          <p className="tabular text-ivory/80">
            {formatNumber(active.sales_sats)} sats · {active.count} {active.count === 1 ? "payment" : "payments"}
          </p>
        </div>
      )}
      <table className="sr-only">
        <caption>Paid sales per day</caption>
        <thead>
          <tr>
            <th>Day</th>
            <th>Sales (sats)</th>
            <th>Payments</th>
          </tr>
        </thead>
        <tbody>
          {days.map((d) => (
            <tr key={d.day}>
              <td>{longDay(d.day)}</td>
              <td>{d.sales_sats}</td>
              <td>{d.count}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
