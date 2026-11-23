pub mod storage;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use storage::{HybridStorage, MudEventLog, PlayerSave};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    pub id: u32,
    pub title: String,
    pub required_sync_state: Option<String>,
    pub starting_room: String,
    pub rooms: HashMap<String, Room>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub description: String,
    pub exits: HashMap<String, String>,
    pub items: Vec<String>,
    pub sync_event_hook: Option<String>,
}

pub struct GameSession {
    pub storage: HybridStorage,
    pub player: PlayerSave,
    pub current_level: Level,
}

impl GameSession {
    pub fn move_player(&mut self, direction: &str) -> Result<String> {
        let room = self
            .current_level
            .rooms
            .get(&self.player.current_room_id)
            .ok_or_else(|| anyhow::anyhow!("Current room missing from level graph"))?;

        let next_room_id = room
            .exits
            .get(direction)
            .ok_or_else(|| anyhow::anyhow!("No exit '{}'", direction))?
            .clone();

        // 1. Update canonical state in redb
        self.player.current_room_id = next_room_id.clone();
        self.storage.save_player(&self.player)?;

        // 2. Append movement log to fjall
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        self.storage.append_event(&MudEventLog {
            timestamp_unix: now,
            player_id: self.player.player_id.clone(),
            event_type: "MOVE".into(),
            payload: format!("level:{} -> room:{}", self.player.active_level_id, next_room_id),
        })?;

        let next_room = &self.current_level.rooms[&next_room_id];
        Ok(format!("\n[{}]\n{}", next_room.name, next_room.description))
    }
}
