use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkingMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

impl WorkingMessage {
    fn validate(&self) -> Result<(), String> {
        if !matches!(self.role.as_str(), "user" | "assistant") {
            return Err("Working Memory message role is invalid".to_string());
        }
        if self.content.trim().is_empty() || self.content.len() > 100_000 {
            return Err("Working Memory message content is invalid".to_string());
        }
        Ok(())
    }
}

#[tauri::command]
pub fn persist_message(state: State<'_, AppState>, message: WorkingMessage) -> Result<(), String> {
    message.validate()?;
    state
        .database
        .persist_message(&message)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_recent_messages(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<WorkingMessage>, String> {
    state
        .database
        .get_recent_messages(limit.unwrap_or(50).clamp(1, 100))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_current_conversation(state: State<'_, AppState>) -> Result<usize, String> {
    state
        .database
        .clear_current_conversation(crate::memory::timestamp_ms())
        .map_err(|error| error.to_string())
}
