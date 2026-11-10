# Soko

**Every market. Every payment.**
Smarter payments for everyday business.

Soko helps small businesses and market traders accept Bitcoin over the Lightning Network. Each payment is verified directly with the merchant's own Lightning node, gets a digital receipt, and lands in simple business records, without the merchant needing to understand any of the technology.

## What it does

- **Receive payment in three taps:** enter an amount, show the QR code, done.
- **Payments verified by the merchant's own node:** a payment is marked paid only when the merchant's LND node reports the invoice settled for the right amount. The browser can never mark something paid.
- **Digital receipts:** every payment gets a receipt number and a shareable customer link. Receipts are signed as **Nostr** events, so anyone can check they're genuine, and they contain no payment secrets or customer data.
- **Business records:** today's sales, totals, waiting payments, and a searchable history filtered by date and status.
- **Non-custodial:** Soko never holds funds or keys. Its LND credential can create invoices but **cannot spend**.

## Quick start (Docker + Polar)

```bash
cp .env.example .env               # fill in Postgres password, LND paths, Nostr secret
scripts/polar-setup.sh             # one time: open a regtest channel customer → merchant
docker compose up -d --build       # PostgreSQL + backend + frontend
# then open http://localhost:3000
```

Make a payment as the customer:

```bash
scripts/pay-invoice.sh <lnbcrt... payment request from the app>
```

- Full setup, development mode and troubleshooting: **[docs/setup.md](docs/setup.md)**
- Step-by-step demo script, video plan and presentation outline: **[docs/demo-guide.md](docs/demo-guide.md)**

## Architecture

```
Customer wallet ──Lightning──► Merchant's LND node ◄── REST + pinned TLS + invoice macaroon ──┐
                                                                                           │
Browser ──► Next.js (UI, /api proxy) ──► Rust / Axum API ──► PostgreSQL                    │
                                            └── payment watcher (LND invoice stream ───────┘
                                                + 15 s reconciliation)
```

| Layer | Tech |
|---|---|
| Backend | Rust, Axum, Tokio, SQLx, PostgreSQL |
| Frontend | Next.js, TypeScript, Tailwind CSS (mobile-first) |
| Lightning | LND REST API; Polar regtest for development |
| Receipts | Nostr NIP-01 events, BIP-340 Schnorr signatures |

Design decisions are in **[docs/architecture.md](docs/architecture.md)**; the threat model is in **[docs/security.md](docs/security.md)**.

## API

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET | `/api/health` | – | Database and LND status |
| POST | `/api/auth/register` | – | Create merchant account |
| POST | `/api/auth/login` | – | Log in (sets `HttpOnly` session cookie) |
| POST | `/api/auth/logout` | ✓ | End session |
| GET | `/api/auth/me` | ✓ | Current merchant and receipt-signing key |
| POST | `/api/payments/invoices` | ✓ | Create invoice `{amount_sats, description?}` |
| GET | `/api/payments` | ✓ | History `?status=&q=&from=&to=&tz=&limit=&offset=` |
| GET | `/api/payments/{payment_hash}` | ✓ | Payment status (re-verified with LND if pending) |
| GET | `/api/payments/{payment_hash}/receipt` | ✓ | Receipt with signed Nostr event and share link |
| GET | `/api/dashboard/summary` | ✓ | Today and total sales, counts, last 7 days `?tz=` |
| GET | `/api/public/receipts/{receipt_number}` | – | Customer receipt (paper-receipt fields only) |
| POST | `/api/public/receipts/verify` | – | Verify a signed receipt event |

Errors are JSON: `{"error": {"code": "...", "message": "..."}}`. Rate-limited requests get `429` with `Retry-After`.

## Tests

49 backend tests: unit tests plus integration tests against real PostgreSQL with a fake Lightning node. They cover payment creation, settlement verification, invalid input, duplicate and concurrent settlement events, expiry, underpayment, per-merchant isolation, auth, rate limits, receipts and signature forgery. The full flow has also been run end to end with real regtest Lightning payments.

```bash
docker compose up -d postgres
cd backend && set -a && . ../.env && set +a && cargo test
cd frontend && npm run lint && npm run build
```

CI (GitHub Actions) runs `cargo fmt`, `clippy -D warnings`, the backend tests, and the frontend lint and build.

## Project structure

```
backend/     Rust API: config, handlers, services (payments, receipts, watcher),
             repositories (SQL), lightning (LND client), nostr, middleware (auth, rate limits)
frontend/    Next.js app: dashboard, receive, payment status, history, receipts, verify
scripts/     Polar helpers: polar-setup.sh, pay-invoice.sh, demo-reset.sh
docs/        architecture, security, setup, demo guide
```

## Scope and next steps

Built as a hackathon MVP on regtest. Next steps: local-currency display (KES, NGN and others), hosted onboarding for merchant nodes, an offline-friendly mobile app, and **Soko AI**, an assistant that answers "How much did I sell today?" from stored records only. It will never be used to verify payments.

## License

MIT
