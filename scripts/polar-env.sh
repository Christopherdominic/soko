# Shared settings for the Polar helper scripts. Override with env vars if your
# Polar network uses different container names.
POLAR_BITCOIND="${POLAR_BITCOIND:-polar-n1-backend1}"
POLAR_MERCHANT_LND="${POLAR_MERCHANT_LND:-polar-n1-alice}"   # Soko's node (LND)
POLAR_CUSTOMER_CLN="${POLAR_CUSTOMER_CLN:-polar-n1-bob}"     # plays the customer wallet (Core Lightning)

btc() { docker exec "$POLAR_BITCOIND" bitcoin-cli -regtest -rpcuser=polaruser -rpcpassword=polarpass "$@"; }
merchant_lncli() { docker exec "$POLAR_MERCHANT_LND" lncli --network regtest --lnddir=/home/lnd/.lnd "$@"; }
customer_cli() { docker exec "$POLAR_CUSTOMER_CLN" lightning-cli --network=regtest --lightning-dir=/home/clightning/.lightning "$@"; }
mine() { btc generatetoaddress "$1" "$(btc getnewaddress)" >/dev/null; }
