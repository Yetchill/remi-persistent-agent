use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::{database::Database, state::AppState};

pub const BUILTIN_PET_PACK_ID: &str = "remi-hanfu-v1";
pub const PET_PACK_CHANGED_EVENT: &str = "pet-pack-changed";
const PET_PACK_DIRECTORY: &str = "pet-packs";
const MANIFEST_FILENAME: &str = "manifest.json";
const STANDARD_STATES: [&str; 4] = ["idle", "talk", "think", "sleep"];

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetPackManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub default_state: String,
    pub states: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub suggested_loops: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPetPackManifest {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    default_state: Option<String>,
    #[serde(default)]
    states: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    suggested_loops: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetPack {
    #[serde(flatten)]
    pub manifest: PetPackManifest,
    /// `builtin` frames are bundled by Vite; `imported` frames live below rootPath.
    pub source: String,
    pub root_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetPackCatalog {
    pub active_pet_pack_id: String,
    pub packs: Vec<PetPack>,
}

fn pack_store_from_state(state: &AppState) -> Result<PathBuf, String> {
    state
        .soul_path
        .parent()
        .map(|directory| directory.join(PET_PACK_DIRECTORY))
        .ok_or_else(|| "Remi app-data directory is unavailable".to_string())
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_pack_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 80
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(
            "Pet Pack id must use 1-80 ASCII letters, numbers, hyphens, or underscores".to_string(),
        );
    }
    Ok(())
}

fn derive_pack_id(folder_name: &str) -> String {
    let mut id = String::new();
    let mut previous_was_separator = false;
    for character in folder_name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            id.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !id.is_empty() && !previous_was_separator {
            id.push('-');
            previous_was_separator = true;
        }
        if id.len() >= 72 {
            break;
        }
    }
    let id = id.trim_matches('-');
    if id.is_empty() {
        format!("pet-pack-{}", &Uuid::new_v4().simple().to_string()[..8])
    } else {
        id.to_string()
    }
}

fn normalize_frame_reference(reference: &str) -> Result<String, String> {
    let reference = reference.trim();
    let path = Path::new(reference);
    if reference.is_empty()
        || reference.contains('\\')
        || path.is_absolute()
        || path.components().any(|component| {
            !matches!(component, Component::Normal(_))
                || component.as_os_str().to_string_lossy().starts_with('.')
        })
        || !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        return Err(format!(
            "Pet Pack frame reference must be a relative PNG path: {reference}"
        ));
    }
    Ok(reference.to_string())
}

fn normalize_frame_map(
    frame_map: BTreeMap<String, Vec<String>>,
    frame_root: Option<&Path>,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut normalized = BTreeMap::new();
    for (state, references) in frame_map {
        let state = state.trim().to_string();
        if state.is_empty() || state.len() > 60 {
            return Err("Pet Pack state names must contain 1-60 characters".to_string());
        }
        let mut frames = Vec::with_capacity(references.len());
        for reference in references {
            let reference = normalize_frame_reference(&reference)?;
            if let Some(root) = frame_root {
                let source = root.join(&reference);
                if !source.is_file() {
                    return Err(format!(
                        "Pet Pack referenced frame does not exist: {reference}"
                    ));
                }
            }
            frames.push(reference);
        }
        normalized.insert(state, frames);
    }
    Ok(normalized)
}

