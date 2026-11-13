"use client";

import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";
import { Search, SearchX, ReceiptText, X } from "lucide-react";
import { api, ApiError, type PaymentFilters } from "@/lib/api";
import { PaymentList } from "@/components/PaymentRow";
import { Button, Card, EmptyState, Loading, PageHeader } from "@/components/ui";
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
  const clear = () => {
    setStatus("");
    setSearch("");
    setFrom("");
    setTo("");
  };

  return (
    <>
      <PageHeader title="Transactions" subtitle="Every payment request, its status and receipt." />

      <Card>
        <div className="space-y-4 border-b border-line p-4 sm:p-5">
          <div className="flex flex-col gap-3 lg:flex-row lg:items-end">
            <label className="relative block flex-1">
              <span className="sr-only">Search transactions</span>
              <Search aria-hidden className="pointer-events-none absolute top-1/2 left-3.5 h-4 w-4 -translate-y-1/2 text-subtle" />
              <input
                type="search"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                maxLength={64}
                placeholder="Search description or receipt number"
                className="h-11 w-full rounded-xl border border-line-strong bg-surface pr-4 pl-10 text-sm outline-none placeholder:text-subtle focus:border-forest focus:ring-4 focus:ring-forest/10"
              />
            </label>
            <div className="grid grid-cols-2 gap-3 lg:w-80">
              <DateField label="From" value={from} max={to || undefined} onChange={setFrom} />
              <DateField label="To" value={to} min={from || undefined} onChange={setTo} />
            </div>
          </div>
          <div className="flex items-center justify-between gap-3">
            <div className="-mx-1 flex gap-1 overflow-x-auto px-1 pb-0.5" role="group" aria-label="Filter by status">
              {STATUSES.map((s) => (
                <button
                  key={s.value}
                  onClick={() => setStatus(s.value)}
                  aria-pressed={status === s.value}
                  className={`h-9 shrink-0 rounded-lg px-3 text-sm font-semibold transition-colors ${
                    status === s.value ? "bg-forest text-ivory" : "text-muted hover:bg-ivory hover:text-charcoal"
                  }`}
                >
                  {s.label}
                </button>
              ))}
            </div>
            {filtered && (
              <button onClick={clear} className="inline-flex shrink-0 items-center gap-1 text-sm font-semibold text-forest hover:underline">
                <X aria-hidden className="h-4 w-4" /> Clear
              </button>
            )}
          </div>
        </div>

        {error && <p className="px-5 pt-4 text-sm text-red-700">{error}</p>}
        {!payments ? (
          <Loading />
        ) : payments.length === 0 ? (
          filtered ? (
            <EmptyState icon={<SearchX aria-hidden className="h-6 w-6" />} title="No matches" text="No transactions match these filters." />
          ) : (
            <EmptyState icon={<ReceiptText aria-hidden className="h-6 w-6" />} title="No transactions yet" text="Payments you receive will appear here." />
          )
        ) : (
          <>
            <PaymentList payments={payments} />
            {hasMore && (
              <div className="border-t border-line p-4">
                <Button variant="secondary" full onClick={loadMore} loading={loadingMore}>
                  Load more
                </Button>
              </div>
            )}
          </>
        )}
      </Card>
    </>
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
      <span className="mb-1 block text-xs font-semibold text-muted">{label}</span>
      <input
        type="date"
        value={value}
        min={min}
        max={max}
        onChange={(e) => onChange(e.target.value)}
        className="h-11 w-full rounded-xl border border-line-strong bg-surface px-3 text-sm outline-none focus:border-forest focus:ring-4 focus:ring-forest/10"
      />
    </label>
  );
}
