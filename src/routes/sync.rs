use crate::auth::{slug, AuthCtx};
use crate::chunkmap::{container_of, empty_bitmap, popcount, set_bit, CONTAINER_BYTES};
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use base64::Engine;
use std::collections::HashMap;

pub async fn health() -> &'static str {
    "ok"
}

pub async fn whoami(ctx: AuthCtx) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "player": ctx.player_name,
        "slug": slug(&ctx.player_name),
        "crew_id": ctx.crew_id,
        "role": ctx.role,
    }))
}

pub async fn push_chunks(
    State(state): State<AppState>,
    ctx: AuthCtx,
    Json(req): Json<ChunkPushRequest>,
) -> AppResult<Json<ChunkPushResponse>> {
    if req.world.is_empty() || req.dimension.is_empty() {
        return Err(AppError::BadRequest("world and dimension required".into()));
    }
    let player_slug = slug(&ctx.player_name);

    let mut groups: HashMap<(i16, i32, i32), Vec<[i32; 2]>> = HashMap::new();
    for (name, coords) in &req.categories {
        let Some(cat) = category_id(name) else { continue };
        for c in coords {
            let (cx, cz) = container_of(c[0], c[1]);
            groups.entry((cat, cx, cz)).or_default().push(*c);
        }
    }

    let mut tx = state.pool.begin().await?;
    let mut applied = 0i64;
    let mut max_seq = 0i64;

    for ((cat, cx, cz), coords) in groups {
        let existing: Option<Vec<u8>> = sqlx::query_scalar(
            "select bitmap from chunk_containers where crew_id=$1 and world_key=$2 and dimension=$3 \
             and category=$4 and player_slug=$5 and container_x=$6 and container_z=$7 for update",
        )
        .bind(ctx.crew_id)
        .bind(&req.world)
        .bind(&req.dimension)
        .bind(cat)
        .bind(&player_slug)
        .bind(cx)
        .bind(cz)
        .fetch_optional(&mut *tx)
        .await?;

        let mut bitmap = existing.unwrap_or_else(empty_bitmap);
        if bitmap.len() != CONTAINER_BYTES {
            bitmap.resize(CONTAINER_BYTES, 0);
        }
        let mut newly = 0i64;
        for c in coords {
            if set_bit(&mut bitmap, c[0], c[1]) {
                newly += 1;
            }
        }
        if newly == 0 {
            continue;
        }
        let count = popcount(&bitmap);
        let seq: i64 = sqlx::query_scalar("select nextval('chunk_change_seq')")
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query(
            "insert into chunk_containers \
             (crew_id,world_key,dimension,category,player_slug,container_x,container_z,bitmap,chunk_count,seq,updated_at) \
             values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,now()) \
             on conflict (crew_id,world_key,dimension,category,player_slug,container_x,container_z) \
             do update set bitmap=excluded.bitmap, chunk_count=excluded.chunk_count, seq=excluded.seq, updated_at=now()",
        )
        .bind(ctx.crew_id)
        .bind(&req.world)
        .bind(&req.dimension)
        .bind(cat)
        .bind(&player_slug)
        .bind(cx)
        .bind(cz)
        .bind(bitmap.as_slice())
        .bind(count)
        .bind(seq)
        .execute(&mut *tx)
        .await?;

        applied += newly;
        if seq > max_seq {
            max_seq = seq;
        }
    }

    tx.commit().await?;
    Ok(Json(ChunkPushResponse { seq: max_seq, applied }))
}

