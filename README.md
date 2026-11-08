# Soko

**Every market. Every payment.**

Soko helps small businesses and market traders accept Bitcoin over the Lightning Network, verify each payment directly with their own Lightning node, issue digital receipts, and keep simple business records.

> Status: Milestones 1–2 complete. A merchant can log in, create a Lightning invoice, show it as a QR code, and see it marked paid once their own LND node confirms settlement, with a receipt issued. See [docs/architecture.md](docs/architecture.md).

## Stack

- Backend: Rust, Axum, Tokio, SQLx, PostgreSQL
- Frontend: Next.js, TypeScript, Tailwind CSS
- Lightning: LND (REST), Polar regtest for development

## Prerequisites

- Rust (stable), Node.js 20+, Docker
- [Polar](https://lightningpolar.com) with a network containing an **LND** node, started

## Run locally

```bash
# 1. Configure
cp .env.example .env
#    Set POSTGRES_PASSWORD (and the same value in DATABASE_URL).
#    Point LND_REST_URL, LND_TLS_CERT_PATH and LND_MACAROON_PATH at your Polar LND node.
#    Polar shows these under the node's "Connect" tab. Use invoice.macaroon, not admin.

# 2. Database
docker compose up -d

# 3. Backend (runs migrations on start) — http://127.0.0.1:4000
cd backend && cargo run

# 4. Frontend — http://localhost:3000
cd frontend && npm install && npm run dev
```

Check it is working:

```bash
curl http://localhost:3000/api/health
# {"status":"ok","database":"ok","lightning":"ok"}
```

### Make a test payment (Polar regtest)

```bash
# One-time: fund the customer node (bob) and open a channel to the merchant's LND (alice)
scripts/polar-setup.sh

# In the app: register, tap "Receive payment", enter an amount, then copy the payment request and:
scripts/pay-invoice.sh <lnbcrt...>
```

The payment screen switches to **Payment received** after Soko verifies the settlement with LND. Container names default to Polar network 1 (`polar-n1-*`). Override them with `POLAR_BITCOIND`, `POLAR_MERCHANT_LND` and `POLAR_CUSTOMER_CLN`.

If `lightning` shows `starting`, LND is still syncing. In Polar, mine a block (or run `bitcoin-cli -regtest generatetoaddress 1 <addr>` on the bitcoind node).

## Tests

Backend integration tests use a real PostgreSQL (each test gets a fresh database) and a fake Lightning node.

```bash
docker compose up -d
cd backend && set -a && . ../.env && set +a && cargo test
cd frontend && npm run lint && npm run build
```

## Security

Soko is non-custodial: it connects to the merchant's own LND node using a macaroon that can create and read invoices but **cannot spend funds**. Payment status is decided only by the backend after asking LND, never by the browser. See [docs/security.md](docs/security.md) for the threat model.

## API

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET | `/api/health` | – | Database and LND status |
| POST | `/api/auth/register` | – | Create merchant account |
| POST | `/api/auth/login` | – | Log in (sets session cookie) |
| POST | `/api/auth/logout` | ✓ | End session |
| GET | `/api/auth/me` | ✓ | Current merchant |
| POST | `/api/payments/invoices` | ✓ | Create invoice `{amount_sats, description?}` |
| GET | `/api/payments` | ✓ | List payments `?status=&limit=&offset=` |
| GET | `/api/payments/{payment_hash}` | ✓ | Payment status (re-verified with LND if pending) |
| GET | `/api/payments/{payment_hash}/receipt` | ✓ | Receipt for a paid payment |

## License

MIT
