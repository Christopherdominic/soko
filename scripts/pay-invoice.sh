#!/usr/bin/env bash
# Regtest only: pays a Lightning invoice from the customer node, like a
# customer's wallet would. Usage: scripts/pay-invoice.sh <lnbcrt...>
set -euo pipefail
. "$(dirname "$0")/polar-env.sh"
[ $# -eq 1 ] || { echo "usage: $0 <payment_request>" >&2; exit 1; }
customer_cli pay "$1" | python3 -c 'import json,sys; d=json.load(sys.stdin); print("status=%s amount_sent_msat=%s" % (d["status"], d["amount_sent_msat"]))'
