use serde::{Deserialize, Serialize};
use tauri::State;

use crate::trace::ActionTraceSummary;
use crate::{pet_state::PetState, state::AppState};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub pet_name: String,
    pub pet_size: String,
    pub auto_wander: bool,
    pub wander_interval_seconds: i64,
    pub movement_speed: String,
    pub proactive_interaction: bool,
    pub proactive_frequency: String,
    pub agent_heartbeat: bool,
    pub agent_heartbeat_interval_seconds: i64,
    pub proactive_cooldown_minutes: i64,
    pub max_proactive_messages_per_hour: i64,
    pub do_not_disturb: bool,
    pub quiet_hours_enabled: bool,
    pub quiet_hours_start: String,
    pub quiet_hours_end: String,
}

impl AppSettings {
    pub(crate) fn validate(&mut self) -> Result<(), String> {
        self.pet_name = self.pet_name.trim().chars().take(60).collect();
        if self.pet_name.is_empty() {
            return Err("Pet name is required".to_string());
        }
        if !matches!(self.pet_size.as_str(), "small" | "medium" | "large") {
            return Err("Pet size must be small, medium, or large".to_string());
        }
        self.wander_interval_seconds = self.wander_interval_seconds.clamp(20, 120);
        if !matches!(self.movement_speed.as_str(), "slow" | "normal" | "fast") {
            return Err("Movement speed must be slow, normal, or fast".to_string());
        }
        self.agent_heartbeat_interval_seconds =
            self.agent_heartbeat_interval_seconds.clamp(30, 3_600);
        self.proactive_cooldown_minutes = self.proactive_cooldown_minutes.clamp(5, 240);
        self.max_proactive_messages_per_hour = self.max_proactive_messages_per_hour.clamp(0, 10);
        if !matches!(self.proactive_frequency.as_str(), "low" | "normal" | "high") {
            return Err("Proactive frequency must be low, normal, or high".to_string());
        }
        if !valid_time(&self.quiet_hours_start) || !valid_time(&self.quiet_hours_end) {
            return Err("Quiet hours must use HH:MM format".to_string());
        }
        Ok(())
    }
}

fn valid_time(value: &str) -> bool {
    let Some((hours, minutes)) = value.split_once(':') else {
        return false;
    };
    matches!(
        (hours.parse::<u8>(), minutes.parse::<u8>()),
        (Ok(hours), Ok(minutes)) if hours < 24 && minutes < 60
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOverview {
    pub pet_state: PetState,
    pub memory_count: i64,
    pub semantic_count: i64,
    pub episodic_count: i64,
    pub relationship_count: i64,
    pub trace_count: i64,
    pub last_proactive_action: Option<ActionTraceSummary>,
}

#[tauri::command]
pub fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    state
        .database
        .get_app_settings()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_app_settings(
    state: State<'_, AppState>,
    mut settings: AppSettings,
) -> Result<AppSettings, String> {
    settings.validate()?;
    state
        .database
        .save_app_settings(&settings)
        .map_err(|error| error.to_string())?;
    Ok(settings)
}

#[tauri::command]
pub fn get_runtime_overview(state: State<'_, AppState>) -> Result<RuntimeOverview, String> {
    let pet_state = state
        .database
        .get_pet_state()
        .map_err(|error| error.to_string())?;
    let (memory_count, semantic_count, episodic_count, relationship_count, trace_count) = state
        .database
        .runtime_counts()
        .map_err(|error| error.to_string())?;
    Ok(RuntimeOverview {
        pet_state,
        memory_count,
        semantic_count,
        episodic_count,
        relationship_count,
        trace_count,
        last_proactive_action: state
            .database
            .get_last_heartbeat_action()
            .map_err(|error| error.to_string())?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_simple_behavior_ranges() {
        let mut settings = AppSettings {
            pet_name: " Miku ".to_string(),
            pet_size: "large".to_string(),
            auto_wander: true,
            wander_interval_seconds: 500,
            movement_speed: "fast".to_string(),
            proactive_interaction: false,
            proactive_frequency: "normal".to_string(),
            agent_heartbeat: false,
            agent_heartbeat_interval_seconds: 60,
            proactive_cooldown_minutes: 30,
            max_proactive_messages_per_hour: 2,
            do_not_disturb: false,
            quiet_hours_enabled: false,
            quiet_hours_start: "23:00".to_string(),
            quiet_hours_end: "08:00".to_string(),
        };
        settings.validate().unwrap();
        assert_eq!(settings.pet_name, "Miku");
        assert_eq!(settings.wander_interval_seconds, 120);
    }
}
