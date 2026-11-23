use anyhow::{Context, Result};
use fjall::{Config as FjallConfig, Keyspace, PartitionCreateOptions, PartitionHandle};
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::path::Path;

// redb Tables: TableDefinition<Key, Value>
const PLAYERS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("players");
const LEVELS_TABLE: TableDefinition<u32, &[u8]> = TableDefinition::new("levels");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSave {
    pub player_id: String,
    pub active_level_id: u32,
    pub current_room_id: String,
    pub inventory: Vec<String>,
    pub xp: u64,
    pub last_synced_session: Option<String>, // Links to C500-TimeSync booking ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelRecord {
    pub level_id: u32,
    pub title: String,
    pub unlocked: bool,
    pub raw_toml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MudEventLog {
    pub timestamp_unix: u64,
    pub player_id: String,
    pub event_type: String, // e.g., "MOVE", "TIMESYNC_UNLOCK", "EMERGENCY_RECALL"
    pub payload: String,
}

pub struct HybridStorage {
    db: Database,
    _keyspace: Keyspace,
    event_partition: PartitionHandle,
}

impl HybridStorage {
    /// Initializes state.redb and events_lsm/ inside the Termux data directory
    pub fn open<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let dir = data_dir.as_ref();
        std::fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create MUD data dir at {:?}", dir))?;

        // 1. Open redb for ACID state
        let redb_path = dir.join("state.redb");
        let db = Database::create(&redb_path)?;

        // Ensure tables exist on first boot
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(PLAYERS_TABLE)?;
            let _ = write_txn.open_table(LEVELS_TABLE)?;
        }
        write_txn.commit()?;

        // 2. Open fjall LSM keyspace for append-only telemetry & session logs
        let fjall_path = dir.join("events_lsm");
        let keyspace = FjallConfig::new(&fjall_path).open()?;
        let event_partition =
            keyspace.open_partition("mud_events", PartitionCreateOptions::default())?;

        Ok(Self {
            db,
            _keyspace: keyspace,
            event_partition,
        })
    }

    /// Atomically saves or updates a player's state in redb
    pub fn save_player(&self, save: &PlayerSave) -> Result<()> {
        let encoded = bincode::serialize(save)?;
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(PLAYERS_TABLE)?;
            table.insert(save.player_id.as_str(), encoded.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Loads a player's canonical state from redb
    pub fn load_player(&self, player_id: &str) -> Result<Option<PlayerSave>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(PLAYERS_TABLE)?;

        match table.get(player_id)? {
            Some(bytes) => {
                let save: PlayerSave = bincode::deserialize(bytes.value())?;
                Ok(Some(save))
            }
            None => Ok(None),
        }
    }

    /// Persists or unlocks a MUD level definition in redb
    pub fn save_level(&self, level: &LevelRecord) -> Result<()> {
        let encoded = bincode::serialize(level)?;
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(LEVELS_TABLE)?;
            table.insert(level.level_id, encoded.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Appends a time-ordered event into fjall (key format: "<player_id>:<zero_padded_ts>")
    pub fn append_event(&self, event: &MudEventLog) -> Result<()> {
        let key = format!("{}:{:020}", event.player_id, event.timestamp_unix);
        let value = bincode::serialize(event)?;
        self.event_partition.insert(key.as_bytes(), value)?;
        Ok(())
    }

    /// Scans the last N events for a player from fjall using prefix iteration
    pub fn recent_events(&self, player_id: &str, limit: usize) -> Result<Vec<MudEventLog>> {
        let prefix = format!("{}:", player_id);
        let mut results = Vec::new();

        for item in self.event_partition.prefix(prefix.as_bytes()).rev().take(limit) {
            let (_k, v) = item?;
            let log: MudEventLog = bincode::deserialize(&v)?;
            results.push(log);
        }

        results.reverse();
        Ok(results)
    }
}
