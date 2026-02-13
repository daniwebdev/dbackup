# Rust Database Backup Tool

A robust, high-performance PostgreSQL database backup utility written in **Rust**, designed for reliability and ease of use through YAML-based configuration.

## 🚀 Key Features

- **YAML-Driven Configuration**: Manage all backup settings via a simple `.yml` file
- **PostgreSQL Support**: Native support for PostgreSQL databases using `pg_dump`
- **Flexible Storage**: Backup to local filesystems with automatic compression
- **High-Efficiency Compression**: Automatically applies gzip compression to minimize storage usage
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
      cron: "0 2 * * *"  # Daily at 2 AM (not yet implemented)
    storage:
      driver: local
      path: "/var/backups/databases/postgresql"
      filename_prefix: "prod_db_"
```

### Configuration Options

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

Backups are created with the following naming convention:
```
{filename_prefix}{timestamp}.sql.gz
```

Example:
```
prod_db_20260213_154530.sql.gz
```

The backup files are:
- Plain SQL format (compatible with `psql` restore)
- Compressed with gzip
- Include verbose output for debugging

## 🔄 Restoring Backups

To restore a PostgreSQL backup:

```bash
# Decompress and restore
gunzip -c prod_db_20260213_154530.sql.gz | psql -h localhost -U postgres -d production_db
```

Or in two steps:
```bash
# Decompress
gunzip prod_db_20260213_154530.sql.gz

# Restore
psql -h localhost -U postgres -d production_db -f prod_db_20260213_154530.sql
```

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
