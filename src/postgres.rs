use crate::config::BackupConfig;
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
}

impl PostgresBackup {
    pub fn new(config: BackupConfig) -> Self {
        Self { config }
    }

    pub async fn execute(&self) -> Result<PathBuf> {
        info!("Starting PostgreSQL backup for: {}", self.config.name);

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!(
            "{}{}.sql.gz",
            self.config.storage.filename_prefix, timestamp
        );
        let output_path = self.config.storage.path.join(&filename);

        // Ensure the output directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create output directory")?;
        }

        info!("Backing up to: {}", output_path.display());

        // Execute pg_dump
        self.dump_database(&output_path).await?;

        info!("Backup completed successfully: {}", output_path.display());
        Ok(output_path)
    }

    async fn dump_database(&self, output_path: &PathBuf) -> Result<()> {
        let conn = &self.config.connection;

        // Build pg_dump command
        let mut cmd = Command::new("pg_dump");

        // Set environment variable for password (more secure than command line)
        cmd.env("PGPASSWORD", &conn.password);

        // Add connection parameters
        cmd.arg("--host").arg(&conn.host);
        cmd.arg("--port").arg(conn.port.to_string());
        cmd.arg("--username").arg(&conn.username);
        cmd.arg("--dbname").arg(&conn.database);

        // Add dump options
        cmd.arg("--format=plain");
        cmd.arg("--no-owner");
        cmd.arg("--no-acl");
        cmd.arg("--verbose");

        // Capture stdout
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        info!("Executing pg_dump command...");
        let mut child = cmd.spawn().context("Failed to spawn pg_dump process")?;

        // Get stdout
        let stdout = child
            .stdout
            .take()
            .context("Failed to capture pg_dump stdout")?;

        // Create compressed output file
        let output_file = File::create(output_path)
            .context("Failed to create output file")?;
        let mut encoder = GzEncoder::new(output_file, Compression::default());

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
    use crate::config::StorageConfig;

    fn create_test_config() -> BackupConfig {
        BackupConfig {
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
            storage: StorageConfig {
                driver: "local".to_string(),
                path: PathBuf::from("/tmp/backups"),
                filename_prefix: "test_".to_string(),
            },
        }
    }

    #[test]
    fn test_validate_connection() {
        let config = create_test_config();
        let backup = PostgresBackup::new(config);
        assert!(backup.validate_connection().is_ok());
    }

    #[test]
    fn test_validate_connection_empty_host() {
        let mut config = create_test_config();
        config.connection.host = "".to_string();
        let backup = PostgresBackup::new(config);
        assert!(backup.validate_connection().is_err());
    }
}
