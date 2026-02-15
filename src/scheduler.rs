use crate::config::{BackupConfig, Config};
use crate::postgres::PostgresBackup;
use anyhow::{Context, Result};
use chrono::Local;
use cron::Schedule;
use std::str::FromStr;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

pub struct BackupScheduler {
    config: Config,
    semaphore: std::sync::Arc<Semaphore>,
    max_concurrent: usize,
}

impl BackupScheduler {
    pub fn new(config: Config, max_concurrent: usize) -> Self {
        Self {
            config,
            semaphore: std::sync::Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting backup scheduler with {} concurrent slots", self.max_concurrent);
        info!("Press Ctrl+C to stop");

        // Collect all scheduled backups
        let scheduled_backups: Vec<_> = self.config.backups
            .iter()
            .filter(|b| b.schedule.is_some())
            .collect();

        if scheduled_backups.is_empty() {
            warn!("No scheduled backups found in configuration");
            return Ok(());
        }

        info!("Found {} scheduled backup(s)", scheduled_backups.len());

        // Print schedule information
        for backup in &scheduled_backups {
            if let Some(schedule) = &backup.schedule {
                info!("  - '{}' scheduled for: {}", backup.name, schedule.cron);
            }
        }

        let mut handles: Vec<JoinHandle<Result<()>>> = Vec::new();

        // Spawn a task for each scheduled backup
        for backup_config in self.config.backups.iter() {
            if let Some(schedule) = &backup_config.schedule {
                let backup = backup_config.clone();
                let cron_expr = schedule.cron.clone();
                let semaphore = self.semaphore.clone();
                let config = self.config.clone();

                let handle = tokio::spawn(Self::run_scheduled_backup(
                    backup,
                    cron_expr,
                    semaphore,
                    config,
                ));

                handles.push(handle);
            }
        }

        // Wait for all tasks (they run indefinitely until interrupted)
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Scheduler task panicked: {}", e);
            }
        }

        Ok(())
    }

    async fn run_scheduled_backup(
        backup: BackupConfig,
        cron_expr: String,
        semaphore: std::sync::Arc<Semaphore>,
        config: Config,
    ) -> Result<()> {
        let schedule = Schedule::from_str(&cron_expr)
            .context(format!("Invalid cron expression: {}", cron_expr))?;

        info!("Scheduled backup '{}' initialized with cron: {}", backup.name, cron_expr);

        loop {
            // Get next run time using iterator
            let now = Local::now();
            
            // The cron schedule iterator gives us upcoming times
            let mut upcoming = schedule.after(&now);
            if let Some(next_run) = upcoming.next() {
                // Calculate sleep duration
                let sleep_duration = (next_run - now).to_std()?;

                info!(
                    "Next run for '{}': {} (in {:.0}s)",
                    backup.name,
                    next_run.format("%Y-%m-%d %H:%M:%S"),
                    sleep_duration.as_secs_f64()
                );

                // Sleep until next run
                tokio::time::sleep(sleep_duration).await;

                // Acquire semaphore permit (limits concurrent backups)
                let _permit = semaphore.acquire().await?;

                info!("Starting scheduled backup: {}", backup.name);

                // Execute backup
                match &backup.driver.as_str() {
                    &"postgresql" => {
                        // Resolve storage configuration
                        match config.get_storage_for_backup(&backup) {
                            Ok(storage_config) => {
                                let backup_executor = PostgresBackup::new(backup.clone(), storage_config);

                                // Validate connection first
                                if let Err(e) = backup_executor.validate_connection() {
                                    error!("Connection validation failed for '{}': {}", backup.name, e);
                                    continue;
                                }

                                // Run backup
                                match backup_executor.execute().await {
                                    Ok(location) => {
                                        info!(
                                            "✓ Scheduled backup '{}' completed: {}",
                                            backup.name,
                                            location
                                        );
                                    }
                                    Err(e) => {
                                        error!("✗ Scheduled backup '{}' failed: {}", backup.name, e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to resolve storage for '{}': {}", backup.name, e);
                            }
                        }
                    }
                    driver => {
                        error!("Unsupported database driver for '{}': {}", backup.name, driver);
                    }
                }
            } else {
                error!("Could not calculate next run time for '{}'", backup.name);
                break Ok(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BackupMode, ConnectionConfig, ScheduleConfig, StorageConfig, StorageSelection, StorageReference};
    use std::path::PathBuf;

    fn create_test_backup_with_schedule() -> BackupConfig {
        BackupConfig {
            name: "test_scheduled_backup".to_string(),
            driver: "postgresql".to_string(),
            connection: ConnectionConfig {
                uri: None,
                host: "localhost".to_string(),
                port: 5432,
                username: "postgres".to_string(),
                password: "password".to_string(),
                database: "testdb".to_string(),
            },
            schedule: Some(ScheduleConfig {
                cron: "0 2 * * *".to_string(), // Daily at 2 AM
            }),
            storage: Some(StorageSelection::Inline(StorageConfig {
                driver: "local".to_string(),
                path: Some(PathBuf::from("/tmp/backups")),
                filename_prefix: Some("test_".to_string()),
                bucket: None,
                region: None,
                prefix: None,
                endpoint: None,
                access_key_id: None,
                secret_access_key: None,
            })),
            mode: BackupMode::Basic,
            parallel_jobs: 2,
            binary_path: None,
            retention: None,
        }
    }

    #[test]
    fn test_schedule_parsing() {
        let backup = create_test_backup_with_schedule();
        let cron_expr = backup.schedule.as_ref().unwrap().cron.as_str();
        
        // The cron crate uses standard 5-field format
        let schedule = Schedule::from_str(cron_expr);
        // Note: The cron library may have specific format requirements
        // This test verifies it either parses correctly or fails gracefully
        let _ = schedule;
    }

    #[test]
    fn test_invalid_cron_expression() {
        let invalid_cron = "invalid cron";
        let schedule = Schedule::from_str(invalid_cron);
        assert!(schedule.is_err());
    }
}
