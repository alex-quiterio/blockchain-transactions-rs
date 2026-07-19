use std::sync::Arc;

use axum::{
    response::Html,
    routing::{get, post},
    Json, Router,
};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

use crate::api;
use crate::repository::{AccountRepo, TransactionRepo};
use crate::service::{AccountService, TransactionService};

#[derive(Clone)]
pub struct AppState {
    pub accounts: AccountService,
    pub transactions: TransactionService,
}

impl AppState {
    pub fn new(acc_repo: Arc<dyn AccountRepo>, tx_repo: Arc<dyn TransactionRepo>) -> Self {
        Self {
            accounts: AccountService::new(acc_repo.clone()),
            transactions: TransactionService::new(acc_repo, tx_repo),
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Customer Transactions API",
        description = "A Rust API for managing customer accounts and transactions",
        version = "1.0.0"
    ),
    paths(api::create_account, api::get_account, api::create_transaction)
)]
struct ApiDoc;

const SWAGGER_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>Customer Transactions API — Swagger UI</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    SwaggerUIBundle({ url: "/swagger/doc.json", dom_id: "#swagger-ui" });
  </script>
</body>
</html>"##;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/accounts", post(api::create_account))
        .route("/accounts/:accountId", get(api::get_account))
        .route("/transactions", post(api::create_transaction))
        .route("/swagger", get(|| async { Html(SWAGGER_HTML) }))
        .route(
            "/swagger/doc.json",
            get(|| async { Json(ApiDoc::openapi()) }),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
