```markdown
# Rust Database Backup Tool

A robust, high-performance database backup utility written in **Rust**, designed for reliability and ease of use through YAML-based configuration.

## Key Features

- **YAML-Driven Configuration**: Manage all backup settings via a simple `.yml` file.
- **Multi-Engine Support**: Native support for **MySQL** and **PostgreSQL**.
- **Flexible Storage**: Backup to **Local** filesystems or **S3-compatible** cloud storage.
- **Automated Scheduling**: Built-in **Cron** support for hands-off operations.
- **High-Efficiency Compression**: Automatically applies optimal compression to minimize storage usage.
- **Integrated Notifications**: Stay informed with status alerts for every backup job.
- **Multi-Database Management**: Handle multiple database instances and targets within a single config.