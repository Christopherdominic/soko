"use client";

import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import type { Rate } from "@/types/api";

/** Live BTC rate in the merchant's currency, refreshed every minute. Null if unavailable. */
export function useRate(currency: string): { rate: Rate | null; loading: boolean } {
  const [rate, setRate] = useState<Rate | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    const load = () =>
      api
        .rate()
        .then((r) => !cancelled && setRate(r))
        .catch(() => !cancelled && setRate(null))
        .finally(() => !cancelled && setLoading(false));
    load();
    const id = setInterval(load, 60_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [currency]);

  return { rate, loading };
}

/** Number of decimals a currency uses (e.g. 0 for UGX, XOF). */
export function currencyDecimals(currency: string): number {
  try {
    return new Intl.NumberFormat("en", { style: "currency", currency }).resolvedOptions().maximumFractionDigits ?? 2;
  } catch {
    return 2;
  }
}
