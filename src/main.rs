use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;
use tokio::signal;

use customer_transactions::repository::PgRepo;
use customer_transactions::server::{router, AppState};

const DEFAULT_DSN: &str = "postgres://customer:secret@localhost:5432/transactions?sslmode=disable";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .init();

    let dsn = std::env::var("DATABASE_DSN").unwrap_or_else(|_| DEFAULT_DSN.to_string());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&dsn)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let repo = Arc::new(PgRepo::new(pool));
    let state = AppState::new(repo.clone(), repo);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("Server listening on :8080");

    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server gracefully stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to listen for ctrl-c");
    };
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
