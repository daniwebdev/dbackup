# Scheduled Backups Quick Start

This guide shows how to set up automated scheduled backups with dbackup.

## Quick Setup (5 minutes)

### 1. Create Configuration

```bash
dbackup generate -o backup.yml
```

### 2. Edit Configuration

Add schedules to your backups:

```yaml
backups:
  - name: "Production Database"
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
      driver: local
      path: "/var/backups/postgresql"
      filename_prefix: "prod_"
```

### 3. Validate Configuration

```bash
dbackup validate -c backup.yml
```

### 4. Run Scheduled Backups

In the foreground (useful for testing):
```bash
dbackup run -c backup.yml --concurrency 2
```

With logging:
```bash
RUST_LOG=info dbackup run -c backup.yml
```

## Production Setup with Systemd

### 1. Install Binary

```bash
sudo cp target/release/dbackup /usr/local/bin/
```

### 2. Create Directories

```bash
sudo mkdir -p /etc/dbackup /var/lib/dbackup /var/backups/postgresql
sudo chown postgres:postgres /etc/dbackup /var/lib/dbackup /var/backups/postgresql
sudo chmod 700 /etc/dbackup
```

### 3. Copy Configuration

```bash
sudo cp backup.yml /etc/dbackup/backup.yml
sudo chown postgres:postgres /etc/dbackup/backup.yml
sudo chmod 600 /etc/dbackup/backup.yml
```

### 4. Install Service

```bash
sudo cp dbackup.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable dbackup.service
sudo systemctl start dbackup.service
```

### 5. Monitor

```bash
# Check status
sudo systemctl status dbackup.service

# View logs (last 50 lines)
sudo journalctl -u dbackup.service -n 50

# Follow logs in real-time
sudo journalctl -u dbackup.service -f

# View logs for specific date
sudo journalctl -u dbackup.service --since "2026-02-14"
```

## Understanding Concurrency

The `--concurrency` parameter controls how many backups can run simultaneously:

- **Default: 2** - Good for most setups
- **1** - Sequential (one at a time, slowest)
- **4** - Up to 4 backups in parallel (faster but uses more resources)
- **N** - Adjust based on CPU cores and available memory

Example: 4 concurrent jobs
```bash
dbackup run -c backup.yml --concurrency 4
```

## Cron Expression Examples

Format: `minute hour day_of_month month day_of_week`

```
# Every day at 2 AM
0 2 * * *

# Every 6 hours
0 */6 * * *

# Every day at 2 AM and 2 PM
0 2,14 * * *

# Every Monday at midnight
0 0 * * 1

# Every 1st day of month at 3 AM
0 3 1 * *

# Every 15 minutes
*/15 * * * *

# Every day at 1:30 AM
30 1 * * *

# Every Sunday at 2 AM (weekly)
0 2 * * 0
```

## Troubleshooting

### Backups not running

1. Check the cron expression: `dbackup validate -c backup.yml`
2. Verify logs: `sudo journalctl -u dbackup.service -f`
3. Check if PostgreSQL is accessible: `psql -h localhost -U postgres -d your_database`

### High memory usage

Reduce concurrency:
```bash
dbackup run -c backup.yml --concurrency 1
```

Update systemd service:
```bash
sudo systemctl edit dbackup.service
# Change: ExecStart=/usr/local/bin/dbackup run -c /etc/dbackup/backup.yml --concurrency 1
```

### Service not starting

1. Check syntax: `sudo systemctl status dbackup.service`
2. View error logs: `sudo journalctl -u dbackup.service -n 20`
3. Test manually: `dbackup run -c /etc/dbackup/backup.yml`

## Performance Tips

1. **Adjust parallel_jobs**: Use 4-8 for large databases
2. **Set appropriate concurrency**: Balance between speed and resource usage
3. **Schedule backups during off-peak hours**: Reduces database load
4. **Use separate backup drives**: Improves I/O performance
5. **Monitor memory**: Check with `free -h` or systemd limits

## Memory Efficiency

dbackup is designed to be memory-efficient:
- Streams data using async I/O
- Uses semaphore-based concurrency limits
- The systemd service has memory limits set (512M-1G)
- Cleans up temporary files after each backup

Example resource usage:
- Small DB (1 GB): ~50-100 MB
- Medium DB (10 GB): ~100-200 MB
- Large DB (100+ GB): ~200-500 MB

## Next Steps

- Set up multiple backup schedules
- Configure retention policies (coming soon)
- Enable notifications (coming soon)
- Add S3 storage support (coming soon)
