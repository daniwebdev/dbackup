FROM alpine:latest

# Install PostgreSQL, MySQL/MariaDB clients, and dependencies for downloading dbackup
RUN apk add --no-cache \
    postgresql-client \
    mariadb-client \
    curl \
    jq \
    && rm -rf /var/cache/apk/*

# Verify installations
RUN pg_dump --version && \
    mysqldump --version

# Download and install dbackup from latest GitHub release
RUN RELEASE_URL=$(curl -s https://api.github.com/repos/daniwebdev/dbackup/releases/latest | jq -r '.assets[] | select(.name | contains("x86_64-unknown-linux-musl")) | .browser_download_url' | head -1) && \
    if [ -z "$RELEASE_URL" ]; then \
      echo "Failed to find dbackup release for x86_64-unknown-linux-musl"; \
      exit 1; \
    fi && \
    curl -sL "$RELEASE_URL" -o /tmp/dbackup.tar.gz && \
    tar -xzf /tmp/dbackup.tar.gz -C /usr/local/bin/ && \
    chmod +x /usr/local/bin/dbackup && \
    rm /tmp/dbackup.tar.gz && \
    dbackup --version

# Set working directory
WORKDIR /backups

# Default command
CMD ["/bin/sh"]