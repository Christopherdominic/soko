"use client";

import { useState, type FormEvent } from "react";
import { api, ApiError } from "@/lib/api";
import { PublicHeader } from "@/components/PublicHeader";
import { VerifyBadge } from "@/components/NostrProof";
import { ErrorMessage, PrimaryButton } from "@/components/ui";
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
      <main className="mx-auto w-full max-w-md flex-1 px-4 py-6">
        <h1 className="text-2xl font-extrabold">Verify a receipt</h1>
        <p className="mt-1 mb-5 text-muted">Check that a Soko receipt is genuine and hasn&apos;t been changed.</p>
        <form onSubmit={onSubmit} className="space-y-4">
          <label className="block">
            <span className="mb-1.5 block font-semibold">Receipt file</span>
            <input
              type="file"
              accept="application/json,.json"
              onChange={(e) => loadFile(e.target.files?.[0])}
              className="block w-full text-sm"
            />
          </label>
          <label className="block">
            <span className="mb-1.5 block font-semibold">…or paste the signed receipt</span>
            <textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              rows={8}
              placeholder='{"id": "...", "pubkey": "...", "sig": "...", ...}'
              className="w-full rounded-xl border-2 border-line bg-white p-3 font-mono text-xs outline-none focus:border-forest"
            />
          </label>
          <ErrorMessage>{error}</ErrorMessage>
          <PrimaryButton type="submit" disabled={busy || !text.trim()}>
            {busy ? "Checking…" : "Verify"}
          </PrimaryButton>
        </form>
        {result && (
          <section className="mt-6 space-y-2 rounded-2xl border border-line bg-white px-4 py-4">
            <VerifyBadge result={result} />
            {result.receipt_number && (
              <p className="text-sm text-muted">
                Receipt {result.receipt_number}
                {result.amount_sats !== null && <> · {formatSats(result.amount_sats)}</>}
              </p>
            )}
          </section>
        )}
      </main>
    </>
  );
}
