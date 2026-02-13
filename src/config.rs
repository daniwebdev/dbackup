use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub drivers: Drivers,
    pub backups: Vec<BackupConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Drivers {
    pub filesystems: Vec<FilesystemDriver>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FilesystemDriver {
    pub name: String,
    #[serde(rename = "type")]
    pub driver_type: String,
    pub base_path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BackupConfig {
    pub name: String,
    pub driver: String,
    pub connection: ConnectionConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<ScheduleConfig>,
    pub storage: StorageConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConnectionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScheduleConfig {
    pub cron: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageConfig {
    pub driver: String,
    pub path: PathBuf,
    pub filename_prefix: String,
}

impl Config {
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
