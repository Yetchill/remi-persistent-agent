use std::{fs, path::Path};

use serde::Serialize;
use tauri::State;

use crate::state::AppState;

const DEFAULT_SOUL: &str = include_str!("../../src/soul/SOUL.md");

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoulDocument {
    content: String,
    soul_version: i64,
}

pub fn ensure_soul_exists(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        fs::write(path, DEFAULT_SOUL)?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_soul(state: State<'_, AppState>) -> Result<SoulDocument, String> {
    let content = fs::read_to_string(&state.soul_path).map_err(|error| error.to_string())?;
    let soul_version = state
        .database
        .get_soul_version()
        .map_err(|error| error.to_string())?;
    Ok(SoulDocument {
        content,
        soul_version,
    })
}

#[tauri::command]
pub fn update_soul(state: State<'_, AppState>, content: String) -> Result<SoulDocument, String> {
    let content = content.trim();
    if content.is_empty() {
        return Err("SOUL.md cannot be empty".to_string());
    }
    if content.len() > 100_000 {
        return Err("SOUL.md is too large".to_string());
    }
    let temporary = state.soul_path.with_extension("md.tmp");
    fs::write(&temporary, format!("{content}\n")).map_err(|error| error.to_string())?;
    fs::rename(&temporary, &state.soul_path).map_err(|error| error.to_string())?;
    state
        .database
        .increment_soul_version()
        .map_err(|error| error.to_string())?;
    get_soul(state)
}
