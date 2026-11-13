"use client";

import Link from "next/link";
import { useEffect, useState, type ReactNode } from "react";
import { ArrowRight, Banknote, CheckCircle2, Clock3, TrendingUp, Wallet } from "lucide-react";
import { api } from "@/lib/api";
import { formatNumber } from "@/lib/format";
import { formatFiat } from "@/lib/money";
import { Card, CardHeader, Loading } from "@/components/ui";
import { SalesChart } from "@/components/SalesChart";
import type { DashboardSummary } from "@/types/api";

export function DashboardStats() {
  const [s, setS] = useState<DashboardSummary | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const load = () =>
      api
        .dashboard()
        .then((d) => {
          if (!cancelled) {
            setS(d);
            setError(false);
          }
        })
        .catch(() => !cancelled && setError(true));
    load();
    const id = setInterval(load, 10_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  if (!s) {
    return error ? <p className="text-sm text-red-700">Couldn&apos;t load your sales summary.</p> : <Loading />;
  }

  const week = s.last_7_days.reduce((sum, d) => sum + d.sales_sats, 0);

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-3 gap-3 sm:gap-4 lg:grid-cols-4">
        <div className="col-span-3 rounded-2xl bg-forest p-5 text-ivory shadow-[var(--shadow-raised)] lg:col-span-1">
          <div className="flex items-center gap-2 text-sm text-ivory/75">
            <Wallet aria-hidden className="h-4 w-4" /> Today&apos;s sales
          </div>
          {s.today_sales_fiat ? (
            <>
              <p className="tabular mt-3 font-heading text-3xl font-extrabold text-lime">{formatFiat(s.today_sales_fiat)}</p>
              <p className="tabular mt-1 text-sm text-ivory/70">
                {formatNumber(s.today_sales_sats)} sats · {s.today_count} {s.today_count === 1 ? "payment" : "payments"}
              </p>
            </>
          ) : (
            <>
              <p className="tabular mt-3 font-heading text-3xl font-extrabold text-lime">
                {formatNumber(s.today_sales_sats)} <span className="text-lg font-bold text-lime/80">sats</span>
              </p>
              <p className="mt-1 text-sm text-ivory/70">
                {s.today_count} {s.today_count === 1 ? "payment" : "payments"} today
              </p>
            </>
          )}
        </div>
        <Stat
          icon={<TrendingUp aria-hidden className="h-4 w-4" />}
          label="Total sales"
          value={s.total_sales_fiat ? formatFiat(s.total_sales_fiat) : formatNumber(s.total_sales_sats)}
          unit={s.total_sales_fiat ? `${formatNumber(s.total_sales_sats)} sats` : "sats"}
        />
        <Stat
          icon={<CheckCircle2 aria-hidden className="h-4 w-4" />}
          label="Paid"
          value={String(s.paid_count)}
          unit={s.paid_count === 1 ? "payment" : "payments"}
          href="/history?status=paid"
        />
        <Stat
          icon={<Clock3 aria-hidden className="h-4 w-4" />}
          label="Waiting"
          value={String(s.pending_count)}
          unit="pending"
          href="/history?status=pending"
          highlight={s.pending_count > 0}
        />
      </div>

      <Link
        href="/cashout"
        className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-line bg-surface px-5 py-4 shadow-[var(--shadow-card)] transition-colors hover:border-forest/40"
      >
        <span className="flex items-center gap-3">
          <span className="flex h-10 w-10 items-center justify-center rounded-xl bg-lime-soft text-forest">
            <Banknote aria-hidden className="h-5 w-5" />
          </span>
          <span>
            <span className="block text-sm text-muted">Available to cash out</span>
            <span className="tabular block font-heading text-lg font-extrabold">
              {s.available_fiat ? formatFiat(s.available_fiat) : `${formatNumber(s.available_sats)} sats`}
              {s.available_fiat && (
                <span className="ml-2 text-sm font-medium text-muted">{formatNumber(s.available_sats)} sats</span>
              )}
            </span>
          </span>
        </span>
        <span className="inline-flex items-center gap-1 text-sm font-semibold text-forest">
          Cash out to {s.currency} <ArrowRight aria-hidden className="h-4 w-4" />
        </span>
      </Link>

      <Card>
        <CardHeader
          title="Sales, last 7 days"
          subtitle={`${formatNumber(week)} sats from paid payments`}
        />
        <div className="px-3 pt-4 pb-2 sm:px-5">
          <SalesChart days={s.last_7_days} />
        </div>
      </Card>
    </div>
  );
}

function Stat({
  icon,
  label,
  value,
  unit,
  href,
  highlight,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  unit: string;
  href?: string;
  highlight?: boolean;
}) {
  const body = (
    <>
      <div className="flex items-center gap-2 text-xs text-muted sm:text-sm">
        <span className={`hidden h-7 w-7 sm:flex items-center justify-center rounded-lg ${highlight ? "bg-amber-50 text-amber-700" : "bg-ivory text-forest"}`}>
          {icon}
        </span>
        {label}
      </div>
      <p className="tabular mt-2 font-heading text-xl font-extrabold text-charcoal sm:mt-3 sm:text-2xl">{value}</p>
      <p className="text-xs text-muted sm:text-sm">{unit}</p>
    </>
  );
  const cls = "block min-w-0 rounded-2xl border border-line bg-surface p-3.5 shadow-[var(--shadow-card)] sm:p-5";
  return href ? (
    <Link href={href} className={`${cls} transition-colors hover:border-forest/40`}>
      {body}
    </Link>
  ) : (
    <div className={cls}>{body}</div>
  );
}
