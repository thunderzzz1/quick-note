use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::io_atomic::atomic_write;
use crate::paths::join_under;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.deepseek.com/v1".into(),
            model: "deepseek-chat".into(),
            api_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub data_dir: PathBuf,
    pub hotkey: String,
    pub org_time: String, // "HH:MM"，24 小时制
    pub auto_org_enabled: bool,
    pub last_org_date: Option<String>, // 上一次成功整理的日期 yyyy-MM-dd
    pub ai: AiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            hotkey: "Alt+Shift+N".into(),
            org_time: "22:00".into(),
            auto_org_enabled: true,
            last_org_date: None,
            ai: AiConfig::default(),
        }
    }
}

pub fn default_data_dir() -> PathBuf {
    dirs::document_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("QuickNote")
}

fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join("config.json")
}

pub fn load(data_dir: &Path) -> Result<Config, String> {
    let path = config_path(data_dir);
    if !path.exists() {
        let mut cfg = Config::default();
        cfg.data_dir = data_dir.to_path_buf();
        save(data_dir, &cfg)?;
        return Ok(cfg);
    }
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("读取配置失败: {e}"))?;
    let mut cfg: Config =
        serde_json::from_str(&raw).map_err(|e| format!("配置格式错误: {e}"))?;
    cfg.data_dir = data_dir.to_path_buf();
    Ok(cfg)
}

pub fn save(data_dir: &Path, cfg: &Config) -> Result<(), String> {
    let path = join_under(data_dir, Path::new("config.json")).map_err(|e| e)?;
    let raw = serde_json::to_string_pretty(cfg).map_err(|e| format!("序列化配置失败: {e}"))?;
    atomic_write(&path, raw.as_bytes()).map_err(|e| format!("写入配置失败: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn default_config_when_missing() {
        let dir = temp_dir().join(format!("qncfg-{}", std::process::id()));
        let cfg = load(&dir).unwrap();
        assert_eq!(cfg.hotkey, "Alt+Shift+N");
        assert_eq!(cfg.ai.base_url, "https://api.deepseek.com/v1");
        assert_eq!(cfg.org_time, "22:00");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = temp_dir().join(format!("qncfg2-{}", std::process::id()));
        let mut cfg = load(&dir).unwrap();
        cfg.ai.api_key = "sk-test".into();
        cfg.org_time = "23:30".into();
        save(&dir, &cfg).unwrap();
        let loaded = load(&dir).unwrap();
        assert_eq!(loaded.ai.api_key, "sk-test");
        assert_eq!(loaded.org_time, "23:30");
        std::fs::remove_dir_all(&dir).ok();
    }
}
