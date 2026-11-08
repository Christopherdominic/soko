-- Public Nostr identity of each merchant (x-only pubkey hex). The private key is
-- derived from a server secret and never stored.
ALTER TABLE merchants ADD COLUMN nostr_pubkey TEXT UNIQUE
    CHECK (nostr_pubkey IS NULL OR nostr_pubkey ~ '^[0-9a-f]{64}$');

-- Signed receipt event and the private salt for its payment commitment.
-- Both are set together, once, the first time the receipt is viewed.
ALTER TABLE receipts
    ADD COLUMN commitment_salt BYTEA CHECK (commitment_salt IS NULL OR length(commitment_salt) = 32),
    ADD COLUMN nostr_event JSONB,
    ADD CONSTRAINT receipts_nostr_complete CHECK ((commitment_salt IS NULL) = (nostr_event IS NULL));
