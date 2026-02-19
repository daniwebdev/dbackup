FROM alpine:latest

# Install PostgreSQL and MySQL/MariaDB clients
RUN apk add --no-cache \
    postgresql-client \
    mariadb-client \
    && rm -rf /var/cache/apk/*

# Verify installations
RUN pg_dump --version && \
    mysqldump --version

# Set working directory
WORKDIR /backups

# Default command
CMD ["/bin/sh"]