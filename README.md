# Soko

**Every market. Every payment.**

Soko helps small businesses and market traders accept Bitcoin over the Lightning Network, verify each payment directly with their own Lightning node, issue digital receipts, and keep simple business records.

> Status: early development. Milestone 1 (foundation) is complete. See [docs/architecture.md](docs/architecture.md).

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

If `lightning` shows `starting`, LND is still syncing. In Polar, mine a block (or run `bitcoin-cli -regtest generatetoaddress 1 <addr>` on the bitcoind node).

## Tests

```bash
cd backend && cargo test
cd frontend && npm run lint
```

## Security

Soko is non-custodial: it connects to the merchant's own LND node using a macaroon that can create and read invoices but **cannot spend funds**. Payment status is decided only by the backend after asking LND, never by the browser. See [docs/architecture.md](docs/architecture.md).

## License

MIT
