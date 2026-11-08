-- Core schema: merchants and the Lightning payments they receive.

CREATE TABLE merchants (
    id            UUID PRIMARY KEY,
    business_name TEXT NOT NULL CHECK (length(business_name) BETWEEN 1 AND 120),
    email         TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX merchants_email_lower_idx ON merchants (lower(email));

CREATE TABLE payments (
    id              UUID PRIMARY KEY,
    merchant_id     UUID NOT NULL REFERENCES merchants (id) ON DELETE RESTRICT,
    -- Hex-encoded SHA-256 payment hash from LND. Globally unique per invoice.
    payment_hash    TEXT NOT NULL UNIQUE CHECK (payment_hash ~ '^[0-9a-f]{64}$'),
    payment_request TEXT NOT NULL,
    amount_sats     BIGINT NOT NULL CHECK (amount_sats > 0),
    description     TEXT CHECK (description IS NULL OR length(description) <= 200),
    -- Only the backend's LND verification path may move a payment out of 'pending'.
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'paid', 'expired', 'failed')),
    amount_paid_msat BIGINT,
    -- Demo rows are never created by the payment flow; they exist only for UI previews.
    is_demo         BOOLEAN NOT NULL DEFAULT false,
    expires_at      TIMESTAMPTZ NOT NULL,
    settled_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((status = 'paid') = (settled_at IS NOT NULL))
);

CREATE INDEX payments_merchant_created_idx ON payments (merchant_id, created_at DESC);
CREATE INDEX payments_pending_idx ON payments (status) WHERE status = 'pending';
