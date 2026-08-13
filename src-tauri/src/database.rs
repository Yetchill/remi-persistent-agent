use std::{path::Path, sync::Mutex};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    memory::{Memory, MemoryOperation, MemoryRelation, NewMemory},
    pet_state::{PetState, PetStatePatch},
    profile::CompanionProfile,
    provider::{ModelConfig, ProviderCatalog, ProviderConfig},
    settings::AppSettings,
    trace::{ActionTrace, ActionTraceSummary, EventTrace, LlmCallTrace},
    working_memory::WorkingMessage,
};

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        let database = Self {
            connection: Mutex::new(connection),
        };
        database.initialize()?;
        Ok(database)
    }

    #[cfg(test)]
    pub fn in_memory() -> rusqlite::Result<Self> {
        let connection = Connection::open_in_memory()?;
        let database = Self {
            connection: Mutex::new(connection),
        };
        database.initialize()?;
        Ok(database)
    }

    fn initialize(&self) -> rusqlite::Result<()> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute_batch(
                "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS agent_events (
                 id TEXT PRIMARY KEY,
                 event_type TEXT NOT NULL,
                 source TEXT NOT NULL,
                 timestamp INTEGER NOT NULL,
                 payload_json TEXT
             );
             CREATE TABLE IF NOT EXISTS agent_actions (
                 id TEXT PRIMARY KEY,
                 event_id TEXT NOT NULL,
                 action_type TEXT NOT NULL,
                 timestamp INTEGER NOT NULL,
                 payload_json TEXT,
                 success INTEGER NOT NULL,
                 error TEXT
             );
             CREATE TABLE IF NOT EXISTS llm_calls (
                 id TEXT PRIMARY KEY,
                 event_id TEXT NOT NULL,
                 provider TEXT NOT NULL,
                 model TEXT NOT NULL,
                 event_type TEXT NOT NULL,
                 timestamp INTEGER NOT NULL,
                 request_json TEXT NOT NULL,
                 response_json TEXT,
                 input_tokens INTEGER,
                 output_tokens INTEGER,
                 latency_ms INTEGER NOT NULL,
                 success INTEGER NOT NULL,
                 error TEXT
             );
             CREATE TABLE IF NOT EXISTS pet_state (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 energy REAL NOT NULL,
                 boredom REAL NOT NULL,
                 mood TEXT NOT NULL,
                 activity TEXT NOT NULL,
                 current_goal TEXT,
                 x INTEGER NOT NULL,
                 y INTEGER NOT NULL,
                 opacity REAL NOT NULL,
                 last_user_interaction_at INTEGER,
                 last_agent_interaction_at INTEGER,
                 last_heartbeat_at INTEGER
             );
             CREATE TABLE IF NOT EXISTS conversations (
                 id TEXT PRIMARY KEY,
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS messages (
                 id TEXT PRIMARY KEY,
                 conversation_id TEXT NOT NULL,
                 role TEXT NOT NULL,
                 content TEXT NOT NULL,
                 timestamp INTEGER NOT NULL,
                 FOREIGN KEY (conversation_id) REFERENCES conversations(id)
             );
             CREATE INDEX IF NOT EXISTS idx_messages_conversation_time
                 ON messages(conversation_id, timestamp DESC);
             CREATE TABLE IF NOT EXISTS memories (
                 id TEXT PRIMARY KEY,
                 kind TEXT NOT NULL,
                 content TEXT NOT NULL,
                 importance REAL NOT NULL DEFAULT 0.5,
                 pinned INTEGER NOT NULL DEFAULT 0,
                 confidence REAL NOT NULL DEFAULT 1.0,
                 status TEXT NOT NULL DEFAULT 'active',
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL,
                 last_accessed_at INTEGER,
                 access_count INTEGER NOT NULL DEFAULT 0,
                 valid_from INTEGER,
                 valid_to INTEGER,
                 source_type TEXT,
                 source_ref TEXT,
                 embedding_json TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_memories_active_kind
                 ON memories(status, kind, updated_at DESC);
             CREATE TABLE IF NOT EXISTS memory_relations (
                 source_id TEXT NOT NULL,
                 target_id TEXT NOT NULL,
                 relation TEXT NOT NULL,
                 created_at INTEGER NOT NULL,
                 PRIMARY KEY (source_id, target_id, relation),
                 FOREIGN KEY (source_id) REFERENCES memories(id),
                 FOREIGN KEY (target_id) REFERENCES memories(id)
             );
             CREATE TABLE IF NOT EXISTS memory_operations (
                 id TEXT PRIMARY KEY,
                 memory_id TEXT,
                 operation TEXT NOT NULL,
                 timestamp INTEGER NOT NULL,
                 source_event_id TEXT,
                 reason_label TEXT,
                 related_memory_ids_json TEXT,
                 detail_json TEXT,
                 FOREIGN KEY (memory_id) REFERENCES memories(id)
             );
             CREATE INDEX IF NOT EXISTS idx_memory_operations_time
                 ON memory_operations(timestamp DESC);
             CREATE TABLE IF NOT EXISTS agent_metadata (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS app_settings (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 pet_name TEXT NOT NULL,
                 pet_size TEXT NOT NULL,
                 auto_wander INTEGER NOT NULL,
                 wander_interval_seconds INTEGER NOT NULL,
                 movement_speed TEXT NOT NULL,
                 proactive_interaction INTEGER NOT NULL,
                 agent_heartbeat INTEGER NOT NULL,
                 agent_heartbeat_interval_seconds INTEGER NOT NULL DEFAULT 60,
                 proactive_cooldown_minutes INTEGER NOT NULL DEFAULT 30,
                 max_proactive_messages_per_hour INTEGER NOT NULL DEFAULT 2,
                 do_not_disturb INTEGER NOT NULL DEFAULT 0,
                 proactive_frequency TEXT NOT NULL DEFAULT 'normal',
                 quiet_hours_enabled INTEGER NOT NULL DEFAULT 0,
                 quiet_hours_start TEXT NOT NULL DEFAULT '23:00',
                 quiet_hours_end TEXT NOT NULL DEFAULT '08:00'
             );
             CREATE TABLE IF NOT EXISTS providers (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 provider_type TEXT NOT NULL,
                 base_url TEXT NOT NULL,
                 enabled INTEGER NOT NULL,
                 has_api_key INTEGER NOT NULL DEFAULT 0,
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS provider_models (
                 id TEXT PRIMARY KEY,
                 provider_id TEXT NOT NULL,
                 name TEXT NOT NULL,
                 model_id TEXT NOT NULL,
                 enabled INTEGER NOT NULL,
                 FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
             );
             CREATE TABLE IF NOT EXISTS provider_selection (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 active_provider_id TEXT,
                 active_model_id TEXT
             );",
            )?;
        {
            let connection = self.connection.lock().expect("database lock poisoned");
            ensure_column(&connection, "memory_operations", "source_event_id", "TEXT")?;
            ensure_column(&connection, "memory_operations", "reason_label", "TEXT")?;
            ensure_column(
                &connection,
                "memories",
                "pinned",
                "INTEGER NOT NULL DEFAULT 0",
            )?;
            ensure_column(
                &connection,
                "memory_operations",
                "related_memory_ids_json",
                "TEXT",
            )?;
            ensure_column(
                &connection,
                "app_settings",
                "agent_heartbeat_interval_seconds",
                "INTEGER NOT NULL DEFAULT 60",
            )?;
            ensure_column(
                &connection,
                "app_settings",
                "proactive_cooldown_minutes",
                "INTEGER NOT NULL DEFAULT 30",
            )?;
            ensure_column(
                &connection,
                "app_settings",
                "max_proactive_messages_per_hour",
                "INTEGER NOT NULL DEFAULT 2",
            )?;
            ensure_column(
                &connection,
                "app_settings",
                "do_not_disturb",
                "INTEGER NOT NULL DEFAULT 0",
            )?;
            ensure_column(
                &connection,
                "app_settings",
                "proactive_frequency",
                "TEXT NOT NULL DEFAULT 'normal'",
            )?;
            ensure_column(
                &connection,
                "app_settings",
                "quiet_hours_enabled",
                "INTEGER NOT NULL DEFAULT 0",
            )?;
            ensure_column(
                &connection,
                "app_settings",
                "quiet_hours_start",
                "TEXT NOT NULL DEFAULT '23:00'",
            )?;
            ensure_column(
                &connection,
                "app_settings",
                "quiet_hours_end",
                "TEXT NOT NULL DEFAULT '08:00'",
            )?;
            connection.execute(
                "INSERT OR IGNORE INTO agent_metadata (key, value) VALUES ('soul_version', '1')",
                [],
            )?;
            connection.execute(
                "INSERT OR IGNORE INTO agent_metadata (key, value)
                 VALUES ('active_pet_pack_id', 'remi-hanfu-v1')",
                [],
            )?;
        }
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "INSERT OR IGNORE INTO pet_state
             (id, energy, boredom, mood, activity, x, y, opacity)
             VALUES (1, 100.0, 0.0, 'neutral', 'idle', 0, 0, 1.0)",
                [],
            )?;
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "INSERT OR IGNORE INTO conversations (id, created_at, updated_at)
             VALUES ('main', 0, 0)",
                [],
            )?;
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "INSERT OR IGNORE INTO app_settings
             (id, pet_name, pet_size, auto_wander, wander_interval_seconds,
              movement_speed, proactive_interaction, agent_heartbeat,
              agent_heartbeat_interval_seconds, proactive_cooldown_minutes,
              max_proactive_messages_per_hour, do_not_disturb, proactive_frequency,
              quiet_hours_enabled, quiet_hours_start, quiet_hours_end)
             VALUES (1, 'Remi', 'large', 1, 30, 'normal', 0, 0, 60, 30, 2, 0,
                     'normal', 0, '23:00', '08:00')",
                [],
            )?;
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "INSERT OR IGNORE INTO provider_selection (id) VALUES (1)",
                [],
            )?;
        Ok(())
    }

    pub fn insert_event(&self, trace: &EventTrace) -> rusqlite::Result<()> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "INSERT OR REPLACE INTO agent_events
             (id, event_type, source, timestamp, payload_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    trace.id,
                    trace.event_type,
                    trace.source,
                    trace.timestamp,
                    trace.payload_json
                ],
            )?;
        Ok(())
    }

    pub fn insert_action(&self, trace: &ActionTrace) -> rusqlite::Result<()> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "INSERT OR REPLACE INTO agent_actions
             (id, event_id, action_type, timestamp, payload_json, success, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    trace.id,
                    trace.event_id,
                    trace.action_type,
                    trace.timestamp,
                    trace.payload_json,
                    trace.success,
                    trace.error,
                ],
            )?;
        Ok(())
    }

    pub fn get_last_heartbeat_action(&self) -> rusqlite::Result<Option<ActionTraceSummary>> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .query_row(
                "SELECT a.action_type, a.timestamp, a.payload_json, a.success, a.error
                 FROM agent_actions a
                 JOIN agent_events e ON e.id = a.event_id
                 WHERE e.event_type = 'AGENT_HEARTBEAT'
                 ORDER BY a.timestamp DESC, a.rowid DESC LIMIT 1",
                [],
                |row| {
                    Ok(ActionTraceSummary {
                        action_type: row.get(0)?,
                        timestamp: row.get(1)?,
                        payload_json: row.get(2)?,
                        success: row.get(3)?,
                        reason: row.get(4)?,
                    })
                },
            )
            .optional()
    }

    pub fn insert_llm_call(&self, trace: &LlmCallTrace) -> rusqlite::Result<()> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "INSERT INTO llm_calls
             (id, event_id, provider, model, event_type, timestamp, request_json,
              response_json, input_tokens, output_tokens, latency_ms, success, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    trace.id,
                    trace.event_id,
                    trace.provider,
                    trace.model,
                    trace.event_type,
                    trace.timestamp,
                    trace.request_json,
                    trace.response_json,
                    trace.input_tokens,
                    trace.output_tokens,
                    trace.latency_ms,
                    trace.success,
                    trace.error,
                ],
            )?;
        Ok(())
    }

    pub fn get_pet_state(&self) -> rusqlite::Result<PetState> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .query_row(
                "SELECT energy, boredom, mood, activity, current_goal, x, y, opacity,
                    last_user_interaction_at, last_agent_interaction_at, last_heartbeat_at
             FROM pet_state WHERE id = 1",
                [],
                |row| {
                    Ok(PetState {
                        energy: row.get(0)?,
                        boredom: row.get(1)?,
                        mood: row.get(2)?,
                        activity: row.get(3)?,
                        current_goal: row.get(4)?,
                        x: row.get(5)?,
                        y: row.get(6)?,
                        opacity: row.get(7)?,
                        last_user_interaction_at: row.get(8)?,
                        last_agent_interaction_at: row.get(9)?,
                        last_heartbeat_at: row.get(10)?,
                    })
                },
            )
    }

    pub fn update_pet_state(&self, patch: &PetStatePatch) -> rusqlite::Result<PetState> {
        let mut state = self.get_pet_state()?;
        state.apply(patch);
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "UPDATE pet_state SET energy = ?1, boredom = ?2, mood = ?3, activity = ?4,
                    current_goal = ?5, x = ?6, y = ?7, opacity = ?8,
                    last_user_interaction_at = ?9, last_agent_interaction_at = ?10,
                    last_heartbeat_at = ?11 WHERE id = 1",
                params![
                    state.energy,
                    state.boredom,
                    state.mood,
                    state.activity,
                    state.current_goal,
                    state.x,
                    state.y,
                    state.opacity,
                    state.last_user_interaction_at,
                    state.last_agent_interaction_at,
                    state.last_heartbeat_at,
                ],
            )?;
        Ok(state)
    }

    pub fn persist_message(&self, message: &WorkingMessage) -> rusqlite::Result<()> {
        let mut connection = self.connection.lock().expect("database lock poisoned");
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT OR IGNORE INTO messages
             (id, conversation_id, role, content, timestamp) VALUES (?1, 'main', ?2, ?3, ?4)",
            params![message.id, message.role, message.content, message.timestamp],
        )?;
        transaction.execute(
            "UPDATE conversations SET updated_at = MAX(updated_at, ?1) WHERE id = 'main'",
            [message.timestamp],
        )?;
        transaction.commit()
    }

    pub fn get_recent_messages(&self, limit: usize) -> rusqlite::Result<Vec<WorkingMessage>> {
        let connection = self.connection.lock().expect("database lock poisoned");
        let mut statement = connection.prepare(
            "SELECT id, role, content, timestamp FROM (
                 SELECT rowid AS sequence, id, role, content, timestamp FROM messages
                 WHERE conversation_id = 'main' ORDER BY timestamp DESC, rowid DESC LIMIT ?1
             ) ORDER BY timestamp ASC, sequence ASC",
        )?;
        statement
            .query_map([limit as i64], |row| {
                Ok(WorkingMessage {
                    id: row.get(0)?,
                    role: row.get(1)?,
                    content: row.get(2)?,
                    timestamp: row.get(3)?,
                })
            })?
            .collect()
    }

    pub fn clear_current_conversation(&self, timestamp: i64) -> rusqlite::Result<usize> {
        let mut connection = self.connection.lock().expect("database lock poisoned");
        let transaction = connection.transaction()?;
        let deleted =
            transaction.execute("DELETE FROM messages WHERE conversation_id = 'main'", [])?;
        transaction.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = 'main'",
            [timestamp],
        )?;
        transaction.commit()?;
        Ok(deleted)
    }

    pub fn insert_memory(&self, memory: &NewMemory) -> rusqlite::Result<Memory> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "INSERT INTO memories
             (id, kind, content, importance, confidence, status, created_at, updated_at,
              valid_from, valid_to, source_type, source_ref)
             VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?6, ?7, ?8, ?9, ?10)",
                params![
                    memory.id,
                    memory.kind,
                    memory.content,
                    memory.importance,
                    memory.confidence,
                    memory.created_at,
                    memory.valid_from,
                    memory.valid_to,
                    memory.source_type,
                    memory.source_ref,
                ],
            )?;
        self.get_memory(&memory.id)
    }

    pub fn get_memory(&self, id: &str) -> rusqlite::Result<Memory> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .query_row(
                "SELECT id, kind, content, importance, confidence, status, created_at, updated_at,
                    last_accessed_at, access_count, valid_from, valid_to, source_type, source_ref,
                    pinned
             FROM memories WHERE id = ?1",
                [id],
                map_memory,
            )
    }

    pub fn list_active_memories(&self) -> rusqlite::Result<Vec<Memory>> {
        let connection = self.connection.lock().expect("database lock poisoned");
        let mut statement = connection.prepare(
            "SELECT id, kind, content, importance, confidence, status, created_at, updated_at,
                    last_accessed_at, access_count, valid_from, valid_to, source_type, source_ref,
                    pinned
             FROM memories WHERE status = 'active' ORDER BY updated_at DESC",
        )?;
        statement.query_map([], map_memory)?.collect()
    }

    pub fn list_memories(
        &self,
        kind: Option<&str>,
        status: Option<&str>,
    ) -> rusqlite::Result<Vec<Memory>> {
        self.search_memories(kind, status, None)
    }

    pub fn search_memories(
        &self,
        kind: Option<&str>,
        status: Option<&str>,
        query: Option<&str>,
    ) -> rusqlite::Result<Vec<Memory>> {
        let connection = self.connection.lock().expect("database lock poisoned");
        let mut statement = connection.prepare(
            "SELECT id, kind, content, importance, confidence, status, created_at, updated_at,
                    last_accessed_at, access_count, valid_from, valid_to, source_type, source_ref,
                    pinned
             FROM memories
             WHERE (?1 IS NULL OR kind = ?1)
               AND (?2 IS NULL OR status = ?2)
               AND (?3 IS NULL OR instr(lower(content), lower(?3)) > 0)
             ORDER BY updated_at DESC, created_at DESC",
        )?;
        statement
            .query_map(params![kind, status, query], map_memory)?
            .collect()
    }

    pub fn memory_status_counts(&self) -> rusqlite::Result<(i64, i64, i64, i64)> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .query_row(
                "SELECT
                    SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status = 'outdated' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status = 'archived' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status = 'merged' THEN 1 ELSE 0 END)
                 FROM memories",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<i64>>(0)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                    ))
                },
            )
    }

    // Mirrors the bounded set of columns updated by one lifecycle decision.
    #[allow(clippy::too_many_arguments)]
    pub fn update_memory(
        &self,
        id: &str,
        content: &str,
        kind: &str,
        importance: f64,
        confidence: f64,
        timestamp: i64,
        valid_from: Option<i64>,
        valid_to: Option<i64>,
    ) -> rusqlite::Result<Memory> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "UPDATE memories SET content = ?1, kind = ?2, importance = ?3,
                    confidence = ?4, updated_at = ?5, valid_from = ?6, valid_to = ?7
                 WHERE id = ?8",
                params![
                    content, kind, importance, confidence, timestamp, valid_from, valid_to, id
                ],
            )?;
        self.get_memory(id)
    }

    // User corrections intentionally use their own provenance instead of reusing the
    // source associated with the original lifecycle write.
    #[allow(clippy::too_many_arguments)]
    pub fn update_memory_as_user(
        &self,
        id: &str,
        content: &str,
        kind: &str,
        importance: f64,
        confidence: f64,
        timestamp: i64,
    ) -> rusqlite::Result<Memory> {
        let changed = self
            .connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "UPDATE memories SET content = ?1, kind = ?2, importance = ?3,
                    confidence = ?4, updated_at = ?5, source_type = 'user_correction',
                    source_ref = 'memory-inspector'
                 WHERE id = ?6",
                params![content, kind, importance, confidence, timestamp, id],
            )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        self.get_memory(id)
    }

    pub fn set_memory_status(
        &self,
        id: &str,
        status: &str,
        timestamp: i64,
    ) -> rusqlite::Result<()> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "UPDATE memories SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status, timestamp, id],
            )?;
        Ok(())
    }

    pub fn pin_memory(&self, id: &str, timestamp: i64) -> rusqlite::Result<Memory> {
        let changed = self
            .connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "UPDATE memories
                 SET pinned = 1, importance = MIN(1.0, importance + 0.2), updated_at = ?1
                 WHERE id = ?2 AND pinned = 0",
                params![timestamp, id],
            )?;
        if changed == 0 {
            // An already-pinned memory is a successful idempotent request. A missing
            // memory still returns QueryReturnedNoRows through get_memory.
            return self.get_memory(id);
        }
        self.get_memory(id)
    }

    pub fn expire_memories(&self, timestamp: i64) -> rusqlite::Result<Vec<String>> {
        let mut connection = self.connection.lock().expect("database lock poisoned");
        let transaction = connection.transaction()?;
        let ids = {
            let mut statement = transaction.prepare(
                "SELECT id FROM memories
                 WHERE status = 'active' AND valid_to IS NOT NULL AND valid_to <= ?1",
            )?;
            statement
                .query_map([timestamp], |row| row.get(0))?
                .collect::<rusqlite::Result<Vec<String>>>()?
        };
        transaction.execute(
            "UPDATE memories SET status = 'outdated', updated_at = ?1
             WHERE status = 'active' AND valid_to IS NOT NULL AND valid_to <= ?1",
            [timestamp],
        )?;
        transaction.commit()?;
        Ok(ids)
    }

    pub fn insert_memory_relation(&self, relation: &MemoryRelation) -> rusqlite::Result<()> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "INSERT OR IGNORE INTO memory_relations
                 (source_id, target_id, relation, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    relation.source_id,
                    relation.target_id,
                    relation.relation,
                    relation.created_at
                ],
            )?;
        Ok(())
    }

    pub fn list_memory_relations(&self) -> rusqlite::Result<Vec<MemoryRelation>> {
        let connection = self.connection.lock().expect("database lock poisoned");
        let mut statement = connection.prepare(
            "SELECT source_id, target_id, relation, created_at
             FROM memory_relations ORDER BY created_at DESC",
        )?;
        statement
            .query_map([], |row| {
                Ok(MemoryRelation {
                    source_id: row.get(0)?,
                    target_id: row.get(1)?,
                    relation: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect()
    }

    pub fn list_memory_relations_for(
        &self,
        memory_id: &str,
    ) -> rusqlite::Result<Vec<MemoryRelation>> {
        let connection = self.connection.lock().expect("database lock poisoned");
        let mut statement = connection.prepare(
            "SELECT source_id, target_id, relation, created_at
             FROM memory_relations
             WHERE source_id = ?1 OR target_id = ?1
             ORDER BY created_at DESC",
        )?;
        statement
            .query_map([memory_id], |row| {
                Ok(MemoryRelation {
                    source_id: row.get(0)?,
                    target_id: row.get(1)?,
                    relation: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect()
    }

    pub fn touch_memories(&self, ids: &[String], timestamp: i64) -> rusqlite::Result<()> {
        let mut connection = self.connection.lock().expect("database lock poisoned");
        let transaction = connection.transaction()?;
        for id in ids {
            transaction.execute(
                "UPDATE memories SET last_accessed_at = ?1, access_count = access_count + 1
                 WHERE id = ?2",
                params![timestamp, id],
            )?;
        }
        transaction.commit()
    }

    pub fn insert_memory_operation(&self, operation: &MemoryOperation) -> rusqlite::Result<()> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "INSERT INTO memory_operations
             (id, memory_id, operation, timestamp, source_event_id, reason_label,
              related_memory_ids_json, detail_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    operation.id,
                    operation.memory_id,
                    operation.operation,
                    operation.timestamp,
                    operation.source_event_id,
                    operation.reason_label,
                    operation.related_memory_ids_json,
                    operation.detail_json,
                ],
            )?;
        Ok(())
    }

    pub fn list_memory_operations(&self, limit: usize) -> rusqlite::Result<Vec<MemoryOperation>> {
        let connection = self.connection.lock().expect("database lock poisoned");
        let mut statement = connection.prepare(
            "SELECT id, memory_id, operation, timestamp, source_event_id, reason_label,
                    related_memory_ids_json, detail_json
             FROM memory_operations ORDER BY timestamp DESC, rowid DESC LIMIT ?1",
        )?;
        statement
            .query_map([limit.clamp(1, 500) as i64], |row| {
                Ok(MemoryOperation {
                    id: row.get(0)?,
                    memory_id: row.get(1)?,
                    operation: row.get(2)?,
                    timestamp: row.get(3)?,
                    source_event_id: row.get(4)?,
                    reason_label: row.get(5)?,
                    related_memory_ids_json: row.get(6)?,
                    detail_json: row.get(7)?,
                })
            })?
            .collect()
    }

    pub fn list_memory_operations_for(
        &self,
        memory_id: &str,
        limit: usize,
    ) -> rusqlite::Result<Vec<MemoryOperation>> {
        let connection = self.connection.lock().expect("database lock poisoned");
        let related_pattern = format!("%\"{memory_id}\"%");
        let mut statement = connection.prepare(
            "SELECT id, memory_id, operation, timestamp, source_event_id, reason_label,
                    related_memory_ids_json, detail_json
             FROM memory_operations
             WHERE memory_id = ?1 OR related_memory_ids_json LIKE ?2
             ORDER BY timestamp DESC, rowid DESC LIMIT ?3",
        )?;
        statement
            .query_map(
                params![memory_id, related_pattern, limit.clamp(1, 500) as i64],
                |row| {
                    Ok(MemoryOperation {
                        id: row.get(0)?,
                        memory_id: row.get(1)?,
                        operation: row.get(2)?,
                        timestamp: row.get(3)?,
                        source_event_id: row.get(4)?,
                        reason_label: row.get(5)?,
                        related_memory_ids_json: row.get(6)?,
                        detail_json: row.get(7)?,
                    })
                },
            )?
            .collect()
    }

    /// Physically removes a memory and every relation/operation that refers to it,
    /// then writes a privacy-safe USER_DELETE trace that has no foreign-key link.
    pub fn delete_memory_completely(
        &self,
        memory_id: &str,
        delete_operation: &MemoryOperation,
    ) -> rusqlite::Result<(usize, usize)> {
        let mut connection = self.connection.lock().expect("database lock poisoned");
        let transaction = connection.transaction()?;
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM memories WHERE id = ?1)",
            [memory_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        let relations_deleted = transaction.execute(
            "DELETE FROM memory_relations WHERE source_id = ?1 OR target_id = ?1",
            [memory_id],
        )?;
        let related_pattern = format!("%\"{memory_id}\"%");
        let operations_deleted = transaction.execute(
            "DELETE FROM memory_operations
             WHERE memory_id = ?1 OR related_memory_ids_json LIKE ?2",
            params![memory_id, related_pattern],
        )?;
        transaction.execute("DELETE FROM memories WHERE id = ?1", [memory_id])?;
        transaction.execute(
            "INSERT INTO memory_operations
             (id, memory_id, operation, timestamp, source_event_id, reason_label,
              related_memory_ids_json, detail_json)
             VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                delete_operation.id,
                delete_operation.operation,
                delete_operation.timestamp,
                delete_operation.source_event_id,
                delete_operation.reason_label,
                delete_operation.related_memory_ids_json,
                delete_operation.detail_json,
            ],
        )?;
        transaction.commit()?;
        Ok((relations_deleted, operations_deleted))
    }

    /// Replaces portable Companion state in one SQLite transaction. Provider rows,
    /// conversations, pet position, and Agent/LLM traces stay local.
    pub fn replace_companion_profile(&self, profile: &CompanionProfile) -> rusqlite::Result<()> {
        let mut connection = self.connection.lock().expect("database lock poisoned");
        let transaction = connection.transaction()?;

        transaction.execute("DELETE FROM memory_operations", [])?;
        transaction.execute("DELETE FROM memory_relations", [])?;
        transaction.execute("DELETE FROM memories", [])?;
        for memory in &profile.memories {
            transaction.execute(
                "INSERT INTO memories
                 (id, kind, content, importance, pinned, confidence, status, created_at,
                  updated_at, last_accessed_at, access_count, valid_from, valid_to,
                  source_type, source_ref)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    memory.id,
                    memory.kind,
                    memory.content,
                    memory.importance,
                    memory.pinned,
                    memory.confidence,
                    memory.status,
                    memory.created_at,
                    memory.updated_at,
                    memory.last_accessed_at,
                    memory.access_count,
                    memory.valid_from,
                    memory.valid_to,
                    memory.source_type,
                    memory.source_ref,
                ],
            )?;
        }
        for relation in &profile.relations {
            transaction.execute(
                "INSERT INTO memory_relations
                 (source_id, target_id, relation, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    relation.source_id,
                    relation.target_id,
                    relation.relation,
                    relation.created_at,
                ],
            )?;
        }

        let settings = &profile.behavior;
        transaction.execute(
            "UPDATE app_settings SET pet_name = ?1, pet_size = ?2, auto_wander = ?3,
                wander_interval_seconds = ?4, movement_speed = ?5,
                proactive_interaction = ?6, agent_heartbeat = ?7,
                agent_heartbeat_interval_seconds = ?8, proactive_cooldown_minutes = ?9,
                max_proactive_messages_per_hour = ?10, do_not_disturb = ?11,
                proactive_frequency = ?12, quiet_hours_enabled = ?13,
                quiet_hours_start = ?14, quiet_hours_end = ?15 WHERE id = 1",
            params![
                settings.pet_name,
                settings.pet_size,
                settings.auto_wander,
                settings.wander_interval_seconds,
                settings.movement_speed,
                settings.proactive_interaction,
                settings.agent_heartbeat,
                settings.agent_heartbeat_interval_seconds,
                settings.proactive_cooldown_minutes,
                settings.max_proactive_messages_per_hour,
                settings.do_not_disturb,
                settings.proactive_frequency,
                settings.quiet_hours_enabled,
                settings.quiet_hours_start,
                settings.quiet_hours_end,
            ],
        )?;
        transaction.execute(
            "INSERT INTO agent_metadata (key, value) VALUES ('soul_version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [profile.soul_version.to_string()],
        )?;
        transaction.execute(
            "INSERT INTO agent_metadata (key, value) VALUES ('active_pet_pack_id', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [&profile.active_pet_pack_id],
        )?;
        transaction.commit()
    }

    pub fn get_soul_version(&self) -> rusqlite::Result<i64> {
        let value: Option<String> = self
            .connection
            .lock()
            .expect("database lock poisoned")
            .query_row(
                "SELECT value FROM agent_metadata WHERE key = 'soul_version'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value.and_then(|value| value.parse().ok()).unwrap_or(1))
    }

    pub fn increment_soul_version(&self) -> rusqlite::Result<i64> {
        let next = self.get_soul_version()?.saturating_add(1);
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "INSERT INTO agent_metadata (key, value) VALUES ('soul_version', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [next.to_string()],
            )?;
        Ok(next)
    }

    pub fn get_active_pet_pack_id(&self) -> rusqlite::Result<String> {
        let value: Option<String> = self
            .connection
            .lock()
            .expect("database lock poisoned")
            .query_row(
                "SELECT value FROM agent_metadata WHERE key = 'active_pet_pack_id'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value.unwrap_or_else(|| "remi-hanfu-v1".to_string()))
    }

    pub fn set_active_pet_pack_id(&self, pet_pack_id: &str) -> rusqlite::Result<()> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "INSERT INTO agent_metadata (key, value) VALUES ('active_pet_pack_id', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [pet_pack_id],
            )?;
        Ok(())
    }

    pub fn get_app_settings(&self) -> rusqlite::Result<AppSettings> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .query_row(
                "SELECT pet_name, pet_size, auto_wander, wander_interval_seconds,
                    movement_speed, proactive_interaction, agent_heartbeat,
                    agent_heartbeat_interval_seconds, proactive_cooldown_minutes,
                    max_proactive_messages_per_hour, do_not_disturb, proactive_frequency,
                    quiet_hours_enabled, quiet_hours_start, quiet_hours_end
             FROM app_settings WHERE id = 1",
                [],
                |row| {
                    Ok(AppSettings {
                        pet_name: row.get(0)?,
                        pet_size: row.get(1)?,
                        auto_wander: row.get(2)?,
                        wander_interval_seconds: row.get(3)?,
                        movement_speed: row.get(4)?,
                        proactive_interaction: row.get(5)?,
                        agent_heartbeat: row.get(6)?,
                        agent_heartbeat_interval_seconds: row.get(7)?,
                        proactive_cooldown_minutes: row.get(8)?,
                        max_proactive_messages_per_hour: row.get(9)?,
                        do_not_disturb: row.get(10)?,
                        proactive_frequency: row.get(11)?,
                        quiet_hours_enabled: row.get(12)?,
                        quiet_hours_start: row.get(13)?,
                        quiet_hours_end: row.get(14)?,
                    })
                },
            )
    }

    pub fn save_app_settings(&self, settings: &AppSettings) -> rusqlite::Result<()> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "UPDATE app_settings SET pet_name = ?1, pet_size = ?2, auto_wander = ?3,
                    wander_interval_seconds = ?4, movement_speed = ?5,
                    proactive_interaction = ?6, agent_heartbeat = ?7,
                    agent_heartbeat_interval_seconds = ?8, proactive_cooldown_minutes = ?9,
                    max_proactive_messages_per_hour = ?10, do_not_disturb = ?11,
                    proactive_frequency = ?12, quiet_hours_enabled = ?13,
                    quiet_hours_start = ?14, quiet_hours_end = ?15 WHERE id = 1",
                params![
                    settings.pet_name,
                    settings.pet_size,
                    settings.auto_wander,
                    settings.wander_interval_seconds,
                    settings.movement_speed,
                    settings.proactive_interaction,
                    settings.agent_heartbeat,
                    settings.agent_heartbeat_interval_seconds,
                    settings.proactive_cooldown_minutes,
                    settings.max_proactive_messages_per_hour,
                    settings.do_not_disturb,
                    settings.proactive_frequency,
                    settings.quiet_hours_enabled,
                    settings.quiet_hours_start,
                    settings.quiet_hours_end,
                ],
            )?;
        Ok(())
    }

    pub fn runtime_counts(&self) -> rusqlite::Result<(i64, i64, i64, i64, i64)> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .query_row(
                "SELECT
                (SELECT COUNT(*) FROM memories WHERE status = 'active'),
                (SELECT COUNT(*) FROM memories WHERE status = 'active' AND kind = 'semantic'),
                (SELECT COUNT(*) FROM memories WHERE status = 'active' AND kind = 'episodic'),
                (SELECT COUNT(*) FROM memories WHERE status = 'active' AND kind = 'relationship'),
                (SELECT COUNT(*) FROM agent_events) + (SELECT COUNT(*) FROM agent_actions)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
    }

    pub fn load_provider_catalog(&self) -> rusqlite::Result<ProviderCatalog> {
        let connection = self.connection.lock().expect("database lock poisoned");
        let mut provider_statement = connection.prepare(
            "SELECT id, name, provider_type, base_url, enabled
             FROM providers ORDER BY created_at ASC, name ASC",
        )?;
        let provider_rows = provider_statement.query_map([], |row| {
            Ok(ProviderConfig {
                id: row.get(0)?,
                display_name: row.get(1)?,
                provider_type: row.get(2)?,
                base_url: row.get(3)?,
                enabled: row.get(4)?,
                has_api_key: false,
                models: Vec::new(),
            })
        })?;
        let mut providers: Vec<ProviderConfig> = provider_rows.collect::<rusqlite::Result<_>>()?;
        let mut model_statement = connection.prepare(
            "SELECT id, name, model_id, enabled FROM provider_models
             WHERE provider_id = ?1 ORDER BY rowid ASC",
        )?;
        for provider in &mut providers {
            provider.models = model_statement
                .query_map([&provider.id], |row| {
                    Ok(ModelConfig {
                        id: row.get(0)?,
                        display_name: row.get(1)?,
                        model_id: row.get(2)?,
                        enabled: row.get(3)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
        }
        let (active_provider_id, active_model_id) = connection.query_row(
            "SELECT active_provider_id, active_model_id FROM provider_selection WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(ProviderCatalog {
            providers,
            active_provider_id,
            active_model_id,
        })
    }

    pub fn save_provider(&self, provider: &ProviderConfig) -> rusqlite::Result<()> {
        let now = chrono_timestamp_ms();
        let mut connection = self.connection.lock().expect("database lock poisoned");
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO providers
             (id, name, provider_type, base_url, enabled, has_api_key, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name,
                 provider_type = excluded.provider_type, base_url = excluded.base_url,
                 enabled = excluded.enabled, updated_at = excluded.updated_at",
            params![
                provider.id,
                provider.display_name,
                provider.provider_type,
                provider.base_url,
                provider.enabled,
                now,
            ],
        )?;
        transaction.execute(
            "DELETE FROM provider_models WHERE provider_id = ?1",
            [&provider.id],
        )?;
        for model in &provider.models {
            transaction.execute(
                "INSERT INTO provider_models (id, provider_id, name, model_id, enabled)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    model.id,
                    provider.id,
                    model.display_name,
                    model.model_id,
                    model.enabled
                ],
            )?;
        }
        transaction.execute(
            "UPDATE provider_selection SET active_provider_id = NULL, active_model_id = NULL
             WHERE id = 1 AND active_provider_id = ?1 AND (
                 ?2 = 0 OR active_model_id NOT IN (
                     SELECT id FROM provider_models WHERE provider_id = ?1 AND enabled = 1
                 )
             )",
            params![provider.id, provider.enabled],
        )?;
        transaction.commit()
    }

    pub fn delete_provider(&self, provider_id: &str) -> rusqlite::Result<()> {
        let mut connection = self.connection.lock().expect("database lock poisoned");
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM providers WHERE id = ?1", [provider_id])?;
        transaction.execute(
            "UPDATE provider_selection SET active_provider_id = NULL, active_model_id = NULL
             WHERE id = 1 AND active_provider_id = ?1",
            [provider_id],
        )?;
        transaction.commit()
    }

    pub fn set_active_model(&self, provider_id: &str, model_id: &str) -> rusqlite::Result<()> {
        self.connection
            .lock()
            .expect("database lock poisoned")
            .execute(
                "UPDATE provider_selection SET active_provider_id = ?1, active_model_id = ?2
             WHERE id = 1",
                params![provider_id, model_id],
            )?;
        Ok(())
    }
}

fn chrono_timestamp_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn ensure_column(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> rusqlite::Result<()> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let exists = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .any(|name| name == column);
    if !exists {
        connection.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }
    Ok(())
}

fn map_memory(row: &rusqlite::Row<'_>) -> rusqlite::Result<Memory> {
    Ok(Memory {
        id: row.get(0)?,
        kind: row.get(1)?,
        content: row.get(2)?,
        importance: row.get(3)?,
        confidence: row.get(4)?,
        status: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        last_accessed_at: row.get(8)?,
        access_count: row.get(9)?,
        valid_from: row.get(10)?,
        valid_to: row.get(11)?,
        source_type: row.get(12)?,
        source_ref: row.get(13)?,
        pinned: row.get(14)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_observability_tables() {
        let database = Database::in_memory().unwrap();
        let connection = database.connection.lock().unwrap();
        for table in [
            "agent_events",
            "agent_actions",
            "llm_calls",
            "pet_state",
            "conversations",
            "messages",
            "memories",
            "memory_relations",
            "memory_operations",
            "app_settings",
            "providers",
            "provider_models",
            "provider_selection",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "missing table {table}");
        }
    }

    #[test]
    fn persists_validated_pet_state() {
        let database = Database::in_memory().unwrap();
        let state = database
            .update_pet_state(&PetStatePatch {
                energy: Some(200.0),
                mood: Some("curious".to_string()),
                x: Some(840),
                ..PetStatePatch::default()
            })
            .unwrap();
        assert_eq!(state.energy, 100.0);
        assert_eq!(state.mood, "curious");
        assert_eq!(database.get_pet_state().unwrap().x, 840);
    }

    #[test]
    fn persists_working_memory_in_chronological_order() {
        let database = Database::in_memory().unwrap();
        for (id, timestamp) in [("later", 20), ("earlier", 10)] {
            database
                .persist_message(&WorkingMessage {
                    id: id.to_string(),
                    role: "user".to_string(),
                    content: id.to_string(),
                    timestamp,
                })
                .unwrap();
        }
        let messages = database.get_recent_messages(10).unwrap();
        assert_eq!(messages[0].id, "earlier");
        assert_eq!(messages[1].id, "later");
    }

    #[test]
    fn clears_working_conversation_without_deleting_long_term_memory() {
        let database = Database::in_memory().unwrap();
        database
            .persist_message(&WorkingMessage {
                id: "recent-message".to_string(),
                role: "user".to_string(),
                content: "temporary chat".to_string(),
                timestamp: 10,
            })
            .unwrap();
        database
            .insert_memory(&NewMemory {
                id: "durable-memory".to_string(),
                kind: "semantic".to_string(),
                content: "User prefers tea".to_string(),
                importance: 0.8,
                confidence: 1.0,
                created_at: 10,
                valid_from: None,
                valid_to: None,
                source_type: "user_explicit".to_string(),
                source_ref: Some("test".to_string()),
            })
            .unwrap();

        assert_eq!(database.clear_current_conversation(20).unwrap(), 1);
        assert!(database.get_recent_messages(10).unwrap().is_empty());
        assert_eq!(
            database.get_memory("durable-memory").unwrap().status,
            "active"
        );
    }

    #[test]
    fn persists_multiple_providers_models_and_active_selection() {
        let database = Database::in_memory().unwrap();
        for provider_id in ["openai", "deepseek"] {
            database
                .save_provider(&ProviderConfig {
                    id: provider_id.to_string(),
                    display_name: provider_id.to_string(),
                    provider_type: "openai-compatible".to_string(),
                    base_url: format!("https://{provider_id}.example/v1"),
                    enabled: true,
                    has_api_key: false,
                    models: vec![
                        ModelConfig {
                            id: format!("{provider_id}-chat"),
                            display_name: "Chat".to_string(),
                            model_id: "chat-model".to_string(),
                            enabled: true,
                        },
                        ModelConfig {
                            id: format!("{provider_id}-reasoner"),
                            display_name: "Reasoner".to_string(),
                            model_id: "reasoner-model".to_string(),
                            enabled: true,
                        },
                    ],
                })
                .unwrap();
        }
        database
            .set_active_model("deepseek", "deepseek-reasoner")
            .unwrap();

        let catalog = database.load_provider_catalog().unwrap();
        assert_eq!(catalog.providers.len(), 2);
        assert!(
            catalog
                .providers
                .iter()
                .all(|provider| provider.models.len() == 2)
        );
        assert_eq!(catalog.active_provider_id.as_deref(), Some("deepseek"));
        assert_eq!(
            catalog.active_model_id.as_deref(),
            Some("deepseek-reasoner")
        );
    }
}
