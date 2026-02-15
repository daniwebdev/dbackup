mod config;
mod postgres;
mod scheduler;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::Config;
use postgres::PostgresBackup;
use scheduler::BackupScheduler;
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber;

// Version information from build environment
// const VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD_DATE_ENV: Option<&str> = option_env!("BUILD_DATE");
const GIT_VERSION_ENV: Option<&str> = option_env!("GIT_VERSION");

#[derive(Parser)]
#[command(name = "dbackup")]
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
    /// Run scheduled backups (listens for cron schedules)
    Run {
        /// Path to the configuration file
        #[arg(short, long, default_value = "backup.yml")]
        config: PathBuf,

        /// Maximum number of concurrent backup jobs (default: 2)
        #[arg(short, long, default_value = "2")]
        concurrency: usize,
    },
    /// Show version and build information
    Version,
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
        Commands::Run { config, concurrency } => {
            run_scheduled_backups(config, concurrency).await?;
        }
        Commands::Version => {
            show_version();
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

async fn run_scheduled_backups(config_path: PathBuf, concurrency: usize) -> Result<()> {
    info!("Loading configuration from: {}", config_path.display());
    let config = Config::from_file(&config_path)
        .context("Failed to load configuration file")?;

    // Validate concurrency setting
    if concurrency == 0 {
        anyhow::bail!("Concurrency must be greater than 0");
    }

    // Create scheduler
    let scheduler = BackupScheduler::new(config, concurrency);

    // Run the scheduler (infinite loop until Ctrl+C)
    scheduler.run().await
}

fn generate_sample_config(output_path: PathBuf) -> Result<()> {
    let sample_config = r#"# Database Backup Configuration

settings:
  binary:
    # Optional: Specify full paths to backup binaries
    # If not specified, the tool will use the binary from PATH
    pg_dump: /usr/bin/pg_dump
    mysqldump: /usr/bin/mysqldump

backups:
  - name: "Production PostgreSQL Database"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: your_password_here
      database: production_db
    # Backup mode: 'basic' (single-threaded, custom format) or 'parallel' (multi-threaded, directory format)
    mode: parallel
    # Number of parallel jobs (only used in parallel mode, default: 2)
    parallel_jobs: 4
    schedule:
      cron: "0 2 * * *"  # Daily at 2 AM
    storage:
      driver: local
      path: "/var/backups/databases/postgresql"
      filename_prefix: "prod_"

  - name: "Development PostgreSQL Database"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: your_password_here
      database: dev_db
    # Using basic mode for smaller databases (mode is optional, defaults to 'basic')
    mode: basic
    storage:
      driver: local
      path: "/var/backups/databases/postgresql"
      filename_prefix: "dev_"
"#;

    std::fs::write(&output_path, sample_config)
        .context("Failed to write sample configuration")?;

    info!("✓ Sample configuration generated: {}", output_path.display());
    info!("  Edit this file with your database credentials and paths");
    
    Ok(())
}

fn show_version() {
    let git_version = GIT_VERSION_ENV.unwrap_or("develop");
    let build_date = BUILD_DATE_ENV.unwrap_or("unknown");
    
    if git_version == "develop" {
        println!("dbackup (develop)");
    } else {
        if build_date == "unknown" {
            println!("dbackup ({})", git_version);
        } else {
            println!("dbackup ({}) built on {}", git_version, build_date);
        }
    }
}
