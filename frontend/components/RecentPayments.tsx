"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { formatDateTime, formatSats } from "@/lib/format";
import { StatusBadge } from "@/components/StatusBadge";
import type { Payment } from "@/types/api";

export function RecentPayments() {
  const [payments, setPayments] = useState<Payment[] | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const load = () =>
      api
        .payments({ limit: 10 })
        .then((r) => !cancelled && (setPayments(r.payments), setError(false)))
        .catch(() => !cancelled && setError(true));
    load();
    const id = setInterval(load, 5000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  if (error && !payments) return <p className="text-red-700">Couldn&apos;t load payments.</p>;
  if (!payments) return <p className="text-muted">Loading…</p>;
  if (payments.length === 0) {
    return (
      <p className="rounded-2xl border border-dashed border-line bg-white px-4 py-8 text-center text-muted">
        No payments yet. Tap <strong>Receive payment</strong> to create your first one.
      </p>
    );
  }

  return (
    <ul className="divide-y divide-line overflow-hidden rounded-2xl border border-line bg-white">
      {payments.map((p) => (
        <li key={p.payment_hash}>
          <Link href={`/payments/${p.payment_hash}`} className="flex items-center justify-between gap-3 px-4 py-3 hover:bg-ivory">
            <div className="min-w-0">
              <p className="font-semibold">{formatSats(p.amount_sats)}</p>
              <p className="truncate text-sm text-muted">
                {p.description ?? "No description"} · {formatDateTime(p.created_at)}
              </p>
            </div>
            <StatusBadge status={p.status} />
          </Link>
        </li>
      ))}
    </ul>
  );
}
