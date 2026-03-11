# Spec 09: Containerization & Deployment

**Status:** Draft
**Priority:** High — blocks production deployment
**Issues:** —

## Goal

Package Mouseion as a Docker/Podman container following the same patterns as the existing *arr media stack. Managed via Portainer on the NAS, running on worker-node, with persistent config and NAS media access. The container should be as simple to deploy as any LinuxServer.io image: pull, set volumes, set port, start.

## Current State

### What Exists
- Self-contained linux-x64 binary (~222MB) built via `dotnet publish --self-contained`
- SQLite database (mouseion.db, logs.db) + secrets (.jwt-secret, .webhook-secret) in data directory
- Running as nohup process on worker-node:8787 with `--data=/home/syn/mouseion-data`
- No health check endpoint beyond `/api/v3/system/status` (requires auth)
- No graceful shutdown handling
- Build requires .NET 10 SDK (only on Metis, not worker-node)

### Existing Infrastructure
- **Portainer** on NAS (192.168.0.120:9000) managing worker-node via portainer-agent (:9001)
- **Docker** on worker-node with 9 running containers (Plex, Audiobookshelf, Tautulli, etc.)
- **Config pattern**: `~/docker_configs/<app>/config` → `/config` inside container
- **Media pattern**: `/mnt/nas/Media/<type>` → `/media/<type>` inside container
- **Media ownership**: UID 1028, GID 100 (users group)
- **Network**: `media_media_network` bridge for inter-container communication
- **Storage**: NAS docker configs at `/mnt/nas/docker/`, some local at `~/docker_configs/`

### Media Paths
| Host Path | Content | Size |
|-----------|---------|------|
| `/mnt/nas/Media/movies` | Movies | ~76K entries |
| `/mnt/nas/Media/tv_shows` | TV series | ~6K entries |
| `/mnt/nas/Media/music` | Music library | ~13K entries |
| `/mnt/nas/Media/books` | Books + audiobooks | Small |
| `/mnt/nas/Media/podcasts` | Podcasts | Empty |

## Phases

### Phase 1: Dockerfile + Build Pipeline
Create a multi-stage Dockerfile that produces a minimal runtime image.

**Dockerfile strategy:**
```
Stage 1: Build (mcr.microsoft.com/dotnet/sdk:10.0)
  - COPY solution + project files → dotnet restore
  - COPY source → dotnet publish --self-contained -r linux-x64

Stage 2: Runtime (mcr.microsoft.com/dotnet/runtime-deps:10.0-noble)
  - runtime-deps only (self-contained binary includes its own runtime)
  - Create /config volume mount point
  - Create /media volume mount point
  - EXPOSE 8787
  - ENTRYPOINT ["./Mouseion.Host", "--data=/config"]
```

**Why `runtime-deps` not `aspnet`:** Self-contained publish bundles the entire runtime. The `runtime-deps` image only provides native dependencies (libssl, libicu, etc.) — smallest possible image.

**Build considerations:**
- .dockerignore: exclude bin/, obj/, .git/, tests, specs, docs
- Layer caching: restore before source copy (cache NuGet packages)
- Final image size target: <300MB (222MB binary + ~50MB base image)
- ARM64 support: add `--platform linux/amd64,linux/arm64` for future NAS-native containers
- Tag convention: `mouseion:latest`, `mouseion:<git-sha-short>`, `mouseion:<date>`

**Deliverables:**
- [ ] Dockerfile (multi-stage, optimized layers)
- [ ] .dockerignore
- [ ] `docker build` works locally on Metis

### Phase 2: Compose + Volume Architecture
Design the volume mounts and compose file for Portainer deployment.

**Volume design:**
```yaml
volumes:
  # Persistent config — survives container recreation
  - /home/cody/docker_configs/mouseion/config:/config
  # Media library — read-only access to NAS shares
  - /mnt/nas/Media/movies:/media/movies:ro
  - /mnt/nas/Media/tv_shows:/media/tv:ro
  - /mnt/nas/Media/music:/media/music:ro
  - /mnt/nas/Media/books:/media/books:ro
  - /mnt/nas/Media/books/audiobooks:/media/audiobooks:ro
  - /mnt/nas/Media/podcasts:/media/podcasts:ro
```

