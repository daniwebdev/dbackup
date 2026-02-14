# Automated Backup Setup Guide

This guide shows you how to set up automated, scheduled PostgreSQL backups using the Database Backup Tool.

## 🎯 Goal

Set up daily automated backups of your PostgreSQL databases with:
- Automatic execution via cron
- Log rotation
- Email notifications (optional)
- Backup verification

## 📋 Prerequisites

- Database Backup Tool installed
- PostgreSQL client tools installed
- Sufficient disk space for backups
- (Optional) Mail server configured for notifications

## 🔧 Step-by-Step Setup

### 1. Install the Application

```bash
# Build the application
cd /path/to/db-backup-tools
cargo build --release

# Install globally
sudo cp target/release/dbackup /usr/local/bin/
sudo chmod +x /usr/local/bin/dbackup

# Verify installation
dbackup --help
```

### 2. Create Configuration Directory

```bash
# Create configuration directory
sudo mkdir -p /etc/db-backup

# Create backup storage directory
sudo mkdir -p /var/backups/postgresql

# Set permissions
sudo chown $USER:$USER /etc/db-backup
sudo chown $USER:$USER /var/backups/postgresql
```

### 3. Generate and Configure

```bash
# Generate configuration
dbackup generate -o /etc/db-backup/backup.yml

# Edit configuration
nano /etc/db-backup/backup.yml
```

Example production configuration:

```yaml
drivers:
  filesystems:
    - name: "Local"
      type: "local"
      base_path: "/var/backups"

backups:
  - name: "Production Database"
    driver: postgresql
    connection:
      host: localhost
      port: 5432
      username: backup_user
      password: secure_password_here
      database: production_db
    storage:
      driver: local
      path: "/var/backups/postgresql"
      filename_prefix: "prod_"

  - name: "Analytics Database"
    driver: postgresql
    connection:
      host: analytics.example.com
      port: 5432
      username: backup_user
      password: secure_password_here
      database: analytics_db
    storage:
      driver: local
      path: "/var/backups/postgresql"
      filename_prefix: "analytics_"
```

### 4. Secure the Configuration

```bash
# Restrict access to config file (contains passwords)
chmod 600 /etc/db-backup/backup.yml

# Verify permissions
ls -la /etc/db-backup/backup.yml
# Should show: -rw------- 1 user user
```

### 5. Test the Backup

```bash
# Validate configuration
dbackup validate -c /etc/db-backup/backup.yml

# Run a test backup
dbackup backup -c /etc/db-backup/backup.yml

# Verify backup files
ls -lh /var/backups/postgresql/
```

### 6. Create Backup Script

Create `/usr/local/bin/run-db-backup.sh`:

```bash
#!/bin/bash

# Database Backup Automation Script
# This script runs the backup and handles logging and notifications

# Configuration
CONFIG_FILE="/etc/db-backup/backup.yml"
LOG_DIR="/var/log/db-backup"
LOG_FILE="$LOG_DIR/backup-$(date +%Y%m%d).log"
BACKUP_TOOL="/usr/local/bin/dbackup"
NOTIFICATION_EMAIL="admin@example.com"

# Create log directory if it doesn't exist
mkdir -p "$LOG_DIR"

# Function to send notification
send_notification() {
    local subject="$1"
    local message="$2"
    
    if command -v mail &> /dev/null; then
        echo "$message" | mail -s "$subject" "$NOTIFICATION_EMAIL"
    fi
}

# Start backup
echo "=== Database Backup Started at $(date) ===" >> "$LOG_FILE"

# Run backup with logging
if RUST_LOG=info "$BACKUP_TOOL" backup -c "$CONFIG_FILE" >> "$LOG_FILE" 2>&1; then
    echo "=== Database Backup Completed Successfully at $(date) ===" >> "$LOG_FILE"
    send_notification "✓ Database Backup Success" "Backup completed successfully at $(date)"
    exit 0
else
    echo "=== Database Backup Failed at $(date) ===" >> "$LOG_FILE"
    send_notification "✗ Database Backup Failed" "Backup failed at $(date). Check logs at $LOG_FILE"
    exit 1
fi
```