fn normalize_manifest(
    raw: RawPetPackManifest,
    fallback_id: &str,
    frame_root: Option<&Path>,
) -> Result<PetPackManifest, String> {
    let explicit_id = trim_optional(raw.id);
    let id = explicit_id
        .clone()
        .unwrap_or_else(|| derive_pack_id(fallback_id));
    validate_pack_id(&id)?;

    // displayName is accepted for compatibility with the original bundled manifest.
    let name = if explicit_id.is_some() {
        trim_optional(raw.name).or_else(|| trim_optional(raw.display_name))
    } else {
        trim_optional(raw.display_name).or_else(|| trim_optional(raw.name))
    }
    .unwrap_or_else(|| id.clone());
    if name.chars().count() > 80 {
        return Err("Pet Pack name cannot exceed 80 characters".to_string());
    }
    let version = trim_optional(raw.version).unwrap_or_else(|| "1.0".to_string());
    if version.chars().count() > 40 {
        return Err("Pet Pack version cannot exceed 40 characters".to_string());
    }

    let mut states = normalize_frame_map(raw.states, frame_root)?;
    let idle = states
        .get("idle")
        .filter(|frames| !frames.is_empty())
        .cloned()
        .ok_or_else(|| "Pet Pack manifest must contain at least one idle frame".to_string())?;
    for state in STANDARD_STATES {
        if states.get(state).is_none_or(Vec::is_empty) {
            states.insert(state.to_string(), idle.clone());
        }
    }

    let mut suggested_loops = normalize_frame_map(raw.suggested_loops, frame_root)?;
    for state in ["idle", "talk"] {
        if suggested_loops.get(state).is_none_or(Vec::is_empty) {
            suggested_loops.insert(
                state.to_string(),
                states.get(state).cloned().unwrap_or_else(|| idle.clone()),
            );
        }
    }

    let requested_default = trim_optional(raw.default_state).unwrap_or_else(|| "idle".to_string());
    let default_state = if states
        .get(&requested_default)
        .is_some_and(|frames| !frames.is_empty())
    {
        requested_default
    } else {
        "idle".to_string()
    };

    Ok(PetPackManifest {
        id,
        name,
        version,
        default_state,
        states,
        suggested_loops,
    })
}

fn read_manifest(directory: &Path, fallback_id: &str) -> Result<PetPackManifest, String> {
    let manifest_path = directory.join(MANIFEST_FILENAME);
    let json = fs::read_to_string(&manifest_path).map_err(|error| {
        format!(
            "Pet Pack manifest is missing or unreadable at {}: {error}",
            manifest_path.display()
        )
    })?;
    let raw: RawPetPackManifest = serde_json::from_str(&json)
        .map_err(|error| format!("Pet Pack manifest JSON is invalid: {error}"))?;
    normalize_manifest(raw, fallback_id, Some(directory))
}

fn builtin_pet_pack() -> Result<PetPack, String> {
    let raw: RawPetPackManifest = serde_json::from_str(include_str!(
        "../../src/assets/pets/remi_hanfu/manifest.json"
    ))
    .map_err(|error| format!("Bundled Remi Pet Pack manifest is invalid: {error}"))?;
    Ok(PetPack {
        manifest: normalize_manifest(raw, BUILTIN_PET_PACK_ID, None)?,
        source: "builtin".to_string(),
        root_path: None,
    })
}

fn imported_pet_pack(directory: &Path) -> Result<PetPack, String> {
    let fallback_id = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Pet Pack folder name is invalid".to_string())?;
    let manifest = read_manifest(directory, fallback_id)?;
    Ok(PetPack {
        manifest,
        source: "imported".to_string(),
        root_path: Some(directory.to_string_lossy().into_owned()),
    })
}

