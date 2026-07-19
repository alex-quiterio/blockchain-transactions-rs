use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ----------------------
// Domain models
// ----------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ToSchema)]
pub struct Account {
    #[schema(example = 1)]
    pub account_id: i64,
    #[schema(example = "12345678900")]
    pub document_number: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Transaction {
    pub transaction_id: i64,
    pub account_id: i64,
    pub operation_type_id: i32,
    pub amount: f64,
    pub event_date: DateTime<Utc>,
}

/// A transaction as submitted by the client, before it is persisted.
#[derive(Debug, Clone)]
pub struct NewTransaction {
    pub account_id: i64,
    pub operation_type_id: i32,
    pub amount: f64,
}

// ----------------------
// Account DTOs
// ----------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    #[schema(example = "12345678900")]
    pub document_number: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AccountResponse {
    #[schema(example = 1)]
    pub account_id: i64,
    #[schema(example = "12345678900")]
    pub document_number: String,
}

// ----------------------
// Transaction DTOs
// ----------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct TransactionRequest {
    #[schema(example = 1)]
    pub account_id: i64,
    #[schema(example = 1)]
    pub operation_type_id: i32,
    #[schema(example = 50.75)]
    pub amount: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TransactionResponse {
    #[schema(example = 1)]
    pub transaction_id: i64,
    #[schema(example = 1)]
    pub account_id: i64,
    #[schema(example = 4)]
    pub operation_type_id: i32,
    #[schema(example = 123.45)]
    pub amount: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BalanceResponse {
    #[schema(example = 1)]
    pub account_id: i64,
    #[schema(example = 123.45)]
    pub balance: f64,
}

// ----------------------
// Generic error response
// ----------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    #[schema(example = "validation failed")]
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "operation_type_id must be between 1 and 4")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "operation_type_id")]
    pub field: Option<String>,
}