**/config contains:** mouseion.db, logs.db, .jwt-secret, .webhook-secret, appsettings.json (if customized). Survives upgrades.

**/media is read-only** for scanning/streaming. Mouseion doesn't write to media directories (downloads go through download clients, not Mouseion directly).

**Environment variables:**
```yaml
environment:
  - PUID=1028          # Match NAS media ownership
  - PGID=100           # users group
  - TZ=America/Chicago
  - ASPNETCORE_URLS=http://0.0.0.0:8787
```

**Network:**
```yaml
networks:
  - media_media_network  # Same network as Plex, *arr suite
```

This enables direct container-to-container communication for:
- Webhook delivery from Jellyfin/Plex/Emby
- Download client API calls (qBittorrent on same network)
- Future: indexer proxying through Prowlarr

**Port mapping:**
```yaml
ports:
  - "8787:8787"
```

**Restart policy:**
```yaml
restart: unless-stopped
```

**Deliverables:**
- [ ] docker-compose.yml (production stack for Portainer)
- [ ] docker-compose.dev.yml (local development with hot reload)
- [ ] Document config migration from `~/mouseion-data/` to `/home/cody/docker_configs/mouseion/config/`

### Phase 3: Health, Signals, and Lifecycle
Make the container a well-behaved citizen in Docker's process management.

**Health check:**
```dockerfile
HEALTHCHECK --interval=30s --timeout=5s --start-period=60s --retries=3 \
  CMD curl -f http://localhost:8787/ping || exit 1
```

Requires adding a `/ping` endpoint that:
- Returns 200 with no auth required
- Checks database connectivity (can open SQLite)
- Returns 503 if migrations haven't completed yet

**Graceful shutdown:**
- Handle SIGTERM from Docker (15s default before SIGKILL)
- Flush pending download client operations
- Close database connections cleanly
- ASP.NET's `IHostApplicationLifetime` already handles this — verify it propagates through our housekeeping tasks

**Startup readiness:**
- Migrations run on first start — can take several seconds
- Health check `start-period=60s` gives time for migration + initial scan
- `/ping` returns 503 until migrations complete, then 200

**Logging:**
- Docker expects stdout/stderr (no file logging needed)
- Current Serilog config writes to files — add console sink for container mode
- Detect container environment: check `DOTNET_RUNNING_IN_CONTAINER=true` (set by Microsoft base images)
- In container mode: console JSON logging only (structured, parseable by Portainer/Loki)
- Outside container: keep existing file-based logging

**Deliverables:**
- [ ] `/ping` health endpoint (no auth, checks DB)
- [ ] HEALTHCHECK in Dockerfile
- [ ] Console logging sink for container mode
- [ ] Verify SIGTERM handling through housekeeping scheduler

### Phase 4: PUID/PGID and Permissions
Match the LinuxServer.io pattern for user/group mapping.

**Problem:** Container runs as root by default. Files created in /config will be owned by root. Media files on NAS are owned by 1028:100. If Mouseion ever needs to write to media paths (cover art cache, .strm files), permissions will fail.

**LinuxServer.io pattern:**
```bash
# Entrypoint script creates user with matching UID/GID
groupadd -g $PGID mouseion
useradd -u $PUID -g $PGID mouseion
chown -R mouseion:mouseion /config
exec gosu mouseion ./Mouseion.Host --data=/config
```

**Simpler alternative (recommended for now):**
Run as non-root user built into the image. Set container user to match PUID/PGID at deploy time:
```yaml
user: "1028:100"
```
This is simpler and avoids the entrypoint script complexity. Config directory permissions need to be set once on the host:
```bash
mkdir -p ~/docker_configs/mouseion/config
chown 1028:100 ~/docker_configs/mouseion/config
```

**When we'd need the full entrypoint script:**
- If different deployments need different UIDs
- If we add a web UI that needs to download cover art to media dirs
- If .strm file generation writes to the media directory tree

For now: `user: "1028:100"` in compose file. Upgrade to entrypoint script in Phase 6 if needed.

**Deliverables:**
- [ ] Non-root user in Dockerfile
- [ ] `user:` directive in docker-compose.yml
- [ ] Document host permission setup

