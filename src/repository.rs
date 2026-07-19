use async_trait::async_trait;
use sqlx::PgPool;

use crate::model::{Account, NewTransaction, Transaction};

#[derive(Debug)]
pub enum RepoError {
    NotFound,
    Database(sqlx::Error),
}

impl std::fmt::Display for RepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoError::NotFound => write!(f, "not found"),
            RepoError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for RepoError {}

impl From<sqlx::Error> for RepoError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => RepoError::NotFound,
            other => RepoError::Database(other),
        }
    }
}

#[async_trait]
pub trait AccountRepo: Send + Sync {
    async fn create_account(&self, document_number: &str) -> Result<Account, RepoError>;
    async fn get_by_id(&self, id: i64) -> Result<Account, RepoError>;
    async fn get_by_document_number(&self, doc: &str) -> Result<Account, RepoError>;
}

#[async_trait]
pub trait TransactionRepo: Send + Sync {
    async fn create_transaction(&self, t: NewTransaction) -> Result<Transaction, RepoError>;
}

/// Postgres implementation of both repositories, backed by a sqlx pool.
#[derive(Clone)]
pub struct PgRepo {
    pool: PgPool,
}

impl PgRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AccountRepo for PgRepo {
    async fn create_account(&self, document_number: &str) -> Result<Account, RepoError> {
        let account = sqlx::query_as::<_, Account>(
            "INSERT INTO accounts (document_number) VALUES ($1)
             RETURNING account_id::int8, document_number",
        )
        .bind(document_number)
        .fetch_one(&self.pool)
        .await?;
        Ok(account)
    }

    async fn get_by_id(&self, id: i64) -> Result<Account, RepoError> {
        let account = sqlx::query_as::<_, Account>(
            "SELECT account_id::int8, document_number FROM accounts WHERE account_id = $1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(account)
    }

    async fn get_by_document_number(&self, doc: &str) -> Result<Account, RepoError> {
        let account = sqlx::query_as::<_, Account>(
            "SELECT account_id::int8, document_number FROM accounts WHERE document_number = $1",
        )
        .bind(doc)
        .fetch_one(&self.pool)
        .await?;
        Ok(account)
    }
}

#[async_trait]
impl TransactionRepo for PgRepo {
    async fn create_transaction(&self, t: NewTransaction) -> Result<Transaction, RepoError> {
        let row: (i64, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
            "INSERT INTO transactions (account_id, operation_type_id, amount, event_date)
             VALUES ($1, $2, $3, now())
             RETURNING transaction_id::int8, date_trunc('second', event_date)",
        )
        .bind(t.account_id)
        .bind(t.operation_type_id)
        .bind(t.amount)
        .fetch_one(&self.pool)
        .await?;

        Ok(Transaction {
            transaction_id: row.0,
            account_id: t.account_id,
            operation_type_id: t.operation_type_id,
            amount: t.amount,
            event_date: row.1,
        })
    }
}
