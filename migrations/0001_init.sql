CREATE TABLE accounts (
  account_id   SERIAL PRIMARY KEY,
  document_number VARCHAR(30) NOT NULL UNIQUE
);

CREATE TABLE operation_types (
  operation_type_id SERIAL PRIMARY KEY,
  description VARCHAR(100) NOT NULL
);

INSERT INTO operation_types (operation_type_id, description) VALUES
  (1, 'PURCHASE'),
  (2, 'INSTALLMENT PURCHASE'),
  (3, 'WITHDRAWAL'),
  (4, 'PAYMENT');

CREATE TABLE transactions (
  transaction_id SERIAL PRIMARY KEY,
  account_id INT NOT NULL REFERENCES accounts(account_id),
  operation_type_id INT NOT NULL REFERENCES operation_types(operation_type_id),
  amount NUMERIC(12, 2) NOT NULL,
  event_date TIMESTAMP WITH TIME ZONE DEFAULT now()
);
