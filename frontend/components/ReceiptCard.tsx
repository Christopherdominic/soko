import type { ReactNode } from "react";
import { CheckCircle2 } from "lucide-react";
import { formatDate, formatNumber, formatTime } from "@/lib/format";
import { formatFiat } from "@/lib/money";
import type { FiatValue } from "@/types/api";
import { LogoMark } from "@/components/Logo";

/** Paper-style receipt. Shows only what belongs on a receipt. */
export function ReceiptCard({
  businessName,
  receiptNumber,
  amountSats,
  paidAt,
  description,
  fiat,
}: {
  businessName: string;
  receiptNumber: string;
  amountSats: number;
  paidAt: string | null;
  description?: string | null;
  fiat?: FiatValue | null;
}) {
  return (
    <article className="relative overflow-hidden rounded-2xl border border-line bg-surface shadow-[var(--shadow-card)]">
      <div className="h-1.5 bg-forest" aria-hidden />
      <div className="px-6 pt-6 pb-5 text-center sm:px-8">
        <LogoMark className="mx-auto h-10 w-10" />
        <p className="mt-3 font-heading text-lg font-extrabold text-forest">{businessName}</p>
        <p className="text-xs font-semibold tracking-[0.18em] text-muted uppercase">Payment receipt</p>
      </div>

      <div className="mx-6 border-t border-dashed border-line-strong sm:mx-8" aria-hidden />

      <div className="px-6 py-6 text-center sm:px-8">
        <p className="text-sm text-muted">Amount paid</p>
        <p className="tabular mt-1 font-heading text-4xl font-extrabold text-charcoal">
          {formatNumber(amountSats)} <span className="text-xl font-bold text-muted">sats</span>
        </p>
        {fiat && <p className="tabular mt-1 text-lg font-semibold text-charcoal">≈ {formatFiat(fiat)}</p>}
        <p className="mt-3 inline-flex items-center gap-1.5 rounded-full bg-emerald-50 px-3 py-1 text-xs font-semibold text-emerald-800">
          <CheckCircle2 aria-hidden className="h-3.5 w-3.5" /> Paid with Bitcoin · Lightning
        </p>
      </div>

      <div className="mx-6 border-t border-dashed border-line-strong sm:mx-8" aria-hidden />

      <dl className="space-y-3 px-6 py-5 text-sm sm:px-8">
        <Row label="Receipt no." value={<span className="font-mono font-semibold">{receiptNumber}</span>} />
        {paidAt && <Row label="Date" value={formatDate(paidAt)} />}
        {paidAt && <Row label="Time" value={formatTime(paidAt)} />}
        {description && <Row label="For" value={description} />}
        {fiat && (
          <Row
            label="Rate"
            value={`1 BTC = ${new Intl.NumberFormat("en", { maximumFractionDigits: 0 }).format(fiat.btc_price)} ${fiat.currency}`}
          />
        )}
      </dl>
    </article>
  );
}

function Row({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="flex justify-between gap-4">
      <dt className="text-muted">{label}</dt>
      <dd className="text-right font-medium text-charcoal">{value}</dd>
    </div>
  );
}
