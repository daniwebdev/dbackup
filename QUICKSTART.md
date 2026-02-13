# Quick Start Guide

This guide will help you get started with the Database Backup Tool in just a few minutes.

## Step 1: Install Prerequisites

Make sure you have PostgreSQL client tools installed:

**macOS:**
```bash
brew install postgresql
```

**Ubuntu/Debian:**
```bash
sudo apt-get install postgresql-client
```

## Step 2: Build the Application

```bash
cd db-backup-tools
cargo build --release
```

The binary will be at `target/release/db-backup-tools`.

## Step 3: Generate Configuration

```bash
./target/release/db-backup-tools generate -o my-backup.yml
```

## Step 4: Edit Configuration

Open `my-backup.yml` and update with your database details:

```yaml
backups:
  - name: "My Database"
    driver: postgresql
    connection:
      host: localhost          # Your database host
      port: 5432              # Your database port
      username: postgres      # Your database username
      password: mypassword    # Your database password
      database: mydb          # Database name to backup
    storage:
      driver: local
      path: "/tmp/backups"    # Where to save backups
      filename_prefix: "mydb_"
```

## Step 5: Create Backup Directory

```bash
mkdir -p /tmp/backups
```

## Step 6: Validate Configuration

```bash
./target/release/db-backup-tools validate -c my-backup.yml
```

## Step 7: Run Your First Backup

```bash
./target/release/db-backup-tools backup -c my-backup.yml
```

## Step 8: Verify the Backup

```bash
ls -lh /tmp/backups/
```

You should see a file like `mydb_20260213_154530.sql.gz`.

## Step 9: Test Restore (Optional)

To verify your backup works:

```bash
# Create a test database
createdb test_restore

# Restore the backup
gunzip -c /tmp/backups/mydb_*.sql.gz | psql -d test_restore

# Verify the data
psql -d test_restore -c "\dt"

# Clean up
dropdb test_restore
```

## Next Steps

- **Automate backups**: Set up a cron job to run backups regularly
- **Secure your config**: Run `chmod 600 my-backup.yml` to protect passwords
- **Monitor backups**: Check logs with `RUST_LOG=info`
- **Multiple databases**: Add more entries to the `backups` array in your config

## Common Commands

```bash
# Generate config
db-backup-tools generate -o backup.yml

# Validate config
db-backup-tools validate -c backup.yml

# Run all backups
db-backup-tools backup -c backup.yml

# Run specific backup
db-backup-tools backup -c backup.yml -n "My Database"

# Enable debug logging
RUST_LOG=debug db-backup-tools backup -c backup.yml
```

## Troubleshooting

**"pg_dump: command not found"**
- Install PostgreSQL client tools (see Step 1)

**"Permission denied"**
- Create the backup directory: `mkdir -p /path/to/backups`
- Or change the path in config to a writable location

**"Connection refused"**
- Check if PostgreSQL is running: `pg_isready`
- Verify host, port, username, and password in config
- Check PostgreSQL's `pg_hba.conf` for access permissions

## Example Cron Setup

To run daily backups at 2 AM:

```bash
# Edit crontab
crontab -e

# Add this line:
0 2 * * * /path/to/db-backup-tools backup -c /path/to/backup.yml >> /var/log/db-backup.log 2>&1
```

That's it! You're now backing up your PostgreSQL databases with a robust, Rust-powered tool. 🎉
