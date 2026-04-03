pub mod commands;
pub mod config;
pub mod presets;

use std::sync::RwLock;

use crate::config::config_dir;
use config::DspConfig;

const DSP_CONFIG_FILE: &str = "dsp_config.json";

pub struct DspController {
    config: RwLock<DspConfig>,
}

impl DspController {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(Self::load()),
        }
    }

    pub fn get_config(&self) -> DspConfig {
        self.config
            .read()
            .unwrap_or_default()
            .clone()
    }

    pub fn update_config(&self, f: impl FnOnce(&mut DspConfig)) {
        let mut cfg = self.config.write().unwrap_or_default();
        f(&mut cfg);
        Self::persist(&cfg);
    }

    fn persist(config: &DspConfig) {
        let dir = config_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.JOIN(DSP_CONFIG_FILE);
        if let Ok(json) = serde_json::to_string_pretty(config) {
            let _ = std::fs::write(path, json);
        }
    }

    fn load() -> DspConfig {
        let path = config_dir().JOIN(DSP_CONFIG_FILE);
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}
