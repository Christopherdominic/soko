#!/usr/bin/env bash
# Exposes the local Polar LND REST API through ngrok so the hosted (Render) Soko
# can create invoices and verify payments. Regtest only.
# Usage: scripts/tunnel.sh            (reads NGROK_DOMAIN or LND_REST_URL from .env.render)
#        NGROK_DOMAIN=my-name.ngrok-free.dev scripts/tunnel.sh
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/polar-env.sh

LND_PORT="${LND_REST_PORT:-8082}"
DOMAIN="${NGROK_DOMAIN:-}"
if [ -z "$DOMAIN" ] && [ -f .env.render ]; then
  DOMAIN=$(grep -E '^NGROK_DOMAIN=' .env.render | cut -d= -f2- || true)
  [ -n "$DOMAIN" ] || DOMAIN=$(grep -E '^LND_REST_URL=' .env.render | cut -d= -f2- | sed -E 's#^https://##; s#/.*$##' || true)
fi
[ -n "$DOMAIN" ] || { echo "Set NGROK_DOMAIN (your static ngrok domain)." >&2; exit 1; }

chain=$(btc getblockchaininfo | python3 -c 'import json,sys; print(json.load(sys.stdin)["chain"])')
[ "$chain" = "regtest" ] || { echo "Refusing: bitcoind is on '$chain', not regtest." >&2; exit 1; }
curl -sk -m 5 "https://localhost:$LND_PORT/v1/state" | grep -q SERVER_ACTIVE \
  || { echo "LND is not ready on https://localhost:$LND_PORT. Start Polar and run scripts/polar-setup.sh." >&2; exit 1; }

echo "Tunnel: https://$DOMAIN -> https://localhost:$LND_PORT (Ctrl+C to stop)"
exec ngrok http "https://localhost:$LND_PORT" --url "https://$DOMAIN"
