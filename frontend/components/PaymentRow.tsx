import Link from "next/link";
import { ChevronRight } from "lucide-react";
import { formatDateTime, formatNumber } from "@/lib/format";
import { StatusBadge } from "@/components/StatusBadge";
import type { Payment } from "@/types/api";

/** Transactions list: stacked rows on phones, a table layout from md up. */
export function PaymentList({ payments }: { payments: Payment[] }) {
  return (
    <div role="table" aria-label="Transactions" className="text-sm">
      <div
        role="row"
        className="hidden grid-cols-[1fr_1.6fr_1.2fr_1.2fr_auto] gap-4 border-b border-line px-5 py-2.5 text-xs font-semibold tracking-wide text-muted uppercase md:grid"
      >
        <span role="columnheader">Amount</span>
        <span role="columnheader">Description</span>
        <span role="columnheader">Receipt</span>
        <span role="columnheader">Date</span>
        <span role="columnheader" className="w-24">Status</span>
      </div>
      <ul className="divide-y divide-line">
        {payments.map((p) => (
          <li key={p.payment_hash} role="row">
            <Link
              href={`/payments/${p.payment_hash}`}
              className="group flex items-center gap-3 px-5 py-3.5 transition-colors hover:bg-ivory/70 md:grid md:grid-cols-[1fr_1.6fr_1.2fr_1.2fr_auto] md:gap-4"
            >
              <span role="cell" className="min-w-0 flex-1 md:flex-none">
                <span className="tabular block font-semibold text-charcoal">
                  {formatNumber(p.amount_sats)} <span className="font-normal text-muted">sats</span>
                  {p.is_demo && <span className="ml-2 rounded bg-stone-200 px-1.5 py-0.5 text-[10px] font-bold tracking-wide">DEMO</span>}
                </span>
                <span className="mt-0.5 block truncate text-muted md:hidden">
                  {p.description ?? "No description"} · {formatDateTime(p.created_at)}
                </span>
              </span>
              <span role="cell" className="hidden truncate text-charcoal md:block">
                {p.description ?? <span className="text-subtle">—</span>}
              </span>
              <span role="cell" className="hidden font-mono text-xs text-muted md:block">
                {p.receipt_number ?? <span className="text-subtle">—</span>}
              </span>
              <span role="cell" className="tabular hidden text-muted md:block">
                {formatDateTime(p.created_at)}
              </span>
              <span role="cell" className="flex w-24 items-center justify-end gap-1 md:justify-start">
                <StatusBadge status={p.status} />
                <ChevronRight aria-hidden className="h-4 w-4 text-subtle md:hidden" />
              </span>
            </Link>
          </li>
        ))}
      </ul>
    </div>
  );
}
