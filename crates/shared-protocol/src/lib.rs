//! Shared command protocol between the front‑end (TypeScript) and the simulation core (Rust).
//!
//! This module defines a minimal set of commands sufficient for the Phase 0 skeleton.
//! It mirrors the TypeScript definitions in `packages/protocol`.

use serde::{Deserialize, Serialize};

/// Types of commands recognized by the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandType {
    Init,
    Tick,
    SetPolicy,
    PlaceRoad,
    ZoneArea,
}

/// A player command with a unique ID, a type, and an arbitrary JSON payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerCommand {
    /// Monotonically increasing identifier.
    pub id: u64,
    pub r#type: CommandType,
    /// Arbitrary key/value payload; using `serde_json::Value` keeps it flexible.
    pub payload: serde_json::Value,
}

/// Result returned by the simulation after processing a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub id: u64,
    pub ok: bool,
    /// Optional data produced by the simulation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl PlayerCommand {
    /// Serialize the command to a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize PlayerCommand")
    }

    /// Deserialize from a JSON string.
    pub fn from_json(s: &str) -> Self {
        serde_json::from_str(s).expect("failed to deserialize PlayerCommand")
    }
}

impl CommandResult {
    /// Serialize to JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize CommandResult")
    }

    /// Deserialize from JSON.
    pub fn from_json(s: &str) -> Self {
        serde_json::from_str(s).expect("failed to deserialize CommandResult")
    }
}