fn installed_pet_packs(store_root: &Path) -> Result<Vec<PetPack>, String> {
    fs::create_dir_all(store_root)
        .map_err(|error| format!("Could not create the Pet Pack store: {error}"))?;
    let mut packs = vec![builtin_pet_pack()?];
    let entries = fs::read_dir(store_root)
        .map_err(|error| format!("Could not list installed Pet Packs: {error}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_staging = entry.file_name().to_string_lossy().starts_with(".import-");
        if path.is_dir() && !is_staging {
            // A corrupt folder is ignored so one manually damaged pack cannot prevent
            // the built-in fallback from loading.
            if let Ok(pack) = imported_pet_pack(&path) {
                packs.push(pack);
            }
        }
    }
    packs[1..].sort_by(|left, right| left.manifest.name.cmp(&right.manifest.name));
    Ok(packs)
}

fn pet_pack_catalog(database: &Database, store_root: &Path) -> Result<PetPackCatalog, String> {
    let packs = installed_pet_packs(store_root)?;
    let requested = database
        .get_active_pet_pack_id()
        .map_err(|error| error.to_string())?;
    let active_pet_pack_id = if packs.iter().any(|pack| pack.manifest.id == requested) {
        requested
    } else {
        database
            .set_active_pet_pack_id(BUILTIN_PET_PACK_ID)
            .map_err(|error| error.to_string())?;
        BUILTIN_PET_PACK_ID.to_string()
    };
    Ok(PetPackCatalog {
        active_pet_pack_id,
        packs,
    })
}

fn referenced_frames(manifest: &PetPackManifest) -> BTreeSet<&str> {
    manifest
        .states
        .values()
        .chain(manifest.suggested_loops.values())
        .flatten()
        .map(String::as_str)
        .collect()
}

fn copy_validated_pack(
    source: &Path,
    store_root: &Path,
    manifest: &PetPackManifest,
) -> Result<PathBuf, String> {
    if manifest.id == BUILTIN_PET_PACK_ID {
        return Err("The bundled Remi Hanfu Pet Pack cannot be replaced".to_string());
    }
    fs::create_dir_all(store_root)
        .map_err(|error| format!("Could not create the Pet Pack store: {error}"))?;
    let destination = store_root.join(&manifest.id);
    if destination.exists() {
        return Err(format!("Pet Pack '{}' is already installed", manifest.id));
    }

    let staging = store_root.join(format!(".import-{}", Uuid::new_v4()));
    let copy_result = (|| -> Result<(), String> {
        fs::create_dir(&staging)
            .map_err(|error| format!("Could not prepare Pet Pack import: {error}"))?;
        let normalized_manifest = serde_json::to_vec_pretty(manifest)
            .map_err(|error| format!("Could not serialize Pet Pack manifest: {error}"))?;
        fs::write(staging.join(MANIFEST_FILENAME), normalized_manifest)
            .map_err(|error| format!("Could not store Pet Pack manifest: {error}"))?;

        for reference in referenced_frames(manifest) {
            let target = staging.join(reference);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("Could not create Pet Pack folders: {error}"))?;
            }
            fs::copy(source.join(reference), &target)
                .map_err(|error| format!("Could not copy Pet Pack frame '{reference}': {error}"))?;
        }
        fs::rename(&staging, &destination)
            .map_err(|error| format!("Could not finish Pet Pack import: {error}"))?;
        Ok(())
    })();

    if let Err(error) = copy_result {
        if staging.exists() {
            let _ = fs::remove_dir_all(&staging);
        }
        return Err(error);
    }
    Ok(destination)
}

fn import_from_folder(store_root: &Path, folder_path: &str) -> Result<PetPack, String> {
    let folder_path = folder_path.trim();
    if folder_path.is_empty() {
        return Err("Choose a Pet Pack folder".to_string());
    }
    let source = fs::canonicalize(folder_path)
        .map_err(|error| format!("Pet Pack folder is unavailable: {error}"))?;
    if !source.is_dir() {
        return Err("Pet Pack import path must be a folder".to_string());
    }
    let fallback_id = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Pet Pack folder name is invalid".to_string())?;
    let manifest = read_manifest(&source, fallback_id)?;
    let destination = copy_validated_pack(&source, store_root, &manifest)?;
    imported_pet_pack(&destination)
}

fn activate_pack(
    database: &Database,
    store_root: &Path,
    pet_pack_id: &str,
) -> Result<(PetPackCatalog, PetPack), String> {
    let mut catalog = pet_pack_catalog(database, store_root)?;
    let pack = catalog
        .packs
        .iter()
        .find(|pack| pack.manifest.id == pet_pack_id)
        .cloned()
        .ok_or_else(|| format!("Pet Pack '{pet_pack_id}' is not installed"))?;
    database
        .set_active_pet_pack_id(pet_pack_id)
        .map_err(|error| error.to_string())?;
    catalog.active_pet_pack_id = pet_pack_id.to_string();
    Ok((catalog, pack))
}

