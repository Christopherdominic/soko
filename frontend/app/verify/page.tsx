"use client";

import { useState, type FormEvent } from "react";
import { api, ApiError } from "@/lib/api";
import { PublicFooter, PublicHeader } from "@/components/PublicHeader";
import { VerifyBadge } from "@/components/NostrProof";
import { Button, Card, ErrorMessage } from "@/components/ui";
import { ShieldCheck, Upload } from "lucide-react";
import { formatSats } from "@/lib/format";
import type { NostrEvent, VerifyResult } from "@/types/api";

export default function VerifyPage() {
  const [text, setText] = useState("");
  const [result, setResult] = useState<VerifyResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setResult(null);
    setError(null);
    let event: NostrEvent;
    try {
      event = JSON.parse(text);
    } catch {
      setError("That doesn't look like a receipt file. Paste the full JSON.");
      return;
    }
    setBusy(true);
    try {
      setResult(await api.verifyReceipt(event));
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Couldn't verify");
    } finally {
      setBusy(false);
    }
  };

  const loadFile = async (file: File | undefined) => {
    if (file && file.size < 16_000) setText(await file.text());
  };

  return (
    <>
      <PublicHeader />
      <main className="mx-auto w-full max-w-xl flex-1 px-4 py-8">
        <div className="mb-6 flex items-start gap-3">
          <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-lime-soft text-forest">
            <ShieldCheck aria-hidden className="h-5 w-5" />
          </span>
          <div>
            <h1 className="text-2xl font-extrabold text-forest">Verify a receipt</h1>
            <p className="mt-1 text-muted">Check that a Soko receipt is genuine and hasn&apos;t been changed.</p>
          </div>
        </div>
        <Card className="p-5 sm:p-6">
          <form onSubmit={onSubmit} className="space-y-5">
            <label className="flex cursor-pointer flex-col items-center gap-2 rounded-xl border-2 border-dashed border-line-strong bg-ivory/50 px-4 py-6 text-center hover:border-forest/40">
              <Upload aria-hidden className="h-6 w-6 text-forest" />
              <span className="text-sm font-semibold">Upload a receipt file (.json)</span>
              <span className="text-xs text-muted">The file you downloaded from a Soko receipt</span>
              <input type="file" accept="application/json,.json" onChange={(e) => loadFile(e.target.files?.[0])} className="sr-only" />
            </label>
            <label className="block">
              <span className="mb-1.5 block text-sm font-semibold">Or paste the signed receipt</span>
              <textarea
                value={text}
                onChange={(e) => setText(e.target.value)}
                rows={7}
                placeholder='{"id": "...", "pubkey": "...", "sig": "...", ...}'
                className="w-full rounded-xl border border-line-strong bg-surface p-3 font-mono text-xs outline-none focus:border-forest focus:ring-4 focus:ring-forest/10"
              />
            </label>
            <ErrorMessage>{error}</ErrorMessage>
            <Button type="submit" size="lg" full loading={busy} disabled={!text.trim()}>
              {busy ? "Checking…" : "Verify receipt"}
            </Button>
          </form>
        </Card>
        {result && (
          <Card className="mt-4 space-y-3 p-5">
            <VerifyBadge result={result} />
            {result.receipt_number && (
              <p className="tabular border-t border-line pt-3 text-sm text-muted">
                Receipt <span className="font-mono text-charcoal">{result.receipt_number}</span>
                {result.amount_sats !== null && <> · {formatSats(result.amount_sats)}</>}
              </p>
            )}
          </Card>
        )}
      </main>
      <PublicFooter />
    </>
  );
}
