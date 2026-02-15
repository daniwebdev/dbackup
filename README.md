# DBackup Tool

A robust, high-performance PostgreSQL database backup utility written in **Rust**, designed for reliability and ease of use through YAML-based configuration.

## 🚀 Key Features

- **YAML-Driven Configuration**: Manage all backup settings via a simple `.yml` file
- **PostgreSQL Support**: Native support for PostgreSQL databases using `pg_dump`
- **Dual Backup Modes**: 
  - **Basic Mode**: Single-threaded with maximum compression for smaller databases
  - **Parallel Mode**: Multi-threaded backups for faster processing of large databases
- **Scheduled Backups**: Cron-based scheduling with concurrent execution
- **Systemd Integration**: Run as a service with automatic restart and resource limits
- **Flexible Storage**: Backup to local filesystems or S3-compatible cloud storage
- **S3 Storage Support**: Amazon S3 and S3-compatible services (MinIO, DigitalOcean Spaces, etc.)
- **High-Efficiency Compression**: Automatically applies optimal compression to minimize storage usage
- **Robust Error Handling**: Comprehensive error reporting and validation
- **Memory Efficient**: Streaming I/O, concurrent jobs with semaphore-based limits
- **CLI Interface**: Easy-to-use command-line interface for backup operations
- **Multi-Database Management**: Handle multiple database instances within a single config

## 📋 Prerequisites

- **Rust** 1.70 or higher
- **PostgreSQL Client Tools** (`pg_dump` must be installed and accessible in PATH)

### Installing PostgreSQL Client Tools

**macOS:**
```bash
brew install postgresql
```

**Ubuntu/Debian:**
```bash
sudo apt-get install postgresql-client
```

**RHEL/CentOS:**
```bash
sudo yum install postgresql
```

## 🔧 Installation

### From Source

```bash
git clone <repository-url>
cd db-backup-tools
cargo build --release
```

The compiled binary will be available at `target/release/dbackup`.

### Install Globally

```bash
cargo install --path .
```

## 📖 Usage

### Generate Sample Configuration

```bash
dbackup generate -o backup.yml
```

This creates a sample configuration file that you can customize.

### Validate Configuration

```bash
dbackup validate -c backup.yml
```

Validates your configuration file without running backups.

### Run Backup (One-Time)

Run all configured backups:
```bash
dbackup backup -c backup.yml
```

Run a specific backup by name:
```bash
dbackup backup -c backup.yml -n "Production PostgreSQL Database"
```

### Run Scheduled Backups (Daemon Mode)

Listen for scheduled backups with up to 4 concurrent jobs:
```bash
dbackup run -c backup.yml --concurrency 4
```

Or use the default concurrency of 2:
```bash
dbackup run -c backup.yml
```

This command will:
- Parse all backups with `schedule` defined
- Listen for scheduled times based on cron expressions
- Execute backups automatically at scheduled times
- Run up to N backups concurrently
- Continue listening until stopped (Ctrl+C)

### Enable Verbose Logging

Set the `RUST_LOG` environment variable:
```bash
RUST_LOG=debug dbackup backup -c backup.yml
```

For scheduled backups:
```bash
RUST_LOG=info dbackup run -c backup.yml
```

## ⚙️ Configuration Format

```yaml
# Database Backup Configuration

settings:
  binary:
    # Optional: Specify full paths to backup binaries
    # If not specified, the tool will use the binary from PATH
    pg_dump: /usr/bin/pg_dump
    mysqldump: /usr/bin/mysqldump

  # Define reusable storage configurations (NEW!)
  # Each backup references a storage by key, making configs DRY
  storages:
    local_backup:
      driver: local
      path: "/var/backups/databases/postgresql"
      filename_prefix: "backup_"
    
    s3_prod:
      driver: s3
      bucket: my-production-backups
      region: us-east-1
      prefix: postgresql/prod/

backups:
  - name: "Production PostgreSQL Database"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: your_password_here
      database: production_db
    # Backup mode: 'basic' or 'parallel'
    mode: parallel
    # Number of parallel jobs (only for parallel mode)
    parallel_jobs: 4
    schedule:
      cron: "0 2 * * *"  # Daily at 2 AM
    # Storage reference - can override prefix/filename_prefix
    storage:
      ref: local_backup
      # Optional: Override path prefix or filename prefix
      # prefix: "/custom/backup/path"
      # filename_prefix: "custom_"

  - name: "Production PostgreSQL to S3"
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
    # Storage reference with custom S3 path prefix
    storage:
      ref: s3_prod
      prefix: postgresql/prod/daily/  # Override S3 path for daily backups
```

