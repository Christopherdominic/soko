"use client";

import { useRouter } from "next/navigation";
import { useState, type FormEvent, type ReactNode } from "react";
import { ArrowLeftRight, QrCode, ReceiptText, ScanLine, ShieldCheck } from "lucide-react";
import { AppShell } from "@/components/AppShell";
import { Button, Card, ErrorMessage, Field, PageHeader } from "@/components/ui";
import { useMerchant } from "@/hooks/useMerchant";
import { currencyDecimals, useRate } from "@/hooks/useRate";
import { api, ApiError } from "@/lib/api";
import { formatNumber } from "@/lib/format";
import { fiatForSats, formatMoney, quickAmounts, satsForFiat } from "@/lib/money";

const MAX_SATS = 10_000_000;
const QUICK_SATS = [500, 1000, 2000, 5000, 10000];

export default function ReceivePage() {
  return (
    <AppShell>
      <ReceiveForm />
    </AppShell>
  );
}

function ReceiveForm() {
  const router = useRouter();
  const merchant = useMerchant();
  const currency = merchant.currency;
  const decimals = currencyDecimals(currency);
  const { rate, loading: rateLoading } = useRate(currency);
  const canUseFiat = !!rate?.fresh;

  const [mode, setMode] = useState<"fiat" | "sats">("fiat");
  const [amount, setAmount] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Fall back to sats when there's no usable rate.
  const effectiveMode = mode === "fiat" && !canUseFiat ? "sats" : mode;
  const switchMode = (m: "fiat" | "sats") => {
    setMode(m);
    setAmount("");
  };

  const value = Number(amount || 0);
  const sats = effectiveMode === "sats" ? value : rate ? satsForFiat(value, rate.btc_price) : 0;
  const fiatEquivalent = effectiveMode === "sats" && rate ? fiatForSats(value, rate.btc_price) : null;

  const onAmountChange = (raw: string) => {
    if (effectiveMode === "sats") {
      setAmount(raw.replace(/[^0-9]/g, "").slice(0, 8));
      return;
    }
    // Digits plus up to `decimals` decimal places.
    let v = raw.replace(/[^0-9.]/g, "");
    const [whole, ...rest] = v.split(".");
    v = decimals > 0 && rest.length ? `${whole}.${rest.join("").slice(0, decimals)}` : whole;
    setAmount(v.slice(0, 14));
  };

  const onSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!(value > 0) || sats < 1 || sats > MAX_SATS) {
      setError(
        effectiveMode === "sats"
          ? `Enter a whole number of sats between 1 and ${formatNumber(MAX_SATS)}.`
          : `Enter an amount in ${currency} worth at most ${formatNumber(MAX_SATS)} sats.`,
      );
      return;
    }
    const description = String(new FormData(e.currentTarget).get("description") ?? "").trim();
    setBusy(true);
    setError(null);
    try {
      const payment = await api.createInvoice(
        effectiveMode === "sats" ? { sats: value } : { fiat: amount },
        description || undefined,
      );
      router.push(`/payments/${payment.payment_hash}`);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Something went wrong");
      setBusy(false);
    }
  };

  const quick = effectiveMode === "sats" ? QUICK_SATS : rate ? quickAmounts(rate.btc_price) : [];
  const display = effectiveMode === "sats" ? (amount ? formatNumber(value) : "") : amount;
  const submitLabel = !amount
    ? "Show QR code"
    : effectiveMode === "sats"
      ? `Show QR code for ${formatNumber(value)} sats`
      : `Show QR code for ${formatMoney(value, currency)}`;

  return (
    <>
      <PageHeader title="Receive payment" subtitle="Enter the price and show the QR code to your customer." />
      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_320px]">
        <Card className="p-5 sm:p-8">
          <form onSubmit={onSubmit} className="space-y-6">
            <div>
              <div className="flex items-center justify-between gap-3">
                <label htmlFor="amount" className="block text-sm font-semibold">
                  Amount
                </label>
                <div className="inline-flex rounded-lg bg-ivory p-1" role="group" aria-label="Amount unit">
                  <UnitButton active={effectiveMode === "fiat"} disabled={!canUseFiat} onClick={() => switchMode("fiat")}>
                    {currency}
                  </UnitButton>
                  <UnitButton active={effectiveMode === "sats"} onClick={() => switchMode("sats")}>
                    sats
                  </UnitButton>
                </div>
              </div>
              <div className="mt-2 flex items-baseline gap-3 border-b-2 border-line-strong pb-2 focus-within:border-forest">
                {effectiveMode === "fiat" && <span className="text-xl font-semibold text-muted">{currency}</span>}
                <input
                  id="amount"
                  name="amount"
                  inputMode={effectiveMode === "fiat" && decimals > 0 ? "decimal" : "numeric"}
                  autoFocus
                  required
                  autoComplete="off"
                  placeholder="0"
                  value={display}
                  onChange={(e) => onAmountChange(e.target.value)}
                  className="tabular w-full min-w-0 bg-transparent font-heading text-5xl font-extrabold text-forest outline-none placeholder:text-line-strong sm:text-6xl"
                />
                {effectiveMode === "sats" && <span className="text-xl font-semibold text-muted">sats</span>}
              </div>
              <p className="tabular mt-2 flex min-h-5 items-center gap-1.5 text-sm text-muted">
                {amount && rate ? (
                  <>
                    <ArrowLeftRight aria-hidden className="h-3.5 w-3.5" />
                    {effectiveMode === "fiat"
                      ? `≈ ${formatNumber(sats)} sats`
                      : `≈ ${formatMoney(fiatEquivalent ?? 0, currency)}`}
                    <span className="text-subtle">· live rate</span>
                  </>
                ) : !rateLoading && !canUseFiat ? (
                  <>Exchange rate unavailable right now. Enter the amount in sats.</>
                ) : null}
              </p>
              <div className="mt-3 flex flex-wrap gap-2" role="group" aria-label="Quick amounts">
                {quick.map((q) => (
                  <button
                    key={q}
                    type="button"
                    onClick={() => setAmount(String(q))}
                    aria-pressed={value === q}
                    className={`h-9 rounded-full px-4 text-sm font-semibold transition-colors ${
                      value === q ? "bg-forest text-ivory" : "border border-line-strong bg-surface text-charcoal hover:border-forest/40"
                    }`}
                  >
                    {effectiveMode === "sats" ? formatNumber(q) : formatMoney(q, currency, 0)}
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
              {busy ? "Creating payment request…" : submitLabel}
            </Button>
            {effectiveMode === "fiat" && (
              <p className="-mt-2 text-center text-xs text-muted">
                The customer pays in bitcoin. Soko converts at the live rate when the QR code is created.
              </p>
            )}
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

function UnitButton({
  active,
  disabled,
  onClick,
  children,
}: {
  active: boolean;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      aria-pressed={active}
      className={`h-8 rounded-md px-3 text-sm font-semibold transition-colors disabled:opacity-40 ${
        active ? "bg-surface text-forest shadow-sm" : "text-muted hover:text-charcoal"
      }`}
    >
      {children}
    </button>
  );
}

function Step({ icon, title, text }: { icon: ReactNode; title: string; text: string }) {
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
