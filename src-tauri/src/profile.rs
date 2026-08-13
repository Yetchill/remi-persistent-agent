use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::{
    database::Database,
    memory::{Memory, MemoryRelation},
    provider::{ModelConfig, ProviderCatalog},
    settings::AppSettings,
    state::AppState,
};

const PROFILE_FORMAT_VERSION: u32 = 1;
const MAX_PROFILE_BYTES: u64 = 25 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileProviderMetadata {
    pub provider_id: Option<String>,
    pub provider_name: Option<String>,
    pub provider_type: Option<String>,
    pub model_id: Option<String>,
    pub model_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipState {
    pub available: bool,
    pub summary: String,
    pub memory_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionProfile {
    pub format_version: u32,
    pub agent_name: String,
    pub soul: String,
    pub soul_version: i64,
    pub created_at: i64,
    pub memories: Vec<Memory>,
    pub relations: Vec<MemoryRelation>,
    pub relationship_state: RelationshipState,
    pub behavior: AppSettings,
    pub active_pet_pack_id: String,
    pub provider: ProfileProviderMetadata,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePreview {
    pub format_version: u32,
    pub agent_name: String,
    pub soul_version: i64,
    pub created_at: i64,
    pub memory_count: usize,
    pub relationship_available: bool,
    pub relationship_memory_count: usize,
    pub active_pet_pack_id: String,
    pub provider: ProfileProviderMetadata,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileExportResult {
    pub path: String,
    pub preview: ProfilePreview,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileImportResult {
    pub backup_path: String,
    pub preview: ProfilePreview,
}

fn timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn safe_file_stem(value: &str) -> String {
    let stem: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let stem = stem.trim_matches('-');
    if stem.is_empty() {
        "remi".to_string()
    } else {
        stem.chars().take(60).collect()
    }
}

fn active_provider_metadata(catalog: &ProviderCatalog) -> ProfileProviderMetadata {
    let provider = catalog
        .active_provider_id
        .as_deref()
        .and_then(|id| catalog.providers.iter().find(|provider| provider.id == id));
    let model: Option<&ModelConfig> = provider.and_then(|provider| {
        catalog
            .active_model_id
            .as_deref()
            .and_then(|id| provider.models.iter().find(|model| model.id == id))
    });
    ProfileProviderMetadata {
        provider_id: provider.map(|provider| provider.id.clone()),
        provider_name: provider.map(|provider| provider.display_name.clone()),
        provider_type: provider.map(|provider| provider.provider_type.clone()),
        model_id: model.map(|model| model.model_id.clone()),
        model_name: model.map(|model| model.display_name.clone()),
    }
}

fn relationship_state(memories: &[Memory]) -> RelationshipState {
    let relationship_memories = memories
        .iter()
        .filter(|memory| memory.kind == "relationship")
        .collect::<Vec<_>>();
    RelationshipState {
        available: !relationship_memories.is_empty(),
        summary: relationship_memories
            .iter()
            .filter(|memory| memory.status == "active")
            .take(3)
            .map(|memory| memory.content.as_str())
            .collect::<Vec<_>>()
            .join("; "),
        memory_ids: relationship_memories
            .iter()
            .map(|memory| memory.id.clone())
            .collect(),
    }
}

fn build_profile(
    database: &Database,
    soul_path: &Path,
    provider_catalog: &ProviderCatalog,
    now: i64,
) -> Result<CompanionProfile, String> {
    let soul = fs::read_to_string(soul_path).map_err(|error| error.to_string())?;
    let behavior = database
        .get_app_settings()
        .map_err(|error| error.to_string())?;
    let memories = database
        .list_memories(None, None)
        .map_err(|error| error.to_string())?;
    let relations = database
        .list_memory_relations()
        .map_err(|error| error.to_string())?;
    Ok(CompanionProfile {
        format_version: PROFILE_FORMAT_VERSION,
        agent_name: behavior.pet_name.clone(),
        soul,
        soul_version: database
            .get_soul_version()
            .map_err(|error| error.to_string())?,
        created_at: now,
        relationship_state: relationship_state(&memories),
        memories,
        relations,
        behavior,
        active_pet_pack_id: database
            .get_active_pet_pack_id()
            .map_err(|error| error.to_string())?,
        provider: active_provider_metadata(provider_catalog),
    })
}

fn preview(profile: &CompanionProfile) -> ProfilePreview {
    ProfilePreview {
        format_version: profile.format_version,
        agent_name: profile.agent_name.clone(),
        soul_version: profile.soul_version,
        created_at: profile.created_at,
        memory_count: profile.memories.len(),
        relationship_available: profile.relationship_state.available,
        relationship_memory_count: profile
            .memories
            .iter()
            .filter(|memory| memory.kind == "relationship")
            .count(),
        active_pet_pack_id: profile.active_pet_pack_id.clone(),
        provider: profile.provider.clone(),
    }
}

fn validate_profile(profile: &mut CompanionProfile) -> Result<(), String> {
    if profile.format_version != PROFILE_FORMAT_VERSION {
        return Err(format!(
            "Unsupported Companion Profile format version {}",
            profile.format_version
        ));
    }
    if profile.soul.trim().is_empty() || profile.soul.len() > 100_000 {
        return Err("Companion Profile contains an invalid SOUL document".to_string());
    }
    if profile.memories.len() > 100_000 || profile.relations.len() > 500_000 {
        return Err("Companion Profile contains too many memory records".to_string());
    }
    if profile.soul_version < 1 {
        return Err("Companion Profile soul version must be positive".to_string());
    }
    if profile.active_pet_pack_id.is_empty()
        || profile.active_pet_pack_id.len() > 80
        || !profile
            .active_pet_pack_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("Companion Profile contains an invalid Pet Pack preference".to_string());
    }
    profile.behavior.validate()?;

    let ids = profile
        .memories
        .iter()
        .map(|memory| memory.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    if ids.len() != profile.memories.len() {
        return Err("Companion Profile contains duplicate memory ids".to_string());
    }
    for memory in &profile.memories {
        if memory.id.trim().is_empty()
            || memory.content.trim().is_empty()
            || !matches!(
                memory.kind.as_str(),
                "semantic" | "episodic" | "relationship"
            )
            || !matches!(
                memory.status.as_str(),
                "active" | "outdated" | "archived" | "merged"
            )
            || !(0.0..=1.0).contains(&memory.importance)
            || !(0.0..=1.0).contains(&memory.confidence)
        {
            return Err(format!(
                "Companion Profile contains invalid memory {}",
                memory.id
            ));
        }
    }
    let mut relation_keys = std::collections::HashSet::new();
    for relation in &profile.relations {
        if !ids.contains(relation.source_id.as_str())
            || !ids.contains(relation.target_id.as_str())
            || !matches!(
                relation.relation.as_str(),
                "supports" | "contradicts" | "supersedes" | "derived_from" | "merged_into"
            )
        {
            return Err("Companion Profile contains an invalid memory relation".to_string());
        }
        if !relation_keys.insert((
            relation.source_id.as_str(),
            relation.target_id.as_str(),
            relation.relation.as_str(),
        )) {
            return Err("Companion Profile contains duplicate memory relations".to_string());
        }
    }
    profile.relationship_state = relationship_state(&profile.memories);
    profile.agent_name = profile.behavior.pet_name.clone();
    Ok(())
}

fn read_profile_json(profile_json: &str) -> Result<CompanionProfile, String> {
    if profile_json.len() as u64 > MAX_PROFILE_BYTES {
        return Err("Companion Profile must be smaller than 25 MB".to_string());
    }
    let mut value: serde_json::Value = serde_json::from_str(profile_json)
        .map_err(|error| format!("Invalid profile JSON: {error}"))?;
    // Format v1 profiles created before the behavior controls were finalized can
    // still be restored with the conservative application defaults.
    if let Some(behavior) = value
        .get_mut("behavior")
        .and_then(|value| value.as_object_mut())
    {
        behavior
            .entry("proactiveFrequency")
            .or_insert_with(|| serde_json::Value::String("normal".to_string()));
        behavior
            .entry("quietHoursEnabled")
            .or_insert(serde_json::Value::Bool(false));
        behavior
            .entry("quietHoursStart")
            .or_insert_with(|| serde_json::Value::String("23:00".to_string()));
        behavior
            .entry("quietHoursEnd")
            .or_insert_with(|| serde_json::Value::String("08:00".to_string()));
    }
    if let Some(memories) = value
        .get_mut("memories")
        .and_then(|value| value.as_array_mut())
    {
        for memory in memories {
            if let Some(memory) = memory.as_object_mut() {
                memory
                    .entry("pinned")
                    .or_insert(serde_json::Value::Bool(false));
            }
        }
    }
    let mut profile: CompanionProfile =
        serde_json::from_value(value).map_err(|error| format!("Invalid profile data: {error}"))?;
    validate_profile(&mut profile)?;
    Ok(profile)
}

fn write_profile(path: &Path, profile: &CompanionProfile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(profile).map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    fs::rename(&temporary, path).map_err(|error| error.to_string())
}

fn default_export_path(soul_path: &Path, agent_name: &str, now: i64) -> PathBuf {
    soul_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("profile-exports")
        .join(format!("{}-profile-{now}.json", safe_file_stem(agent_name)))
}

fn backup_path(soul_path: &Path, now: i64) -> PathBuf {
    soul_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("profile-backups")
        .join(format!("remi-profile-backup-{now}.json"))
}

fn apply_profile(
    database: &Database,
    soul_path: &Path,
    profile: &CompanionProfile,
    previous: &CompanionProfile,
) -> Result<(), String> {
    let temporary_soul = soul_path.with_extension(format!("{}.import.tmp", Uuid::new_v4()));
    fs::write(&temporary_soul, profile.soul.as_bytes()).map_err(|error| error.to_string())?;
    if let Err(error) = database.replace_companion_profile(profile) {
        let _ = fs::remove_file(&temporary_soul);
        return Err(error.to_string());
    }
    if let Err(error) = fs::rename(&temporary_soul, soul_path) {
        let rollback = database.replace_companion_profile(previous);
        let _ = fs::write(soul_path, previous.soul.as_bytes());
        return match rollback {
            Ok(()) => Err(format!("Could not replace SOUL document: {error}")),
            Err(rollback_error) => Err(format!(
                "Could not replace SOUL document ({error}); database rollback also failed ({rollback_error})"
            )),
        };
    }
    Ok(())
}

#[tauri::command]
pub fn export_companion_profile(
    state: State<'_, AppState>,
    destination_path: Option<String>,
) -> Result<ProfileExportResult, String> {
    let now = timestamp_ms();
    let catalog = state
        .provider
        .read()
        .map_err(|_| "Provider lock poisoned")?
        .catalog
        .clone();
    let profile = build_profile(&state.database, &state.soul_path, &catalog, now)?;
    let path = destination_path
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| default_export_path(&state.soul_path, &profile.agent_name, now));
    write_profile(&path, &profile)?;
    Ok(ProfileExportResult {
        path: path.to_string_lossy().into_owned(),
        preview: preview(&profile),
    })
}

#[tauri::command]
pub fn preview_companion_profile(profile_json: String) -> Result<ProfilePreview, String> {
    read_profile_json(&profile_json).map(|profile| preview(&profile))
}

#[tauri::command]
pub fn import_companion_profile(
    state: State<'_, AppState>,
    profile_json: String,
    confirm_replace: bool,
) -> Result<ProfileImportResult, String> {
    if !confirm_replace {
        return Err("Profile replacement was not confirmed".to_string());
    }
    let imported = read_profile_json(&profile_json)?;
    let imported_preview = preview(&imported);
    let now = timestamp_ms();
    let catalog = state
        .provider
        .read()
        .map_err(|_| "Provider lock poisoned")?
        .catalog
        .clone();
    let current = build_profile(&state.database, &state.soul_path, &catalog, now)?;
    let backup = backup_path(&state.soul_path, now);
    write_profile(&backup, &current)?;
    apply_profile(&state.database, &state.soul_path, &imported, &current)?;
    Ok(ProfileImportResult {
        backup_path: backup.to_string_lossy().into_owned(),
        preview: imported_preview,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{memory::NewMemory, provider::ProviderConfig};

    fn write_memory(database: &Database, id: &str, content: &str, kind: &str) {
        database
            .insert_memory(&NewMemory {
                id: id.to_string(),
                kind: kind.to_string(),
                content: content.to_string(),
                importance: 0.8,
                confidence: 1.0,
                created_at: 100,
                valid_from: None,
                valid_to: None,
                source_type: "user_explicit".to_string(),
                source_ref: Some("test-event".to_string()),
            })
            .unwrap();
    }

    #[test]
    fn export_contains_profile_state_but_no_provider_secret() {
        let database = Database::in_memory().unwrap();
        write_memory(
            &database,
            "relationship-1",
            "The user named me Remi",
            "relationship",
        );
        let directory = std::env::temp_dir().join(format!("remi-profile-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let soul_path = directory.join("SOUL.md");
        fs::write(&soul_path, "# Identity\nName: Remi\n").unwrap();
        let provider = ProviderConfig {
            id: "provider-1".to_string(),
            display_name: "Test Provider".to_string(),
            provider_type: "openai-compatible".to_string(),
            base_url: "https://example.test/v1".to_string(),
            enabled: true,
            has_api_key: true,
            models: vec![ModelConfig {
                id: "model-config-1".to_string(),
                display_name: "Test Model".to_string(),
                model_id: "test-model".to_string(),
                enabled: true,
            }],
        };
        let catalog = ProviderCatalog {
            providers: vec![provider],
            active_provider_id: Some("provider-1".to_string()),
            active_model_id: Some("model-config-1".to_string()),
        };
        let profile = build_profile(&database, &soul_path, &catalog, 123).unwrap();
        let serialized = serde_json::to_string_pretty(&profile).unwrap();

        assert_eq!(profile.memories.len(), 1);
        assert!(profile.relationship_state.available);
        assert!(serialized.contains("test-model"));
        assert!(!serialized.contains("baseUrl"));
        assert!(!serialized.contains("hasApiKey"));
        assert!(!serialized.to_lowercase().contains("api_key"));
        assert!(!serialized.to_lowercase().contains("apikey"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn exported_profile_restores_soul_memory_relationship_and_preferences() {
        let database = Database::in_memory().unwrap();
        write_memory(
            &database,
            "semantic-1",
            "User currently prefers tea",
            "semantic",
        );
        write_memory(
            &database,
            "relationship-1",
            "The user named the companion Remi",
            "relationship",
        );
        database
            .insert_memory_relation(&MemoryRelation {
                source_id: "relationship-1".to_string(),
                target_id: "semantic-1".to_string(),
                relation: "supports".to_string(),
                created_at: 200,
            })
            .unwrap();
        let mut settings = database.get_app_settings().unwrap();
        settings.pet_name = "Remi Original".to_string();
        settings.proactive_interaction = true;
        settings.quiet_hours_enabled = true;
        database.save_app_settings(&settings).unwrap();
        database.set_active_pet_pack_id("original-pack").unwrap();

        let directory =
            std::env::temp_dir().join(format!("remi-profile-restore-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let soul_path = directory.join("SOUL.md");
        fs::write(&soul_path, "# Original Soul\n").unwrap();
        let original =
            build_profile(&database, &soul_path, &ProviderCatalog::default(), 300).unwrap();
        let exported_json = serde_json::to_string_pretty(&original).unwrap();

        fs::write(&soul_path, "# Locally Changed Soul\n").unwrap();
        database
            .set_memory_status("semantic-1", "archived", 400)
            .unwrap();
        write_memory(
            &database,
            "local-only",
            "This state should be replaced",
            "semantic",
        );
        let mut changed_settings = database.get_app_settings().unwrap();
        changed_settings.pet_name = "Changed Companion".to_string();
        changed_settings.proactive_interaction = false;
        changed_settings.quiet_hours_enabled = false;
        database.save_app_settings(&changed_settings).unwrap();
        database.set_active_pet_pack_id("changed-pack").unwrap();
        database.increment_soul_version().unwrap();
        let changed =
            build_profile(&database, &soul_path, &ProviderCatalog::default(), 500).unwrap();

        let backup = directory.join("backup.json");
        write_profile(&backup, &changed).unwrap();
        let imported = read_profile_json(&exported_json).unwrap();
        apply_profile(&database, &soul_path, &imported, &changed).unwrap();

        assert_eq!(fs::read_to_string(&soul_path).unwrap(), "# Original Soul\n");
        assert_eq!(database.get_soul_version().unwrap(), original.soul_version);
        assert_eq!(database.get_active_pet_pack_id().unwrap(), "original-pack");
        let restored_settings = database.get_app_settings().unwrap();
        assert_eq!(restored_settings.pet_name, "Remi Original");
        assert!(restored_settings.proactive_interaction);
        assert!(restored_settings.quiet_hours_enabled);
        let memories = database.list_memories(None, None).unwrap();
        assert_eq!(memories.len(), 2);
        assert!(
            memories
                .iter()
                .any(|memory| { memory.id == "semantic-1" && memory.status == "active" })
        );
        assert!(!memories.iter().any(|memory| memory.id == "local-only"));
        assert_eq!(database.list_memory_relations().unwrap().len(), 1);

        let backup_json = fs::read_to_string(backup).unwrap();
        let restored_backup = read_profile_json(&backup_json).unwrap();
        assert_eq!(restored_backup.agent_name, "Changed Companion");
        assert!(
            restored_backup
                .memories
                .iter()
                .any(|memory| memory.id == "local-only")
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn profile_replace_rolls_back_the_database_on_relation_failure() {
        let database = Database::in_memory().unwrap();
        write_memory(&database, "original", "Original durable fact", "semantic");
        let directory =
            std::env::temp_dir().join(format!("remi-profile-transaction-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let soul_path = directory.join("SOUL.md");
        fs::write(&soul_path, "# Soul\n").unwrap();
        let mut invalid =
            build_profile(&database, &soul_path, &ProviderCatalog::default(), 100).unwrap();
        invalid.relations = vec![
            MemoryRelation {
                source_id: "original".to_string(),
                target_id: "original".to_string(),
                relation: "supports".to_string(),
                created_at: 1,
            },
            MemoryRelation {
                source_id: "original".to_string(),
                target_id: "original".to_string(),
                relation: "supports".to_string(),
                created_at: 2,
            },
        ];
        invalid.behavior.pet_name = "Should Roll Back".to_string();

        assert!(database.replace_companion_profile(&invalid).is_err());
        assert_eq!(database.get_app_settings().unwrap().pet_name, "Remi");
        let memories = database.list_memories(None, None).unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].id, "original");
        assert!(database.list_memory_relations().unwrap().is_empty());
        let _ = fs::remove_dir_all(directory);
    }
}