Make it executable:

```bash
sudo chmod +x /usr/local/bin/run-db-backup.sh
```

### 7. Set Up Cron Job

```bash
# Edit crontab
crontab -e
```

Add one of these schedules:

```bash
# Daily at 2:00 AM
0 2 * * * /usr/local/bin/run-db-backup.sh

# Every 6 hours
0 */6 * * * /usr/local/bin/run-db-backup.sh

# Daily at 2:00 AM on weekdays only
0 2 * * 1-5 /usr/local/bin/run-db-backup.sh

# Twice daily (2 AM and 2 PM)
0 2,14 * * * /usr/local/bin/run-db-backup.sh
```

### 8. Set Up Log Rotation

Create `/etc/logrotate.d/db-backup`:

```
/var/log/db-backup/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 0644 user user
}
```

Test log rotation:

```bash
sudo logrotate -f /etc/logrotate.d/db-backup
```

### 9. Set Up Backup Retention

Create `/usr/local/bin/cleanup-old-backups.sh`:

```bash
#!/bin/bash

# Cleanup old backups (keep last 30 days)
BACKUP_DIR="/var/backups/postgresql"
RETENTION_DAYS=30

echo "Cleaning up backups older than $RETENTION_DAYS days..."

find "$BACKUP_DIR" -name "*.sql.gz" -type f -mtime +$RETENTION_DAYS -delete

echo "Cleanup completed at $(date)"
```

Make it executable:

```bash
sudo chmod +x /usr/local/bin/cleanup-old-backups.sh
```

Add to crontab (runs daily at 3 AM):

```bash
0 3 * * * /usr/local/bin/cleanup-old-backups.sh >> /var/log/db-backup/cleanup.log 2>&1
```

### 10. Set Up Monitoring

Create `/usr/local/bin/check-backup-health.sh`:

```bash
#!/bin/bash

# Check if backups are running successfully
BACKUP_DIR="/var/backups/postgresql"
MAX_AGE_HOURS=26  # Alert if no backup in last 26 hours

# Find most recent backup
LATEST_BACKUP=$(find "$BACKUP_DIR" -name "*.sql.gz" -type f -printf '%T@ %p\n' | sort -n | tail -1 | cut -d' ' -f2-)

if [ -z "$LATEST_BACKUP" ]; then
    echo "ERROR: No backups found in $BACKUP_DIR"
    exit 1
fi

# Check age of latest backup
BACKUP_AGE_SECONDS=$(( $(date +%s) - $(stat -c %Y "$LATEST_BACKUP") ))
BACKUP_AGE_HOURS=$(( BACKUP_AGE_SECONDS / 3600 ))

if [ $BACKUP_AGE_HOURS -gt $MAX_AGE_HOURS ]; then
    echo "WARNING: Latest backup is $BACKUP_AGE_HOURS hours old"
    echo "Latest backup: $LATEST_BACKUP"
    exit 1
else
    echo "OK: Latest backup is $BACKUP_AGE_HOURS hours old"
    echo "Latest backup: $LATEST_BACKUP"
    exit 0
fi
```

Make it executable:

```bash
sudo chmod +x /usr/local/bin/check-backup-health.sh
```

Add to crontab (check every hour):

```bash
0 * * * * /usr/local/bin/check-backup-health.sh >> /var/log/db-backup/health.log 2>&1
```

## 🧪 Testing the Setup

### Test Manual Execution

```bash
# Run the backup script manually
/usr/local/bin/run-db-backup.sh

# Check the logs
tail -f /var/log/db-backup/backup-$(date +%Y%m%d).log

# Verify backup files
ls -lh /var/backups/postgresql/
```

