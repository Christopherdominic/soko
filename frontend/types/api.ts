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
  fiat: FiatValue | null;
}

/** Local-currency value: decimal string in major units + the BTC price used. */
export interface FiatValue {
  currency: string;
  amount: string;
  btc_price: number;
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
  fiat: FiatValue | null;
}

export interface MerchantWithKey extends Merchant {
  nostr_npub: string | null;
  currency: string;
}

export interface CurrencyInfo {
  code: string;
  name: string;
  decimals: number;
  payout_providers: string[];
}

export interface Settings {
  currency: string;
  currencies: CurrencyInfo[];
}

export interface Rate {
  currency: string;
  btc_price: number;
  sats_per_unit: number;
  updated_at: string;
  source: string;
  fresh: boolean;
}

export interface Payout {
  id: string;
  amount_sats: number;
  fiat: FiatValue;
  provider: string;
  destination: string;
  status: "completed";
  is_demo: boolean;
  created_at: string;
}

export interface PaymentList {
  payments: Payment[];
  has_more: boolean;
}

export interface DashboardSummary {
  time_zone: string;
  total_sales_sats: number;
  paid_count: number;
  pending_count: number;
  today_sales_sats: number;
  today_count: number;
  last_7_days: { day: string; sales_sats: number; count: number }[];
  currency: string;
  total_sales_fiat: FiatValue | null;
  today_sales_fiat: FiatValue | null;
  available_sats: number;
  available_fiat: FiatValue | null;
  paid_out_sats: number;
}

export interface NostrEvent {
  id: string;
  pubkey: string;
  created_at: number;
  kind: number;
  tags: string[][];
  content: string;
  sig: string;
}

export interface SignedReceipt {
  event: NostrEvent;
  npub: string | null;
  verification_code: string;
}

export interface MerchantReceipt extends Receipt {
  public_path: string;
  nostr: SignedReceipt | null;
}

export interface PublicReceipt {
  receipt_number: string;
  business_name: string;
  amount_sats: number;
  denomination: "sats";
  paid_at: string | null;
  issued_at: string;
  fiat: FiatValue | null;
  nostr: SignedReceipt | null;
}

export interface VerifyResult {
  signature_valid: boolean;
  issued_by_known_merchant: boolean;
  matches_records: boolean;
  business_name: string | null;
  receipt_number: string | null;
  amount_sats: number | null;
  paid_at: number | null;
  npub: string | null;
  problem: string | null;
}
