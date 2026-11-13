"use client";

import { useEffect, useMemo, useState } from "react";
import { Check, Search } from "lucide-react";
import { useMerchant } from "@/hooks/useMerchant";
import { useRate } from "@/hooks/useRate";
import { api, ApiError } from "@/lib/api";
import { formatDateTime, formatNumber } from "@/lib/format";
import { formatMoney } from "@/lib/money";
import { Button, Card, CardHeader, ErrorMessage, Loading, PageHeader } from "@/components/ui";
import type { CurrencyInfo } from "@/types/api";

export function SettingsView() {
  const merchant = useMerchant();
  const { rate } = useRate(merchant.currency);
  const [currencies, setCurrencies] = useState<CurrencyInfo[] | null>(null);
  const [selected, setSelected] = useState(merchant.currency);
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.settings().then((s) => setCurrencies(s.currencies)).catch(() => setError("Couldn't load settings."));
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (currencies ?? []).filter(
      (c) => !q || c.code.toLowerCase().includes(q) || c.name.toLowerCase().includes(q),
    );
  }, [currencies, query]);

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.updateSettings(selected);
      // Reload so every screen picks up the new currency.
      window.location.reload();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Couldn't save");
      setBusy(false);
    }
  };

  return (
    <>
      <PageHeader title="Settings" subtitle="Your business details and local currency." />
      <div className="grid gap-6 lg:grid-cols-[minmax(0,1.3fr)_minmax(0,1fr)] lg:items-start">
        <Card>
          <CardHeader
            title="Local currency"
            subtitle="Used for prices, totals, receipts and cash-out. Customers always pay in bitcoin."
          />
          <div className="p-5">
            <label className="relative block">
              <span className="sr-only">Search currencies</span>
              <Search aria-hidden className="pointer-events-none absolute top-1/2 left-3.5 h-4 w-4 -translate-y-1/2 text-subtle" />
              <input
                type="search"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search by country currency or code"
                className="h-11 w-full rounded-xl border border-line-strong bg-surface pr-4 pl-10 text-sm outline-none focus:border-forest focus:ring-4 focus:ring-forest/10"
              />
            </label>
            {!currencies ? (
              <Loading />
            ) : (
              <ul className="mt-4 grid max-h-96 gap-2 overflow-y-auto pr-1 sm:grid-cols-2" role="radiogroup" aria-label="Currency">
                {filtered.map((c) => (
                  <li key={c.code}>
                    <button
                      type="button"
                      role="radio"
                      aria-checked={selected === c.code}
                      onClick={() => setSelected(c.code)}
                      className={`flex w-full items-center justify-between gap-2 rounded-xl border px-3.5 py-2.5 text-left text-sm transition-colors ${
                        selected === c.code ? "border-forest bg-lime-soft" : "border-line hover:border-forest/40"
                      }`}
                    >
                      <span className="min-w-0">
                        <span className="block font-semibold">{c.code}</span>
                        <span className="block truncate text-muted">{c.name}</span>
                      </span>
                      {selected === c.code && <Check aria-hidden className="h-4 w-4 shrink-0 text-forest" />}
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <div className="mt-5 space-y-3">
              <ErrorMessage>{error}</ErrorMessage>
              <Button size="lg" full onClick={save} loading={busy} disabled={selected === merchant.currency}>
                {selected === merchant.currency ? `Using ${merchant.currency}` : `Switch to ${selected}`}
              </Button>
            </div>
          </div>
        </Card>

        <div className="space-y-6">
          <Card>
            <CardHeader title="Live exchange rate" />
            <div className="p-5">
              {rate ? (
                <>
                  <p className="tabular font-heading text-2xl font-extrabold text-forest">
                    1 BTC = {formatMoney(rate.btc_price, rate.currency, 0)}
                  </p>
                  <p className="tabular mt-1 text-sm text-muted">
                    {formatMoney(1, rate.currency)} ≈ {formatNumber(Math.round(rate.sats_per_unit * 100) / 100)} sats
                  </p>
                  <p className="mt-3 text-xs text-subtle">
                    BTC price from {rate.source}, updated {formatDateTime(rate.updated_at)}. Currency rates update daily.
                    {!rate.fresh && " The price is out of date, so new prices fall back to sats."}
                  </p>
                </>
              ) : (
                <p className="text-sm text-muted">Exchange rate unavailable right now.</p>
              )}
            </div>
          </Card>

          <Card>
            <CardHeader title="Business" />
            <dl className="divide-y divide-line text-sm">
              <Row label="Name" value={merchant.business_name} />
              <Row label="Email" value={merchant.email} />
              {merchant.nostr_npub && (
                <div className="px-5 py-3">
                  <dt className="text-muted">Receipt signing key</dt>
                  <dd className="mt-1 break-all font-mono text-xs">{merchant.nostr_npub}</dd>
                </div>
              )}
            </dl>
          </Card>
        </div>
      </div>
    </>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-4 px-5 py-3">
      <dt className="text-muted">{label}</dt>
      <dd className="truncate font-medium">{value}</dd>
    </div>
  );
}
