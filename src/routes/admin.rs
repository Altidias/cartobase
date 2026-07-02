use crate::auth::{generate_token, hash_token, AdminCtx};
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

pub async fn stats(State(state): State<AppState>, _admin: AdminCtx) -> AppResult<Json<StatsResponse>> {
    let crews: i64 = sqlx::query_scalar("select count(*) from crews").fetch_one(&state.pool).await?;
    let tokens: i64 = sqlx::query_scalar("select count(*) from tokens where revoked=false")
        .fetch_one(&state.pool)
        .await?;
    let players: i64 = sqlx::query_scalar("select count(distinct player_slug) from chunk_containers")
        .fetch_one(&state.pool)
        .await?;
    let waypoints: i64 = sqlx::query_scalar("select count(*) from waypoints where deleted=false")
        .fetch_one(&state.pool)
        .await?;
    let total_chunks: i64 = sqlx::query_scalar("select coalesce(sum(chunk_count),0)::bigint from chunk_containers")
        .fetch_one(&state.pool)
        .await?;

    let rows = sqlx::query_as::<_, (String, String, i16, i64)>(
        "select world_key, dimension, category, coalesce(sum(chunk_count),0)::bigint \
         from chunk_containers group by world_key, dimension, category order by world_key, dimension, category",
    )
    .fetch_all(&state.pool)
    .await?;
    let by_dimension = rows
        .into_iter()
        .map(|(world, dimension, cat, chunks)| DimensionStat {
            world,
            dimension,
            category: category_name(cat).unwrap_or("unknown").to_string(),
            chunks,
        })
        .collect();

    Ok(Json(StatsResponse {
        crews,
        tokens,
        players,
        waypoints,
        total_chunks,
        by_dimension,
    }))
}

pub async fn list_crews(State(state): State<AppState>, _admin: AdminCtx) -> AppResult<Json<Vec<CrewDto>>> {
    let rows = sqlx::query_as::<_, (Uuid, String)>("select id, name from crews order by name")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(rows.into_iter().map(|(id, name)| CrewDto { id, name }).collect()))
}

pub async fn create_crew(
    State(state): State<AppState>,
    _admin: AdminCtx,
    Json(req): Json<CreateCrewRequest>,
) -> AppResult<Json<CrewDto>> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("crew name required".into()));
    }
    let crew = get_or_create_crew(&state, name).await?;
    Ok(Json(crew))
}

pub async fn list_tokens(State(state): State<AppState>, _admin: AdminCtx) -> AppResult<Json<Vec<TokenDto>>> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, bool)>(
        "select id, crew_id, player_name, role, revoked from tokens order by created_at desc",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, crew_id, player_name, role, revoked)| TokenDto {
                id,
                crew_id,
                player_name,
                role,
                revoked,
            })
            .collect(),
    ))
}

pub async fn create_token(
    State(state): State<AppState>,
    _admin: AdminCtx,
    Json(req): Json<CreateTokenRequest>,
) -> AppResult<Json<TokenCreated>> {
    let player_name = req.player_name.trim();
    if player_name.is_empty() {
        return Err(AppError::BadRequest("player_name required".into()));
    }
    let role = match req.role.as_str() {
        "admin" => "admin",
        _ => "member",
    };
    let crew = get_or_create_crew(&state, req.crew.trim()).await?;

    let token = generate_token();
    let hash = hash_token(&token);
    let id = Uuid::new_v4();
    sqlx::query(
        "insert into tokens (id, crew_id, token_hash, player_name, role) values ($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(crew.id)
    .bind(&hash)
    .bind(player_name)
    .bind(role)
    .execute(&state.pool)
    .await?;

    Ok(Json(TokenCreated {
        id,
        token,
        crew_id: crew.id,
        player_name: player_name.to_string(),
        role: role.to_string(),
    }))
}

pub async fn revoke_token(
    State(state): State<AppState>,
    _admin: AdminCtx,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("update tokens set revoked=true where id=$1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({ "revoked": id })))
}

async fn get_or_create_crew(state: &AppState, name: &str) -> AppResult<CrewDto> {
    if name.is_empty() {
        return Err(AppError::BadRequest("crew name required".into()));
    }
    if let Some((id, name)) = sqlx::query_as::<_, (Uuid, String)>("select id, name from crews where name=$1")
        .bind(name)
        .fetch_optional(&state.pool)
        .await?
    {
        return Ok(CrewDto { id, name });
    }
    let id = Uuid::new_v4();
    sqlx::query("insert into crews (id, name) values ($1,$2) on conflict (name) do nothing")
        .bind(id)
        .bind(name)
        .execute(&state.pool)
        .await?;
    let (id, name) = sqlx::query_as::<_, (Uuid, String)>("select id, name from crews where name=$1")
        .bind(name)
        .fetch_one(&state.pool)
        .await?;
    Ok(CrewDto { id, name })
}