### Configuration Options

#### Global Settings

- `settings.binary.pg_dump`: (Optional) Full path to `pg_dump` binary
  - If omitted, uses `pg_dump` from system PATH
  - Example: `/usr/bin/pg_dump`, `/opt/postgresql/bin/pg_dump`

- `settings.binary.mysqldump`: (Optional) Full path to `mysqldump` binary
  - If omitted, uses `mysqldump` from system PATH
  - Example: `/usr/bin/mysqldump`, `/opt/mysql/bin/mysqldump`

#### Backup Mode Settings

- `mode`: Backup mode (optional, default: `basic`)
  - **`basic`**: Single-threaded backup using custom format with maximum compression
    - Best for: Small to medium databases
    - Format: Custom PostgreSQL format with gzip compression
    - Filename: `{prefix}{timestamp}.dump.gz`
  - **`parallel`**: Multi-threaded backup using directory format
    - Best for: Large databases (faster backup)
    - Format: Directory format compressed to tar.gz
    - Filename: `{prefix}{timestamp}.dir.tar.gz`
    - Requires: `parallel_jobs` setting

- `parallel_jobs`: Number of parallel jobs (optional, default: `2`)
  - Only used when `mode: parallel`
  - Recommended: Number of CPU cores or less
  - Higher values = faster backups but more resource usage

#### Per-Backup Binary Override

- `binary_path`: (Optional) Override the default binary path for this specific backup
  - If specified, this takes precedence over global `settings.binary.*` and system PATH
  - Example: `binary_path: /custom/path/to/pg_dump`

#### Connection Settings

- `host`: Database server hostname or IP address
- `port`: Database server port (default: 5432 for PostgreSQL)
- `username`: Database username
- `password`: Database password (stored in config - consider using environment variables)
- `database`: Name of the database to backup

#### Storage Settings

The tool supports two approaches to define storage:

**Approach 1: Centralized Storage Configuration (Recommended)**

Define reusable storage configurations under `settings.storages` and reference them via `storage` object:

```yaml
settings:
  storages:
    local:
      driver: local
      path: "/var/backups"
      filename_prefix: "backup_"
    
    s3_prod:
      driver: s3
      bucket: my-bucket
      region: us-east-1
      prefix: backups/

backups:
  - name: MyBackup
    # ... other config ...
    storage:
      ref: local  # Reference the storage above
```

Benefits:
- DRY principle: Define storage once, use in multiple backups
- Easy maintenance: Change one storage config, all backups using it update automatically
- Clear organization: All storage settings in one place

**Approach 2: Inline Storage Configuration (Backward Compatible)**

Define storage directly in each backup config:

```yaml
backups:
  - name: MyBackup
    # ... other config ...
    storage:
      driver: local
      path: "/var/backups"
      filename_prefix: "backup_"
```

**Storage Configuration Options:**

For **local** storage (via `storage` object or inline):
- `driver: "local"`
- `path`: Directory where backups will be stored
- `filename_prefix`: Prefix for backup filenames (timestamp will be appended)

For **S3** storage (via `storage` object or inline):
- `driver: "s3"`
- `bucket`: S3 bucket name (required)
- `region`: AWS region (required, e.g., "us-east-1")
- `prefix`: Key prefix for backups in S3 (optional, default: "backups/")
- `endpoint`: Custom endpoint for S3-compatible services (optional)
- `access_key_id`: AWS access key (optional, uses AWS SDK env vars if not specified)
- `secret_access_key`: AWS secret key (optional, uses AWS SDK env vars if not specified)

