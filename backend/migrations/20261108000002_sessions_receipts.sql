-- Login sessions. Only a SHA-256 hash of the session token is stored, so a database
-- leak does not reveal usable session cookies.
CREATE TABLE sessions (
    token_hash  BYTEA PRIMARY KEY,
    merchant_id UUID NOT NULL REFERENCES merchants (id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ NOT NULL
);

CREATE INDEX sessions_merchant_idx ON sessions (merchant_id);

-- One receipt per paid payment. The UNIQUE constraint on payment_id makes receipt
-- issuance idempotent even if settlement is processed more than once.
CREATE TABLE receipts (
    id             UUID PRIMARY KEY,
    receipt_number TEXT NOT NULL UNIQUE,
    payment_id     UUID NOT NULL UNIQUE REFERENCES payments (id) ON DELETE RESTRICT,
    merchant_id    UUID NOT NULL REFERENCES merchants (id) ON DELETE RESTRICT,
    amount_sats    BIGINT NOT NULL CHECK (amount_sats > 0),
    issued_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX receipts_merchant_issued_idx ON receipts (merchant_id, issued_at DESC);
