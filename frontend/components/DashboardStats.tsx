"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { formatSats } from "@/lib/format";
import type { DashboardSummary } from "@/types/api";

export function DashboardStats() {
  const [s, setS] = useState<DashboardSummary | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const load = () =>
      api
        .dashboard()
        .then((d) => !cancelled && (setS(d), setError(false)))
        .catch(() => !cancelled && setError(true));
    load();
    const id = setInterval(load, 10_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  if (!s) {
    return <p className="text-muted">{error ? "Couldn't load your sales summary." : "Loading…"}</p>;
  }

  return (
    <div className="space-y-3">
      <div className="rounded-2xl bg-forest px-5 py-5 text-ivory">
        <p className="text-sm text-ivory/80">Today&apos;s sales</p>
        <p className="mt-1 text-3xl font-extrabold text-lime">{formatSats(s.today_sales_sats)}</p>
        <p className="text-sm text-ivory/80">
          {s.today_count} {s.today_count === 1 ? "payment" : "payments"}
        </p>
      </div>
      <div className="grid grid-cols-3 gap-3">
        <Stat label="Total sales" value={formatSats(s.total_sales_sats)} />
        <Stat label="Paid" value={String(s.paid_count)} href="/history?status=paid" />
        <Stat
          label="Waiting"
          value={String(s.pending_count)}
          href="/history?status=pending"
          highlight={s.pending_count > 0}
        />
      </div>
    </div>
  );
}

function Stat({ label, value, href, highlight }: { label: string; value: string; href?: string; highlight?: boolean }) {
  const body = (
    <>
      <p className="text-xs font-medium text-muted">{label}</p>
      <p className={`mt-1 text-lg font-bold leading-tight ${highlight ? "text-amber-800" : ""}`}>{value}</p>
    </>
  );
  const cls = "block rounded-2xl border border-line bg-white px-3 py-3";
  return href ? (
    <Link href={href} className={`${cls} hover:border-forest`}>
      {body}
    </Link>
  ) : (
    <div className={cls}>{body}</div>
  );
}