#### Schedule Settings (Future Feature)

- `cron`: Cron expression for scheduled backups (not yet implemented)

## ☁️ Centralized Storage Configuration

DBackup supports **centralized storage configuration** in the `settings.storages` section, allowing you to define reusable storage profiles that multiple backups can reference. This is the **recommended approach** for managing multiple backups efficiently.

### Benefits

1. **DRY Principle**: Define storage configuration once
2. **Easy Maintenance**: Update storage settings in one place
3. **Reusability**: Multiple backups can share the same storage
4. **Clear Organization**: All storage settings centralized in settings section

### Example: Multiple Backups Using Same Storage

```yaml
settings:
  binary:
    pg_dump: /usr/bin/pg_dump

  storages:
    local_backups:
      driver: local
      path: "/var/backups/postgresql"
      filename_prefix: "backup_"
    
    s3_backups:
      driver: s3
      bucket: company-backups
      region: us-east-1
      prefix: postgresql/

backups:
  - name: "Production Database"
    driver: postgresql
    connection:
      host: prod-db.internal
      port: 5432
      username: postgres
      password: secret
      database: prod_db
    mode: parallel
    parallel_jobs: 4
    # Reference storage with optional prefix override
    storage:
      ref: s3_backups
      # prefix: "prod-backups/  # Optional: override S3 prefix

  - name: "Development Database"
    driver: postgresql
    connection:
      host: dev-db.internal
      port: 5432
      username: postgres
      password: secret
      database: dev_db
    mode: basic
    storage:
      ref: local_backups

  - name: "Staging Database"
    driver: postgresql
    connection:
      host: staging-db.internal
      port: 5432
      username: postgres
      password: secret
      database: staging_db
    mode: parallel
    parallel_jobs: 2
    storage:
      ref: s3_backups
      prefix: staging-backups/  # Custom S3 path for staging
```

### Real-World Use Cases

**Use Case 1: Hybrid Storage**
- Development backups → Local storage
- Production backups → S3 (remote, redundant)

**Use Case 2: Multi-Region S3 Backups**
```yaml
settings:
  storages:
    s3_us_east:
      driver: s3
      bucket: backups-us-east
      region: us-east-1
      prefix: postgresql/
    
    s3_eu_west:
      driver: s3
      bucket: backups-eu-west
      region: eu-west-1
      prefix: postgresql/

backups:
  - name: "Backup to US"
    driver: postgresql
    connection:
      host: db.example.com
      port: 5432
      username: postgres
      password: secret
      database: mydb
    storage:
      ref: s3_us_east
  
  - name: "Backup to EU"
    driver: postgresql
    connection:
      host: db.example.com
      port: 5432
      username: postgres
      password: secret
      database: mydb
    storage:
      ref: s3_eu_west
```

**Use Case 3: MinIO for Development**
```yaml
settings:
  storages:
    minio_local:
      driver: s3
      bucket: backups
      region: minio
      endpoint: http://minio.local:9000
      access_key_id: minioadmin
      secret_access_key: minioadmin
      prefix: dev-backups/

backups:
  - name: "Dev Database"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: secret
      database: devdb
    storage:
      ref: minio_local
```

## ☁️ S3 Storage Configuration

The tool supports backing up directly to Amazon S3 or S3-compatible services like MinIO, DigitalOcean Spaces, and others.

### AWS S3 Setup

1. **Create S3 Bucket**: Create an S3 bucket for your backups
   ```bash
   aws s3api create-bucket --bucket my-backup-bucket --region us-east-1
   ```

2. **Configure AWS Credentials**: Use one of these methods:
   - **Environment Variables** (recommended for containers):
     ```bash
     export AWS_ACCESS_KEY_ID="your_access_key"
     export AWS_SECRET_ACCESS_KEY="your_secret_key"
     export AWS_DEFAULT_REGION="us-east-1"
     ```
   - **AWS Credentials File** (~/.aws/credentials):
     ```ini
     [default]
     aws_access_key_id = your_access_key
     aws_secret_access_key = your_secret_key
     ```
   - **IAM Role** (when running on EC2): Attach an IAM role to your instance with S3 permissions

