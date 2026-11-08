"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useEffect, useState } from "react";
import { api, ApiError } from "@/lib/api";
import { ReceiptCard } from "@/components/ReceiptCard";
import { NostrProof } from "@/components/NostrProof";
import { PublicHeader } from "@/components/PublicHeader";
import type { PublicReceipt } from "@/types/api";

export function PublicReceiptView() {
  const { number } = useParams<{ number: string }>();
  const [receipt, setReceipt] = useState<PublicReceipt | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .publicReceipt(number)
      .then(setReceipt)
      .catch((e) => setError(e instanceof ApiError && e.status === 404 ? "Receipt not found." : "Couldn't load receipt."));
  }, [number]);

  return (
    <>
      <PublicHeader />
      <main className="mx-auto w-full max-w-md flex-1 space-y-4 px-4 py-6">
        {!receipt ? (
          <p className="p-6 text-center text-muted">{error ?? "Loading…"}</p>
        ) : (
          <>
            <ReceiptCard
              businessName={receipt.business_name}
              receiptNumber={receipt.receipt_number}
              amountSats={receipt.amount_sats}
              paidAt={receipt.paid_at}
            />
            <NostrProof signed={receipt.nostr} />
          </>
        )}
        <p className="text-center text-sm text-muted">
          Have a receipt file? <Link href="/verify" className="font-semibold text-forest underline">Verify it</Link>
        </p>
      </main>
    </>
  );
}
