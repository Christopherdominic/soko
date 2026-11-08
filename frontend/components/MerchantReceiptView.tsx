"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useEffect, useState } from "react";
import { api, ApiError } from "@/lib/api";
import { ReceiptCard } from "@/components/ReceiptCard";
import { NostrProof } from "@/components/NostrProof";
import { PrimaryButton } from "@/components/ui";
import type { MerchantReceipt } from "@/types/api";

export function MerchantReceiptView() {
  const { hash } = useParams<{ hash: string }>();
  const [receipt, setReceipt] = useState<MerchantReceipt | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    api
      .receipt(hash)
      .then(setReceipt)
      .catch((e) => setError(e instanceof ApiError && e.status === 404 ? "No receipt for this payment yet." : "Couldn't load receipt."));
  }, [hash]);

  if (!receipt) return <p className="p-6 text-center text-muted">{error ?? "Loading…"}</p>;

  const share = async () => {
    const url = `${window.location.origin}${receipt.public_path}`;
    if (navigator.share) {
      await navigator.share({ title: `Receipt ${receipt.receipt_number}`, url }).catch(() => {});
    } else {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <main className="mx-auto w-full max-w-md flex-1 space-y-4 px-4 py-6">
      <Link href={`/payments/${hash}`} className="text-sm text-muted underline-offset-4 hover:underline">
        ← Payment
      </Link>
      <ReceiptCard
        businessName={receipt.business_name}
        receiptNumber={receipt.receipt_number}
        amountSats={receipt.amount_sats}
        paidAt={receipt.paid_at}
        description={receipt.description}
      />
      <PrimaryButton onClick={share}>{copied ? "Link copied ✓" : "Share receipt with customer"}</PrimaryButton>
      <p className="text-center text-sm text-muted">
        The customer&apos;s link shows the amount, date and receipt number only, not the description.
      </p>
      <NostrProof signed={receipt.nostr} />
    </main>
  );
}