3. **Configuration in backup.yml**:
   ```yaml
   storage:
     driver: s3
     bucket: my-backup-bucket
     region: us-east-1
     prefix: backups/postgresql/  # Optional prefix
   ```

### S3-Compatible Services (MinIO, DigitalOcean, etc.)

For S3-compatible services, use the `endpoint` parameter:

```yaml
storage:
  driver: s3
  bucket: my-backup-bucket
  region: us-east-1
  prefix: backups/
  endpoint: https://minio.example.com:9000
  access_key_id: minioadmin
  secret_access_key: minioadmin
```

### Example: Systemd Service with S3

```yaml
settings:
  binary:
    pg_dump: /usr/bin/pg_dump

backups:
  - name: "Daily Production Backup to S3"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: your_password
      database: production_db
    mode: parallel
    parallel_jobs: 4
    schedule:
      cron: "0 2 * * *"  # Daily at 2 AM
    storage:
      driver: s3
      bucket: my-company-backups
      region: us-east-1
      prefix: postgresql/prod/
```

### S3 Backup Verification

List your backups in S3:
```bash
aws s3 ls s3://my-backup-bucket/backups/postgresql/
```

Download a backup:
```bash
aws s3 cp s3://my-backup-bucket/backups/postgresql/prod_20260215_020000.dump.gz ./
```

Restore from S3:
```bash
# Download and restore
aws s3 cp s3://my-backup-bucket/backups/postgresql/prod_20260215_020000.dump.gz - | \
  gunzip | pg_restore -h localhost -U postgres -d production_db
```

## 📁 Backup File Format

Backups are created with different formats depending on the mode:

### Basic Mode
```
{filename_prefix}{timestamp}.dump.gz
```

Example:
```
prod_20260213_154530.dump.gz
```

The backup files are:
- Custom PostgreSQL format (pg_dump -Fc)
- Double compressed (pg_dump --compress=9 + gzip)
- Optimized for storage efficiency

### Parallel Mode
```
{filename_prefix}{timestamp}.dir.tar.gz
```

Example:
```
prod_20260213_154530.dir.tar.gz
```

The backup files are:
- Directory format (pg_dump -Fd) compressed to tar.gz
- Created using multiple parallel jobs
- Optimized for backup speed on large databases

## 🔄 Restoring Backups

### Restoring Basic Mode Backups

```bash
# Decompress and restore
gunzip -c prod_20260213_154530.dump.gz | pg_restore -h localhost -U postgres -d production_db

# Or restore directly (pg_restore handles compressed files)
pg_restore -h localhost -U postgres -d production_db prod_20260213_154530.dump.gz
```

### Restoring Parallel Mode Backups

```bash
# Extract the tar.gz
tar -xzf prod_20260213_154530.dir.tar.gz -C /tmp/restore_dir

# Restore using pg_restore with parallel jobs
pg_restore -h localhost -U postgres -d production_db -j 4 -Fd /tmp/restore_dir

# Cleanup
rm -rf /tmp/restore_dir
```

## ⚡ Performance Considerations

### When to Use Basic Mode

- **Small to medium databases** (< 10 GB)
- **Storage space is limited** (maximum compression)
- **Backup speed is not critical**
- **Single-core systems**

**Advantages:**
- Smaller backup files (double compression)
- Lower resource usage
- Simpler restore process

### When to Use Parallel Mode

- **Large databases** (> 10 GB)
- **Fast backup is critical**
- **Multi-core systems available**
- **Network or disk I/O is the bottleneck**

**Advantages:**
- Significantly faster backups (2-4x speed improvement)
- Better utilization of multi-core CPUs
- Can parallelize restore as well

**Example Performance:**
- 50 GB database with 4 parallel jobs: ~15-20 minutes (vs 45-60 minutes in basic mode)
- Recommended `parallel_jobs`: 2-4 for most systems, up to 8 for very large databases

## 🛡️ Security Considerations

