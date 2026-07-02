use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub const CATEGORY_NAMES: [&str; 6] = [
    "explored",
    "new",
    "old",
    "block_exploit",
    "being_updated",
    "old_generation",
];

pub fn category_id(name: &str) -> Option<i16> {
    CATEGORY_NAMES.iter().position(|n| *n == name).map(|i| i as i16)
}

pub fn category_name(id: i16) -> Option<&'static str> {
    CATEGORY_NAMES.get(id as usize).copied()
}

#[derive(Debug, Deserialize)]
pub struct ChunkPushRequest {
    pub world: String,
    pub dimension: String,
    pub categories: HashMap<String, Vec<[i32; 2]>>,
}

#[derive(Debug, Serialize)]
pub struct ChunkPushResponse {
    pub seq: i64,
    pub applied: i64,
}

#[derive(Debug, Deserialize)]
pub struct ChunkPullQuery {
    pub world: String,
    pub dim: String,
    #[serde(default)]
    pub since: i64,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ContainerDto {
    pub player: String,
    pub category: String,
    pub cx: i32,
    pub cz: i32,
    pub bitmap: String,
}

#[derive(Debug, Serialize)]
pub struct ChunkPullResponse {
    pub containers: Vec<ContainerDto>,
    pub cursor: i64,
    pub complete: bool,
}

#[derive(Debug, Deserialize)]
pub struct WaypointDto {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    #[serde(default)]
    pub dimensions: String,
    #[serde(default)]
    pub color: i32,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub beacon: bool,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Deserialize)]
pub struct WaypointPushRequest {
    pub world: String,
    pub waypoints: Vec<WaypointDto>,
}

#[derive(Debug, Serialize)]
pub struct WaypointPushResponse {
    pub seq: i64,
    pub applied: i64,
}

#[derive(Debug, Deserialize)]
pub struct WaypointPullQuery {
    pub world: String,
    #[serde(default)]
    pub since: i64,
}

#[derive(Debug, Serialize)]
pub struct WaypointOut {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub dimensions: String,
    pub color: i32,
    pub icon: String,
    pub beacon: bool,
    pub deleted: bool,
    pub author: String,
}

#[derive(Debug, Serialize)]
pub struct WaypointPullResponse {
    pub waypoints: Vec<WaypointOut>,
    pub cursor: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateCrewRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CrewDto {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub crew: String,
    pub player_name: String,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "member".into()
}

#[derive(Debug, Serialize)]
pub struct TokenCreated {
    pub id: Uuid,
    pub token: String,
    pub crew_id: Uuid,
    pub player_name: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct TokenDto {
    pub id: Uuid,
    pub crew_id: Uuid,
    pub player_name: String,
    pub role: String,
    pub revoked: bool,
}

#[derive(Debug, Serialize)]
pub struct DimensionStat {
    pub world: String,
    pub dimension: String,
    pub category: String,
    pub chunks: i64,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub crews: i64,
    pub tokens: i64,
    pub players: i64,
    pub waypoints: i64,
    pub total_chunks: i64,
    pub by_dimension: Vec<DimensionStat>,
}
