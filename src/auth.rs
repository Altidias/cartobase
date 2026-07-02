use crate::error::AppError;
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

// mirrors the client's ChunkShareService.slugFor
pub fn slug(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') { c } else { '_' })
        .collect();
    if cleaned.is_empty() { "player".into() } else { cleaned }
}

pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.trim().as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(Clone, Debug)]
pub struct AuthCtx {
    pub token_id: Uuid,
    pub crew_id: Uuid,
    pub player_name: String,
    pub role: String,
}

impl AuthCtx {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

fn bearer(parts: &Parts) -> Option<String> {
    let header = parts.headers.get(axum::http::header::AUTHORIZATION)?;
    let value = header.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    Some(token.trim().to_string())
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthCtx {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let token = bearer(parts).ok_or(AppError::Unauthorized)?;
        let hash = hash_token(&token);
        let row = sqlx::query_as::<_, (Uuid, Uuid, String, String)>(
            "select id, crew_id, player_name, role from tokens \
             where token_hash = $1 and revoked = false",
        )
        .bind(&hash)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

        let _ = sqlx::query("update tokens set last_seen_at = now() where id = $1")
            .bind(row.0)
            .execute(&state.pool)
            .await;

        Ok(AuthCtx { token_id: row.0, crew_id: row.1, player_name: row.2, role: row.3 })
    }
}

// admin-only wrapper; reuses AuthCtx then checks role
pub struct AdminCtx(pub AuthCtx);

#[axum::async_trait]
impl FromRequestParts<AppState> for AdminCtx {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let ctx = AuthCtx::from_request_parts(parts, state).await?;
        if !ctx.is_admin() {
            return Err(AppError::Forbidden);
        }
        Ok(AdminCtx(ctx))
    }
}