1. **Password Storage**: Currently, passwords are stored in the YAML configuration file. Consider:
   - Using environment variables
   - Restricting file permissions: `chmod 600 backup.yml`
   - Storing the config file in a secure location

2. **Backup File Permissions**: Backup files may contain sensitive data. Ensure proper file permissions on the backup directory.

3. **Network Security**: When backing up remote databases, ensure secure network connections.

## � Running with Systemd

To run dbackup as a systemd service for automatic scheduled backups:

### 1. Install the Binary

```bash
sudo cp target/release/dbackup /usr/local/bin/
sudo chmod +x /usr/local/bin/dbackup
```

### 2. Create Configuration Directory

```bash
sudo mkdir -p /etc/dbackup /var/lib/dbackup /var/backups/postgresql
sudo chown postgres:postgres /etc/dbackup /var/lib/dbackup /var/backups/postgresql
sudo chmod 700 /etc/dbackup
```

### 3. Setup Configuration

```bash
sudo cp backup.yml /etc/dbackup/backup.yml
sudo chown postgres:postgres /etc/dbackup/backup.yml
sudo chmod 600 /etc/dbackup/backup.yml
```

### 4. Install Systemd Service

```bash
sudo cp dbackup.service /etc/systemd/system/
sudo systemctl daemon-reload
```

### 5. Enable and Start the Service

```bash
# Enable on boot
sudo systemctl enable dbackup.service

# Start the service
sudo systemctl start dbackup.service

# Check status
sudo systemctl status dbackup.service

# View logs
sudo journalctl -u dbackup.service -f
```

### Configuration Example with Schedules

Edit `/etc/dbackup/backup.yml`:

```yaml
settings:
  binary:
    pg_dump: /usr/bin/pg_dump
  
  storages:
    production_backups:
      driver: local
      path: "/var/backups/postgresql"
      filename_prefix: "prod_"
    
    analytics_backups:
      driver: local
      path: "/var/backups/postgresql"
      filename_prefix: "analytics_"

backups:
  - name: "Daily Production Backup"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: your_password
      database: production_db
    mode: parallel
    parallel_jobs: 4
    schedule:
      cron: "0 2 * * *"  # Daily at 2 AM
    storage:
      ref: production_backups

  - name: "Hourly Analytics Backup"
    driver: postgresql
    connection:
      host: analytics.example.com
      port: 5432
      username: postgres
      password: your_password
      database: analytics_db
    mode: basic
    schedule:
      cron: "0 * * * *"  # Every hour
    storage:
      ref: analytics_backups
```

### Cron Expression Format

Format: `minute hour day month weekday`

Common examples:
- `0 2 * * *` - Daily at 2:00 AM
- `0 * * * *` - Every hour
- `0 0 * * 0` - Weekly (every Sunday at midnight)
- `0 0 1 * *` - Monthly (1st day at midnight)
- `*/30 * * * *` - Every 30 minutes
- `0 2,14 * * *` - At 2 AM and 2 PM daily

## �️ Retention Policies

DBackup supports automatic cleanup of old backup files based on configurable retention policies. This helps manage storage costs and maintains disk space.

### Retention Policy Format

Retention policies use simple, human-readable duration strings:

- **Seconds**: `30s`, `60s`, `120s`
- **Minutes**: `5m`, `30m`, `60min`
- **Hours**: `1h`, `2hour`, `24hours`
- **Days**: `1d`, `7d`, `30day`
- **Weeks**: `1w`, `2week`, `4weeks`
- **Months**: `1mon`, `3month`, `12months`
- **Years**: `1y`, `2year`

### Configuration Example

Add the `retention` field to any backup configuration:

