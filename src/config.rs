use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub cookie: String,
    pub download_path: String,
}

impl Config {
    pub fn new() -> Self {
        Config {
            cookie: "".to_string(),
            download_path: "".to_string(),
        }
    }

    pub fn load(config_path: &PathBuf) -> Self {
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path).expect("配置文件读取失败");
            serde_json::from_str(&content).expect("配置文件解析失败")
        } else {
            Config::new()
        }
    }

    pub fn save(&self, config_path: &PathBuf) {
        let content = serde_json::to_string_pretty(self).expect("配置文件序列化失败");
        std::fs::write(config_path, content).expect("配置文件写入失败");
    }
}