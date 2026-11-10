#!/usr/bin/env bash
# Resets Soko to a clean state for a demo (regtest only):
#   1. checks Polar is on regtest and the customer->merchant channel exists
#   2. deletes ALL Soko records in the local soko-postgres container
#   3. creates a fresh demo merchant account
# Usage: scripts/demo-reset.sh [--yes] [business name]
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/polar-env.sh

YES=false
if [ "${1:-}" = "--yes" ]; then YES=true; shift; fi
DEFAULT_BUSINESS="Mama Amina's Market Stall"
BUSINESS="${1:-$DEFAULT_BUSINESS}"
EMAIL="demo@soko.test"
PASSWORD="${SOKO_DEMO_PASSWORD:-soko-demo-$(openssl rand -hex 3)}"
APP_URL="${SOKO_APP_URL:-http://localhost:3000}"

chain=$(btc getblockchaininfo | python3 -c 'import json,sys; print(json.load(sys.stdin)["chain"])')
[ "$chain" = "regtest" ] || { echo "Refusing: bitcoind is on '$chain', not regtest." >&2; exit 1; }

if ! $YES; then
  read -r -p "This deletes ALL merchants, payments and receipts in soko-postgres. Continue? [y/N] " ans
  [ "$ans" = "y" ] || [ "$ans" = "Y" ] || { echo "Cancelled."; exit 1; }
fi

scripts/polar-setup.sh

set -a; . ./.env; set +a
docker exec soko-postgres psql -q -U "${POSTGRES_USER:-soko}" -d "${POSTGRES_DB:-soko}" \
  -c "TRUNCATE receipts, payments, sessions, merchants;"
echo "Cleared Soko records."

body=$(python3 -c 'import json,sys; print(json.dumps({"business_name": sys.argv[1], "email": sys.argv[2], "password": sys.argv[3]}))' "$BUSINESS" "$EMAIL" "$PASSWORD")
code=$(curl -s -o /dev/null -w '%{http_code}' -H 'Content-Type: application/json' -d "$body" "$APP_URL/api/auth/register")
[ "$code" = "201" ] || { echo "Could not create demo merchant (HTTP $code). Is the app running at $APP_URL?" >&2; exit 1; }

cat <<MSG

Demo ready at $APP_URL
  Business: $BUSINESS
  Email:    $EMAIL
  Password: $PASSWORD

Pay an invoice from the customer node with: scripts/pay-invoice.sh <lnbcrt...>
MSG
