use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storages: Option<HashMap<String, StorageConfig>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageSelection>,
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
pub struct StorageReference {
    pub r#ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename_prefix: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum StorageSelection {
    Reference(StorageReference),
    Inline(StorageConfig),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageConfig {
    pub driver: String,
    // Local storage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename_prefix: Option<String>,
    
    // S3 storage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
}

impl Config {
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Resolve storage configuration for a backup
    pub fn get_storage_for_backup(&self, backup: &BackupConfig) -> anyhow::Result<StorageConfig> {
        match &backup.storage {
            Some(StorageSelection::Inline(storage)) => {
                // Direct inline storage config
                Ok(storage.clone())
            }
            Some(StorageSelection::Reference(storage_ref)) => {
                // Reference to centralized storage with optional overrides
                if let Some(storages) = &self.settings.as_ref().and_then(|s| s.storages.as_ref()) {
                    if let Some(mut storage) = storages.get(&storage_ref.r#ref).cloned() {
                        // Apply overrides from the reference
                        if let Some(prefix) = &storage_ref.prefix {
                            storage.prefix = Some(prefix.clone());
                        }
                        if let Some(filename_prefix) = &storage_ref.filename_prefix {
                            storage.filename_prefix = Some(filename_prefix.clone());
                        }
                        return Ok(storage);
                    } else {
                        anyhow::bail!("Storage '{}' not found in settings", storage_ref.r#ref);
                    }
                } else {
                    anyhow::bail!("No storages defined in settings but storage reference '{}' specified", storage_ref.r#ref);
                }
            }
            None => {
                anyhow::bail!("Backup '{}' must have a storage configuration defined", backup.name);
            }
        }
    }
}
