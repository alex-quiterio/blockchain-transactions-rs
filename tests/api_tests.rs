use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use chrono::Utc;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use customer_transactions::model::{Account, NewTransaction, Transaction};
use customer_transactions::repository::{AccountRepo, RepoError, TransactionRepo};
use customer_transactions::server::{router, AppState};

struct MockRepo {
    accounts: Mutex<Vec<Account>>,
    transactions: Mutex<Vec<Transaction>>,
}

impl MockRepo {
    fn empty() -> Arc<Self> {
        Arc::new(Self {
            accounts: Mutex::new(vec![]),
            transactions: Mutex::new(vec![]),
        })
    }

    fn with_account(id: i64, doc: &str) -> Arc<Self> {
        let repo = Self::empty();
        repo.accounts.lock().unwrap().push(Account {
            account_id: id,
            document_number: doc.to_string(),
        });
        repo
    }
}

#[async_trait]
impl AccountRepo for MockRepo {
    async fn create_account(&self, document_number: &str) -> Result<Account, RepoError> {
        let mut accounts = self.accounts.lock().unwrap();
        let account = Account {
            account_id: accounts.len() as i64 + 1,
            document_number: document_number.to_string(),
        };
        accounts.push(account.clone());
        Ok(account)
    }

    async fn get_by_id(&self, id: i64) -> Result<Account, RepoError> {
        self.accounts
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.account_id == id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    async fn get_by_document_number(&self, doc: &str) -> Result<Account, RepoError> {
        self.accounts
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.document_number == doc)
            .cloned()
            .ok_or(RepoError::NotFound)
    }
}

#[async_trait]
impl TransactionRepo for MockRepo {
    async fn create_transaction(&self, t: NewTransaction) -> Result<Transaction, RepoError> {
        let mut transactions = self.transactions.lock().unwrap();
        let tx = Transaction {
            transaction_id: transactions.len() as i64 + 1,
            account_id: t.account_id,
            operation_type_id: t.operation_type_id,
            amount: t.amount,
            event_date: Utc::now(),
        };
        transactions.push(tx.clone());
        Ok(tx)
    }

    async fn get_balance_by_account(&self, account_id: i64) -> Result<f64, RepoError> {
        Ok(self
            .transactions
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.account_id == account_id)
            .map(|t| t.amount)
            .sum())
    }
}

fn app(repo: Arc<MockRepo>) -> axum::Router {
    router(AppState::new(repo.clone(), repo))
}

async fn send_json(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = match body {
        Some(json_body) => Request::builder()
            .method(method)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json_body.to_string()))
            .unwrap(),
        None => Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap(),
    };

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into_owned()))
    };
    (status, value)
}

#[tokio::test]
async fn post_accounts_creates_account() {
    let (status, body) = send_json(
        app(MockRepo::empty()),
        "POST",
        "/accounts",
        Some(json!({"document_number": "12345678900"})),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["account_id"], 1);
    assert_eq!(body["document_number"], "12345678900");
}

#[tokio::test]
async fn post_accounts_rejects_empty_document_number() {
    let (status, body) = send_json(
        app(MockRepo::empty()),
        "POST",
        "/accounts",
        Some(json!({"document_number": ""})),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "validation failed");
    assert_eq!(body["field"], "document_number");
}

#[tokio::test]
async fn post_accounts_rejects_duplicate() {
    let (status, body) = send_json(
        app(MockRepo::with_account(1, "12345678900")),
        "POST",
        "/accounts",
        Some(json!({"document_number": "12345678900"})),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "account already exists");
}

#[tokio::test]
async fn get_account_returns_account() {
    let (status, body) = send_json(
        app(MockRepo::with_account(7, "555")),
        "GET",
        "/accounts/7",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["account_id"], 7);
    assert_eq!(body["document_number"], "555");
}

#[tokio::test]
async fn get_account_returns_404_for_missing_account() {
    let (status, _) = send_json(app(MockRepo::empty()), "GET", "/accounts/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_balance_returns_zero_for_account_with_no_transactions() {
    let (status, body) = send_json(
        app(MockRepo::with_account(1, "doc")),
        "GET",
        "/accounts/1/balance",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["account_id"], 1);
    assert_eq!(body["balance"], 0.0);
}

#[tokio::test]
async fn get_balance_returns_404_for_missing_account() {
    let (status, _) = send_json(app(MockRepo::empty()), "GET", "/accounts/1/balance", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_balance_sums_transactions() {
    let repo = MockRepo::with_account(1, "doc");
    let app_router = || router(AppState::new(repo.clone(), repo.clone()));

    send_json(
        app_router(),
        "POST",
        "/transactions",
        Some(json!({"account_id": 1, "operation_type_id": 1, "amount": 100.0})),
    )
    .await;
    send_json(
        app_router(),
        "POST",
        "/transactions",
        Some(json!({"account_id": 1, "operation_type_id": 4, "amount": 30.0})),
    )
    .await;

    let (status, body) = send_json(app_router(), "GET", "/accounts/1/balance", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["balance"], -70.0);
}

#[tokio::test]
async fn post_transactions_creates_debit_with_negative_amount() {
    let (status, body) = send_json(
        app(MockRepo::with_account(1, "doc")),
        "POST",
        "/transactions",
        Some(json!({"account_id": 1, "operation_type_id": 1, "amount": 100.0})),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["amount"], -100.0);
    assert_eq!(body["operation_type_id"], 1);
}

#[tokio::test]
async fn post_transactions_creates_payment_with_positive_amount() {
    let (status, body) = send_json(
        app(MockRepo::with_account(1, "doc")),
        "POST",
        "/transactions",
        Some(json!({"account_id": 1, "operation_type_id": 4, "amount": -50.0})),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["amount"], 50.0);
}

#[tokio::test]
async fn post_transactions_rejects_invalid_operation_type() {
    let (status, body) = send_json(
        app(MockRepo::with_account(1, "doc")),
        "POST",
        "/transactions",
        Some(json!({"account_id": 1, "operation_type_id": 9, "amount": 10.0})),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["details"], "operation_type_id must be between 1 and 4");
    assert_eq!(body["field"], "operation_type_id");
}

#[tokio::test]
async fn post_transactions_returns_404_for_unknown_account() {
    let (status, body) = send_json(
        app(MockRepo::empty()),
        "POST",
        "/transactions",
        Some(json!({"account_id": 42, "operation_type_id": 1, "amount": 10.0})),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["details"], "account not found");
}

#[tokio::test]
async fn swagger_doc_json_is_served() {
    let (status, body) = send_json(app(MockRepo::empty()), "GET", "/swagger/doc.json", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["paths"]["/transactions"].is_object());
    assert!(body["paths"]["/accounts"].is_object());
}
