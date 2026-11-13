"use client";

import { useEffect, useState } from "react";
import { ChevronDown, Download, Loader2, ShieldAlert, ShieldCheck } from "lucide-react";
import { api } from "@/lib/api";
import type { SignedReceipt, VerifyResult } from "@/types/api";

/** Shows whether a receipt's Nostr signature checks out, plus the proof details. */
export function NostrProof({ signed }: { signed: SignedReceipt | null }) {
  const [result, setResult] = useState<VerifyResult | null>(null);

  useEffect(() => {
    if (signed) api.verifyReceipt(signed.event).then(setResult).catch(() => setResult(null));
  }, [signed]);

  if (!signed) {
    return <p className="rounded-2xl bg-stone-100 px-4 py-3 text-sm text-muted">This receipt is not digitally signed.</p>;
  }

  const download = () => {
    const blob = new Blob([JSON.stringify(signed.event, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `soko-receipt-${JSON.parse(signed.event.content).receipt_number}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <section className="rounded-2xl border border-line bg-surface shadow-[var(--shadow-card)]">
      <div className="p-5">
        <VerifyBadge result={result} />
      </div>
      <details className="group border-t border-line">
        <summary className="flex cursor-pointer list-none items-center justify-between px-5 py-3.5 text-sm font-semibold text-forest">
          How to verify this receipt
          <ChevronDown aria-hidden className="h-4 w-4 transition-transform group-open:rotate-180" />
        </summary>
        <div className="space-y-4 px-5 pb-5 text-sm text-muted">
          <p>
            This receipt is a signed <strong className="text-charcoal">Nostr</strong> event. Anyone can check the signature with
            the merchant&apos;s public key. It contains no payment secrets or customer details.
          </p>
          <Detail label="Merchant public key" value={signed.npub ?? signed.event.pubkey} />
          <Detail label="Event ID" value={signed.event.id} />
          <Detail label="Verification code" value={signed.verification_code} />
          <p>
            Paid this yourself? SHA-256 of <code className="text-charcoal">soko/receipt-commitment/v1/</code> + verification
            code + your payment hash equals the <code className="text-charcoal">commitment</code> in the event, proving this
            receipt is for your payment.
          </p>
          <button onClick={download} className="inline-flex items-center gap-1.5 font-semibold text-forest hover:underline">
            <Download aria-hidden className="h-4 w-4" /> Download signed receipt (.json)
          </button>
        </div>
      </details>
    </section>
  );
}

export function VerifyBadge({ result }: { result: VerifyResult | null }) {
  if (!result) {
    return (
      <p className="flex items-center gap-2 text-sm text-muted">
        <Loader2 aria-hidden className="h-4 w-4 animate-spin" /> Checking signature…
      </p>
    );
  }
  const genuine = result.signature_valid && result.issued_by_known_merchant && result.matches_records;
  return genuine ? (
    <div className="flex items-start gap-3">
      <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-emerald-50 text-emerald-700">
        <ShieldCheck aria-hidden className="h-5 w-5" />
      </span>
      <div>
        <p className="font-semibold text-emerald-900">Genuine receipt</p>
        <p className="text-sm text-muted">
          Digitally signed by {result.business_name} and matches their records on Soko.
        </p>
      </div>
    </div>
  ) : (
    <div className="flex items-start gap-3">
      <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-red-50 text-red-700">
        <ShieldAlert aria-hidden className="h-5 w-5" />
      </span>
      <div>
        <p className="font-semibold text-red-900">Could not verify this receipt</p>
        <p className="text-sm text-red-800/80">{result.problem}</p>
      </div>
    </div>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs font-semibold tracking-wide text-muted uppercase">{label}</p>
      <p className="mt-0.5 break-all rounded-lg bg-ivory px-2.5 py-1.5 font-mono text-xs text-charcoal">{value}</p>
    </div>
  );
}
