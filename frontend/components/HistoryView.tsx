"use client";

import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";
import { api, ApiError, type PaymentFilters } from "@/lib/api";
import { PaymentList } from "@/components/PaymentRow";
import { SecondaryButton } from "@/components/ui";
import type { Payment, PaymentStatus } from "@/types/api";

const PAGE = 20;
const STATUSES: { value: PaymentStatus | ""; label: string }[] = [
  { value: "", label: "All" },
  { value: "paid", label: "Paid" },
  { value: "pending", label: "Waiting" },
  { value: "expired", label: "Expired" },
  { value: "failed", label: "Failed" },
];

function initialStatus(v: string | null): PaymentStatus | "" {
  return STATUSES.some((s) => s.value === v) ? (v as PaymentStatus) : "";
}

export function HistoryView() {
  const params = useSearchParams();
  const [status, setStatus] = useState<PaymentStatus | "">(() => initialStatus(params.get("status")));
  const [search, setSearch] = useState("");
  const [debounced, setDebounced] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [payments, setPayments] = useState<Payment[] | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loadingMore, setLoadingMore] = useState(false);

  useEffect(() => {
    const id = setTimeout(() => setDebounced(search.trim()), 300);
    return () => clearTimeout(id);
  }, [search]);

  const filters: PaymentFilters = { status: status || undefined, q: debounced, from, to, limit: PAGE };

  useEffect(() => {
    let cancelled = false;
    api
      .payments({ status: status || undefined, q: debounced, from, to, limit: PAGE })
      .then((r) => {
        if (cancelled) return;
        setPayments(r.payments);
        setHasMore(r.has_more);
        setError(null);
      })
      .catch((e) => !cancelled && setError(e instanceof ApiError ? e.message : "Couldn't load history"));
    return () => {
      cancelled = true;
    };
  }, [status, debounced, from, to]);

  const loadMore = async () => {
    if (!payments) return;
    setLoadingMore(true);
    try {
      const r = await api.payments({ ...filters, offset: payments.length });
      setPayments([...payments, ...r.payments]);
      setHasMore(r.has_more);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Couldn't load more");
    } finally {
      setLoadingMore(false);
    }
  };

  const filtered = Boolean(status || debounced || from || to);

  return (
    <main className="mx-auto w-full max-w-2xl flex-1 px-4 py-6">
      <Link href="/" className="text-sm text-muted underline-offset-4 hover:underline">
        ← Home
      </Link>
      <h1 className="mt-3 mb-4 text-2xl font-extrabold">Transaction history</h1>

      <div className="space-y-3">
        <input
          type="search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          maxLength={64}
          placeholder="Search description or receipt number"
          aria-label="Search transactions"
          className="h-12 w-full rounded-xl border-2 border-line bg-white px-4 outline-none focus:border-forest"
        />
        <div className="flex gap-2 overflow-x-auto pb-1" role="group" aria-label="Filter by status">
          {STATUSES.map((s) => (
            <button
              key={s.value}
              onClick={() => setStatus(s.value)}
              aria-pressed={status === s.value}
              className={`h-10 shrink-0 rounded-full px-4 text-sm font-semibold ${
                status === s.value ? "bg-forest text-ivory" : "border border-line bg-white"
              }`}
            >
              {s.label}
            </button>
          ))}
        </div>
        <div className="grid grid-cols-2 gap-3">
          <DateField label="From" value={from} max={to || undefined} onChange={setFrom} />
          <DateField label="To" value={to} min={from || undefined} onChange={setTo} />
        </div>
        {filtered && (
          <button
            onClick={() => {
              setStatus("");
              setSearch("");
              setFrom("");
              setTo("");
            }}
            className="text-sm font-semibold text-forest underline"
          >
            Clear filters
          </button>
        )}
      </div>

      <div className="mt-5">
        {error && <p className="mb-3 text-red-700">{error}</p>}
        {!payments ? (
          <p className="text-muted">Loading…</p>
        ) : payments.length === 0 ? (
          <p className="rounded-2xl border border-dashed border-line bg-white px-4 py-8 text-center text-muted">
            {filtered ? "No transactions match these filters." : "No transactions yet."}
          </p>
        ) : (
          <>
            <PaymentList payments={payments} />
            {hasMore && (
              <SecondaryButton onClick={loadMore} disabled={loadingMore} className="mt-4">
                {loadingMore ? "Loading…" : "Load more"}
              </SecondaryButton>
            )}
          </>
        )}
      </div>
    </main>
  );
}

function DateField({
  label,
  value,
  onChange,
  min,
  max,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  min?: string;
  max?: string;
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-sm font-semibold">{label}</span>
      <input
        type="date"
        value={value}
        min={min}
        max={max}
        onChange={(e) => onChange(e.target.value)}
        className="h-12 w-full rounded-xl border-2 border-line bg-white px-3 outline-none focus:border-forest"
      />
    </label>
  );
}