pub async fn pull_chunks(
    State(state): State<AppState>,
    ctx: AuthCtx,
    Query(q): Query<ChunkPullQuery>,
) -> AppResult<Json<ChunkPullResponse>> {
    let limit = q.limit.unwrap_or(5000).clamp(1, 20000);
    let rows = sqlx::query_as::<_, (String, i16, i32, i32, Vec<u8>, i64)>(
        "select player_slug, category, container_x, container_z, bitmap, seq \
         from chunk_containers where crew_id=$1 and world_key=$2 and dimension=$3 and seq > $4 \
         order by seq asc limit $5",
    )
    .bind(ctx.crew_id)
    .bind(&q.world)
    .bind(&q.dim)
    .bind(q.since)
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;

    let mut cursor = q.since;
    let complete = (rows.len() as i64) < limit;
    let mut containers = Vec::with_capacity(rows.len());
    let engine = base64::engine::general_purpose::STANDARD;
    for (player, cat, cx, cz, bitmap, seq) in rows {
        if seq > cursor {
            cursor = seq;
        }
        let Some(category) = category_name(cat) else { continue };
        containers.push(ContainerDto {
            player,
            category: category.to_string(),
            cx,
            cz,
            bitmap: engine.encode(bitmap),
        });
    }

    Ok(Json(ChunkPullResponse { containers, cursor, complete }))
}

pub async fn push_waypoints(
    State(state): State<AppState>,
    ctx: AuthCtx,
    Json(req): Json<WaypointPushRequest>,
) -> AppResult<Json<WaypointPushResponse>> {
    if req.world.is_empty() {
        return Err(AppError::BadRequest("world required".into()));
    }
    let mut tx = state.pool.begin().await?;
    let mut applied = 0i64;
    let mut max_seq = 0i64;
    for wp in &req.waypoints {
        if wp.id.is_empty() {
            continue;
        }
        let seq: i64 = sqlx::query_scalar("select nextval('waypoint_change_seq')")
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query(
            "insert into waypoints \
             (crew_id,world_key,wp_id,name,x,y,z,dimensions,color,icon,beacon,deleted,author,seq,updated_at) \
             values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,now()) \
             on conflict (crew_id,world_key,wp_id) do update set \
             name=excluded.name, x=excluded.x, y=excluded.y, z=excluded.z, dimensions=excluded.dimensions, \
             color=excluded.color, icon=excluded.icon, beacon=excluded.beacon, deleted=excluded.deleted, \
             author=excluded.author, seq=excluded.seq, updated_at=now()",
        )
        .bind(ctx.crew_id)
        .bind(&req.world)
        .bind(&wp.id)
        .bind(&wp.name)
        .bind(wp.x)
        .bind(wp.y)
        .bind(wp.z)
        .bind(&wp.dimensions)
        .bind(wp.color)
        .bind(&wp.icon)
        .bind(wp.beacon)
        .bind(wp.deleted)
        .bind(&ctx.player_name)
        .bind(seq)
        .execute(&mut *tx)
        .await?;
        applied += 1;
        if seq > max_seq {
            max_seq = seq;
        }
    }
    tx.commit().await?;
    Ok(Json(WaypointPushResponse { seq: max_seq, applied }))
}

pub async fn pull_waypoints(
    State(state): State<AppState>,
    ctx: AuthCtx,
    Query(q): Query<WaypointPullQuery>,
) -> AppResult<Json<WaypointPullResponse>> {
    let rows = sqlx::query_as::<_, (String, String, i32, i32, i32, String, i32, String, bool, bool, String, i64)>(
        "select wp_id, name, x, y, z, dimensions, color, icon, beacon, deleted, author, seq \
         from waypoints where crew_id=$1 and world_key=$2 and seq > $3 order by seq asc",
    )
    .bind(ctx.crew_id)
    .bind(&q.world)
    .bind(q.since)
    .fetch_all(&state.pool)
    .await?;

    let mut cursor = q.since;
    let mut waypoints = Vec::with_capacity(rows.len());
    for r in rows {
        if r.11 > cursor {
            cursor = r.11;
        }
        waypoints.push(WaypointOut {
            id: r.0,
            name: r.1,
            x: r.2,
            y: r.3,
            z: r.4,
            dimensions: r.5,
            color: r.6,
            icon: r.7,
            beacon: r.8,
            deleted: r.9,
            author: r.10,
        });
    }

    Ok(Json(WaypointPullResponse { waypoints, cursor }))
}