### Phase 5: CI/CD — Automated Image Builds
Build and push images automatically on merge to main.

**GitHub Actions workflow:**
```yaml
on:
  push:
    branches: [main]
    paths-ignore: ['docs/**', 'specs/**', '*.md']

jobs:
  build:
    - Checkout
    - Set up Docker Buildx
    - Login to GHCR (GitHub Container Registry)
    - Build + push: ghcr.io/forkwright/mouseion:latest, :sha-xxx, :YYYY-MM-DD
    - (Optional) Build ARM64 variant
```

**Registry:** GitHub Container Registry (ghcr.io/forkwright/mouseion) — free for public repos, integrated with GitHub, Portainer can pull directly.

**Tag strategy:**
- `latest` — always the most recent main build
- `sha-<7char>` — immutable, for rollback
- `YYYY-MM-DD` — human-readable date tags
- No semver yet — premature for a project this young

**Portainer auto-update (optional):**
Portainer can poll GHCR for new `:latest` tags and auto-recreate the container. Or use Watchtower (already running on the NAS docker stack).

**Deliverables:**
- [ ] `.github/workflows/docker-build.yml`
- [ ] GHCR repository setup (package visibility)
- [ ] Verify Portainer can pull from ghcr.io

### Phase 6: Production Hardening (deferred)

These matter for multi-user or public-facing deployments but are overkill for a single-household server right now:

- [ ] **Reverse proxy config** — Caddy/Traefik/nginx examples for HTTPS termination
- [ ] **Secrets management** — Docker secrets for JWT key, API keys, debrid credentials (instead of env vars / files)
- [ ] **Resource limits** — CPU/memory constraints in compose (`deploy.resources.limits`)
- [ ] **Backup strategy** — cron job to snapshot /config (SQLite + secrets)
- [ ] **Full LinuxServer.io entrypoint** — s6-overlay, PUID/PGID mapping, custom init scripts
- [ ] **Multi-arch manifest** — linux/amd64 + linux/arm64 for NAS-native deployment
- [ ] **Watchtower integration** — auto-update labels

## Dependencies

- .NET 10 SDK (build stage only — not needed at runtime)
- Docker or Podman on worker-node (already installed)
- Portainer agent on worker-node (already running)
- NAS media shares mounted (already at /mnt/nas/Media)
- GitHub Container Registry (free, already have GitHub account)

## Migration Plan

Moving from current bare-metal to container:

1. Build image on Metis (or via GitHub Actions)
2. Create config directory: `mkdir -p ~/docker_configs/mouseion/config`
3. Copy existing data: `cp ~/mouseion-data/* ~/docker_configs/mouseion/config/`
4. Create Portainer stack from docker-compose.yml
5. Verify: login still works (JWT secret preserved), library visible
6. Remove old binary: `rm -rf ~/mouseion ~/mouseion-data`

**Zero-downtime migration** is possible since Mouseion uses SQLite (file-level lock, no connection pooling to manage). Stop bare-metal → copy data → start container → verify → done.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| SQLite file locking in container | Can't run two instances simultaneously | Single-instance by design; health check prevents dual-start |
| NAS mount unavailability | Container starts but can't scan media | Health check could verify /media mount presence |
| .NET 10 preview SDK availability | Build stage may break if SDK image updates | Pin SDK version in Dockerfile |
| Image size bloat | Slow pulls, wastes NAS storage | Multi-stage build, .dockerignore, runtime-deps base |
| GitHub Actions minutes | Free tier may hit limits with frequent pushes | Only build on main pushes, not PRs |

## Notes

- Mouseion is the 10th container on worker-node. Docker storage is 3.5GB total — well within capacity.
- The existing `media_media_network` bridge means Mouseion can talk to Plex, qBittorrent, and the *arr suite without port mapping.
- Portainer's stack feature reads docker-compose.yml directly — the compose file IS the deployment manifest.
- ARM64 support is deferred because the current server is x64. Worth adding when/if containers move to NAS natively (Synology 923+ is AMD64 anyway, but future-proofing).
- SQLite is fine for single-household. If multi-user scaling ever matters, PostgreSQL is a one-line config change (Mouseion already has a PostgreSQL code path).
