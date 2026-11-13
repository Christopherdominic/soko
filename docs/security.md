# Soko security model

Reviewed in Milestone 4 (security pass). Each mitigation listed as tested has an automated test in `backend/tests/`.

## Assets

| Asset | Where it lives | Impact if compromised |
|---|---|---|
| Merchant funds | Merchant's LND node | Theft. **Soko has no ability to spend.** |
| LND macaroon (invoice-only) | Backend env / file | Attacker can create and read invoices, but cannot send funds |
| Payment records and receipts | PostgreSQL | Privacy leak, forged business records |
| Merchant passwords | PostgreSQL (Argon2id hashes) | Account takeover if weak passwords are cracked |
| Session tokens | Browser cookie; SHA-256 hash in DB | Account takeover while the session is valid |

## Trust boundaries and assumptions

- **LND is the source of truth.** A payment becomes `paid` only when the backend's own LND query or subscription reports `SETTLED` with the expected amount. The browser has no way to change payment status: no route accepts a status, and unknown JSON fields are ignored.
- **The browser is untrusted.** Every request is validated server-side (amount range, description length and control characters, hash format, JSON content type).
- **The LND connection is authenticated both ways.** Soko pins LND's exact TLS certificate, and LND checks Soko's macaroon.
- The host running the backend is trusted with the invoice macaroon and database credentials.

## Threats and mitigations

| Threat | Mitigation |
|---|---|
| Fake "paid" status sent from the client | Status is computed only from LND data (`services/payments.rs`). Covered by test `client_cannot_set_payment_status`. |
| Replay or duplicate settlement events | Row lock plus `UPDATE … WHERE status='pending'` plus `UNIQUE(receipts.payment_id)`. Tests cover sequential and 8-way concurrent duplicates. |
| Underpaid or tampered invoice | Settlement requires `amt_paid_msat ≥ amount × 1000` and node invoice value = stored amount; otherwise `failed`. |
| Missed updates (restart, dropped stream) | Reconciliation re-checks all pending payments every 15 s and after each reconnect. |
| Expired invoice paid late | LND rejects HTLCs for expired invoices; Soko marks them `expired` 30 s after expiry. |
| One merchant reading another's data | All queries are scoped by `merchant_id` from the session; other merchants' records return 404. Covered by test. |
| Session theft | `HttpOnly; SameSite=Strict` cookie, `Secure` in production; tokens are 256-bit random; only hashes are stored; logout deletes the session. |
| CSRF | `SameSite=Strict` plus mandatory `application/json` on writes, which simple cross-site forms can't send. |
| Account discovery via login | Identical error for unknown email and wrong password; dummy Argon2 check equalises timing. |
| Secrets in logs or responses | Macaroon header is marked sensitive and redacted in `Debug`; LND error bodies are never forwarded; internal errors return a generic message. |
| Forged or edited receipts | Receipts are Schnorr-signed Nostr events; editing any field breaks the event id or signature. The verify endpoint also checks the key belongs to a merchant here and the event matches the stored one. Covered by tests. |
| Receipt-signing key theft from DB | Keys are derived from `SOKO_NOSTR_SECRET` (env only); the DB stores public keys only. |
| Public receipts leaking private data | Public event and `/r/{number}` contain no payment hash, invoice, description, email or customer data (tested). The payment link is a salted commitment. Relay publishing is off. |
| Receipt-number guessing | ~50 bits of randomness per number; the public view reveals only paper-receipt fields. |
| Password guessing / credential stuffing | Login limited to 10 attempts per account per 15 min (whatever the IP) and 300/min overall; registration 20/hour; per-IP limits when behind a trusted proxy. Tested. |
| Invoice spam (filling the node with invoices) | 30 invoices per minute per merchant. Tested. |
| Abuse of public receipt/verify endpoints | Overall cap of 600/min; verify body limited to 16 KB. |
| Spoofed client IPs | Next.js passes through a client-supplied `X-Forwarded-For`, so per-IP limits are off unless `SOKO_TRUST_PROXY=true` behind a proxy that overwrites it. The limits that matter are keyed by account, merchant or globally. |
| Large or slow requests | 64 KB body limit, 30 s request timeout. |
| Clickjacking / MIME sniffing / XSS impact | CSP (`default-src 'self'`, `frame-ancestors 'none'`, `object-src 'none'`), `X-Frame-Options: DENY`, `nosniff`, `Referrer-Policy: same-origin`, `Permissions-Policy`. API responses are `Cache-Control: no-store`. |
| Container compromise | Backend runs as a non-root user and isn't published to the host; LND cert and macaroon are mounted read-only. |
| Manipulated exchange rate | The server converts local prices with its own live rate (Kraken → CoinGecko fallback, × daily FX). Any client-sent rate is ignored (tested). Rates older than 30 min are not used to price invoices; implausible BTC prices are rejected. Rates never affect payment verification, which is done in sats by LND. |
| Cash-out overdraw | Demo cash-outs lock the merchant row and check `paid sales − previous cash-outs` in one transaction; concurrent requests can't exceed the balance (tested). |
| Cash-out destination leakage | Only the last 4 digits of the phone/account number are stored or returned. |
| Mistaking demo cash-outs for real ones | Every payout row is `is_demo = true`, enforced by a database CHECK. The UI labels every cash-out screen and record DEMO. Soko's invoice-only macaroon cannot spend, so a real cash-out is impossible by design. |
| SQL injection | All values are bound parameters; sqlx 0.9 rejects SQL strings built at runtime. |
| Path injection into LND URLs | Payment hashes must match `^[0-9a-f]{64}$` before use. |

## Known gaps (to address before production)

- Rate limits are in memory, so they reset on restart and aren't shared across several backend instances.
- A per-account login limit can be triggered on purpose to lock a merchant out for up to 15 minutes. This is an accepted trade-off against password guessing.
- CSP allows `'unsafe-inline'` scripts (Next.js without nonces). Moving to nonce-based CSP would tighten this.
- Rotating `SOKO_NOSTR_SECRET` changes every merchant's public key, so previously issued receipts stop matching a known merchant. Treat it as long-lived.
- Registration is open by default (`SOKO_ALLOW_REGISTRATION=true`); disable it after setup.
- No 2FA or password reset.
- The demo uses Polar regtest only; mainnet needs HTTPS, a hardened host and backups of LND channel state (the merchant's responsibility).
