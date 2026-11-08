export type ServiceStatus = "ok" | "starting" | "unavailable";

export interface HealthResponse {
  status: "ok" | "degraded";
  database: "ok" | "unavailable";
  lightning: ServiceStatus;
}

export interface Merchant {
  id: string;
  business_name: string;
  email: string;
}

export type PaymentStatus = "pending" | "paid" | "expired" | "failed";

export interface Payment {
  payment_hash: string;
  payment_request: string;
  amount_sats: number;
  description: string | null;
  status: PaymentStatus;
  is_demo: boolean;
  expires_at: string;
  settled_at: string | null;
  created_at: string;
  receipt_number: string | null;
}

export interface Receipt {
  receipt_number: string;
  business_name: string;
  amount_sats: number;
  denomination: "sats";
  description: string | null;
  payment_hash: string;
  paid_at: string | null;
  issued_at: string;
}
