import type { FiatValue } from "@/types/api";

/** "KSh 1,500.00", "₦12,000.00", "CFA 5,000" — using the browser's currency data. */
export function formatMoney(amount: number | string, currency: string, decimals?: number) {
  const n = typeof amount === "string" ? Number(amount) : amount;
  try {
    return new Intl.NumberFormat("en", {
      style: "currency",
      currency,
      currencyDisplay: "narrowSymbol",
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    }).format(n);
  } catch {
    return `${currency} ${n.toLocaleString("en")}`;
  }
}

export const formatFiat = (f: FiatValue) => formatMoney(f.amount, f.currency);

/** sats -> local amount at a BTC price. */
export const fiatForSats = (sats: number, btcPrice: number) => (sats / 1e8) * btcPrice;

/** local amount -> sats at a BTC price. */
export const satsForFiat = (amount: number, btcPrice: number) => Math.round((amount / btcPrice) * 1e8);

/** Five distinct round local amounts (1-2-5 steps), starting near the value of ~300 sats. */
export function quickAmounts(btcPrice: number): number[] {
  const start = fiatForSats(300, btcPrice);
  if (!(start > 0)) return [];
  let mag = 10 ** Math.floor(Math.log10(start));
  const out: number[] = [];
  while (out.length < 5) {
    for (const step of [1, 2, 5]) {
      const v = step * mag;
      if (v >= start && out.length < 5) out.push(v >= 1 ? Math.round(v) : v);
    }
    mag *= 10;
  }
  return out;
}
