use std::path::PathBuf;
use std::sync::Mutex;

use crate::ai::AiClient;
use crate::config::Config;
use crate::storage::Storage;

pub struct AppState {
    pub config: Mutex<Config>,
    pub storage: Mutex<Storage>,
    pub data_dir: Mutex<PathBuf>,
    pub ai: AiClient,
}

impl AppState {
    pub fn new(config: Config, storage: Storage) -> Self {
        let data_dir = Mutex::new(config.data_dir.clone());
        Self {
            config: Mutex::new(config),
            storage: Mutex::new(storage),
            data_dir,
            ai: AiClient::new(),
        }
    }
}
