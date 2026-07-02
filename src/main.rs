mod auth;
mod chunkmap;
mod config;
mod db;
mod error;
mod models;
mod routes;
mod state;

use crate::config::Config;
use crate::state::AppState;
use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("cartobase=info,tower_http=warn")))
        .init();

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("config error: {e}");
            std::process::exit(1);
        }
    };

    let pool = match db::connect(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("database init failed: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = bootstrap_admin(&pool, &config).await {
        tracing::error!("admin bootstrap failed: {e}");
    }

    let state = AppState { pool, config: Arc::new(config.clone()) };
    let app = router(state);

    let listener = match tokio::net::TcpListener::bind(&config.bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("bind {} failed: {e}", config.bind_addr);
            std::process::exit(1);
        }
    };
    tracing::info!("cartobase listening on {}", config.bind_addr);
    if let Err(e) = axum::serve(listener, app.into_make_service()).await {
        tracing::error!("server error: {e}");
    }
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(routes::sync::health))
        .route("/api/v1/sync/whoami", get(routes::sync::whoami))
        .route("/api/v1/sync/chunks", post(routes::sync::push_chunks).get(routes::sync::pull_chunks))
        .route("/api/v1/sync/waypoints", post(routes::sync::push_waypoints).get(routes::sync::pull_waypoints))
        .route("/api/v1/admin/stats", get(routes::admin::stats))
        .route("/api/v1/admin/crews", get(routes::admin::list_crews).post(routes::admin::create_crew))
        .route("/api/v1/admin/tokens", get(routes::admin::list_tokens).post(routes::admin::create_token))
        .route("/api/v1/admin/tokens/:id/revoke", post(routes::admin::revoke_token))
        .fallback_service(ServeDir::new("web"))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn bootstrap_admin(pool: &PgPool, config: &Config) -> Result<(), sqlx::Error> {
    let admins: i64 = sqlx::query_scalar("select count(*) from tokens where role='admin' and revoked=false")
        .fetch_one(pool)
        .await?;
    if admins > 0 && config.admin_token.is_none() {
        return Ok(());
    }

    let crew_id = Uuid::new_v4();
    sqlx::query("insert into crews (id, name) values ($1,$2) on conflict (name) do nothing")
        .bind(crew_id)
        .bind(&config.admin_crew)
        .execute(pool)
        .await?;
    let crew_id: Uuid = sqlx::query_scalar("select id from crews where name=$1")
        .bind(&config.admin_crew)
        .fetch_one(pool)
        .await?;

    let generated = config.admin_token.is_none();
    let token = config.admin_token.clone().unwrap_or_else(auth::generate_token);
    let hash = auth::hash_token(&token);
    sqlx::query(
        "insert into tokens (id, crew_id, token_hash, player_name, role) values ($1,$2,$3,'admin','admin') \
         on conflict (token_hash) do update set revoked=false, role='admin'",
    )
    .bind(Uuid::new_v4())
    .bind(crew_id)
    .bind(&hash)
    .execute(pool)
    .await?;

    if generated {
        tracing::warn!("no admin token found; generated one (store it now, shown once): {token}");
    } else {
        tracing::info!("admin token ensured for crew '{}'", config.admin_crew);
    }
    Ok(())
}