fn read_imported_frame(
    store_root: &Path,
    pet_pack_id: &str,
    filename: &str,
) -> Result<Vec<u8>, String> {
    validate_pack_id(pet_pack_id)?;
    let filename = normalize_frame_reference(filename)?;
    let pack = installed_pet_packs(store_root)?
        .into_iter()
        .find(|pack| pack.manifest.id == pet_pack_id)
        .ok_or_else(|| format!("Pet Pack '{pet_pack_id}' is not installed"))?;
    if pack.source != "imported" {
        return Err("Bundled Pet Pack frames are loaded from the app bundle".to_string());
    }
    if !referenced_frames(&pack.manifest).contains(filename.as_str()) {
        return Err("Frame is not referenced by this Pet Pack manifest".to_string());
    }
    let root_path = pack
        .root_path
        .ok_or_else(|| "Imported Pet Pack has no asset directory".to_string())?;
    fs::read(Path::new(&root_path).join(filename))
        .map_err(|error| format!("Could not read Pet Pack frame: {error}"))
}

#[tauri::command]
pub fn list_pet_packs(state: State<'_, AppState>) -> Result<PetPackCatalog, String> {
    let store_root = pack_store_from_state(&state)?;
    pet_pack_catalog(&state.database, &store_root)
}

#[tauri::command]
pub fn import_pet_pack(state: State<'_, AppState>, folder_path: String) -> Result<PetPack, String> {
    let store_root = pack_store_from_state(&state)?;
    import_from_folder(&store_root, &folder_path)
}

#[tauri::command]
pub fn read_pet_pack_frame(
    state: State<'_, AppState>,
    pet_pack_id: String,
    filename: String,
) -> Result<tauri::ipc::Response, String> {
    let store_root = pack_store_from_state(&state)?;
    read_imported_frame(&store_root, &pet_pack_id, &filename).map(tauri::ipc::Response::new)
}

