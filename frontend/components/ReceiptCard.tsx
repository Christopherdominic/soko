import { formatDateTime, formatSats } from "@/lib/format";

export function ReceiptCard({
  businessName,
  receiptNumber,
  amountSats,
  paidAt,
  description,
}: {
  businessName: string;
  receiptNumber: string;
  amountSats: number;
  paidAt: string | null;
  description?: string | null;
}) {
  return (
    <article className="overflow-hidden rounded-3xl border border-line bg-white">
      <div className="bg-forest px-6 py-5 text-ivory">
        <p className="text-sm text-lime">Payment receipt</p>
        <p className="mt-1 text-xl font-extrabold">{businessName}</p>
      </div>
      <div className="px-6 py-5">
        <p className="text-sm text-muted">Amount paid</p>
        <p className="text-3xl font-extrabold">{formatSats(amountSats)}</p>
        <p className="mt-1 text-sm text-muted">Paid with Bitcoin (Lightning Network)</p>
      </div>
      <dl className="divide-y divide-line border-t border-line">
        <Row label="Receipt no." value={receiptNumber} mono />
        {paidAt && <Row label="Date" value={formatDateTime(paidAt)} />}
        {description && <Row label="For" value={description} />}
      </dl>
    </article>
  );
}

function Row({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex justify-between gap-4 px-6 py-3">
      <dt className="text-muted">{label}</dt>
      <dd className={`text-right font-semibold ${mono ? "font-mono" : ""}`}>{value}</dd>
    </div>
  );
}
