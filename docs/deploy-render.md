# Deploying Soko on Render (with a Voltage Mutinynet node)

This hosts Soko publicly. **Render** runs the website, the backend and the database. **Voltage** runs the merchant's LND node on **Mutinynet**, a public Bitcoin test network: the payments are real Lightning payments with test coins, and anyone can pay from a Mutinynet-compatible wallet.

Time needed: about 30–45 minutes, most of it waiting for builds and channel confirmations.

## Part 1: Lightning node on Voltage

1. Sign up at [voltage.cloud](https://voltage.cloud) and create a **Lightning node** (LND) on the **Mutinynet** network. Mutinynet nodes are available on the free plan.
2. Wait until the node shows as **running and synced**.
3. Note two things from the node's dashboard:
   - **API endpoint (REST).** It looks like `https://<your-node>.u.voltageapp.io:8080`. Use the exact host and port the dashboard shows.
   - **Invoice macaroon, in hex.** In the node's connect / macaroon section, choose the *invoice* macaroon. Do **not** use the admin macaroon: Soko only needs to create and read invoices, and the invoice macaroon cannot spend funds.
4. If Voltage has an **IP allowlist** setting for API access, allow all addresses (or Render's outbound IPs). Otherwise Render can't connect.
5. **Give the node inbound liquidity**, so it can *receive* payments. A new node has no channels. On Mutinynet, open a channel *to* your node from the [Mutinynet faucet](https://faucet.mutinynet.com) (it can open a channel to a node pubkey and push sats to it), or from any Mutinynet wallet. Wait for the channel to confirm (Mutinynet blocks are about 30 seconds).

> Check: Voltage's dashboard should show an active channel with *inbound* (receivable) balance.

## Part 2: Soko on Render

1. Sign up at [render.com](https://render.com) and connect your GitHub account.
2. **New → Blueprint**, then choose the `soko` repository. Render reads [`render.yaml`](../render.yaml) and proposes three resources: `soko-db` (Postgres), `soko-backend` and `soko-frontend`.
3. Render asks for the values marked `sync: false`:

   | Variable | Value |
   |---|---|
   | `LND_REST_URL` | Voltage REST endpoint from Part 1, e.g. `https://<your-node>.u.voltageapp.io:8080` |
   | `LND_MACAROON_HEX` | the **invoice** macaroon (hex) |
   | `SOKO_BACKEND_URL` | `https://soko-backend.onrender.com`, the backend's public URL. If Render gives the backend a different URL (shown on its page after creation), set that and redeploy the frontend. |

   `SOKO_NOSTR_SECRET` is generated automatically. **Never change it** after launch: it defines every merchant's receipt-signing key.
4. Click **Apply**. The first build takes a while (the Rust backend compiles in about 10–15 minutes).
5. When both services are live, open `https://<backend-url>/api/health`. You should see:
   ```json
   {"status":"ok","database":"ok","lightning":"ok"}
   ```
6. Open the frontend URL, **create an account**, and take a test payment (Part 3).
7. Optional: after you've created your merchant account, set `SOKO_ALLOW_REGISTRATION=false` on `soko-backend` so strangers can't sign up.

## Part 3: Taking a test payment

1. In Soko: **Receive payment**, enter an amount, then **Show QR code**.
2. Pay the invoice with Mutinynet test coins: from a Mutinynet-compatible Lightning wallet, or by pasting the payment request into a Mutinynet tool that can pay invoices.
3. The screen turns green when Soko has verified the settlement with the Voltage node.

## Things to know about the free plan

| Limit | Effect on Soko | Workaround |
|---|---|---|
| Free web services sleep after 15 min without traffic, and take about a minute to wake | The first visit is slow. While asleep, the background payment watcher doesn't run; payments are still verified as soon as someone opens them, and on wake-up. | Open the site a few minutes before a demo, or move `soko-backend` to the Starter plan. |
| Free Postgres expires 30 days after creation | Records are lost after that unless upgraded | Upgrade the database, or recreate it after judging. |
| Rate limits are in memory | Reset when the service sleeps or redeploys | Fine for a demo. |

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| Health shows `"lightning":"unavailable"` | Wrong `LND_REST_URL` or port, wrong macaroon, Voltage IP allowlist, or the node is stopped. Check the `soko-backend` logs on Render. |
| Invoices are created but payments fail with "no route" | The Voltage node has no inbound liquidity (Part 1, step 5). |
| Frontend loads but every API call fails | `SOKO_BACKEND_URL` is wrong. It's baked in at build time, so fix it and **redeploy the frontend**. |
| Backend fails to start with a config error | The log names the missing or invalid variable. |

## Security notes for the hosted version

- The backend is reachable on its own public URL, which is required on Render's free plan. It serves the same API as through the frontend, with the same authentication and rate limits.
- Session cookies are `Secure` (`SOKO_COOKIE_SECURE=true`); Render serves everything over HTTPS.
- Voltage's TLS certificate is verified against public certificate authorities. Local Polar setups still pin LND's self-signed certificate.
