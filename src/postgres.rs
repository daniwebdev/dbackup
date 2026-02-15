use crate::config::{BackupConfig, StorageConfig};
use crate::storage;
use anyhow::{Context, Result};
use chrono::Local;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn};

pub struct PostgresBackup {
    config: BackupConfig,
    storage_config: StorageConfig,
}

impl PostgresBackup {
    pub fn new(config: BackupConfig, storage_config: StorageConfig) -> Self {
        Self { config, storage_config }
    }

    pub async fn execute(&self) -> Result<String> {
        info!("Starting PostgreSQL backup for: {}", self.config.name);

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        
        // Create storage backend
        let storage_backend = storage::create_storage(&self.storage_config).await?;
        
        // For local storage, ensure the output directory exists
        if self.storage_config.driver.to_lowercase() == "local" {
            let path = self.storage_config.path.as_ref()
                .context("Local storage requires 'path' configuration")?;
            std::fs::create_dir_all(path)
                .context("Failed to create output directory")?;
        }

        // Create a temporary directory for the backup file
        let temp_dir = std::env::temp_dir().join(format!("dbackup_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir)
            .context("Failed to create temporary directory")?;

        // Execute backup based on mode
        let (temp_file_path, filename) = match self.config.mode {
            crate::config::BackupMode::Basic => {
                info!("Using basic mode (custom format with compression)...");
                self.dump_basic(&temp_dir, timestamp).await?
            }
            crate::config::BackupMode::Parallel => {
                info!(
                    "Using parallel mode (directory format with {} jobs)...",
                    self.config.parallel_jobs
                );
                self.dump_parallel(&temp_dir, timestamp).await?
            }
        };

        // Move to final location or upload to S3
        let final_location = if self.storage_config.driver.to_lowercase() == "local" {
            // For local storage, file is already in the correct location
            temp_file_path.display().to_string()
        } else {
            // For remote storage (S3), upload the file
            storage_backend.store(&temp_file_path, &filename).await?
        };

        // Cleanup temporary directory
        let _ = std::fs::remove_dir_all(&temp_dir);

        info!("Backup completed successfully: {}", final_location);
        Ok(final_location)
    }

    async fn dump_basic(&self, temp_dir: &PathBuf, timestamp: impl std::fmt::Display) -> Result<(PathBuf, String)> {
        let conn = &self.config.connection;
        let filename = format!(
            "{}{}.dump.gz",
            self.storage_config.filename_prefix.as_ref().unwrap_or(&"backup_".to_string()),
            timestamp
        );
        let output_path = temp_dir.join(&filename);

        info!("Backing up to: {}", output_path.display());

        // Determine pg_dump path
        let pg_dump_path = self.config.binary_path.as_ref()
            .map(|p| p.as_path())
            .unwrap_or_else(|| std::path::Path::new("pg_dump"));

        // Build pg_dump command with custom format and compression
        let mut cmd = Command::new(pg_dump_path);

        // Set environment variable for password
        cmd.env("PGPASSWORD", &conn.password);

        // Add connection parameters
        cmd.arg("--host").arg(&conn.host);
        cmd.arg("--port").arg(conn.port.to_string());
        cmd.arg("--username").arg(&conn.username);
        cmd.arg("--dbname").arg(&conn.database);

        // Custom format with compression level 9
        cmd.arg("-Fc");
        cmd.arg("--compress=9");
        cmd.arg("--no-owner");
        cmd.arg("--verbose");

        // Capture stdout and stderr
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        info!("Executing pg_dump with custom format and compression...");
        let mut child = cmd.spawn().context("Failed to spawn pg_dump process")?;

        // Get stdout
        let stdout = child
            .stdout
            .take()
            .context("Failed to capture pg_dump stdout")?;

        // Create gzip compressed output file
        let output_file = File::create(&output_path)
            .context("Failed to create output file")?;
        let mut encoder = GzEncoder::new(output_file, Compression::best());

        // Read from pg_dump and write compressed data
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut buffer = Vec::new();
        
        tokio::io::copy(&mut reader, &mut buffer)
            .await
            .context("Failed to read pg_dump output")?;

        encoder
            .write_all(&buffer)
            .context("Failed to write compressed data")?;
        encoder.finish().context("Failed to finalize compression")?;

        // Wait for the process to complete
        let status = child.wait().await.context("pg_dump process failed")?;

        if !status.success() {
            let stderr = child.stderr.take();
            if let Some(mut stderr) = stderr {
                let mut error_output = String::new();
                use tokio::io::AsyncReadExt;
                stderr.read_to_string(&mut error_output).await?;
                warn!("pg_dump stderr: {}", error_output);
            }
            anyhow::bail!("pg_dump failed with status: {}", status);
        }

        if self.storage_config.driver.to_lowercase() == "local" {
            // For local storage, move file to the final location
            let final_path = self.storage_config.path.as_ref()
                .context("Local storage requires 'path' configuration")?
                .join(&filename);
            std::fs::rename(&output_path, &final_path)
                .context("Failed to move backup file to final location")?;
            info!("Backup compressed to {}", final_path.display());
            Ok((final_path, filename))
        } else {
            // For remote storage, keep in temp directory for upload
            info!("Backup compressed to {}", output_path.display());
            Ok((output_path, filename))
        }
    }

    async fn dump_parallel(&self, temp_dir: &PathBuf, timestamp: impl std::fmt::Display) -> Result<(PathBuf, String)> {
        let conn = &self.config.connection;
        let basename = format!("{}{}", self.storage_config.filename_prefix.as_ref().unwrap_or(&"backup_".to_string()), timestamp);
        
        // Create temporary directory for directory format backup
        let backup_tmp_dir = temp_dir.join(&basename);
        std::fs::create_dir_all(&backup_tmp_dir)
            .context("Failed to create temporary directory")?;

        info!("Using temporary directory: {}", backup_tmp_dir.display());

        // Determine pg_dump path
        let pg_dump_path = self.config.binary_path.as_ref()
            .map(|p| p.as_path())
            .unwrap_or_else(|| std::path::Path::new("pg_dump"));

        // Build pg_dump command with directory format and parallel jobs
        let mut cmd = Command::new(pg_dump_path);

        // Set environment variable for password
        cmd.env("PGPASSWORD", &conn.password);

        // Add connection parameters
        cmd.arg("--host").arg(&conn.host);
        cmd.arg("--port").arg(conn.port.to_string());
        cmd.arg("--username").arg(&conn.username);
        cmd.arg("--dbname").arg(&conn.database);

        // Directory format with parallel jobs
        cmd.arg("-Fd"); // Directory format
        cmd.arg("-j").arg(self.config.parallel_jobs.to_string()); // Parallel jobs
        cmd.arg("-f").arg(&backup_tmp_dir); // Output directory
        cmd.arg("--no-owner");
        cmd.arg("--verbose");

        // Capture stderr for logging
        cmd.stderr(Stdio::piped());

        info!(
            "Executing pg_dump with directory format and {} parallel jobs...",
            self.config.parallel_jobs
        );
        
        let mut child = cmd.spawn().context("Failed to spawn pg_dump process")?;

        // Wait for the process to complete
        let status = child.wait().await.context("pg_dump process failed")?;

        if !status.success() {
            let stderr = child.stderr.take();
            if let Some(mut stderr) = stderr {
                let mut error_output = String::new();
                use tokio::io::AsyncReadExt;
                stderr.read_to_string(&mut error_output).await?;
                warn!("pg_dump stderr: {}", error_output);
            }
            
            // Cleanup temp directory on failure
            let _ = std::fs::remove_dir_all(&backup_tmp_dir);
            anyhow::bail!("pg_dump failed with status: {}", status);
        }

        // Compress the directory into a tar.gz file
        let tar_filename = format!("{}.dir.tar.gz", basename);
        let tar_path = temp_dir.join(&tar_filename);

        info!("Compressing directory backup to {}", tar_path.display());
        self.compress_directory(&backup_tmp_dir, &tar_path).await?;

        // Cleanup the backup directory
        std::fs::remove_dir_all(&backup_tmp_dir)
            .context("Failed to remove temporary directory")?;

        if self.storage_config.driver.to_lowercase() == "local" {
            // For local storage, move file to the final location
            let final_path = self.storage_config.path.as_ref()
                .context("Local storage requires 'path' configuration")?
                .join(&tar_filename);
            std::fs::rename(&tar_path, &final_path)
                .context("Failed to move backup file to final location")?;
            info!("Backup compressed to {}", final_path.display());
            Ok((final_path, tar_filename))
        } else {
            // For remote storage, keep in temp directory for upload
            info!("Backup compressed to {}", tar_path.display());
            Ok((tar_path, tar_filename))
        }
    }

    async fn compress_directory(&self, source_dir: &PathBuf, output_path: &PathBuf) -> Result<()> {
        use tar::Builder;

        let tar_gz = File::create(output_path)
            .context("Failed to create tar.gz file")?;
        let encoder = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = Builder::new(encoder);

        // Add all files from the directory to the tar archive
        tar.append_dir_all(".", source_dir)
            .context("Failed to add directory to tar archive")?;

        tar.finish().context("Failed to finalize tar archive")?;
        Ok(())
    }

    pub fn validate_connection(&self) -> Result<()> {
        let conn = &self.config.connection;

        // Basic validation
        if conn.host.is_empty() {
            anyhow::bail!("Database host cannot be empty");
        }

        if conn.database.is_empty() {
            anyhow::bail!("Database name cannot be empty");
        }

        if conn.username.is_empty() {
            anyhow::bail!("Database username cannot be empty");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> (BackupConfig, StorageConfig) {
        use crate::config::{BackupMode, ConnectionConfig, StorageSelection, StorageReference};
        
        let backup_config = BackupConfig {
            name: "test_backup".to_string(),
            driver: "postgresql".to_string(),
            connection: ConnectionConfig {
                uri: None,
                host: "localhost".to_string(),
                port: 5432,
                username: "postgres".to_string(),
                password: "password".to_string(),
                database: "testdb".to_string(),
            },
            schedule: None,
            storage: Some(StorageSelection::Reference(StorageReference {
                r#ref: "test_storage".to_string(),
                prefix: None,
                filename_prefix: None,
            })),
            mode: BackupMode::Basic,
            parallel_jobs: 2,
            binary_path: None,
        };

        let storage_config = StorageConfig {
            driver: "local".to_string(),
            path: Some(PathBuf::from("/tmp/backups")),
            filename_prefix: Some("test_".to_string()),
            bucket: None,
            region: None,
            prefix: None,
            endpoint: None,
            access_key_id: None,
            secret_access_key: None,
        };

        (backup_config, storage_config)
    }

    #[test]
    fn test_validate_connection() {
        let (config, storage_config) = create_test_config();
        let backup = PostgresBackup::new(config, storage_config);
        assert!(backup.validate_connection().is_ok());
    }

    #[test]
    fn test_validate_connection_empty_host() {
        let (mut config, storage_config) = create_test_config();
        config.connection.host = "".to_string();
        let backup = PostgresBackup::new(config, storage_config);
        assert!(backup.validate_connection().is_err());
    }
}
