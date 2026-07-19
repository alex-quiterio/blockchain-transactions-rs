# Customer Transactions API (Rust)

A Rust port of [reneelouise/customer-transactions](https://github.com/reneelouise/customer-transactions) — an API for managing customer accounts and transactions. Built with [axum](https://github.com/tokio-rs/axum), [sqlx](https://github.com/launchbadge/sqlx), and PostgreSQL, with OpenAPI/Swagger documentation.

---

## Features

- Create and retrieve customer accounts
- Record transactions (purchases, payments, withdrawals)
- Enforces transaction rules (debits stored as negative amounts, payments as positive)
- Swagger documentation for all endpoints
- Migrations run automatically on startup (no external migrate CLI needed)
- Dockerized database and API for easy local development

## Differences from the Go original

- Migrations are embedded and applied automatically at startup via `sqlx::migrate!`.
- Error responses are structured JSON (`{"message", "details", "field"}`) matching the documented DTOs, instead of plain text.
- `POST /transactions` responses include the real `transaction_id`.

---

## Requirements

- Rust (rustup will pick up the pinned toolchain from `rust-toolchain.toml` automatically)
- Docker & Docker Compose

---

## Getting Started

### 1. Run PostgreSQL with Docker

```bash
docker compose up -d db
```

This will create:

- **User**: `customer`
- **Password**: `secret`
- **Database**: `transactions`
- **Port**: `5432` mapped to localhost

### 2. Run the API server locally

```bash
cargo run
```

The server starts on `http://localhost:8080` and applies the migrations in `migrations/` automatically (schema + the 4 seeded operation types: PURCHASE, INSTALLMENT PURCHASE, WITHDRAWAL, PAYMENT).

Swagger docs are available at:

```
http://localhost:8080/swagger
```

The raw OpenAPI document is served at `http://localhost:8080/swagger/doc.json`.

### 3. Run API + database via Docker Compose

```bash
docker compose up --build
```

---

## Environment Variables

**DATABASE_DSN**: Optional. Defaults to:

```
postgres://customer:secret@localhost:5432/transactions?sslmode=disable
```

---

## Project Structure

```
src/main.rs        # Entrypoint: config, DB pool, migrations, graceful shutdown
src/server.rs      # Router setup, app state, OpenAPI doc
src/api.rs         # HTTP handlers and error → status-code mapping
src/service.rs     # Business logic (+ unit tests with mock repositories)
src/repository.rs  # Repository traits and the Postgres (sqlx) implementation
src/model.rs       # Domain models and request/response DTOs
tests/api_tests.rs # HTTP-level tests against the router with mock repositories
migrations/        # Database migration files (applied automatically on startup)
```

---

## Linting & Formatting

```bash
cargo clippy --all-targets
cargo fmt
```

---

## Running Tests

```bash
cargo test
```

Tests use mock repositories, so no database is required.

---

## Example Requests

### Create Account

```http
POST /accounts
Content-Type: application/json

{
    "document_number": "12345678900"
}
```

### Get Account

```http
GET /accounts/{accountId}
```

### Create Transaction

```http
POST /transactions
Content-Type: application/json

{
    "account_id": 1,
    "operation_type_id": 1,
    "amount": 100.00
}
```

Operation types 1–3 (purchase, installment purchase, withdrawal) are stored as negative amounts; type 4 (payment) is stored as positive.

---

Rust port of the original Go project by Renee-Louise Nzegbulem.
