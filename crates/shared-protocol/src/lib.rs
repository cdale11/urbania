//! Shared protocol between browser client (TypeScript) and Rust server.
//! Defines multi-city types, commands, deltas and WS messages.
//! Mirrors `packages/protocol` on the TS side.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// City identity & metadata
// ---------------------------------------------------------------------------

pub type CityId = i64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CityMeta {
    pub id: CityId,
    pub name: String,
    pub seed: u64,
    pub tick: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCityRequest {
    pub name: String,
    /// Optional seed; if None server generates from OsRng
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCityResponse {
    pub city: CityMeta,
}

// ---------------------------------------------------------------------------
// Legacy + extended command types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandType {
    Init,
    Tick,
    SetPolicy,
    PlaceRoad,
    ZoneArea,
    // Multi-city / transport extensions
    CreateCity,
    DeleteCity,
    SaveCity,
    LoadCity,
    BuildRoad,
    RemoveRoad,
}

/// Player command - now includes city_id for multi-city routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerCommand {
    pub id: u64,
    pub city_id: Option<CityId>,
    pub r#type: CommandType,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub id: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PlayerCommand {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("serialize PlayerCommand")
    }
    pub fn from_json(s: &str) -> Self {
        serde_json::from_str(s).expect("deserialize PlayerCommand")
    }
}

impl CommandResult {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("serialize CommandResult")
    }
    pub fn from_json(s: &str) -> Self {
        serde_json::from_str(s).expect("deserialize CommandResult")
    }
    pub fn ok(id: u64, data: Option<serde_json::Value>) -> Self {
        Self { id, ok: true, data, error: None }
    }
    pub fn err(id: u64, msg: impl Into<String>) -> Self {
        Self { id, ok: false, data: None, error: Some(msg.into()) }
    }
}

// ---------------------------------------------------------------------------
// Chunk / World delta - networking model per spec sec 43
// ---------------------------------------------------------------------------

/// Minimal chunk representation for wire format (mirrors sim-core::Chunk)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkDto {
    pub cx: i32,
    pub cy: i32,
    pub data: serde_json::Value,
}

// Road graph DTOs (spec 9) - shared between transport crate and WS protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct WorldPos {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JunctionType {
    End,
    Straight,
    Intersection,
    Roundabout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadNodeDto {
    pub id: u64,
    pub pos: WorldPos,
    pub junction: JunctionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadEdgeDto {
    pub id: u64,
    pub start: u64,
    pub end: u64,
    pub lanes: u8,
    pub speed_limit: u16,
    pub width: f32,
    pub grade: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoadGraphDto {
    pub nodes: Vec<RoadNodeDto>,
    pub edges: Vec<RoadEdgeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRoadRequest {
    pub from: WorldPos,
    pub to: WorldPos,
    pub lanes: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldDelta {
    pub city_id: CityId,
    pub tick: u64,
    pub changed_chunks: Vec<ChunkDto>,
    /// Road graph delta - full graph for MVP (incremental later)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_roads: Option<RoadGraphDto>,
    /// Generic events (RoadBuilt, etc. - spec sec 45)
    pub events: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRequest {
    pub city_id: CityId,
    pub cx: i32,
    pub cy: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialSnapshot {
    pub city: CityMeta,
    pub tick: u64,
    pub chunks: Vec<ChunkDto>,
    pub road_graph: RoadGraphDto,
}

// ---------------------------------------------------------------------------
// WS envelope - client <-> server
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientMessage {
    Command(PlayerCommand),
    Subscribe { city_id: CityId, radius: Option<u32> },
    RequestChunk(ChunkRequest),
    Ping { t: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerMessage {
    Snapshot(InitialSnapshot),
    Delta(WorldDelta),
    Result(CommandResult),
    Pong { t: u64 },
    Error { message: String },
}

// ---------------------------------------------------------------------------
// Validation trait mirroring spec sec 54
// ---------------------------------------------------------------------------

pub trait ValidatableCommand {
    fn validate(&self, seed: u64) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_command() {
        let cmd = PlayerCommand { id: 1, city_id: Some(42), r#type: CommandType::BuildRoad, payload: serde_json::json!({"from":[0,0],"to":[1,0]}) };
        let s = cmd.to_json();
        let back = PlayerCommand::from_json(&s);
        assert_eq!(back.id, 1);
        assert_eq!(back.city_id, Some(42));
    }
    #[test]
    fn ws_envelope_serializes() {
        let msg = ClientMessage::Command(PlayerCommand { id: 2, city_id: None, r#type: CommandType::Tick, payload: serde_json::json!({}) });
        let s = serde_json::to_string(&msg).unwrap();
        let back: ClientMessage = serde_json::from_str(&s).unwrap();
        assert!(matches!(back, ClientMessage::Command(_)));
    }
}
