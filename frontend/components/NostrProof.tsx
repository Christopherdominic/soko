"use client";

import { useEffect, useState } from "react";
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
    <section className="rounded-2xl border border-line bg-white px-4 py-4">
      <VerifyBadge result={result} />
      <details className="mt-3 text-sm">
        <summary className="cursor-pointer font-semibold text-forest">How to verify this receipt</summary>
        <div className="mt-3 space-y-3 text-muted">
          <p>
            This receipt is a signed <strong>Nostr</strong> event. Anyone can check the signature with the merchant&apos;s
            public key. It contains no payment secrets or customer details.
          </p>
          <Detail label="Merchant public key" value={signed.npub ?? signed.event.pubkey} />
          <Detail label="Event ID" value={signed.event.id} />
          <Detail label="Verification code" value={signed.verification_code} />
          <p>
            Paid this yourself? SHA-256 of <code>soko/receipt-commitment/v1/</code> + verification code + your payment
            hash equals the <code>commitment</code> in the event, which proves this receipt is for your payment.
          </p>
          <button onClick={download} className="font-semibold text-forest underline">
            Download signed receipt (.json)
          </button>
        </div>
      </details>
    </section>
  );
}

export function VerifyBadge({ result }: { result: VerifyResult | null }) {
  if (!result) return <p className="text-sm text-muted">Checking signature…</p>;
  const genuine = result.signature_valid && result.issued_by_known_merchant && result.matches_records;
  return genuine ? (
    <p className="flex items-start gap-2 font-semibold text-green-800">
      <span aria-hidden className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-green-600 text-xs text-white">✓</span>
      <span>
        Genuine receipt, digitally signed by {result.business_name}
        <span className="block text-sm font-normal text-muted">Matches this merchant&apos;s records on Soko</span>
      </span>
    </p>
  ) : (
    <p className="flex items-start gap-2 font-semibold text-red-800">
      <span aria-hidden className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-red-600 text-xs text-white">!</span>
      <span>
        Could not verify this receipt
        <span className="block text-sm font-normal">{result.problem}</span>
      </span>
    </p>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs font-semibold uppercase tracking-wide">{label}</p>
      <p className="break-all font-mono text-xs text-charcoal">{value}</p>
    </div>
  );
}
