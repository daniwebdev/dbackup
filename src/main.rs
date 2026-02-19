mod config;
mod postgres;
mod mysql;
mod scheduler;
mod updater;
mod storage;
mod retention;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::Config;
use postgres::PostgresBackup;
use mysql::MysqlBackup;
use scheduler::BackupScheduler;
use updater::check_and_show_update;
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber;

// Version information from build environment
const VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD_DATE_ENV: Option<&str> = option_env!("BUILD_DATE");
const GIT_VERSION_ENV: Option<&str> = option_env!("GIT_VERSION");
const DEFAULT_CONFIG_PATH: &str = "/etc/dbackup/backup.yml";
const FALLBACK_CONFIG_PATH: &str = "backup.yml";

#[derive(Parser)]
#[command(name = "dbackup")]
#[command(version = VERSION)]
#[command(about = "A robust database backup utility")]
#[command(long_about = "A robust database backup utility with multi-engine support, cloud storage integration, and automated scheduling.\n\nExamples:\n  dbackup backup -c /path/to/config.yml          # Run all backups\n  dbackup backup -c /path/to/config.yml -n pg1  # Run specific backup\n  dbackup validate -c /path/to/config.yml       # Validate configuration\n  dbackup run -c /path/to/config.yml            # Start scheduled backups\n  dbackup update                                 # Check and install updates\n  dbackup version                                # Show version and build info")]
struct Cli {
    /// Show version information
    #[arg(long, global = true)]
    version: bool,

    /// Show help information
    #[arg(long, global = true)]
    help: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a backup based on configuration file
    Backup {
        /// Path to the configuration file (defaults to /etc/dbackup/backup.yml on Linux if available, otherwise backup.yml)
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Name of the backup to run (if not specified, runs all)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Validate the configuration file
    Validate {
        /// Path to the configuration file (defaults to /etc/dbackup/backup.yml on Linux if available, otherwise backup.yml)
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Generate a sample configuration file
    Generate {
        /// Output path for the configuration file
        #[arg(short, long, default_value = "backup.yml")]
        output: PathBuf,
    },
    /// Run scheduled backups (listens for cron schedules)
    Run {
        /// Path to the configuration file (defaults to /etc/dbackup/backup.yml on Linux if available, otherwise backup.yml)
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Maximum number of concurrent backup jobs (default: 2)
        #[arg(short, long, default_value = "2")]
        concurrency: usize,
    },
    /// Show version and build information
    Version,
    /// Check for and install the latest version
    Update,
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
    let git_version = GIT_VERSION_ENV.unwrap_or(VERSION);

    // Handle version flag first (has priority)
    if cli.version {
        show_version(git_version).await;
        return Ok(());
    }

    // Handle help flag
    if cli.help {
        println!("A robust database backup utility with multi-engine support, cloud storage integration, and automated scheduling.\n");
        println!("Usage: dbackup [COMMAND]\n");
        println!("Commands:");
        println!("  backup    Run a backup based on configuration file");
        println!("  validate  Validate the configuration file");
        println!("  generate  Generate a sample configuration file");
        println!("  run       Run scheduled backups (listens for cron schedules)");
        println!("  version   Show version and build information");
        println!("  update    Check for and install the latest version");
        println!("  help      Print this message or the help of the given subcommand(s)\n");
        println!("Options:");
        println!("  --version  Show version information");
        println!("  --help     Show help information");
        println!("  -h, --help Print help\n");
        println!("Examples:");
        println!("  dbackup backup -c /path/to/config.yml          # Run all backups");
        println!("  dbackup backup -c /path/to/config.yml -n pg1  # Run specific backup");
        println!("  dbackup validate -c /path/to/config.yml       # Validate configuration");
        println!("  dbackup run -c /path/to/config.yml            # Start scheduled backups");
        println!("  dbackup update                                 # Check and install updates");
        println!("  dbackup version                                # Show version and build info\n");
        return Ok(());
    }

    // If no subcommand provided, show help
    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            // Print help information
            println!("A robust database backup utility with multi-engine support, cloud storage integration, and automated scheduling.\n");
            println!("Usage: dbackup [COMMAND]\n");
            println!("Commands:");
            println!("  backup    Run a backup based on configuration file");
            println!("  validate  Validate the configuration file");
            println!("  generate  Generate a sample configuration file");
            println!("  run       Run scheduled backups (listens for cron schedules)");
            println!("  version   Show version and build information");
            println!("  update    Check for and install the latest version");
            println!("  help      Print this message or the help of the given subcommand(s)\n");
            println!("Options:");
            println!("  --version  Show version information");
            println!("  --help     Show help information");
            println!("  -h, --help Print help\n");
            println!("Examples:");
            println!("  dbackup backup -c /path/to/config.yml          # Run all backups");
            println!("  dbackup backup -c /path/to/config.yml -n pg1  # Run specific backup");
            println!("  dbackup validate -c /path/to/config.yml       # Validate configuration");
            println!("  dbackup run -c /path/to/config.yml            # Start scheduled backups");
            println!("  dbackup update                                 # Check and install updates");
            println!("  dbackup version                                # Show version and build info\n");
            return Ok(());
        }
    };

