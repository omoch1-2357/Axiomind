# Deployment Guide

Production deployment guide for axm_web server.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Configuration](#configuration)
- [Building for Production](#building-for-production)
- [Deployment Options](#deployment-options)
- [Environment Variables](#environment-variables)
- [Logging and Monitoring](#logging-and-monitoring)
- [Performance Tuning](#performance-tuning)
- [Security Considerations](#security-considerations)
- [Backup and Recovery](#backup-and-recovery)

## Overview

The axm_web server is designed for local single-user deployment. This guide covers:

- Building optimized production binaries
- Configuration options for different environments
- Deployment strategies (local, containerized)
- Monitoring and logging setup
- Security hardening

## Prerequisites

### System Requirements

**Minimum:**
- CPU: 2 cores
- RAM: 2GB
- Disk: 500MB (application) + storage for hand history

**Recommended:**
- CPU: 4+ cores
- RAM: 4GB+
- Disk: 5GB+ for hand history storage

### Software Requirements

- **Rust**: 1.70+ (stable toolchain)
- **Operating System**: Linux, macOS, Windows 10+
- **Optional**: Docker for containerized deployment

## Configuration

### Server Configuration

Create a configuration file or use environment variables:

```rust
ServerConfig {
    host: "127.0.0.1",      // Bind address
    port: 8080,              // Port number
    static_dir: "./static",  // Static files directory
}
```

### Configuration File (Optional)

Create `config.toml` in the working directory:

```toml
[server]
host = "127.0.0.1"
port = 8080
static_dir = "./static"

[game]
default_level = 1
session_timeout_minutes = 30
max_concurrent_sessions = 10

[logging]
level = "info"
format = "json"  # or "pretty"
file = "./logs/axm_web.log"
```

### Static Files Setup

Ensure static files directory exists and contains:

```
static/
├── index.html          # Main game UI
├── styles.css          # Stylesheet
├── app.js             # Client-side JavaScript
└── assets/
    └── cards/         # Card image assets
        ├── AS.svg
        ├── KH.svg
        └── ...
```

## Building for Production

### Release Build

```bash
# Build optimized release binary
cargo build -p axm_web --release

# Binary location
# target/release/libaxm_web.rlib (library)
# target/release/axm (CLI with server functionality)
```

### Optimization Flags

For maximum performance, add to `Cargo.toml`:

```toml
[profile.release]
opt-level = 3
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization, slower compile
strip = true         # Strip symbols for smaller binary
```

### Build Script

Create `build.sh`:

```bash
#!/bin/bash
set -e

echo "Building axm_web for production..."

# Clean previous builds
cargo clean

# Build with optimizations
cargo build -p axm_web --release

# Verify build
cargo test -p axm_web --release

# Run clippy checks
cargo clippy -p axm_web --release -- -D warnings

echo "Build complete: target/release/axm"
```

Make executable and run:
```bash
chmod +x build.sh
./build.sh
```

## Deployment Options

### Option 1: Local Binary Deployment

**Step 1: Build and Install**

```bash
# Build release binary
cargo build -p axm_cli --release

# Install to system (optional)
cargo install --path rust/cli
```

**Step 2: Setup Directory Structure**

```bash
mkdir -p /opt/axiomind
cd /opt/axiomind

# Copy binary
cp target/release/axm .

# Setup static files
mkdir -p static
cp -r rust/web/static/* static/

# Create data directory
mkdir -p data/hands
```

**Step 3: Start Server**

```bash
# Start server
./axm serve --host 127.0.0.1 --port 8080 --static-dir ./static

# Or with systemd (see below)
```

### Option 2: Systemd Service (Linux)

Create `/etc/systemd/system/axm-web.service`:

```ini
[Unit]
Description=Axiomind Web Server
After=network.target

[Service]
Type=simple
User=axiomind
Group=axiomind
WorkingDirectory=/opt/axiomind
ExecStart=/opt/axiomind/axm serve --host 127.0.0.1 --port 8080 --static-dir ./static
Restart=on-failure
RestartSec=10

# Environment
Environment="RUST_LOG=info"
Environment="AXM_WEB_PORT=8080"

# Logging
StandardOutput=append:/var/log/axiomind/output.log
StandardError=append:/var/log/axiomind/error.log

[Install]
WantedBy=multi-user.target
```

**Setup and start:**

```bash
# Create user
sudo useradd -r -s /bin/false axiomind

# Set permissions
sudo chown -R axiomind:axiomind /opt/axiomind

# Create log directory
sudo mkdir -p /var/log/axiomind
sudo chown axiomind:axiomind /var/log/axiomind

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable axm-web
sudo systemctl start axm-web

# Check status
sudo systemctl status axm-web

# View logs
sudo journalctl -u axm-web -f
```

### Option 3: Docker Deployment

**Create `Dockerfile`:**

```dockerfile
FROM rust:1.70 as builder

WORKDIR /app
COPY . .

# Build release binary
RUN cargo build -p axm_cli --release

FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 axiomind

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/axm /app/axm
COPY --from=builder /app/rust/web/static /app/static

# Set ownership
RUN chown -R axiomind:axiomind /app

USER axiomind

# Expose port
EXPOSE 8080

# Run server
CMD ["./axm", "serve", "--host", "0.0.0.0", "--port", "8080", "--static-dir", "./static"]
```

**Create `docker-compose.yml`:**

```yaml
version: '3.8'

services:
  axm-web:
    build: .
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
    volumes:
      - ./data:/app/data  # Persist hand history
      - ./logs:/app/logs  # Persist logs
    restart: unless-stopped
```

**Build and run:**

```bash
# Build image
docker build -t axiomind/axm-web:latest .

# Run with docker-compose
docker-compose up -d

# View logs
docker-compose logs -f

# Stop
docker-compose down
```

## Environment Variables

### Server Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `AXM_WEB_HOST` | Bind address | `127.0.0.1` |
| `AXM_WEB_PORT` | Port number | `8080` |
| `AXM_WEB_STATIC_DIR` | Static files directory | `./static` |

### Logging Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level filter | `info` |
| `RUST_LOG_FORMAT` | Log format (json/pretty) | `pretty` |

**Examples:**

```bash
# Debug level for axm_web only
RUST_LOG=axm_web=debug ./axm serve

# JSON formatted logs
RUST_LOG_FORMAT=json ./axm serve

# Custom port
AXM_WEB_PORT=3000 ./axm serve
```

### Game Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `AXM_DEFAULT_LEVEL` | Default blind level | `1` |
| `AXM_SESSION_TIMEOUT` | Session timeout (minutes) | `30` |

## Logging and Monitoring

### Structured Logging

The server uses `tracing` for structured logging:

```bash
# Set log level
RUST_LOG=debug ./axm serve

# Module-specific levels
RUST_LOG=axm_web=debug,warp=info ./axm serve

# JSON format for log aggregation
RUST_LOG_FORMAT=json ./axm serve
```

### Log Levels

- `error`: Critical errors requiring attention
- `warn`: Warning conditions
- `info`: Informational messages (default)
- `debug`: Detailed debugging information
- `trace`: Very verbose tracing

### Log Output

Logs are written to:
- **stdout**: Application logs
- **stderr**: Error logs
- **File**: Optional file output (configure via CLI or config file)

### Example Log Entries

```json
{
  "timestamp": "2025-11-02T10:30:00.123Z",
  "level": "INFO",
  "target": "axm_web::session",
  "message": "Session created",
  "fields": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "config": {"level": 1, "opponent": "Baseline"}
  }
}
```

### Monitoring Metrics

Access health endpoint for basic metrics:

```bash
curl http://localhost:8080/api/health
```

Response:
```json
{
  "status": "healthy",
  "uptime_seconds": 3625,
  "active_sessions": 3,
  "total_hands_played": 142
}
```

### External Monitoring

For production deployments, integrate with monitoring tools:

- **Prometheus**: Export metrics (future enhancement)
- **Grafana**: Visualization dashboards
- **Loki**: Log aggregation
- **Alertmanager**: Alert on error conditions

## Performance Tuning

### Tokio Runtime Configuration

The server uses Tokio async runtime with default settings. For high-concurrency scenarios:

```rust
tokio::runtime::Builder::new_multi_thread()
    .worker_threads(8)           // Adjust based on CPU cores
    .thread_name("axm-worker")
    .enable_all()
    .build()
```

### Memory Management

Monitor memory usage:

```bash
# Check process memory
ps aux | grep axm

# Docker memory usage
docker stats axiomind-axm-web
```

**Memory limits (systemd):**

```ini
[Service]
MemoryMax=1G
MemoryHigh=800M
```

### Connection Limits

Configure OS limits for concurrent connections:

```bash
# Linux: Increase file descriptor limit
ulimit -n 4096

# Systemd service
[Service]
LimitNOFILE=4096
```

### Static Asset Caching

Static files include cache headers. Configure reverse proxy (nginx) for additional caching:

```nginx
location /assets/ {
    expires 1y;
    add_header Cache-Control "public, immutable";
}
```

## Security Considerations

### Network Security

**Bind to localhost only** for single-user deployment:
```bash
./axm serve --host 127.0.0.1 --port 8080
```

**Reverse proxy** for external access (if needed):

```nginx
server {
    listen 80;
    server_name poker.example.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### File Permissions

Restrict file permissions:

```bash
# Binary
chmod 755 /opt/axiomind/axm

# Configuration
chmod 600 /opt/axiomind/config.toml

# Data directory
chmod 700 /opt/axiomind/data
chown -R axiomind:axiomind /opt/axiomind/data
```

### Input Validation

All API inputs are validated. However, for production:

- **Rate limiting**: Implement API rate limits (not currently included)
- **Request size limits**: Already enforced by Warp
- **CORS configuration**: Restrict origins if exposed externally

### TLS/HTTPS

For secure communication, use reverse proxy with TLS:

**Nginx with Let's Encrypt:**

```bash
# Install certbot
sudo apt install certbot python3-certbot-nginx

# Obtain certificate
sudo certbot --nginx -d poker.example.com

# Auto-renewal
sudo systemctl enable certbot.timer
```

## Backup and Recovery

### Data Backup

**Hand history files:**

```bash
# Backup data directory
tar -czf axiomind-backup-$(date +%Y%m%d).tar.gz /opt/axiomind/data

# Incremental backup (rsync)
rsync -av --progress /opt/axiomind/data/ /backup/axiomind/
```

**Automated backup (cron):**

```bash
# Edit crontab
crontab -e

# Daily backup at 2 AM
0 2 * * * tar -czf /backup/axiomind-$(date +\%Y\%m\%d).tar.gz /opt/axiomind/data
```

### Recovery

**Restore from backup:**

```bash
# Stop server
sudo systemctl stop axm-web

# Restore data
tar -xzf axiomind-backup-20251102.tar.gz -C /opt/axiomind/

# Set permissions
sudo chown -R axiomind:axiomind /opt/axiomind/data

# Start server
sudo systemctl start axm-web
```

### Configuration Backup

Version control your configuration:

```bash
# Initialize git repo for config
cd /opt/axiomind
git init
git add config.toml
git commit -m "Initial configuration"
```

## Troubleshooting

For common issues and solutions, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

### Quick Diagnostics

```bash
# Check if server is running
curl http://localhost:8080/api/health

# Check port binding
netstat -tuln | grep 8080

# View recent logs
sudo journalctl -u axm-web -n 100

# Test configuration
./axm serve --config config.toml --dry-run
```

## Upgrade Procedure

**Step 1: Backup**
```bash
# Backup current binary and data
cp /opt/axiomind/axm /opt/axiomind/axm.backup
tar -czf data-backup.tar.gz /opt/axiomind/data
```

**Step 2: Build New Version**
```bash
# Pull latest code
git pull origin main

# Build release
cargo build -p axm_cli --release
```

**Step 3: Deploy**
```bash
# Stop server
sudo systemctl stop axm-web

# Replace binary
cp target/release/axm /opt/axiomind/axm

# Restart server
sudo systemctl start axm-web

# Verify
curl http://localhost:8080/api/health
```

**Step 4: Rollback (if needed)**
```bash
# Stop server
sudo systemctl stop axm-web

# Restore old binary
cp /opt/axiomind/axm.backup /opt/axiomind/axm

# Restart
sudo systemctl start axm-web
```

## Production Checklist

Before deploying to production:

- [ ] Release build with optimizations enabled
- [ ] Static files properly configured and accessible
- [ ] Logging configured with appropriate level
- [ ] File permissions restricted appropriately
- [ ] Service auto-restart configured (systemd/docker)
- [ ] Backup strategy implemented
- [ ] Monitoring/health checks in place
- [ ] Security hardening applied (bind address, firewall)
- [ ] Documentation accessible to operators
- [ ] Rollback procedure tested

## Additional Resources

- [README.md](README.md) - Setup and usage
- [API.md](API.md) - API reference
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues
- [../../docs/RUNBOOK.md](../../docs/RUNBOOK.md) - Operations runbook
