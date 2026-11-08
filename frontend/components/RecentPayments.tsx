"use client";

import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { PaymentList } from "@/components/PaymentRow";
import type { Payment } from "@/types/api";

export function RecentPayments({ limit = 5 }: { limit?: number }) {
  const [payments, setPayments] = useState<Payment[] | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const load = () =>
      api
        .payments({ limit })
        .then((r) => {
          if (!cancelled) {
            setPayments(r.payments);
            setError(false);
          }
        })
        .catch(() => !cancelled && setError(true));
    load();
    const id = setInterval(load, 5000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [limit]);

  if (error && !payments) return <p className="text-red-700">Couldn&apos;t load payments.</p>;
  if (!payments) return <p className="text-muted">Loading…</p>;
  if (payments.length === 0) {
    return (
      <p className="rounded-2xl border border-dashed border-line bg-white px-4 py-8 text-center text-muted">
        No payments yet. Tap <strong>Receive payment</strong> to create your first one.
      </p>
    );
  }
  return <PaymentList payments={payments} />;
}
