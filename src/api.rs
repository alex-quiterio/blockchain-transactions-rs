use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::model::{
    AccountResponse, CreateAccountRequest, ErrorResponse, NewTransaction, TransactionRequest,
    TransactionResponse,
};
use crate::server::AppState;
use crate::service::ServiceError;

/// Maps service errors onto HTTP status codes and JSON error bodies.
pub struct ApiError {
    status: StatusCode,
    body: ErrorResponse,
}

impl ApiError {
    fn validation(details: &str, field: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: ErrorResponse {
                message: "validation failed".to_string(),
                details: Some(details.to_string()),
                field: Some(field.to_string()),
            },
        }
    }
}

impl From<ServiceError> for ApiError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::InvalidOperation => Self::validation(
                "operation_type_id must be between 1 and 4",
                "operation_type_id",
            ),
            ServiceError::AccountNotFound => Self {
                status: StatusCode::NOT_FOUND,
                body: ErrorResponse {
                    message: "validation failed".to_string(),
                    details: Some("account not found".to_string()),
                    field: Some("account_id".to_string()),
                },
            },
            ServiceError::AccountAlreadyExists => Self {
                status: StatusCode::BAD_REQUEST,
                body: ErrorResponse {
                    message: "account already exists".to_string(),
                    details: None,
                    field: None,
                },
            },
            ServiceError::Internal(details) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: ErrorResponse {
                    message: "internal server error".to_string(),
                    details: Some(details),
                    field: None,
                },
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

/// Create account
#[utoipa::path(
    post,
    path = "/accounts",
    tag = "accounts",
    request_body = CreateAccountRequest,
    responses(
        (status = 201, description = "Account created", body = AccountResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
    )
)]
pub async fn create_account(
    State(state): State<AppState>,
    Json(req): Json<CreateAccountRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if req.document_number.trim().is_empty() {
        return Err(ApiError::validation(
            "document_number is required",
            "document_number",
        ));
    }

    let account = state.accounts.create_account(&req.document_number).await?;

    Ok((
        StatusCode::CREATED,
        Json(AccountResponse {
            account_id: account.account_id,
            document_number: account.document_number,
        }),
    ))
}

/// Get account by ID
#[utoipa::path(
    get,
    path = "/accounts/{accountId}",
    tag = "accounts",
    params(("accountId" = i64, Path, description = "Account ID")),
    responses(
        (status = 200, description = "Account found", body = AccountResponse),
        (status = 404, description = "Account not found", body = ErrorResponse),
    )
)]
pub async fn get_account(
    State(state): State<AppState>,
    Path(account_id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let account = state.accounts.get_account_by_id(account_id).await?;
    Ok(Json(AccountResponse {
        account_id: account.account_id,
        document_number: account.document_number,
    }))
}

/// Create transaction
#[utoipa::path(
    post,
    path = "/transactions",
    tag = "transactions",
    request_body = TransactionRequest,
    responses(
        (status = 201, description = "Transaction created", body = TransactionResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 404, description = "Account not found", body = ErrorResponse),
    )
)]
pub async fn create_transaction(
    State(state): State<AppState>,
    Json(req): Json<TransactionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let transaction = state
        .transactions
        .create_transaction(NewTransaction {
            account_id: req.account_id,
            operation_type_id: req.operation_type_id,
            amount: req.amount,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(TransactionResponse {
            transaction_id: transaction.transaction_id,
            account_id: transaction.account_id,
            operation_type_id: transaction.operation_type_id,
            amount: transaction.amount,
        }),
    ))
}
