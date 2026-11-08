# Soko security model

This is a working document. It is reviewed again in Milestone 4.

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
| SQL injection | All values are bound parameters; sqlx 0.9 rejects SQL strings built at runtime. |
| Path injection into LND URLs | Payment hashes must match `^[0-9a-f]{64}$` before use. |

## Known gaps (to address before production)

- No rate limiting on login or invoice creation yet.
- Registration is open by default (`SOKO_ALLOW_REGISTRATION=true`); disable it after setup.
- No 2FA or password reset.
- The demo uses Polar regtest only; mainnet needs HTTPS, a hardened host and backups of LND channel state (the merchant's responsibility).
