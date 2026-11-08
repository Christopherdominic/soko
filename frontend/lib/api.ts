import type { Merchant, Payment, PaymentStatus, Receipt } from "@/types/api";

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
  me: () => request<Merchant>("/auth/me"),
  login: (email: string, password: string) => post<Merchant>("/auth/login", { email, password }),
  register: (business_name: string, email: string, password: string) =>
    post<Merchant>("/auth/register", { business_name, email, password }),
  logout: () => post<void>("/auth/logout"),

  createInvoice: (amount_sats: number, description?: string) =>
    post<Payment>("/payments/invoices", { amount_sats, description: description || null }),
  payment: (hash: string) => request<Payment>(`/payments/${encodeURIComponent(hash)}`),
  payments: (opts: { status?: PaymentStatus; limit?: number } = {}) => {
    const q = new URLSearchParams();
    if (opts.status) q.set("status", opts.status);
    if (opts.limit) q.set("limit", String(opts.limit));
    return request<{ payments: Payment[] }>(`/payments?${q}`);
  },
  receipt: (hash: string) => request<Receipt>(`/payments/${encodeURIComponent(hash)}/receipt`),
};
