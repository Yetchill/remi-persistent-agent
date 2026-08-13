use serde::Deserialize;
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTrace {
    pub id: String,
    pub event_type: String,
    pub source: String,
    pub timestamp: i64,
    pub payload_json: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionTrace {
    pub id: String,
    pub event_id: String,
    pub action_type: String,
    pub timestamp: i64,
    pub payload_json: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

pub struct LlmCallTrace {
    pub id: String,
    pub event_id: String,
    pub provider: String,
    pub model: String,
    pub event_type: String,
    pub timestamp: i64,
    pub request_json: String,
    pub response_json: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub latency_ms: i64,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionTraceSummary {
    pub action_type: String,
    pub timestamp: i64,
    pub payload_json: Option<String>,
    pub success: bool,
    pub reason: Option<String>,
}

#[tauri::command]
pub fn trace_event(state: State<'_, AppState>, trace: EventTrace) -> Result<(), String> {
    state
        .database
        .insert_event(&trace)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn trace_action(state: State<'_, AppState>, trace: ActionTrace) -> Result<(), String> {
    state
        .database
        .insert_action(&trace)
        .map_err(|error| error.to_string())
}
