# Setting up Soko

Soko runs against a Lightning node you control. For development and demos this is an **LND node in a Polar regtest network**: real Lightning payments with play money.

## 1. Requirements

| Tool | Version used | Needed for |
|---|---|---|
| Docker + Docker Compose | 29 / v5 | Polar, PostgreSQL, full-stack containers |
| [Polar](https://lightningpolar.com) | latest | Local regtest Bitcoin + Lightning network |
| Rust (stable) | 1.97 | Backend development (not needed for Docker-only) |
| Node.js | 22 | Frontend development (not needed for Docker-only) |

## 2. Create the Lightning network in Polar

1. Open Polar and create a network with **1 LND node** (the merchant: Soko's node), **1 Core Lightning node** (plays the customer's wallet) and 1 bitcoind node. Start it.
2. In Polar, click the LND node → **Connect** tab, and note:
   - **REST host**, e.g. `https://127.0.0.1:8082`
   - **TLS cert** file path
   - **Invoice macaroon** file path (use *invoice*, never *admin*)

The helper scripts assume Polar network 1 (`polar-n1-backend1`, `polar-n1-alice`, `polar-n1-bob`). For other names, set `POLAR_BITCOIND`, `POLAR_MERCHANT_LND` and `POLAR_CUSTOMER_CLN`.

## 3. Configure

```bash
cp .env.example .env
```

Edit `.env`:

| Variable | Value |
|---|---|
| `POSTGRES_PASSWORD` | any strong value; use the same one inside `DATABASE_URL` |
| `LND_REST_URL` | `https://localhost:<REST port>` (for running the backend on your machine) |
| `LND_REST_URL_DOCKER` | `https://host.docker.internal:<REST port>` (for the Docker full stack) |
| `LND_TLS_CERT_PATH` | path to the LND `tls.cert` |
| `LND_MACAROON_PATH` | path to `invoice.macaroon` |
| `SOKO_NOSTR_SECRET` | `openssl rand -hex 32` (keep it stable; it defines merchants' receipt keys) |

## 4. Open a channel (one time)

```bash
scripts/polar-setup.sh
```

This funds the customer node with regtest coins and opens a 1,000,000-sat channel to the merchant's LND node. It is safe to re-run.

## 5a. Run with Docker (simplest)

```bash
docker compose up -d --build
open http://localhost:3000        # or visit it in your browser
```

The backend container reaches Polar's LND through the Docker host gateway. Only the frontend port is published, and only on `127.0.0.1` unless you set `SOKO_PUBLISH_ADDR=0.0.0.0` (for example, to open it on a phone on the same Wi-Fi).

## 5b. Run for development

```bash
docker compose up -d postgres
cd backend && cargo run            # http://127.0.0.1:4000, runs migrations
cd frontend && npm install && npm run dev   # http://localhost:3000
```

## 6. Check it works

```bash
curl http://localhost:3000/api/health
# {"status":"ok","database":"ok","lightning":"ok"}
```

| Symptom | Fix |
|---|---|
| `lightning: "starting"` | LND is syncing. Mine a block in Polar (or `scripts/polar-setup.sh`, which mines one). |
| `lightning: "unavailable"` | Polar network stopped, wrong REST port, or wrong cert/macaroon path. |
| LND container restarting in Polar | Its bitcoind container is stopped. Start the network again in Polar. |
| Backend container can't reach LND | Check `LND_REST_URL_DOCKER` uses the REST port Polar shows, and that a firewall allows Docker → host. |

## 7. Tests

```bash
docker compose up -d postgres
cd backend && set -a && . ../.env && set +a && cargo test
cd frontend && npm run lint && npm run build
```

The backend tests need a PostgreSQL user allowed to create databases (the compose one is). Each test runs in its own fresh database.
