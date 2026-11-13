"use client";

import { useRouter } from "next/navigation";
import { useState, type FormEvent } from "react";
import { QrCode, ScanLine, ShieldCheck, ReceiptText } from "lucide-react";
import { AppShell } from "@/components/AppShell";
import { Button, Card, ErrorMessage, Field, PageHeader } from "@/components/ui";
import { api, ApiError } from "@/lib/api";
import { formatNumber } from "@/lib/format";

const MAX_SATS = 10_000_000;
const QUICK = [500, 1000, 2000, 5000, 10000];

export default function ReceivePage() {
  return (
    <AppShell>
      <ReceiveForm />
    </AppShell>
  );
}

function ReceiveForm() {
  const router = useRouter();
  const [amount, setAmount] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const sats = Number(amount || 0);

  const onSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!Number.isInteger(sats) || sats < 1 || sats > MAX_SATS) {
      setError(`Enter a whole number of sats between 1 and ${formatNumber(MAX_SATS)}.`);
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
    <>
      <PageHeader title="Receive payment" subtitle="Enter the price and show the QR code to your customer." />
      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_320px]">
        <Card className="p-5 sm:p-8">
          <form onSubmit={onSubmit} className="space-y-6">
            <div>
              <label htmlFor="amount" className="block text-sm font-semibold">
                Amount
              </label>
              <div className="mt-2 flex items-baseline gap-3 border-b-2 border-line-strong pb-2 focus-within:border-forest">
                <input
                  id="amount"
                  name="amount"
                  inputMode="numeric"
                  autoFocus
                  required
                  autoComplete="off"
                  placeholder="0"
                  value={amount ? formatNumber(sats) : ""}
                  onChange={(e) => setAmount(e.target.value.replace(/[^0-9]/g, "").slice(0, 8))}
                  className="tabular w-full min-w-0 bg-transparent font-heading text-5xl font-extrabold text-forest outline-none placeholder:text-line-strong sm:text-6xl"
                />
                <span className="text-xl font-semibold text-muted">sats</span>
              </div>
              <div className="mt-4 flex flex-wrap gap-2" role="group" aria-label="Quick amounts">
                {QUICK.map((q) => (
                  <button
                    key={q}
                    type="button"
                    onClick={() => setAmount(String(q))}
                    aria-pressed={sats === q}
                    className={`h-9 rounded-full px-4 text-sm font-semibold transition-colors ${
                      sats === q ? "bg-forest text-ivory" : "border border-line-strong bg-surface text-charcoal hover:border-forest/40"
                    }`}
                  >
                    {formatNumber(q)}
                  </button>
                ))}
              </div>
            </div>

            <Field
              label="What is it for? (optional)"
              name="description"
              maxLength={200}
              placeholder="e.g. 2kg rice"
              hint="Shown to the customer in their wallet. Not shown on the public receipt."
            />

            <ErrorMessage>{error}</ErrorMessage>

            <Button type="submit" size="lg" full loading={busy} disabled={!amount}>
              {!busy && <QrCode aria-hidden className="h-5 w-5" />}
              {busy ? "Creating payment request…" : amount ? `Show QR code for ${formatNumber(sats)} sats` : "Show QR code"}
            </Button>
          </form>
        </Card>

        <Card className="hidden h-fit p-6 lg:block">
          <h2 className="text-base font-bold">How it works</h2>
          <ol className="mt-4 space-y-5">
            <Step icon={<QrCode aria-hidden className="h-4 w-4" />} title="Show the QR code" text="Soko asks your own Lightning node for a payment request." />
            <Step icon={<ScanLine aria-hidden className="h-4 w-4" />} title="Customer scans and pays" text="Any Lightning wallet works. Payment arrives in seconds." />
            <Step icon={<ShieldCheck aria-hidden className="h-4 w-4" />} title="Verified by your node" text="Marked paid only when your node confirms it." />
            <Step icon={<ReceiptText aria-hidden className="h-4 w-4" />} title="Receipt issued" text="A signed digital receipt you can share with the customer." />
          </ol>
        </Card>
      </div>
    </>
  );
}

function Step({ icon, title, text }: { icon: React.ReactNode; title: string; text: string }) {
  return (
    <li className="flex gap-3">
      <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-lime-soft text-forest">{icon}</span>
      <span>
        <span className="block text-sm font-semibold">{title}</span>
        <span className="block text-sm text-muted">{text}</span>
      </span>
    </li>
  );
}