```yaml
settings:
  binary:
    pg_dump: /usr/bin/pg_dump
  
  storages:
    local_backup:
      driver: local
      path: "/var/backups/databases"
      filename_prefix: "backup_"
    
    s3_backup:
      driver: s3
      bucket: my-backups
      region: us-east-1
      prefix: postgresql/

backups:
  - name: "Daily Production Backup"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: secret
      database: prod_db
    mode: parallel
    parallel_jobs: 4
    schedule:
      cron: "0 2 * * *"  # Daily at 2 AM
    storage:
      ref: local_backup
    retention: "30d"  # Keep backups for 30 days

  - name: "Weekly Archive Backup"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: secret
      database: prod_db
    mode: parallel
    parallel_jobs: 4
    schedule:
      cron: "0 0 * * 0"  # Weekly on Sunday
    storage:
      ref: s3_backup
    retention: "1y"  # Keep weekly backups for 1 year

  - name: "Hourly Backup (Short Retention)"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: postgres
      password: secret
      database: analytics_db
    mode: basic
    schedule:
      cron: "0 * * * *"  # Every hour
    storage:
      ref: local_backup
      prefix: "analytics/"
    retention: "7d"  # Keep hourly backups for 7 days only
```

### How Retention Works

1. **Local Storage**: Files older than the retention period are deleted from the filesystem
2. **S3 Storage**: Objects (files) older than the retention period are deleted from the S3 bucket
3. **Cleanup happens**: When the backup job completes, old backups are automatically removed
4. **Safe deletion**: Only backups matching the storage prefix are considered for deletion

### Retention Policy Examples

| Policy | Duration | Use Case |
|--------|----------|----------|
| `7d` | 7 days | Short-term local backups |
| `30d` | 30 days | Monthly rotation |
| `90d` | 90 days | Quarterly backups |
| `1y` | 1 year | Archive/long-term backups |
| `2w` | 2 weeks | Development/test environments |
| `1mon` | ~30 days | Monthly archive |
| `3mon` | ~90 days | Quarterly compliance |

### Best Practices

1. **Local Storage**: Shorter retention (7-30 days) to save disk space
2. **S3 Storage**: Longer retention (30-365 days) since cloud storage is cheaper
3. **Multiple Backups**: Use different retention for different backup frequencies
   - Hourly backups: 7 days
   - Daily backups: 30 days
   - Weekly backups: 1 year
4. **Compliance**: Set retention to match your compliance requirements

## �🗺️ Roadmap

- [x] **Scheduled Backups**: Implement cron-based scheduling ✅ DONE
- [x] **S3 Storage**: Add support for S3-compatible cloud storage ✅ DONE
- [x] **Retention Policies**: Automatic cleanup of old backups ✅ DONE
- [ ] **MySQL Support**: Add MySQL/MariaDB backup support
- [ ] **Encryption**: Add encryption for backup files
- [ ] **Notifications**: Email/Slack notifications for backup status
- [ ] **Incremental Backups**: Support for incremental backup strategies
- [ ] **Environment Variables**: Support for credentials via environment variables

## 🧪 Testing

Run the test suite:
```bash
cargo test
```

Run with verbose output:
```bash
cargo test -- --nocapture
```

## 📝 Example Workflow

1. **Generate configuration:**
   ```bash
   dbackup generate -o my-backup.yml
   ```

2. **Edit configuration with your database details:**
   ```bash
   nano my-backup.yml
   ```

3. **Validate configuration:**
   ```bash
   dbackup validate -c my-backup.yml
   ```

4. **Run backup:**
   ```bash
   dbackup backup -c my-backup.yml
   ```

5. **Verify backup file:**
   ```bash
   ls -lh /var/backups/databases/postgresql/
   ```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is open source and available under the MIT License.

## 🐛 Troubleshooting

### "pg_dump: command not found"

Install PostgreSQL client tools (see Prerequisites section).

### Permission Denied

Ensure the backup directory exists and has proper write permissions:
```bash
sudo mkdir -p /var/backups/databases/postgresql
sudo chown $USER:$USER /var/backups/databases/postgresql
```

### Connection Failed

- Verify database credentials
- Check if PostgreSQL is running
- Ensure network connectivity to the database server
- Check PostgreSQL's `pg_hba.conf` for access permissions

### Large Database Backups

For very large databases, consider:
- Ensuring sufficient disk space (backups are compressed but still require space)
- Using `RUST_LOG=info` to monitor progress
- Running backups during off-peak hours
