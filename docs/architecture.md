# Soko architecture

## Overview

```
 Customer wallet ──(Lightning)──► Merchant's LND node ◄──REST+TLS pin+macaroon──┐
                                                                                │
 Merchant browser ──► Next.js (UI, /api proxy) ──► Rust/Axum API ──► PostgreSQL │
                                                     │                          │
                                                     └── payment watcher ───────┘
                                                         (LND invoice stream +
                                                          reconciliation poll)
```

- **Frontend (Next.js):** UI only. It never decides whether a payment is paid; it reads status from the API.
- **Backend (Rust/Axum):** owns all business logic, talks to LND, writes payment state.
- **LND:** the merchant's own node. It creates invoices and is the **only source of truth** for settlement.
- **PostgreSQL:** payment records, receipts, merchants and sessions.

## Key decisions

| Decision | Why |
|---|---|
| **LND REST, not gRPC** | Fewer build dependencies (no protoc), easy to debug with curl, and enough for invoices plus the invoice subscription stream. |
| **Pin LND's TLS certificate** | LND uses a self-signed cert. Instead of turning verification off, the client accepts only that exact certificate, and the handshake signature is still checked. |
| **Least-privilege macaroon** | Soko only needs `invoices:read` and `invoices:write`, so it uses `invoice.macaroon` and never `admin.macaroon`. Even a fully compromised Soko server cannot spend funds. |
| **`LightningBackend` trait** | Verification logic is tested against a fake node; the real LND client is tested against Polar. |
| **Verification is server-side only** | Status changes from `pending` only when LND reports the invoice `SETTLED` (or `CANCELED` → `expired`). No API endpoint accepts a status from a client. |
| **Idempotent state transitions** | `UPDATE … WHERE status = 'pending'` plus a unique `payment_id` on receipts. Duplicate stream events, polls and retries cannot double-process a payment or issue two receipts. |
| **Stream + poll** | The LND subscription gives instant updates. A periodic reconciliation of pending invoices covers missed events (restarts, dropped connections). |
| **Same-origin `/api` proxy** | Next.js rewrites `/api/*` to the backend, so session cookies stay `HttpOnly; SameSite=Strict` without CORS complexity. |
| **Runtime SQL (no compile-time `query!`)** | The backend builds without a live database (CI, Docker). |

## Custody model

Soko is **non-custodial**: each merchant deployment connects to the merchant's own LND node, so funds settle directly into the merchant's channels. Soko never holds private keys or seed phrases, and its macaroon cannot send payments.

For the hackathon MVP, one Soko deployment connects to one LND node (one merchant business, which may have several staff logins later). A hosted multi-tenant version would store a per-merchant node connection. It must never pool several merchants on a shared node, because that would make Soko custodial.

## Payment lifecycle

1. Merchant submits amount (sats) and an optional description → `POST /api/payments/invoices`.
2. Backend validates the input, calls LND `AddInvoice`, and stores a `pending` payment with the payment hash and expiry.
3. UI shows a QR code and polls `GET /api/payments/{hash}`.
4. The watcher receives LND invoice updates. On `SETTLED`, inside one DB transaction it:
   - checks the amount paid is at least the amount requested,
   - sets `pending → paid` (conditional update; a no-op if already processed),
   - inserts the receipt (unique per payment).
5. On `CANCELED`, or after expiry with no settlement, it sets `pending → expired`.

States: `pending`, `paid`, `expired`, `failed` (`failed` is reserved for invoices LND no longer knows, or that were underpaid).

## Receipts and Nostr (Milestone 3)

- Each paid payment gets one receipt with a human-friendly number.
- The receipt is signed as a Nostr (NIP-01) event using a merchant key held by the backend. Anyone with the merchant's public key can verify it.
- The public event contains only: receipt number, amount, denomination, timestamp and a **salted hash commitment** to the payment. It never includes the payment hash, preimage, invoice, customer data or description.
- Publishing to relays is opt-in. By default the signed event is just downloadable and verifiable offline.

## Project structure

```
soko/
  backend/            Rust/Axum API
    src/
      config.rs       env config (secrets redacted in Debug)
      state.rs        shared AppState
      errors/         ApiError → JSON responses
      lightning/      LightningBackend trait + LND REST client
      handlers/       HTTP handlers
      routes/         router
      services/       auth helpers, payment verification, background watcher
      repositories/   SQL access (merchants, sessions, payments, receipts)
      models/         domain types
      nostr/          receipt signing (M3)
      middleware/     CurrentMerchant session extractor
    migrations/
    tests/
  frontend/           Next.js app (UI only)
  docs/
  scripts/            Polar regtest helpers (channel setup, customer payments)
  docker-compose.yml  PostgreSQL for development
```

## Milestones

1. **Foundation**: monorepo, Axum, Postgres plus migrations, LND client with TLS pinning, `/api/health`, Next.js shell with `/api` proxy. ✅
2. **Lightning payments**: merchant login (Argon2 plus `HttpOnly` session cookie, moved up from M3), create invoice, QR code, watcher (stream plus reconciliation), idempotent verification, receipts issued on settlement, payment status page, tests against a fake node, full lifecycle verified on Polar. ✅
3. **Merchant experience**: dashboard summary, history with search and date filters, receipt page, Nostr-signed receipts.
4. **Submission readiness**: security review, Dockerised backend and frontend, docs, demo script.
5. **Soko AI (optional)**: read-only sales summaries over stored records.
