import type {
  DashboardSummary,
  MerchantReceipt,
  MerchantWithKey,
  NostrEvent,
  Payment,
  Payout,
  Rate,
  Settings,
  PaymentList,
  PaymentStatus,
  PublicReceipt,
  VerifyResult,
} from "@/types/api";

export const browserTimeZone = () => Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";

export interface PaymentFilters {
  status?: PaymentStatus;
  q?: string;
  from?: string;
  to?: string;
  limit?: number;
  offset?: number;
}

export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message);
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`/api${path}`, {
      ...init,
      cache: "no-store",
      credentials: "same-origin",
      headers: init?.body ? { "Content-Type": "application/json" } : undefined,
    });
  } catch {
    throw new ApiError(0, "network_error", "Can't reach Soko. Check your connection.");
  }
  if (res.status === 204) return undefined as T;
  const body = await res.json().catch(() => null);
  if (!res.ok) {
    throw new ApiError(
      res.status,
      body?.error?.code ?? "unknown_error",
      body?.error?.message ?? "Something went wrong",
    );
  }
  return body as T;
}

const post = <T>(path: string, data?: unknown) =>
  request<T>(path, { method: "POST", body: data === undefined ? undefined : JSON.stringify(data) });

export const api = {
  me: () => request<MerchantWithKey>("/auth/me"),
  login: (email: string, password: string) => post<MerchantWithKey>("/auth/login", { email, password }),
  register: (business_name: string, email: string, password: string) =>
    post<MerchantWithKey>("/auth/register", { business_name, email, password }),
  logout: () => post<void>("/auth/logout"),

  /** Pass sats, or a decimal string in the merchant's currency (converted server-side). */
  createInvoice: (amount: { sats: number } | { fiat: string }, description?: string) =>
    post<Payment>("/payments/invoices", {
      ...("sats" in amount ? { amount_sats: amount.sats } : { fiat_amount: amount.fiat }),
      description: description || null,
    }),
  payment: (hash: string) => request<Payment>(`/payments/${encodeURIComponent(hash)}`),
  payments: (f: PaymentFilters = {}) => {
    const q = new URLSearchParams();
    for (const [k, v] of Object.entries(f)) {
      if (v !== undefined && v !== "") q.set(k, String(v));
    }
    if (f.from || f.to) q.set("tz", browserTimeZone());
    return request<PaymentList>(`/payments?${q}`);
  },
  receipt: (hash: string) => request<MerchantReceipt>(`/payments/${encodeURIComponent(hash)}/receipt`),
  dashboard: () =>
    request<DashboardSummary>(`/dashboard/summary?tz=${encodeURIComponent(browserTimeZone())}`),
  publicReceipt: (number: string) =>
    request<PublicReceipt>(`/public/receipts/${encodeURIComponent(number)}`),
  verifyReceipt: (event: NostrEvent) => post<VerifyResult>("/public/receipts/verify", event),

  settings: () => request<Settings>("/settings"),
  updateSettings: (currency: string) =>
    request<Settings>("/settings", { method: "PUT", body: JSON.stringify({ currency }) }),
  rate: () => request<Rate>("/rates"),
  payouts: () => request<{ payouts: Payout[] }>("/payouts"),
  createPayout: (p: { fiat_amount?: string; amount_sats?: number; provider: string; destination: string }) =>
    post<Payout>("/payouts", p),
};
