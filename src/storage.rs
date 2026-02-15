use crate::config::StorageConfig;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{info, debug};
use aws_sdk_s3::primitives::ByteStream;

#[async_trait::async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store backup data from file to the configured storage backend
    async fn store(&self, local_path: &PathBuf, filename: &str) -> Result<String>;
    
    /// Get the display name for where the backup was stored
    fn get_location_display(&self) -> String;
}

/// Local filesystem storage backend
pub struct LocalStorage {
    path: PathBuf,
}

impl LocalStorage {
    pub fn new(config: &StorageConfig) -> Result<Self> {
        let path = config.path.as_ref()
            .context("Local storage requires 'path' configuration")?
            .clone();
        
        // Ensure the output directory exists
        std::fs::create_dir_all(&path)
            .context("Failed to create output directory")?;
        
        Ok(Self { path })
    }
}

#[async_trait::async_trait]
impl StorageBackend for LocalStorage {
    async fn store(&self, local_path: &PathBuf, _filename: &str) -> Result<String> {
        // File is already at the desired location for local storage
        // This is handled by the backup process directly
        let location = local_path.display().to_string();
        info!("Backup file available at: {}", location);
        Ok(location)
    }
    
    fn get_location_display(&self) -> String {
        self.path.display().to_string()
    }
}

/// AWS S3 storage backend
pub struct S3Storage {
    bucket: String,
    prefix: String,
    client: aws_sdk_s3::Client,
}

impl S3Storage {
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        let bucket = config.bucket.as_ref()
            .context("S3 storage requires 'bucket' configuration")?
            .clone();
        
        let region = config.region.as_ref()
            .context("S3 storage requires 'region' configuration")?
            .clone();
        
        let prefix = config.prefix.as_ref()
            .map(|p| p.clone())
            .unwrap_or_else(|| "backups/".to_string());
        
        info!("Initializing S3 storage: bucket={}, region={}", bucket, region);
        
        // Configure AWS SDK
        let mut config_builder = aws_config::defaults(aws_config::BehaviorVersion::latest());
        
        // Override endpoint if provided (for S3-compatible services)
        if let Some(endpoint) = &config.endpoint {
            debug!("Using custom endpoint: {}", endpoint);
            config_builder = config_builder
                .region(aws_config::Region::new(region))
                .endpoint_url(endpoint.clone());
        } else {
            config_builder = config_builder.region(aws_config::Region::new(region));
        }
        
        // Build AWS config
        let aws_config = config_builder.load().await;
        
        let mut builder = aws_sdk_s3::config::Builder::from(&aws_config);
        
        // Force path-style URLs for S3-compatible services
        if config.endpoint.is_some() {
            debug!("Using path-style URLs for S3-compatible endpoint");
            builder = builder.force_path_style(true);
        }
        
        // Override credentials if provided
        if let (Some(access_key), Some(secret_key)) = (&config.access_key_id, &config.secret_access_key) {
            debug!("Using provided AWS credentials");
            let credentials = aws_sdk_s3::config::Credentials::new(
                access_key.clone(),
                secret_key.clone(),
                None,
                None,
                "dbackup",
            );
            builder = builder.credentials_provider(credentials);
        }
        
        let s3_config = builder.build();
        let client = aws_sdk_s3::Client::from_conf(s3_config);
        
        // Verify bucket accessibility
        debug!("Verifying S3 bucket access: {}", bucket);
        client
            .head_bucket()
            .bucket(&bucket)
            .send()
            .await
            .context(format!("Failed to access S3 bucket: {}", bucket))?;
        
        info!("Successfully connected to S3 bucket: {}", bucket);
        
        Ok(Self {
            bucket,
            prefix,
            client,
        })
    }
    
    /// Delete old backups based on retention duration
    pub async fn cleanup_old_backups(&self, retention_duration: &std::time::Duration) -> Result<()> {
        use std::time::SystemTime;
        
        info!("Listing objects in bucket: {}, prefix: {}", self.bucket, self.prefix);
        
        let now = SystemTime::now();
        let cutoff_time = now - *retention_duration;
        
        let mut deleted_count = 0;
        let mut continuation_token: Option<String> = None;
        
        loop {
            let mut list_request = self.client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&self.prefix);
            
            if let Some(token) = continuation_token {
                list_request = list_request.continuation_token(token);
            }
            
            let response = list_request
                .send()
                .await
                .context("Failed to list S3 objects")?;
            
            // Process files
            let contents = response.contents();
            for obj in contents {
                if let Some(last_modified) = obj.last_modified() {
                    // Convert AWS DateTime to SystemTime
                    // AWS DateTime is Unix timestamp, convert via u64
                    let timestamp_secs = last_modified.secs();
                    let nanos = last_modified.subsec_nanos();
                    let obj_time = SystemTime::UNIX_EPOCH + std::time::Duration::new(timestamp_secs as u64, nanos);
                    
                    if obj_time < cutoff_time {
                        if let Some(key) = obj.key() {
                            match self.client
                                .delete_object()
                                .bucket(&self.bucket)
                                .key(key)
                                .send()
                                .await {
                                Ok(_) => {
                                    info!("Deleted old S3 object: s3://{}/{}", self.bucket, key);
                                    deleted_count += 1;
                                }
                                Err(e) => {
                                    debug!("Failed to delete S3 object {}: {}", key, e);
                                }
                            }
                        }
                    }
                }
            }
            
            // Check if there are more results
            if response.is_truncated().unwrap_or(false) {
                continuation_token = response.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }
        
        info!("S3 retention cleanup removed {} object(s)", deleted_count);
        Ok(())
    }
}

#[async_trait::async_trait]
impl StorageBackend for S3Storage {
    async fn store(&self, local_path: &PathBuf, filename: &str) -> Result<String> {
        info!("Uploading backup to S3: s3://{}/{}{}", self.bucket, self.prefix, filename);
        
        // Read the backup file
        let file_data = tokio::fs::read(local_path)
            .await
            .context("Failed to read backup file")?;
        
        let key = format!("{}{}", self.prefix, filename);
        
        // Upload to S3
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(file_data))
            .send()
            .await
            .context(format!("Failed to upload to S3: {}", key))?;
        
        info!("Successfully uploaded to S3: s3://{}/{}", self.bucket, key);
        
        let location = format!("s3://{}/{}", self.bucket, key);
        Ok(location)
    }
    
    fn get_location_display(&self) -> String {
        format!("s3://{}/{}", self.bucket, self.prefix)
    }
}

/// Factory function to create the appropriate storage backend
pub async fn create_storage(config: &StorageConfig) -> Result<Box<dyn StorageBackend>> {
    match config.driver.to_lowercase().as_str() {
        "local" => {
            let storage = LocalStorage::new(config)?;
            Ok(Box::new(storage))
        }
        "s3" => {
            let storage = S3Storage::new(config).await?;
            Ok(Box::new(storage))
        }
        driver => {
            anyhow::bail!("Unsupported storage driver: {}", driver);
        }
    }
}
