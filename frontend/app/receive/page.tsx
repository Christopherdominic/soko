"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState, type FormEvent } from "react";
import { RequireMerchant } from "@/hooks/useMerchant";
import { AppHeader } from "@/components/AppHeader";
import { ErrorMessage, Field, PrimaryButton } from "@/components/ui";
import { api, ApiError } from "@/lib/api";

const MAX_SATS = 10_000_000;

export default function ReceivePage() {
  return (
    <RequireMerchant>
      <AppHeader />
      <ReceiveForm />
    </RequireMerchant>
  );
}

function ReceiveForm() {
  const router = useRouter();
  const [amount, setAmount] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const onSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const sats = Number(amount);
    if (!Number.isInteger(sats) || sats < 1 || sats > MAX_SATS) {
      setError(`Enter a whole number of sats between 1 and ${MAX_SATS.toLocaleString("en")}.`);
      return;
    }
    const description = String(new FormData(e.currentTarget).get("description") ?? "").trim();
    setBusy(true);
    setError(null);
    try {
      const payment = await api.createInvoice(sats, description || undefined);
      router.push(`/payments/${payment.payment_hash}`);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Something went wrong");
      setBusy(false);
    }
  };

  return (
    <main className="mx-auto w-full max-w-md flex-1 px-4 py-6">
      <Link href="/" className="text-sm text-muted underline-offset-4 hover:underline">
        ← Back
      </Link>
      <h1 className="mt-3 mb-6 text-2xl font-extrabold">Receive payment</h1>
      <form onSubmit={onSubmit} className="space-y-5">
        <Field
          label="Amount (sats)"
          name="amount"
          inputMode="numeric"
          pattern="[0-9]*"
          autoFocus
          required
          placeholder="e.g. 5000"
          value={amount}
          onChange={(e) => setAmount(e.target.value.replace(/[^0-9]/g, ""))}
        />
        <Field label="What is it for? (optional)" name="description" maxLength={200} placeholder="e.g. 2kg rice" />
        <p className="text-sm text-muted">The description is shown to the customer in their wallet.</p>
        <ErrorMessage>{error}</ErrorMessage>
        <PrimaryButton type="submit" disabled={busy || !amount}>
          {busy ? "Creating…" : "Show QR code"}
        </PrimaryButton>
      </form>
    </main>
  );
}