#[tauri::command]
pub fn activate_pet_pack(
    app: AppHandle,
    state: State<'_, AppState>,
    pet_pack_id: String,
) -> Result<PetPackCatalog, String> {
    validate_pack_id(&pet_pack_id)?;
    let store_root = pack_store_from_state(&state)?;
    let (catalog, active_pack) = activate_pack(&state.database, &store_root, &pet_pack_id)?;
    app.emit(PET_PACK_CHANGED_EVENT, active_pack)
        .map_err(|error| {
            format!("Pet Pack was activated but the renderer was not notified: {error}")
        })?;
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{memory::NewMemory, working_memory::WorkingMessage};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!("remi-{label}-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn write(&self, relative: &str, content: impl AsRef<[u8]>) {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_minimal_pack(directory: &TestDirectory, id: &str) {
        directory.write("idle.png", b"not-decoded-by-manifest-validation");
        directory.write(
            MANIFEST_FILENAME,
            format!(
                r#"{{
                    "id": "{id}",
                    "name": "Minimal Pet",
                    "version": "1.0",
                    "defaultState": "think",
                    "states": {{ "idle": ["idle.png"] }}
                }}"#
            ),
        );
    }

    #[test]
    fn missing_standard_states_and_default_fall_back_to_idle() {
        let source = TestDirectory::new("pack-fallback");
        write_minimal_pack(&source, "minimal-pet");

        let manifest = read_manifest(&source.0, "unused").unwrap();

        assert_eq!(manifest.default_state, "think");
        for state in STANDARD_STATES {
            assert_eq!(manifest.states[state], vec!["idle.png"]);
        }
        assert_eq!(manifest.suggested_loops["talk"], vec!["idle.png"]);
    }

    #[test]
    fn rejects_missing_referenced_frame_and_path_traversal() {
        let missing = TestDirectory::new("pack-missing");
        missing.write(
            MANIFEST_FILENAME,
            r#"{"id":"missing","states":{"idle":["missing.png"]}}"#,
        );
        assert!(
            read_manifest(&missing.0, "missing")
                .unwrap_err()
                .contains("does not exist")
        );

        let traversal = TestDirectory::new("pack-traversal");
        traversal.write(
            MANIFEST_FILENAME,
            r#"{"id":"traversal","states":{"idle":["../outside.png"]}}"#,
        );
        assert!(
            read_manifest(&traversal.0, "traversal")
                .unwrap_err()
                .contains("relative PNG path")
        );
    }

    #[test]
    fn import_and_activation_preserve_agent_identity_memory_and_conversation() {
        let source = TestDirectory::new("pack-source");
        let store = TestDirectory::new("pack-store");
        write_minimal_pack(&source, "minimal-pet");
        let database = Database::in_memory().unwrap();
        database
            .persist_message(&WorkingMessage {
                id: "message-before-pack-switch".to_string(),
                role: "user".to_string(),
                content: "Keep this conversation".to_string(),
                timestamp: 1,
            })
            .unwrap();
        database
            .insert_memory(&NewMemory {
                id: "memory-before-pack-switch".to_string(),
                kind: "semantic".to_string(),
                content: "Keep this memory".to_string(),
                importance: 0.8,
                confidence: 1.0,
                created_at: 1,
                valid_from: None,
                valid_to: None,
                source_type: "user_explicit".to_string(),
                source_ref: None,
            })
            .unwrap();
        let soul_version = database.get_soul_version().unwrap();
        let pet_state = serde_json::to_value(database.get_pet_state().unwrap()).unwrap();

        let imported = import_from_folder(&store.0, source.0.to_str().unwrap()).unwrap();
        assert_eq!(imported.manifest.id, "minimal-pet");
        assert!(store.0.join("minimal-pet/idle.png").is_file());
        let (catalog, _) = activate_pack(&database, &store.0, "minimal-pet").unwrap();

        assert_eq!(catalog.active_pet_pack_id, "minimal-pet");
        assert_eq!(database.get_active_pet_pack_id().unwrap(), "minimal-pet");
        assert_eq!(database.get_soul_version().unwrap(), soul_version);
        assert_eq!(
            serde_json::to_value(database.get_pet_state().unwrap()).unwrap(),
            pet_state
        );
        assert_eq!(database.list_active_memories().unwrap().len(), 1);
        assert_eq!(database.get_recent_messages(10).unwrap().len(), 1);
    }

    #[test]
    fn catalog_self_heals_a_missing_active_pack_to_builtin() {
        let store = TestDirectory::new("pack-catalog");
        let database = Database::in_memory().unwrap();
        database.set_active_pet_pack_id("removed-pack").unwrap();

        let catalog = pet_pack_catalog(&database, &store.0).unwrap();

        assert_eq!(catalog.active_pet_pack_id, BUILTIN_PET_PACK_ID);
        assert_eq!(
            database.get_active_pet_pack_id().unwrap(),
            BUILTIN_PET_PACK_ID
        );
        assert_eq!(catalog.packs[0].manifest.id, BUILTIN_PET_PACK_ID);
    }

    #[test]
    fn frame_reader_only_serves_manifest_references() {
        let source = TestDirectory::new("pack-read-source");
        let store = TestDirectory::new("pack-read-store");
        write_minimal_pack(&source, "readable-pet");
        source.write("private.png", b"not referenced");
        import_from_folder(&store.0, source.0.to_str().unwrap()).unwrap();

        assert_eq!(
            read_imported_frame(&store.0, "readable-pet", "idle.png").unwrap(),
            b"not-decoded-by-manifest-validation"
        );
        assert!(
            read_imported_frame(&store.0, "readable-pet", "private.png")
                .unwrap_err()
                .contains("not referenced")
        );
    }
}
