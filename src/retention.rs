use anyhow::{Context, Result};
use std::time::Duration;
use std::time::SystemTime;
use std::path::Path;
use crate::config::StorageConfig;
use tracing::{info, debug};

/// Parses duration strings like "1d", "2d", "1w", "5m", "30s"
/// Returns the duration in seconds
pub fn parse_duration(duration_str: &str) -> Result<Duration> {
    let duration_str = duration_str.trim().to_lowercase();
    
    if duration_str.is_empty() {
        anyhow::bail!("Duration string cannot be empty");
    }
    
    // Split into number and unit
    let (num_str, unit) = if let Some(pos) = duration_str.find(|c: char| !c.is_numeric()) {
        duration_str.split_at(pos)
    } else {
        anyhow::bail!("Invalid duration format: '{}'. Expected format like '1d', '2w', '30m', '3600s'", duration_str);
    };
    
    let num: u64 = num_str.parse()
        .context(format!("Invalid number in duration: '{}'", num_str))?;
    
    let seconds = match unit {
        "s" | "sec" | "second" | "seconds" => num,
        "m" | "min" | "minute" | "minutes" => num * 60,
        "h" | "hour" | "hours" => num * 3600,
        "d" | "day" | "days" => num * 86400,
        "w" | "week" | "weeks" => num * 604800, // 7 * 86400
        "mon" | "month" | "months" => num * 2592000, // 30 * 86400 (approximate)
        "y" | "year" | "years" => num * 31536000, // 365 * 86400 (approximate)
        _ => anyhow::bail!(
            "Unknown time unit '{}'. Supported: s, m, h, d, w, mon, y (e.g., '1d', '2w', '30m')",
            unit
        ),
    };
    
    Ok(Duration::from_secs(seconds))
}

/// Apply retention policy to local storage
pub fn apply_local_retention(path: &Path, retention_days: &str) -> Result<()> {
    let retention_duration = parse_duration(retention_days)
        .context(format!("Invalid retention policy: '{}'", retention_days))?;
    
    if !path.exists() {
        debug!("Backup path does not exist: {}", path.display());
        return Ok(());
    }
    
    let now = SystemTime::now();
    let cutoff_time = now - retention_duration;
    
    info!("Applying retention policy: {} to local storage: {}", retention_days, path.display());
    
    let mut deleted_count = 0;
    
    // List files in the directory
    for entry in std::fs::read_dir(path)
        .context("Failed to read backup directory")? {
        
        let entry = entry.context("Failed to read directory entry")?;
        let file_path = entry.path();
        
        // Skip directories
        if file_path.is_dir() {
            continue;
        }
        
        // Check file modification time
        if let Ok(metadata) = std::fs::metadata(&file_path) {
            if let Ok(modified) = metadata.modified() {
                if modified < cutoff_time {
                    match std::fs::remove_file(&file_path) {
                        Ok(_) => {
                            info!("Deleted old backup: {}", file_path.display());
                            deleted_count += 1;
                        }
                        Err(e) => {
                            debug!("Failed to delete backup {}: {}", file_path.display(), e);
                        }
                    }
                }
            }
        }
    }
    
    info!("Retention cleanup removed {} backup(s)", deleted_count);
    Ok(())
}

/// Apply retention policy to S3 storage (async)
pub async fn apply_s3_retention(
    bucket: &str,
    prefix: &str,
    retention_policy: &str,
    storage_config: &StorageConfig,
) -> Result<()> {
    use crate::storage::S3Storage;
    
    let retention_duration = parse_duration(retention_policy)
        .context(format!("Invalid retention policy: '{}'", retention_policy))?;
    
    let s3_storage = S3Storage::new(storage_config).await?;
    
    info!("Applying retention policy: {} to S3: s3://{}/{}", 
          retention_policy, bucket, prefix);
    
    s3_storage.cleanup_old_backups(&retention_duration).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_seconds() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("1s").unwrap(), Duration::from_secs(1));
    }
    
    #[test]
    fn test_parse_minutes() {
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("1m").unwrap(), Duration::from_secs(60));
        assert_eq!(parse_duration("30min").unwrap(), Duration::from_secs(1800));
    }
    
    #[test]
    fn test_parse_hours() {
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_duration("2hour").unwrap(), Duration::from_secs(7200));
    }
    
    #[test]
    fn test_parse_days() {
        assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(86400));
        assert_eq!(parse_duration("2day").unwrap(), Duration::from_secs(172800));
    }
    
    #[test]
    fn test_parse_weeks() {
        assert_eq!(parse_duration("1w").unwrap(), Duration::from_secs(604800));
        assert_eq!(parse_duration("2weeks").unwrap(), Duration::from_secs(1209600));
    }
    
    #[test]
    fn test_parse_months() {
        assert_eq!(parse_duration("1mon").unwrap(), Duration::from_secs(2592000));
        assert_eq!(parse_duration("2month").unwrap(), Duration::from_secs(5184000));
    }
    
    #[test]
    fn test_parse_years() {
        assert_eq!(parse_duration("1y").unwrap(), Duration::from_secs(31536000));
    }
    
    #[test]
    fn test_invalid_format() {
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("1").is_err());
        assert!(parse_duration("").is_err());
    }
}
