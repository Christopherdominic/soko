"use client";

import { useEffect, useState } from "react";
import { QrCode, ReceiptText } from "lucide-react";
import { api } from "@/lib/api";
import { PaymentList } from "@/components/PaymentRow";
import { ButtonLink, EmptyState, Loading } from "@/components/ui";
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

  if (error && !payments) return <p className="px-5 py-8 text-center text-sm text-red-700">Couldn&apos;t load payments.</p>;
  if (!payments) return <Loading />;
  if (payments.length === 0) {
    return (
      <EmptyState
        icon={<ReceiptText aria-hidden className="h-6 w-6" />}
        title="No payments yet"
        text="Create a payment request and your sales will appear here."
        action={
          <ButtonLink href="/receive">
            <QrCode aria-hidden className="h-4 w-4" /> Receive payment
          </ButtonLink>
        }
      />
    );
  }
  return <PaymentList payments={payments} />;
}
