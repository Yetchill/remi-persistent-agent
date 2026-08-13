use std::{path::PathBuf, sync::RwLock};

use reqwest::Client;

use crate::{database::Database, provider::ProviderStore};

pub struct AppState {
    pub database: Database,
    pub provider: RwLock<ProviderStore>,
    pub http: Client,
    pub soul_path: PathBuf,
}

impl AppState {
    pub fn new(database: Database, soul_path: PathBuf) -> Self {
        let provider = ProviderStore::load(&database);
        Self {
            database,
            provider: RwLock::new(provider),
            http: Client::new(),
            soul_path,
        }
    }
}
