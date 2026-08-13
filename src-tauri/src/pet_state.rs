use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

const MOODS: &[&str] = &["neutral", "happy", "sleepy", "curious", "sad"];
const ACTIVITIES: &[&str] = &["idle", "wandering", "sleeping", "talking", "thinking"];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetState {
    pub energy: f64,
    pub boredom: f64,
    pub mood: String,
    pub activity: String,
    pub current_goal: Option<String>,
    pub x: i32,
    pub y: i32,
    pub opacity: f64,
    pub last_user_interaction_at: Option<i64>,
    pub last_agent_interaction_at: Option<i64>,
    pub last_heartbeat_at: Option<i64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetStatePatch {
    pub energy: Option<f64>,
    pub boredom: Option<f64>,
    pub mood: Option<String>,
    pub activity: Option<String>,
    pub current_goal: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub opacity: Option<f64>,
    pub last_user_interaction_at: Option<i64>,
    pub last_agent_interaction_at: Option<i64>,
    pub last_heartbeat_at: Option<i64>,
}

impl PetState {
    pub fn apply(&mut self, patch: &PetStatePatch) {
        if let Some(value) = patch.energy {
            self.energy = value.clamp(0.0, 100.0);
        }
        if let Some(value) = patch.boredom {
            self.boredom = value.clamp(0.0, 100.0);
        }
        if let Some(value) = patch.mood.as_deref().filter(|value| MOODS.contains(value)) {
            self.mood = value.to_string();
        }
        if let Some(value) = patch
            .activity
            .as_deref()
            .filter(|value| ACTIVITIES.contains(value))
        {
            self.activity = value.to_string();
        }
        if let Some(value) = &patch.current_goal {
            self.current_goal = Some(value.trim().chars().take(500).collect());
        }
        if let Some(value) = patch.x {
            self.x = value;
        }
        if let Some(value) = patch.y {
            self.y = value;
        }
        if let Some(value) = patch.opacity {
            self.opacity = value.clamp(0.2, 1.0);
        }
        if patch.last_user_interaction_at.is_some() {
            self.last_user_interaction_at = patch.last_user_interaction_at;
        }
        if patch.last_agent_interaction_at.is_some() {
            self.last_agent_interaction_at = patch.last_agent_interaction_at;
        }
        if patch.last_heartbeat_at.is_some() {
            self.last_heartbeat_at = patch.last_heartbeat_at;
        }
    }
}

#[tauri::command]
pub fn get_pet_state(state: State<'_, AppState>) -> Result<PetState, String> {
    state
        .database
        .get_pet_state()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_pet_state(
    state: State<'_, AppState>,
    patch: PetStatePatch,
) -> Result<PetState, String> {
    state
        .database
        .update_pet_state(&patch)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn persist_pet_position(
    state: State<'_, AppState>,
    x: i32,
    y: i32,
) -> Result<PetState, String> {
    state
        .database
        .update_pet_state(&PetStatePatch {
            x: Some(x),
            y: Some(y),
            ..PetStatePatch::default()
        })
        .map_err(|error| error.to_string())
}
