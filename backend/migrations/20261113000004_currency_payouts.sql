-- Merchant's local currency (ISO 4217) for prices, totals and cash-out.
ALTER TABLE merchants
    ADD COLUMN currency TEXT NOT NULL DEFAULT 'KES' CHECK (currency ~ '^[A-Z]{3}$');

-- Local-currency value of each payment at creation time, for records and receipts.
-- Amounts are in minor units of the currency (e.g. cents); the rate is the price of
-- 1 BTC in that currency at the moment the invoice was created.
ALTER TABLE payments
    ADD COLUMN fiat_currency TEXT CHECK (fiat_currency IS NULL OR fiat_currency ~ '^[A-Z]{3}$'),
    ADD COLUMN fiat_amount_minor BIGINT CHECK (fiat_amount_minor IS NULL OR fiat_amount_minor >= 0),
    ADD COLUMN btc_price_fiat DOUBLE PRECISION CHECK (btc_price_fiat IS NULL OR btc_price_fiat > 0),
    ADD CONSTRAINT payments_fiat_complete CHECK (
        (fiat_currency IS NULL) = (fiat_amount_minor IS NULL)
        AND (fiat_currency IS NULL) = (btc_price_fiat IS NULL)
    );

-- Cash-outs to mobile money / bank. DEMO ONLY: no money moves. The CHECK makes it
-- impossible to store anything that looks like a real payout.
CREATE TABLE payouts (
    id                 UUID PRIMARY KEY,
    merchant_id        UUID NOT NULL REFERENCES merchants (id) ON DELETE CASCADE,
    amount_sats        BIGINT NOT NULL CHECK (amount_sats > 0),
    currency           TEXT NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    fiat_amount_minor  BIGINT NOT NULL CHECK (fiat_amount_minor >= 0),
    btc_price_fiat     DOUBLE PRECISION NOT NULL CHECK (btc_price_fiat > 0),
    provider           TEXT NOT NULL CHECK (length(provider) BETWEEN 1 AND 60),
    destination_masked TEXT NOT NULL CHECK (length(destination_masked) BETWEEN 1 AND 40),
    status             TEXT NOT NULL DEFAULT 'completed' CHECK (status IN ('completed')),
    is_demo            BOOLEAN NOT NULL DEFAULT true CHECK (is_demo),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX payouts_merchant_created_idx ON payouts (merchant_id, created_at DESC);
