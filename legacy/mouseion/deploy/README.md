# Mouseion deployment

## Quick start

```bash
# Pull and run
docker compose up -d

# First run: create admin user
curl http://localhost:8787/setup
```

## Configuration

### Volumes
| Container Path | Purpose | Persists? |
|----------------|---------|-----------|
| `/config` | Database, logs, JWT secret, cover art cache | ✅ Yes |
| `/media/*` | Media libraries from NAS (read-only) | N/A |

### Environment variables
| Variable | Default | Description |
|----------|---------|-------------|
| `TZ` | `America/Chicago` | Timezone |
| `MOUSEION_CONFIG` | `~/docker_configs/mouseion/config` | Host config path |
| `ASPNETCORE_URLS` | `http://+:8787` | Listen address |

### Resource limits (default in compose)
| Resource | Limit | Reservation |
|----------|-------|-------------|
| Memory | 2 GB | 256 MB |
| CPU | 2 cores | 0.25 cores |

## Reverse proxy (Caddy)

For HTTPS with automatic certificates:

```bash
# Edit deploy/caddy/Caddyfile — replace mouseion.example.com with your domain
docker compose -f docker-compose.yml -f deploy/caddy/docker-compose.caddy.yml up -d
```

For local-only (no TLS):
```bash
cp deploy/caddy/Caddyfile.local deploy/caddy/Caddyfile
docker compose -f docker-compose.yml -f deploy/caddy/docker-compose.caddy.yml up -d
```

## Docker secrets

For production deployments where env vars / files aren't sufficient:

```bash
# Create secret file
echo "your-jwt-secret-here" > secrets/jwt-secret.txt

# Deploy with secrets overlay
docker compose -f docker-compose.yml -f deploy/docker-compose.secrets.yml up -d
```

The application checks for secrets at `/run/secrets/mouseion_jwt_secret` before falling back to `/config/.jwt-secret`.

## Backups

### Manual
```bash
./deploy/backup.sh
```

### Automated (cron)
```bash
# Daily at 3am
echo "0 3 * * * $(pwd)/deploy/backup.sh >> /var/log/mouseion-backup.log 2>&1" | crontab -
```

**What's backed up:** mouseion.db (via SQLite online backup), logs.db, .jwt-secret, .webhook-secret, appsettings.json

**Retention:** 14 days (configurable via `RETENTION_DAYS` in script)

**Safety:** Uses `sqlite3 .backup` for consistent snapshots even while Mouseion is running.

## Auto-update

### Watchtower
The compose file includes `com.centurylinklabs.watchtower.enable=true`. If Watchtower is running, it will auto-pull new `:latest` images.

### Portainer
In Portainer, enable "Auto-update" on the stack and point to the compose file in the repository.

## Migration from bare-metal

```bash
# 1. Stop the running process
kill $(pgrep -f Mouseion.Host)

# 2. Copy existing data
mkdir -p ~/docker_configs/mouseion/config
cp ~/mouseion-data/* ~/docker_configs/mouseion/config/

# 3. Start container
docker compose up -d

# 4. Verify
curl http://localhost:8787/ping
# Should return: {"status":"ok","version":"1.0.0.0","database":"connected"}

# 5. Clean up old installation
rm -rf ~/mouseion ~/mouseion-data
```

## Health check

```bash
# Docker health status
docker inspect mouseion --format='{{.State.Health.Status}}'

# Direct check
curl http://localhost:8787/ping
```

## Troubleshooting

**Container starts but media scan finds nothing:**
Check NAS mounts are available: `ls /mnt/nas/Media/`

**Permission denied on /config:**
```bash
chown 1028:100 ~/docker_configs/mouseion/config
```

**SQLite locked:**
Only one instance can access the database at a time. Stop any bare-metal process before starting the container.
