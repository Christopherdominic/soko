# Soko demo guide

A 3–4 minute live demo of the full flow: create invoice → customer pays over Lightning → Soko verifies with LND → signed receipt. Everything runs on regtest, so the payments are real Lightning payments with play money.

## Before you start (5 minutes)

1. Start the Polar network (LND + Core Lightning + bitcoind).
2. Start Soko: `docker compose up -d --build` (or the dev servers).
3. Reset to a clean state and create the demo merchant:
   ```bash
   scripts/demo-reset.sh "Mama Amina's Market Stall"
   ```
   Note the printed email and password.
4. Check `http://localhost:3000/api/health` shows everything `ok`.
5. Log in once, then make the browser window phone-sized (DevTools device mode, about 390px wide), or open it on a phone with `SOKO_PUBLISH_ADDR=0.0.0.0`.
6. Keep a terminal ready in the repo root for `scripts/pay-invoice.sh`.

## The demo, step by step

| # | Show | Say |
|---|---|---|
| 1 | Dashboard (empty) | "This is Amina's stall. She sells vegetables. No crypto knowledge needed: one big button to get paid." |
| 2 | Tap **Receive payment**, enter `2500`, description `Sukuma wiki & tomatoes`, tap **Show QR code** | "She types the price. Soko asks *her own* Lightning node for an invoice; Soko never holds her money." |
| 3 | The QR screen with **Waiting** | "The customer scans this with any Lightning wallet." |
| 4 | In the terminal: copy the request (**Copy payment request**) and run `scripts/pay-invoice.sh <paste>` | "Here a second Lightning node plays the customer's phone wallet. This is a real Lightning payment over a real channel." |
| 5 | Screen turns green: **Payment received** | "The browser didn't decide this. The backend heard from Amina's LND node that the invoice settled, checked the amount, and only then marked it paid." |
| 6 | Tap **View & share receipt** | "Every payment gets a receipt automatically." |
| 7 | Tap **Share receipt** and open the link in a private window | "The customer's copy shows only what's on a paper receipt. No description, no payment details." |
| 8 | Point at **Genuine receipt, digitally signed** | "It's signed with Amina's Nostr key. Change one digit and the signature breaks." |
| 9 | (Optional) Download the receipt, edit the amount, paste it into `/verify` | Shows **Could not verify this receipt**. |
| 10 | Back to the dashboard, then **See all** and search | "Today's sales, history, search by item or receipt number: the records she needs for her business." |

### Backup moves if something goes wrong

- **Payment doesn't turn green:** run `scripts/pay-invoice.sh` again with the full request (no line breaks). Reconciliation also re-checks every 15 seconds.
- **"Lightning node offline":** Polar network stopped. Start it, then mine a block.
- **No route / insufficient balance:** run `scripts/polar-setup.sh`.

## Recording the video (≈3 minutes)

1. **0:00–0:20 Problem.** Market traders lose sales to cash shortages and expensive mobile-money fees, and have no records.
2. **0:20–0:35 Solution.** Soko: accept Bitcoin over Lightning in seconds, verified by the merchant's own node, with a digital receipt.
3. **0:35–2:15 Live flow.** Steps 2–8 above, with a phone-sized window and the terminal side by side.
4. **2:15–2:40 Records.** Dashboard totals and history search.
5. **2:40–3:00 Trust and architecture.** One slide: non-custodial, invoice-only macaroon (can't spend), verification only from LND, idempotent processing, privacy-preserving Nostr receipts, 49 automated tests.

Tips: record at 1080p, enlarge the terminal font, and do one dry run right after `demo-reset.sh`.

## Presentation outline (5 slides)

1. **The problem.** Small traders need cheap, instant payments and simple records.
2. **Soko.** "Every market. Every payment." The 3-tap flow, with screenshots.
3. **How it works.** Architecture diagram (browser → Next.js → Rust/Axum → PostgreSQL; Rust ↔ merchant's LND). Verification comes only from LND.
4. **Trust and safety.**
   - Non-custodial; Soko's credential cannot spend funds.
   - No client-side "paid" status; duplicate and concurrent events are safe.
   - Receipts are signed Nostr events without payment secrets or customer data.
   - Rate limits, Argon2 passwords, `HttpOnly` sessions, per-merchant isolation.
5. **What's next.** Local-currency display (KES/NGN), offline-friendly mobile app, hosted onboarding for merchant nodes, and Soko AI sales summaries from stored records.

## Honest scope statement (for judges)

- Runs on Polar **regtest**; mainnet would need HTTPS, a hardened host and the merchant's own node operations (channel liquidity, backups).
- One Soko deployment connects to one merchant's LND node.
- Amounts are in sats; there is no fiat conversion yet.
- Receipts are signed but not auto-published to Nostr relays (on purpose, for merchant privacy).
