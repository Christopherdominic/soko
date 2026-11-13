"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useEffect, useState } from "react";
import { Check, ChevronLeft, ExternalLink, Share2 } from "lucide-react";
import { api, ApiError } from "@/lib/api";
import { ReceiptCard } from "@/components/ReceiptCard";
import { NostrProof } from "@/components/NostrProof";
import { Button, Card, Loading } from "@/components/ui";
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

  if (!receipt) return error ? <p className="py-16 text-center text-muted">{error}</p> : <Loading />;

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
    <>
      <Link href={`/payments/${hash}`} className="mb-4 inline-flex items-center gap-1 text-sm font-medium text-muted hover:text-charcoal">
        <ChevronLeft aria-hidden className="h-4 w-4" /> Payment
      </Link>
      <div className="grid gap-6 lg:grid-cols-[minmax(0,420px)_minmax(0,1fr)] lg:items-start">
        <ReceiptCard
          businessName={receipt.business_name}
          receiptNumber={receipt.receipt_number}
          amountSats={receipt.amount_sats}
          paidAt={receipt.paid_at}
          fiat={receipt.fiat}
          description={receipt.description}
        />
        <div className="space-y-4">
          <Card className="p-5">
            <h2 className="text-base font-bold">Share with your customer</h2>
            <p className="mt-1 text-sm text-muted">
              The customer&apos;s link shows the amount, date and receipt number. Your description stays private.
            </p>
            <div className="mt-4 grid gap-3 sm:grid-cols-2">
              <Button size="lg" full onClick={share}>
                {copied ? <Check aria-hidden className="h-5 w-5" /> : <Share2 aria-hidden className="h-5 w-5" />}
                {copied ? "Link copied" : "Share receipt"}
              </Button>
              <a
                href={receipt.public_path}
                target="_blank"
                rel="noopener"
                className="inline-flex h-14 items-center justify-center gap-2 rounded-xl border border-line-strong bg-surface px-6 font-semibold text-forest hover:bg-ivory"
              >
                <ExternalLink aria-hidden className="h-5 w-5" /> Customer view
              </a>
            </div>
          </Card>
          <NostrProof signed={receipt.nostr} />
        </div>
      </div>
    </>
  );
}
