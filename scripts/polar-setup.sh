#!/usr/bin/env bash
# Regtest only: funds the customer node and opens a channel from it to the
# merchant's LND node so the customer can pay Soko invoices. Safe to re-run.
set -euo pipefail
. "$(dirname "$0")/polar-env.sh"

CHANNEL_SATS="${CHANNEL_SATS:-1000000}"

chain=$(btc getblockchaininfo | python3 -c 'import json,sys; print(json.load(sys.stdin)["chain"])')
[ "$chain" = "regtest" ] || { echo "Refusing to run: bitcoind is on '$chain', not regtest." >&2; exit 1; }

mine 1   # keeps LND's view of the chain fresh
alice_pub=$(merchant_lncli getinfo | python3 -c 'import json,sys; print(json.load(sys.stdin)["identity_pubkey"])')
echo "Merchant node: $alice_pub"

has_channel=$(customer_cli listpeerchannels | python3 -c "
import json,sys
chs=json.load(sys.stdin)['channels']
print(any(c['peer_id']=='$alice_pub' and c['state'] in ('CHANNELD_NORMAL','CHANNELD_AWAITING_LOCKIN') for c in chs))")
if [ "$has_channel" = "True" ]; then
  echo "Channel from customer to merchant already exists."
  # After a restart the nodes may not reconnect on their own.
  customer_cli connect "$alice_pub" alice 9735 >/dev/null 2>&1 && echo "Customer node connected to merchant node."
else
  funds=$(customer_cli listfunds | python3 -c 'import json,sys; print(sum(o["amount_msat"] for o in json.load(sys.stdin)["outputs"] if o["status"]=="confirmed")//1000)')
  if [ "$funds" -lt $((CHANNEL_SATS + 100000)) ]; then
    addr=$(customer_cli newaddr | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("bech32") or d.get("p2tr"))')
    echo "Funding customer node with 0.05 BTC (regtest)…"
    btc sendtoaddress "$addr" 0.05 >/dev/null
    mine 6
    for _ in $(seq 1 30); do
      funds=$(customer_cli listfunds | python3 -c 'import json,sys; print(sum(o["amount_msat"] for o in json.load(sys.stdin)["outputs"] if o["status"]=="confirmed")//1000)')
      [ "$funds" -ge "$CHANNEL_SATS" ] && break; sleep 1
    done
  fi
  echo "Connecting and opening a $CHANNEL_SATS sat channel…"
  customer_cli connect "$alice_pub" alice 9735 >/dev/null
  customer_cli fundchannel "$alice_pub" "$CHANNEL_SATS" >/dev/null
  mine 6
fi

for _ in $(seq 1 30); do
  active=$(merchant_lncli listchannels --active_only | python3 -c 'import json,sys; print(len(json.load(sys.stdin)["channels"]))')
  [ "$active" -ge 1 ] && break; sleep 2
done
merchant_lncli listchannels | python3 -c '
import json,sys
for c in json.load(sys.stdin)["channels"]:
    print("Merchant channel: capacity %s sats, merchant can receive %s sats, active=%s" % (c["capacity"], c["remote_balance"], c["active"]))'
