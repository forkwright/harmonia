# External Integrations

**Analysis Date:** 2026-03-01

## APIs & External Services

**Metadata Providers:**
- Last.fm - Music metadata and user scrobbling
  - SDK/Client: Built-in HTTP calls (Retrofit on Android, fetch on web)
  - Auth: API key + secret (BuildConfig.LASTFM_API_KEY, BuildConfig.LASTFM_API_SECRET)
  - Resilience: Polly retry policies on backend

**Media Services:**
- MusicBrainz (implied via metadata pipeline)
  - Client: HTTP via backend
  - Auth: None (public API)

**Streaming/Feeds:**
- RSS/Atom feed readers
  - Client: System.ServiceModel.Syndication (.NET)
  - Auth: None for public feeds, optional HTTP basic auth for private

## Data Storage

**Databases:**
- PostgreSQL 12+ (primary)
  - Connection: Npgsql (backend)
  - ORM: Dapper (query-first, type-safe)
  - Migrations: FluentMigrator 7.2

- SQLite (fallback/local)
  - Connection: Microsoft.Data.Sqlite (backend), Room (Android)
  - Desktop/web: Vite dev/test uses in-memory SQLite via MSW mocks

**File Storage:**
- Local filesystem only
  - Media files: Served by backend via HTTP range requests
  - Images: Cached/processed via SixLabors.ImageSharp
  - Metadata: Stored in database (PostgreSQL/SQLite)

**Caching:**
- In-Memory: IMemoryCache (.NET) with 15-minute TTL for metadata responses
- Browser: Service Worker via vite-plugin-pwa (web PWA cache)
- Android: Room database + SharedPreferences (encrypted via androidx.security.crypto)

## Authentication & Identity

**Auth Provider:**
- Custom JWT implementation (self-hosted)
  - Libraries: System.IdentityModel.Tokens.Jwt, Microsoft.IdentityModel.Tokens
  - Token validation: JWT Bearer authentication on protected endpoints
  - Token issuer: Backend (Mouseion)
  - Client storage: httpOnly cookie (web) or encrypted SharedPreferences (Android)

**SignalR:**
- Real-time messaging for playback state, metadata updates
  - Server: Mouseion.SignalR (.NET)
  - Clients: Web (implicit via React), Android (via Retrofit + WebSocket)

## Monitoring & Observability

**Error Tracking:**
- Not detected in current stack
- Console logging available via OpenTelemetry console exporter

**Telemetry/Tracing:**
- OpenTelemetry 1.15 (backend)
  - Exporters: Console, OTLP (OpenTelemetry Protocol), Prometheus
  - Instrumentation: AspNetCore, HTTP client
  - Metrics: Prometheus-compatible endpoint (`/metrics`)
  - Use: `MouseionMetrics` class tags database system (e.g., "db.system" = "sqlite")

**Logs:**
- Backend: ASP.NET Core logging (ILogger) → OpenTelemetry exporter
- Android: Timber logging framework
- Web: Console.log + optional error boundary logging via `errorLogger.ts`
- Structured logging: Tracing crate (Rust)

## CI/CD & Deployment

**Hosting:**
- Self-hosted (no cloud provider detected)
- Backend: Containerized (.NET 10 runtime) or standalone executable
- Web: Static files served via HTTP (nginx/caddy likely)
- Android: Google Play Store or F-Droid
- Desktop: Tauri executable (Windows, macOS, Linux)

**CI Pipeline:**
- Not detected in repository (likely GitHub Actions or similar)
- Build triggers: Per-component (mouseion/, akroasis/)
- Artifact storage: Not applicable yet (self-hosted planned)

## Environment Configuration

**Required env vars:**
- `LASTFM_API_KEY` - Last.fm API key (Android/web)
- `LASTFM_API_SECRET` - Last.fm API secret (Android/web)
- `NVD_API_KEY` - OWASP NVD API key (Android security scanning, optional)
- Backend database connection strings (inferred):
  - `POSTGRES_CONNECTION` or similar for PostgreSQL
  - `SQLITE_DB_PATH` for SQLite fallback
- JWT signing key (inferred, not in repo)
- ASPNETCORE_ENVIRONMENT - .NET environment selector (Development/Production)

**Secrets location:**
- Android: `local.properties` (local dev only, not committed)
- Backend: `appsettings.{Environment}.json` (not in repo) or environment variables
- Web: Environment variables via `.env.local` (Vite) or `.env` (not committed)

## Webhooks & Callbacks

**Incoming:**
- None detected

**Outgoing:**
- Last.fm scrobbling (implicit, via Last.fm API calls)
- No webhook callbacks observed

## Email

**Provider:**
- MailKit 4.15 (backend)
  - SMTP: Configurable via appsettings (host, port, credentials)
  - Mocking: MSW handlers in web tests
  - Use case: Notifications, password reset (inferred)

## Media Processing

**Audio Decoding:**
- Rust audio core (akroasis-core)
  - FLAC: Claxon pure Rust decoder
  - Sample rate conversion: Rubato
  - DSP: DASP library

**Image Processing:**
- SixLabors.ImageSharp (backend)
  - Formats: JPEG, PNG
  - Use: Album artwork thumbnail generation, caching

**EPUB Reader:**
- Readium (Kotlin Toolkit) on Android only
  - No web EPUB support yet (can read in web via browser, not native reader)

## Rate Limiting

**Provider:**
- AspNetCoreRateLimit (backend)
  - Configuration: Per-endpoint or global limits
  - Storage: In-memory (local) or distributed cache (not configured yet)

## Security & Dependency Scanning

**Dependency Check:**
- OWASP Dependency Check (Android)
  - CVE scanning: Fails build at CVSS 7.0 or higher
  - Suppressions: `dependency-check-suppressions.xml`
  - API key: `NVD_API_KEY` (environment variable)

---

*Integration audit: 2026-03-01*