### Test Cron Execution

```bash
# Wait for scheduled time or temporarily change cron to run soon
# Then check logs
tail -f /var/log/db-backup/backup-$(date +%Y%m%d).log
```

### Test Backup Restoration

```bash
# Create test database
createdb test_restore

# Find latest backup
LATEST_BACKUP=$(ls -t /var/backups/postgresql/*.sql.gz | head -1)

# Restore
gunzip -c "$LATEST_BACKUP" | psql -d test_restore

# Verify
psql -d test_restore -c "\dt"

# Cleanup
dropdb test_restore
```

## 📊 Monitoring and Maintenance

### Check Backup Status

```bash
# View recent logs
tail -100 /var/log/db-backup/backup-$(date +%Y%m%d).log

# Check disk usage
df -h /var/backups/postgresql

# List recent backups
ls -lht /var/backups/postgresql/ | head -10

# Check backup sizes
du -sh /var/backups/postgresql/*
```

### Monthly Maintenance Checklist

- [ ] Verify backups are running successfully
- [ ] Check disk space usage
- [ ] Test restore procedure
- [ ] Review and update retention policy
- [ ] Verify log rotation is working
- [ ] Check for any error messages in logs
- [ ] Update passwords if needed

## 🔐 Security Best Practices

1. **Database User**: Create a dedicated backup user with minimal permissions:

```sql
-- Create backup user
CREATE USER backup_user WITH PASSWORD 'secure_password';

-- Grant read-only access
GRANT CONNECT ON DATABASE production_db TO backup_user;
GRANT USAGE ON SCHEMA public TO backup_user;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO backup_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO backup_user;
```

2. **File Permissions**: Ensure proper permissions:

```bash
chmod 600 /etc/db-backup/backup.yml
chmod 700 /var/backups/postgresql
chmod 600 /var/backups/postgresql/*.sql.gz
```

3. **Network Security**: Use SSL for remote connections:

```yaml
connection:
  host: database.example.com
  port: 5432
  username: backup_user
  password: secure_password
  database: production_db
  # Add SSL parameters to pg_dump command if needed
```

## 📧 Email Notifications Setup

If you want email notifications, install and configure mail:

```bash
# Install mail utility (Ubuntu/Debian)
sudo apt-get install mailutils

# Test email
echo "Test message" | mail -s "Test Subject" admin@example.com
```

## 🚨 Troubleshooting

### Cron Job Not Running

```bash
# Check cron service
sudo systemctl status cron

# Check cron logs
grep CRON /var/log/syslog

# Verify crontab
crontab -l
```

### Permission Errors

```bash
# Check file ownership
ls -la /etc/db-backup/backup.yml
ls -la /var/backups/postgresql/

# Fix permissions
sudo chown $USER:$USER /etc/db-backup/backup.yml
sudo chmod 600 /etc/db-backup/backup.yml
```

### Disk Space Issues

```bash
# Check disk usage
df -h /var/backups

# Find large backups
du -sh /var/backups/postgresql/* | sort -h

# Manually cleanup old backups
find /var/backups/postgresql -name "*.sql.gz" -mtime +30 -delete
```

## ✅ Verification Checklist

After setup, verify:

- [ ] Configuration file is secure (chmod 600)
- [ ] Manual backup works
- [ ] Backup files are created in correct location
- [ ] Backup files are compressed
- [ ] Cron job is scheduled
- [ ] Logs are being created
- [ ] Log rotation is configured
- [ ] Retention cleanup is scheduled
- [ ] Health check is running
- [ ] Email notifications work (if configured)
- [ ] Restore procedure tested

## 🎉 Success!

You now have a fully automated PostgreSQL backup system with:
- ✅ Scheduled daily backups
- ✅ Automatic log rotation
- ✅ Backup retention policy
- ✅ Health monitoring
- ✅ Email notifications
- ✅ Secure configuration

Your databases are now protected! 🛡️