    match command {
        Commands::Backup { config, name } => {
            let config_path = resolve_config_path(config)?;
            run_backup(config_path, name).await?;
        }
        Commands::Validate { config } => {
            let config_path = resolve_config_path(config)?;
            validate_config(config_path).await?;
        }
        Commands::Generate { output } => {
            generate_sample_config(output)?;
        }
        Commands::Run { config, concurrency } => {
            let config_path = resolve_config_path(config)?;
            run_scheduled_backups(config_path, concurrency).await?;
        }
        Commands::Version => {
            show_version(git_version).await;
        }
        Commands::Update => {
            updater::update_binary(git_version).await?;
        }
    }

    Ok(())
}

fn resolve_config_path(explicit_config: Option<PathBuf>) -> Result<PathBuf> {
    // If config is explicitly provided via -c flag, use it (forced)
    if let Some(config) = explicit_config {
        return Ok(config);
    }

    // On Linux, check if default config exists at /etc/dbackup/backup.yml
    #[cfg(target_os = "linux")]
    {
        let default_path = PathBuf::from(DEFAULT_CONFIG_PATH);
        if default_path.exists() {
            info!("Using default configuration from: {}", DEFAULT_CONFIG_PATH);
            return Ok(default_path);
        }
    }

    // Fallback to backup.yml in current directory
    let fallback_path = PathBuf::from(FALLBACK_CONFIG_PATH);
    if fallback_path.exists() {
        info!("Using configuration from: {}", FALLBACK_CONFIG_PATH);
        Ok(fallback_path)
    } else {
        anyhow::bail!(
            "Configuration file not found. Tried: {} and {}",
            DEFAULT_CONFIG_PATH,
            FALLBACK_CONFIG_PATH
        )
    }
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
        match backup_config.driver.to_lowercase().as_str() {
            "postgresql" => {
                // Resolve storage configuration
                let storage_config = config.get_storage_for_backup(&backup_config)
                    .context(format!("Failed to resolve storage for backup '{}'", backup_config.name))?;

                let backup = PostgresBackup::new(backup_config.clone(), storage_config);
                
                // Validate connection before attempting backup
                backup.validate_connection()
                    .context("Connection validation failed")?;

                match backup.execute().await {
                    Ok(location) => {
                        info!("✓ Backup '{}' completed: {}", backup_config.name, location);
                    }
                    Err(e) => {
                        error!("✗ Backup '{}' failed: {}", backup_config.name, e);
                        return Err(e);
                    }
                }
            }
            "mysql" => {
                // Resolve storage configuration
                let storage_config = config.get_storage_for_backup(&backup_config)
                    .context(format!("Failed to resolve storage for backup '{}'", backup_config.name))?;

                let backup = MysqlBackup::new(backup_config.clone(), storage_config);
                
                // Validate connection before attempting backup
                backup.validate_connection()
                    .context("Connection validation failed")?;

                match backup.execute().await {
                    Ok(location) => {
                        info!("✓ Backup '{}' completed: {}", backup_config.name, location);
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

async fn validate_config(config_path: PathBuf) -> Result<()> {
    info!("Validating configuration: {}", config_path.display());
    
    let config = Config::from_file(&config_path)
        .context("Failed to load configuration file")?;

    // Validate each backup configuration
    for backup_config in &config.backups {
        info!("Validating backup: {}", backup_config.name);

        match backup_config.driver.to_lowercase().as_str() {
            "postgresql" => {
                // Resolve storage configuration
                let storage_config = config.get_storage_for_backup(backup_config)
                    .context(format!("Failed to resolve storage for backup '{}'", backup_config.name))?;

                // Validate storage connection
                info!("  Testing {} storage connection...", storage_config.driver);
                match storage_config.driver.to_lowercase().as_str() {
                    "s3" => {
                        storage::S3Storage::new(&storage_config)
                            .await
                            .context(format!("S3 storage validation failed for backup '{}'", backup_config.name))?;
                        info!("  ✓ S3 storage connection validated");
                    }
                    "local" => {
                        storage::LocalStorage::new(&storage_config)
                            .context(format!("Local storage validation failed for backup '{}'", backup_config.name))?;
                        info!("  ✓ Local storage validated");
                    }
                    driver => {
                        error!("Unsupported storage driver: {}", driver);
                        anyhow::bail!("Unsupported storage driver: {}", driver);
                    }
                }

                // Validate database connection
                let backup = PostgresBackup::new(backup_config.clone(), storage_config);
                backup.validate_connection()
                    .context(format!("Database validation failed for backup '{}'", backup_config.name))?;
                info!("  ✓ PostgreSQL connection validated");
            }
            "mysql" => {
                // Resolve storage configuration
                let storage_config = config.get_storage_for_backup(backup_config)
                    .context(format!("Failed to resolve storage for backup '{}'", backup_config.name))?;

                // Validate storage connection
                info!("  Testing {} storage connection...", storage_config.driver);
                match storage_config.driver.to_lowercase().as_str() {
                    "s3" => {
                        storage::S3Storage::new(&storage_config)
                            .await
                            .context(format!("S3 storage validation failed for backup '{}'", backup_config.name))?;
                        info!("  ✓ S3 storage connection validated");
                    }
                    "local" => {
                        storage::LocalStorage::new(&storage_config)
                            .context(format!("Local storage validation failed for backup '{}'", backup_config.name))?;
                        info!("  ✓ Local storage validated");
                    }
                    driver => {
                        error!("Unsupported storage driver: {}", driver);
                        anyhow::bail!("Unsupported storage driver: {}", driver);
                    }
                }

                // Validate database connection
                let backup = MysqlBackup::new(backup_config.clone(), storage_config);
                backup.validate_connection()
                    .context(format!("Database validation failed for backup '{}'", backup_config.name))?;
                info!("  ✓ MySQL connection validated");
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
  
  # Define reusable storage configurations
  storages:
    local_backup:
      driver: local
      path: "/var/backups/databases/postgresql"
      filename_prefix: "backup_"
    
    s3_aws:
      driver: s3
      bucket: my-backup-bucket
      region: us-east-1
      prefix: backups/postgresql/
      # Optional: For S3-compatible services (MinIO, etc.)
      # endpoint: https://minio.example.com:9000
      # Optional: Credentials (uses AWS SDK environment variables if not specified)
      # access_key_id: AKIAIOSFODNN7EXAMPLE
      # secret_access_key: wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
    
    s3_minio:
      driver: s3
      bucket: backups
      region: us-east-1
      prefix: postgresql/
      endpoint: http://minio.example.com:9000
      access_key_id: minioadmin
      secret_access_key: minioadmin

backups:
  - name: "Production PostgreSQL - Local"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: your_password_here
      database: production_db
    mode: parallel
    parallel_jobs: 4
    schedule:
      cron: "0 2 * * *"  # Daily at 2 AM
    # Reference centralized storage without custom prefix
    storage:
      ref: local_backup

  - name: "Production PostgreSQL - S3 AWS"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: your_password_here
      database: production_db
    mode: parallel
    parallel_jobs: 4
    schedule:
      cron: "0 3 * * *"  # Daily at 3 AM
    # Reference centralized storage with custom S3 prefix override
    storage:
      ref: s3_aws
      prefix: prod-backups/daily/  # Override the S3 prefix for this backup

  - name: "Development PostgreSQL - MinIO"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: your_password_here
      database: dev_db
    mode: basic
    # Reference MinIO storage with custom filename prefix
    storage:
      ref: s3_minio
      filename_prefix: dev_
      prefix: dev-backups/

  - name: "Production MySQL - Local"
    driver: mysql
    connection:
      host: db.example.com
      port: 3306
      username: backup_user
      password: your_mysql_password
      database: production_db
    mode: parallel
    parallel_jobs: 4
    schedule:
      cron: "0 2 * * *"  # Daily at 2 AM
    storage:
      ref: local_backup
      filename_prefix: mysql_

  - name: "Production MySQL - S3"
    driver: mysql
    connection:
      host: db.example.com
      port: 3306
      username: backup_user
      password: your_mysql_password
      database: production_db
    mode: parallel
    parallel_jobs: 4
    schedule:
      cron: "0 4 * * *"  # Daily at 4 AM
    storage:
      ref: s3_aws
      prefix: prod-backups/mysql/
"#;

    std::fs::write(&output_path, sample_config)
        .context("Failed to write sample configuration")?;

    info!("✓ Sample configuration generated: {}", output_path.display());
    info!("  Edit this file with your database credentials and paths");
    
    Ok(())
}

async fn show_version(git_version: &str) {
    let build_date = BUILD_DATE_ENV.unwrap_or("unknown");
    
    // Use git_version (from GitHub tag) if available, otherwise use package version
    let version = if git_version != "develop" {
        // Ensure version has 'v' prefix for GitHub tags
        if git_version.starts_with('v') {
            git_version.to_string()
        } else {
            format!("v{}", git_version)
        }
    } else {
        format!("v{}", VERSION)
    };
    
    println!("dbackup {}", version);
    
    if build_date != "unknown" {
        println!("Built: {}", build_date);
    }
    
    // Print system info
    println!("Platform: {}", std::env::consts::OS);
    println!("Architecture: {}", std::env::consts::ARCH);
    
    // Check for updates asynchronously
    if let Err(e) = check_and_show_update(git_version).await {
        // Silently ignore update check errors
        tracing::debug!("Update check failed: {}", e);
    }
}
