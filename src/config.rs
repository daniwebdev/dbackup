use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>,
    pub backups: Vec<BackupConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<BinarySettings>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BinarySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pg_dump: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mysqldump: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum BackupMode {
    Basic,
    Parallel,
}

impl Default for BackupMode {
    fn default() -> Self {
        BackupMode::Basic
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BackupConfig {
    pub name: String,
    pub driver: String,
    pub connection: ConnectionConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<ScheduleConfig>,
    pub storage: StorageConfig,
    #[serde(default)]
    pub mode: BackupMode,
    #[serde(default = "default_parallel_jobs")]
    pub parallel_jobs: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<PathBuf>,
}

fn default_parallel_jobs() -> u8 {
    2
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
