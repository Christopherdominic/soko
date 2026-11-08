import Link from "next/link";
import { formatDateTime, formatSats } from "@/lib/format";
import { StatusBadge } from "@/components/StatusBadge";
import type { Payment } from "@/types/api";

export function PaymentList({ payments }: { payments: Payment[] }) {
  return (
    <ul className="divide-y divide-line overflow-hidden rounded-2xl border border-line bg-white">
      {payments.map((p) => (
        <li key={p.payment_hash}>
          <PaymentRow payment={p} />
        </li>
      ))}
    </ul>
  );
}

function PaymentRow({ payment: p }: { payment: Payment }) {
  return (
    <Link href={`/payments/${p.payment_hash}`} className="flex items-center justify-between gap-3 px-4 py-3 hover:bg-ivory">
      <div className="min-w-0">
        <p className="font-semibold">
          {formatSats(p.amount_sats)}
          {p.is_demo && <span className="ml-2 rounded bg-stone-200 px-1.5 py-0.5 text-xs font-medium">DEMO</span>}
        </p>
        <p className="truncate text-sm text-muted">
          {p.description ?? "No description"} · {formatDateTime(p.created_at)}
        </p>
        {p.receipt_number && <p className="text-xs text-muted">Receipt {p.receipt_number}</p>}
      </div>
      <StatusBadge status={p.status} />
    </Link>
  );
}
