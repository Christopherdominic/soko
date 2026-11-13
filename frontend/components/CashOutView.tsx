"use client";

import { useCallback, useEffect, useState, type FormEvent } from "react";
import { ArrowRight, Banknote, CheckCircle2, FlaskConical, Smartphone } from "lucide-react";
import { useMerchant } from "@/hooks/useMerchant";
import { currencyDecimals, useRate } from "@/hooks/useRate";
import { api, ApiError } from "@/lib/api";
import { formatDateTime, formatNumber } from "@/lib/format";
import { fiatForSats, formatFiat, formatMoney, satsForFiat } from "@/lib/money";
import { Button, Card, CardHeader, EmptyState, ErrorMessage, Field, Loading, PageHeader } from "@/components/ui";
import type { DashboardSummary, Payout } from "@/types/api";

export function DemoBadge() {
  return (
    <span className="inline-flex items-center gap-1 rounded-md bg-amber-100 px-2 py-0.5 text-[11px] font-bold tracking-wide text-amber-900 uppercase">
      <FlaskConical aria-hidden className="h-3 w-3" /> Demo
    </span>
  );
}

export function CashOutView() {
  const merchant = useMerchant();
  const currency = merchant.currency;
  const decimals = currencyDecimals(currency);
  const { rate } = useRate(currency);

  const [providers, setProviders] = useState<string[]>([]);
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [payouts, setPayouts] = useState<Payout[] | null>(null);

  const [amount, setAmount] = useState("");
  const [provider, setProvider] = useState("");
  const [destination, setDestination] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<Payout | null>(null);

  const refresh = useCallback(() => {
    api.dashboard().then(setSummary).catch(() => {});
    api.payouts().then((r) => setPayouts(r.payouts)).catch(() => setPayouts([]));
  }, []);

  useEffect(() => {
    api
      .settings()
      .then((s) => {
        const list = s.currencies.find((c) => c.code === s.currency)?.payout_providers ?? [];
        setProviders(list);
        setProvider((p) => p || list[0] || "");
      })
      .catch(() => {});
    refresh();
  }, [refresh]);

  const available = summary?.available_sats ?? 0;
  const availableFiat = rate ? fiatForSats(available, rate.btc_price) : null;
  const value = Number(amount || 0);
  const sats = rate ? satsForFiat(value, rate.btc_price) : 0;
  const tooMuch = sats > available;

  const setMax = () => {
    if (availableFiat === null) return;
    const factor = 10 ** decimals;
    setAmount(String(Math.floor(availableFiat * factor) / factor));
  };

  const onAmountChange = (raw: string) => {
    let v = raw.replace(/[^0-9.]/g, "");
    const [whole, ...rest] = v.split(".");
    v = decimals > 0 && rest.length ? `${whole}.${rest.join("").slice(0, decimals)}` : whole;
    setAmount(v.slice(0, 14));
  };

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!(value > 0)) return setError(`Enter an amount in ${currency}.`);
    if (tooMuch) return setError("That's more than your available balance.");
    setBusy(true);
    try {
      const p = await api.createPayout({ fiat_amount: amount, provider, destination });
      setDone(p);
      setAmount("");
      setDestination("");
      refresh();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Something went wrong");
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <PageHeader
        title="Cash out"
        subtitle={`Turn your bitcoin sales into ${currency} on mobile money or in the bank.`}
        action={<DemoBadge />}
      />

      <div className="mb-6 flex gap-3 rounded-2xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">
        <FlaskConical aria-hidden className="mt-0.5 h-5 w-5 shrink-0" />
        <p>
          <strong>Demo mode: no real money moves.</strong> Cash-outs are recorded at the live exchange rate so you can see
          the flow. In production, your own Lightning node would pay a cash-out provider (for example, a mobile-money
          Lightning address) and you&apos;d receive {currency}.
        </p>
      </div>

      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] lg:items-start">
        <Card>
          {done ? (
            <div className="p-6 text-center sm:p-8">
              <CheckCircle2 aria-hidden className="mx-auto h-14 w-14 text-emerald-600" strokeWidth={1.75} />
              <p className="mt-4 font-heading text-2xl font-extrabold text-forest">{formatFiat(done.fiat)} sent</p>
              <p className="mt-1 text-muted">
                to {done.provider} {done.destination}
              </p>
              <p className="mt-3 inline-flex items-center gap-2 text-sm text-muted">
                <DemoBadge /> {formatNumber(done.amount_sats)} sats · no real money moved
              </p>
              <div className="mt-6">
                <Button variant="secondary" size="lg" onClick={() => setDone(null)}>
                  Cash out again
                </Button>
              </div>
            </div>
          ) : (
            <form onSubmit={onSubmit} className="space-y-6 p-5 sm:p-7">
              <div className="rounded-xl bg-forest p-5 text-ivory">
                <p className="text-sm text-ivory/75">Available to cash out</p>
                {summary ? (
                  <>
                    <p className="tabular mt-1 font-heading text-3xl font-extrabold text-lime">
                      {availableFiat !== null ? formatMoney(availableFiat, currency) : `${formatNumber(available)} sats`}
                    </p>
                    {availableFiat !== null && <p className="tabular text-sm text-ivory/70">{formatNumber(available)} sats</p>}
                  </>
                ) : (
                  <p className="mt-1 text-ivory/70">Loading…</p>
                )}
              </div>

              <div>
                <div className="mb-1.5 flex items-center justify-between">
                  <label htmlFor="cashout-amount" className="text-sm font-semibold">
                    Amount ({currency})
                  </label>
                  <button type="button" onClick={setMax} className="text-sm font-semibold text-forest hover:underline" disabled={!available}>
                    Max
                  </button>
                </div>
                <input
                  id="cashout-amount"
                  inputMode={decimals > 0 ? "decimal" : "numeric"}
                  placeholder="0"
                  value={amount}
                  onChange={(e) => onAmountChange(e.target.value)}
                  className={`tabular h-14 w-full rounded-xl border bg-surface px-4 font-heading text-2xl font-bold outline-none focus:ring-4 focus:ring-forest/10 ${
                    tooMuch ? "border-red-400" : "border-line-strong focus:border-forest"
                  }`}
                />
                <p className={`tabular mt-1.5 text-sm ${tooMuch ? "text-red-700" : "text-muted"}`}>
                  {amount && rate ? `${formatNumber(sats)} sats${tooMuch ? " · more than available" : ""}` : " "}
                </p>
              </div>

              <fieldset>
                <legend className="mb-2 text-sm font-semibold">Send to</legend>
                <div className="flex flex-wrap gap-2">
                  {providers.map((p) => (
                    <button
                      key={p}
                      type="button"
                      onClick={() => setProvider(p)}
                      aria-pressed={provider === p}
                      className={`inline-flex h-10 items-center gap-1.5 rounded-xl px-3.5 text-sm font-semibold transition-colors ${
                        provider === p ? "bg-forest text-ivory" : "border border-line-strong bg-surface hover:border-forest/40"
                      }`}
                    >
                      {p.toLowerCase().includes("bank") ? <Banknote aria-hidden className="h-4 w-4" /> : <Smartphone aria-hidden className="h-4 w-4" />}
                      {p}
                    </button>
                  ))}
                </div>
              </fieldset>

              <Field
                label={provider.toLowerCase().includes("bank") ? "Account number" : "Phone number"}
                name="destination"
                inputMode="tel"
                autoComplete="off"
                placeholder={provider.toLowerCase().includes("bank") ? "e.g. 0123456789" : "e.g. 0712 345 678"}
                value={destination}
                onChange={(e) => setDestination(e.target.value)}
                hint="Only the last 4 digits are stored."
              />

              {amount && rate && provider && destination && !tooMuch && (
                <p className="flex flex-wrap items-center gap-2 rounded-xl bg-ivory px-4 py-3 text-sm">
                  <span className="tabular font-semibold">{formatNumber(sats)} sats</span>
                  <ArrowRight aria-hidden className="h-4 w-4 text-muted" />
                  <span className="tabular font-semibold">{formatMoney(value, currency)}</span>
                  <span className="text-muted">via {provider}</span>
                </p>
              )}

              <ErrorMessage>{error}</ErrorMessage>
              <Button type="submit" size="lg" full loading={busy} disabled={!amount || !provider || !destination || !rate}>
                {!busy && <Banknote aria-hidden className="h-5 w-5" />}
                {busy ? "Processing…" : "Cash out (demo)"}
              </Button>
              {!rate && <p className="text-center text-sm text-muted">Waiting for the exchange rate…</p>}
            </form>
          )}
        </Card>

        <Card>
          <CardHeader title="Cash-out history" subtitle="Demo records at the rate on the day" />
          {!payouts ? (
            <Loading />
          ) : payouts.length === 0 ? (
            <EmptyState icon={<Banknote aria-hidden className="h-6 w-6" />} title="No cash-outs yet" text="Your demo cash-outs will appear here." />
          ) : (
            <ul className="divide-y divide-line">
              {payouts.map((p) => (
                <li key={p.id} className="flex items-center justify-between gap-3 px-5 py-3.5 text-sm">
                  <div className="min-w-0">
                    <p className="tabular font-semibold">{formatFiat(p.fiat)}</p>
                    <p className="truncate text-muted">
                      {p.provider} {p.destination} · {formatDateTime(p.created_at)}
                    </p>
                  </div>
                  <div className="flex shrink-0 flex-col items-end gap-1">
                    <DemoBadge />
                    <span className="tabular text-xs text-muted">{formatNumber(p.amount_sats)} sats</span>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>
    </>
  );
}
