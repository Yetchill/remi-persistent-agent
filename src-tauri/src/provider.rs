use std::{
    collections::HashMap,
    env,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::State;
use uuid::Uuid;

use crate::{database::Database, state::AppState, trace::LlmCallTrace};

const KEYCHAIN_SERVICE: &str = "dev.remi.personal-agent.llm-provider";

fn keychain_entry(provider_id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, provider_id)
        .map_err(|error| format!("Could not access the macOS Keychain: {error}"))
}

fn load_keychain_api_key(provider_id: &str) -> Result<Option<String>, String> {
    match keychain_entry(provider_id)?.get_password() {
        Ok(api_key) => Ok(Some(api_key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!(
            "Could not read the API key from macOS Keychain: {error}"
        )),
    }
}

fn save_keychain_api_key(provider_id: &str, api_key: &str) -> Result<(), String> {
    keychain_entry(provider_id)?
        .set_password(api_key)
        .map_err(|error| format!("Could not save the API key to macOS Keychain: {error}"))
}

fn delete_keychain_api_key(provider_id: &str) -> Result<(), String> {
    match keychain_entry(provider_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!(
            "Could not delete the API key from macOS Keychain: {error}"
        )),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfig {
    pub id: String,
    pub display_name: String,
    pub model_id: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub base_url: String,
    pub enabled: bool,
    #[serde(default)]
    pub has_api_key: bool,
    pub models: Vec<ModelConfig>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalog {
    pub providers: Vec<ProviderConfig>,
    pub active_provider_id: Option<String>,
    pub active_model_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProviderInput {
    pub provider: ProviderConfig,
    #[serde(default)]
    pub api_key: String,
}

pub struct ProviderStore {
    pub catalog: ProviderCatalog,
    api_keys: HashMap<String, String>,
}

impl ProviderStore {
    pub fn load(database: &Database) -> Self {
        let mut catalog = database.load_provider_catalog().unwrap_or_default();
        let mut api_keys = HashMap::new();
        if catalog.providers.is_empty() {
            let model_id = env::var("REMI_LLM_MODEL").unwrap_or_default();
            if !model_id.is_empty() {
                let provider_id = "environment".to_string();
                let model_config_id = "environment-model".to_string();
                let provider = ProviderConfig {
                    id: provider_id.clone(),
                    display_name: "Environment".to_string(),
                    provider_type: "openai-compatible".to_string(),
                    base_url: env::var("REMI_LLM_BASE_URL")
                        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
                    enabled: true,
                    has_api_key: false,
                    models: vec![ModelConfig {
                        id: model_config_id.clone(),
                        display_name: model_id.clone(),
                        model_id,
                        enabled: true,
                    }],
                };
                let _ = database.save_provider(&provider);
                let _ = database.set_active_model(&provider_id, &model_config_id);
                catalog = database.load_provider_catalog().unwrap_or_default();
            }
        }
        if let Ok(key) = env::var("REMI_LLM_API_KEY")
            && let Some(provider) = catalog
                .providers
                .iter()
                .find(|provider| provider.id == "environment")
        {
            api_keys.insert(provider.id.clone(), key);
        }
        for provider in &catalog.providers {
            if api_keys.contains_key(&provider.id) {
                continue;
            }
            match load_keychain_api_key(&provider.id) {
                Ok(Some(api_key)) => {
                    api_keys.insert(provider.id.clone(), api_key);
                }
                Ok(None) => {}
                Err(error) => eprintln!("{error}"),
            }
        }
        let mut store = Self { catalog, api_keys };
        store.refresh_key_flags();
        store
    }

    fn refresh_key_flags(&mut self) {
        for provider in &mut self.catalog.providers {
            provider.has_api_key = self.api_keys.contains_key(&provider.id);
        }
    }

    fn active(&self) -> Result<(&ProviderConfig, &ModelConfig, Option<&str>), String> {
        let provider_id = self
            .catalog
            .active_provider_id
            .as_deref()
            .ok_or_else(|| "No active Provider is selected".to_string())?;
        let model_id = self
            .catalog
            .active_model_id
            .as_deref()
            .ok_or_else(|| "No active Model is selected".to_string())?;
        let provider = self
            .catalog
            .providers
            .iter()
            .find(|provider| provider.id == provider_id && provider.enabled)
            .ok_or_else(|| "The active Provider is missing or disabled".to_string())?;
        let model = provider
            .models
            .iter()
            .find(|model| model.id == model_id && model.enabled)
            .ok_or_else(|| "The active Model is missing or disabled".to_string())?;
        Ok((
            provider,
            model,
            self.api_keys.get(provider_id).map(String::as_str),
        ))
    }
}

fn validate_provider(provider: &ProviderConfig) -> Result<(), String> {
    if provider.id.trim().is_empty() || provider.display_name.trim().is_empty() {
        return Err("Provider id and display name are required".to_string());
    }
    if provider.provider_type != "openai-compatible" {
        return Err("Only openai-compatible Providers are supported".to_string());
    }
    if !(provider.base_url.starts_with("https://") || provider.base_url.starts_with("http://")) {
        return Err("Provider base URL must use http:// or https://".to_string());
    }
    if provider.models.iter().any(|model| {
        model.id.trim().is_empty()
            || model.display_name.trim().is_empty()
            || model.model_id.trim().is_empty()
    }) {
        return Err("Every Model requires id, display name, and model ID".to_string());
    }
    let unique: std::collections::HashSet<&str> = provider
        .models
        .iter()
        .map(|model| model.id.as_str())
        .collect();
    if unique.len() != provider.models.len() {
        return Err("Model ids must be unique within a Provider".to_string());
    }
    Ok(())
}

fn completion_endpoint(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{base}/chat/completions")
    }
}

fn timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmRequest {
    pub event_id: String,
    pub event_type: String,
    pub messages: Vec<LlmMessage>,
    pub trace_metadata: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmResponse {
    pub content: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

#[tauri::command]
pub fn get_provider_catalog(state: State<'_, AppState>) -> Result<ProviderCatalog, String> {
    let store = state
        .provider
        .read()
        .map_err(|_| "Provider lock poisoned")?;
    Ok(store.catalog.clone())
}

#[tauri::command]
pub fn save_provider(
    state: State<'_, AppState>,
    input: SaveProviderInput,
) -> Result<ProviderCatalog, String> {
    validate_provider(&input.provider)?;
    let api_key = input.api_key.trim();
    if !api_key.is_empty() {
        save_keychain_api_key(&input.provider.id, api_key)?;
    }
    state
        .database
        .save_provider(&input.provider)
        .map_err(|error| error.to_string())?;
    let mut store = state
        .provider
        .write()
        .map_err(|_| "Provider lock poisoned")?;
    if !api_key.is_empty() {
        store
            .api_keys
            .insert(input.provider.id.clone(), api_key.to_string());
    }
    store.catalog = state
        .database
        .load_provider_catalog()
        .map_err(|error| error.to_string())?;
    store.refresh_key_flags();
    Ok(store.catalog.clone())
}

#[tauri::command]
pub fn delete_provider(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<ProviderCatalog, String> {
    delete_keychain_api_key(&provider_id)?;
    state
        .database
        .delete_provider(&provider_id)
        .map_err(|error| error.to_string())?;
    let mut store = state
        .provider
        .write()
        .map_err(|_| "Provider lock poisoned")?;
    store.api_keys.remove(&provider_id);
    store.catalog = state
        .database
        .load_provider_catalog()
        .map_err(|error| error.to_string())?;
    store.refresh_key_flags();
    Ok(store.catalog.clone())
}

#[tauri::command]
pub fn set_active_model(
    state: State<'_, AppState>,
    provider_id: String,
    model_id: String,
) -> Result<ProviderCatalog, String> {
    let mut store = state
        .provider
        .write()
        .map_err(|_| "Provider lock poisoned")?;
    let provider = store
        .catalog
        .providers
        .iter()
        .find(|provider| provider.id == provider_id && provider.enabled)
        .ok_or_else(|| "Provider is missing or disabled".to_string())?;
    if !provider
        .models
        .iter()
        .any(|model| model.id == model_id && model.enabled && !model.model_id.trim().is_empty())
    {
        return Err("Model is missing or disabled".to_string());
    }
    state
        .database
        .set_active_model(&provider_id, &model_id)
        .map_err(|error| error.to_string())?;
    store.catalog.active_provider_id = Some(provider_id);
    store.catalog.active_model_id = Some(model_id);
    Ok(store.catalog.clone())
}

#[tauri::command]
pub async fn complete_llm(
    state: State<'_, AppState>,
    request: LlmRequest,
) -> Result<LlmResponse, String> {
    let (provider, model, api_key) = {
        let store = state
            .provider
            .read()
            .map_err(|_| "Provider lock poisoned")?;
        let (provider, model, api_key) = store.active()?;
        (
            provider.clone(),
            model.clone(),
            api_key.map(ToString::to_string),
        )
    };

    let body = json!({
        "model": model.model_id,
        "messages": request.messages,
        "temperature": 0.7
    });
    let request_json = json!({
        "backendRequest": body,
        "traceMetadata": request.trace_metadata,
    })
    .to_string();
    let started = Instant::now();
    let timestamp = timestamp_ms();
    let mut http_request = state
        .http
        .post(completion_endpoint(&provider.base_url))
        .json(&body);
    if let Some(api_key) = api_key {
        http_request = http_request.bearer_auth(api_key);
    }

    let result = async {
        let response = http_request
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let status = response.status();
        let response_text = response.text().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("Provider returned {status}: {response_text}"));
        }
        let value: Value =
            serde_json::from_str(&response_text).map_err(|error| error.to_string())?;
        let content = value["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| "Provider response has no text content".to_string())?
            .to_string();
        Ok((
            content,
            value["usage"]["prompt_tokens"].as_i64(),
            value["usage"]["completion_tokens"].as_i64(),
            response_text,
        ))
    }
    .await;

    let latency_ms = started.elapsed().as_millis() as i64;
    let (response_json, input_tokens, output_tokens, success, error) = match &result {
        Ok((_, input, output, raw)) => (Some(raw.clone()), *input, *output, true, None),
        Err(error) => (None, None, None, false, Some(error.clone())),
    };
    state
        .database
        .insert_llm_call(&LlmCallTrace {
            id: Uuid::new_v4().to_string(),
            event_id: request.event_id,
            provider: provider.display_name,
            model: model.model_id,
            event_type: request.event_type,
            timestamp,
            request_json,
            response_json,
            input_tokens,
            output_tokens,
            latency_ms,
            success,
            error,
        })
        .map_err(|error| error.to_string())?;

    result.map(|(content, input_tokens, output_tokens, _)| LlmResponse {
        content,
        input_tokens,
        output_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(id: &str, model_id: &str) -> ProviderConfig {
        ProviderConfig {
            id: id.to_string(),
            display_name: id.to_string(),
            provider_type: "openai-compatible".to_string(),
            base_url: format!("https://{id}.example/v1"),
            enabled: true,
            has_api_key: false,
            models: vec![ModelConfig {
                id: format!("{id}-model"),
                display_name: model_id.to_string(),
                model_id: model_id.to_string(),
                enabled: true,
            }],
        }
    }

    #[test]
    fn builds_openai_compatible_endpoint() {
        assert_eq!(
            completion_endpoint("https://example.test/v1/"),
            "https://example.test/v1/chat/completions"
        );
        assert_eq!(
            completion_endpoint("http://localhost:1234/v1/chat/completions"),
            "http://localhost:1234/v1/chat/completions"
        );
    }

    #[test]
    fn active_backend_uses_the_selected_provider_model_and_its_own_key() {
        let database = Database::in_memory().unwrap();
        database
            .save_provider(&provider("openai", "gpt-test"))
            .unwrap();
        database
            .save_provider(&provider("deepseek", "deepseek-test"))
            .unwrap();
        database
            .set_active_model("deepseek", "deepseek-model")
            .unwrap();
        let mut store = ProviderStore::load(&database);
        store
            .api_keys
            .insert("openai".to_string(), "key-a".to_string());
        store
            .api_keys
            .insert("deepseek".to_string(), "key-b".to_string());

        let (active_provider, active_model, key) = store.active().unwrap();

        assert_eq!(active_provider.id, "deepseek");
        assert_eq!(active_model.model_id, "deepseek-test");
        assert_eq!(key, Some("key-b"));
    }
}
