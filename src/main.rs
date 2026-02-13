mod config;
mod postgres;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::Config;
use postgres::PostgresBackup;
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "db-backup-tools")]
#[command(about = "A robust database backup utility", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a backup based on configuration file
    Backup {
        /// Path to the configuration file
        #[arg(short, long, default_value = "backup.yml")]
        config: PathBuf,

        /// Name of the backup to run (if not specified, runs all)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Validate the configuration file
    Validate {
        /// Path to the configuration file
        #[arg(short, long, default_value = "backup.yml")]
        config: PathBuf,
    },
    /// Generate a sample configuration file
    Generate {
        /// Output path for the configuration file
        #[arg(short, long, default_value = "backup.yml")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Backup { config, name } => {
            run_backup(config, name).await?;
        }
        Commands::Validate { config } => {
            validate_config(config)?;
        }
        Commands::Generate { output } => {
            generate_sample_config(output)?;
        }
    }

    Ok(())
}

async fn run_backup(config_path: PathBuf, backup_name: Option<String>) -> Result<()> {
    info!("Loading configuration from: {}", config_path.display());
    let config = Config::from_file(&config_path)
        .context("Failed to load configuration file")?;

    let backups_to_run: Vec<_> = match backup_name {
        Some(name) => config
            .backups
            .iter()
            .filter(|b| b.name == name)
            .cloned()
            .collect(),
        None => config.backups.clone(),
    };

    if backups_to_run.is_empty() {
        error!("No backups found to run");
        anyhow::bail!("No backups configured or matching the specified name");
    }

    info!("Running {} backup(s)", backups_to_run.len());

    for backup_config in backups_to_run {
        match backup_config.driver.as_str() {
            "postgresql" => {
                let backup = PostgresBackup::new(backup_config.clone());
                
                // Validate connection before attempting backup
                backup.validate_connection()
                    .context("Connection validation failed")?;

                match backup.execute().await {
                    Ok(path) => {
                        info!("✓ Backup '{}' completed: {}", backup_config.name, path.display());
                    }
                    Err(e) => {
                        error!("✗ Backup '{}' failed: {}", backup_config.name, e);
                        return Err(e);
                    }
                }
            }
            driver => {
                error!("Unsupported database driver: {}", driver);
                anyhow::bail!("Unsupported driver: {}", driver);
            }
        }
    }

    info!("All backups completed successfully");
    Ok(())
}

fn validate_config(config_path: PathBuf) -> Result<()> {
    info!("Validating configuration: {}", config_path.display());
    
    let config = Config::from_file(&config_path)
        .context("Failed to load configuration file")?;

    // Validate each backup configuration
    for backup_config in &config.backups {
        info!("Validating backup: {}", backup_config.name);

        match backup_config.driver.as_str() {
            "postgresql" => {
                let backup = PostgresBackup::new(backup_config.clone());
                backup.validate_connection()
                    .context(format!("Validation failed for backup '{}'", backup_config.name))?;
            }
            driver => {
                error!("Unsupported database driver: {}", driver);
                anyhow::bail!("Unsupported driver: {}", driver);
            }
        }

        info!("✓ Backup '{}' configuration is valid", backup_config.name);
    }

    info!("✓ Configuration is valid");
    Ok(())
}

fn generate_sample_config(output_path: PathBuf) -> Result<()> {
    let sample_config = r#"# Database Backup Configuration
drivers:
  filesystems:
    - name: "Local"
      type: "local"
      base_path: "/var/backups/databases"

backups:
  - name: "Production PostgreSQL Database"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: your_password_here
      database: production_db
    schedule:
      cron: "0 2 * * *"  # Daily at 2 AM
    storage:
      driver: local
      path: "/var/backups/databases/postgresql"
      filename_prefix: "prod_db_"

  - name: "Development PostgreSQL Database"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: your_password_here
      database: dev_db
    storage:
      driver: local
      path: "/var/backups/databases/postgresql"
      filename_prefix: "dev_db_"
"#;

    std::fs::write(&output_path, sample_config)
        .context("Failed to write sample configuration")?;

    info!("✓ Sample configuration generated: {}", output_path.display());
    info!("  Edit this file with your database credentials and paths");
    
    Ok(())
}
