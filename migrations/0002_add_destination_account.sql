-- destination_account_id names the counterparty of a transaction for
-- reconciliation purposes. It is intentionally NOT a foreign key: the
-- destination does not need to be an account that exists in this database.
ALTER TABLE transactions
  ADD COLUMN destination_account_id BIGINT NOT NULL DEFAULT 0;

ALTER TABLE transactions
  ALTER COLUMN destination_account_id DROP DEFAULT;

CREATE INDEX idx_transactions_destination_account_id
  ON transactions (destination_account_id);
