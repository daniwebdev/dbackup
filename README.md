# Rust Database Backup Tool

A robust, high-performance PostgreSQL database backup utility written in **Rust**, designed for reliability and ease of use through YAML-based configuration.

## 🚀 Key Features

- **YAML-Driven Configuration**: Manage all backup settings via a simple `.yml` file
- **PostgreSQL Support**: Native support for PostgreSQL databases using `pg_dump`
- **Dual Backup Modes**: 
  - **Basic Mode**: Single-threaded with maximum compression for smaller databases
  - **Parallel Mode**: Multi-threaded backups for faster processing of large databases
- **Flexible Storage**: Backup to local filesystems with automatic compression
- **High-Efficiency Compression**: Automatically applies optimal compression to minimize storage usage
- **Robust Error Handling**: Comprehensive error reporting and validation
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

The compiled binary will be available at `target/release/db-backup-tools`.

### Install Globally

```bash
cargo install --path .
```

## 📖 Usage

### Generate Sample Configuration

```bash
db-backup-tools generate -o backup.yml
```

This creates a sample configuration file that you can customize.

### Validate Configuration

```bash
db-backup-tools validate -c backup.yml
```

Validates your configuration file without running backups.

### Run Backup

Run all configured backups:
```bash
db-backup-tools backup -c backup.yml
```

Run a specific backup by name:
```bash
db-backup-tools backup -c backup.yml -n "Production PostgreSQL Database"
```

### Enable Verbose Logging

Set the `RUST_LOG` environment variable:
```bash
RUST_LOG=debug db-backup-tools backup -c backup.yml
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
      cron: "0 2 * * *"  # Daily at 2 AM (not yet implemented)
    storage:
      driver: local
      path: "/var/backups/databases/postgresql"
      filename_prefix: "prod_"
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

- `driver`: Storage driver type (currently supports "local")
- `path`: Directory where backups will be stored
- `filename_prefix`: Prefix for backup filenames (timestamp will be appended)

#### Schedule Settings (Future Feature)

- `cron`: Cron expression for scheduled backups (not yet implemented)

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

## 🗺️ Roadmap

- [ ] **Scheduled Backups**: Implement cron-based scheduling
- [ ] **S3 Storage**: Add support for S3-compatible cloud storage
- [ ] **MySQL Support**: Add MySQL/MariaDB backup support
- [ ] **Retention Policies**: Automatic cleanup of old backups
- [ ] **Encryption**: Add encryption for backup files
- [ ] **Notifications**: Email/Slack notifications for backup status
- [ ] **Incremental Backups**: Support for incremental backup strategies
- [ ] **Parallel Backups**: Run multiple backups concurrently
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
   db-backup-tools generate -o my-backup.yml
   ```

2. **Edit configuration with your database details:**
   ```bash
   nano my-backup.yml
   ```

3. **Validate configuration:**
   ```bash
   db-backup-tools validate -c my-backup.yml
   ```

4. **Run backup:**
   ```bash
   db-backup-tools backup -c my-backup.yml
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
