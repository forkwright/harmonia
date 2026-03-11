# Authentication Architecture

> How Harmonia identifies users, issues and validates credentials, and authorizes access.
> Exousia owns all identity and auth. See [subsystems.md](subsystems.md) for Exousia's domain boundaries.
> JWT secret distribution is governed by [configuration.md](configuration.md).

## Purpose

Exousia manages all identity, authentication, and authorization in Harmonia. This document specifies the JWT structure, API key format, session lifecycle, and permission model for household users. The design is for a private self-hosted system — the threat model is credential theft and unauthorized external access, not multi-tenant isolation or fine-grained permission matrices.

---

## User Model

Two user tiers, nothing else.

**Admin:** Full access. Can manage users, manage configuration, approve requests, and perform all actions available to Members.

**Member:** Can browse the library, stream media, submit requests, and manage their own playback state.

No child or guest tiers — content gating is unnecessary because video is served via Plex, which has its own user management. Audio and books are household-use only. All-or-nothing library access — no per-library, per-media-type, or per-collection permissions.

**User struct:**

```rust
pub struct User {
    pub id: UserId,                            // newtype over UUID
    pub username: String,
    pub display_name: String,
    pub password_hash: String,                 // Argon2id, never stored plaintext
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserRole {
    Admin,
    Member,
}
```

`UserId` is a newtype wrapper over `uuid::Uuid` — zero-cost, prevents accidental parameter swaps with other ID types.

---

## JWT Structure

Access tokens are JSON Web Tokens signed with HS256 (HMAC-SHA256). HS256 is symmetric and adequate for single-server deployment — the same secret signs and verifies, and that secret never leaves the server.

**Claims struct:**

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,           // UserId as UUID string — standard "subject" claim
    pub iss: String,           // "harmonia" — issuer
    pub aud: String,           // "harmonia-clients" — audience
    pub exp: u64,              // expiry as unix timestamp
    pub iat: u64,              // issued-at as unix timestamp
    pub jti: String,           // UUIDv4 — unique token identity, enables targeted revocation
    pub role: String,          // "admin" | "member" — not the enum, serialized as string
    pub display_name: String,  // for UI display without a separate profile fetch
}
```

**Token lifetimes:**
- Access token TTL: **15 minutes** — short enough that a stolen token expires quickly
- Refresh token TTL: **30 days**, rotated on every use

**Signing:**
- Algorithm: HS256
- Signing secret: from `secrets.toml` or `HARMONIA__EXOUSIA__JWT_SECRET` environment variable
- Never stored in `harmonia.toml` (the committed config file)
- Horismos validates at startup that the JWT secret is not the compiled-in default placeholder value

**Refresh token storage:** Stored as SHA-256 hash server-side. Refresh tokens are 64 cryptographically random bytes — a high-entropy source. SHA-256 is sufficient for this; Argon2id is reserved for user passwords where the input is user-chosen and potentially low-entropy.

---

## Token Lifecycle

**Login** — `POST /api/auth/login`

1. Receive `{ username, password }` in request body
2. Look up user by username — return 401 if not found or not active
3. Verify password against `password_hash` using Argon2id — return 401 on mismatch
4. Issue access token (JWT, 15 min TTL) and refresh token (random bytes, 30 day TTL)
5. Store SHA-256 hash of refresh token in the `refresh_tokens` table
6. Return access token in response body; refresh token in response body or `httpOnly` cookie

**Refresh** — `POST /api/auth/refresh`

1. Receive refresh token
2. Compute SHA-256 hash, look up in `refresh_tokens` — return 401 if not found or expired
3. Verify the token has not been revoked — return 401 if revoked
4. Revoke the old refresh token (mark as revoked in DB)
5. Issue new access token and new refresh token (rotation — old token is single-use)
6. Return new token pair

**Logout** — `POST /api/auth/logout`

1. Receive refresh token
2. Mark as revoked in `refresh_tokens` table
3. Client discards the access token locally — access tokens are not server-revocable (short TTL handles this)

**Concurrent sessions:** Unlimited. No device caps, no session count limits. Each login issues an independent refresh token. Users can be logged in on multiple devices simultaneously.

---

## API Key Structure

API keys follow the `prefixed-api-key` format: `hmn_{short_token}_{long_token}`

- `hmn_` prefix: identifies Harmonia keys in secret scanning tools and git pre-commit hooks. Any key matching `hmn_[a-z0-9]+_[a-z0-9]+` is a Harmonia credential.
- `short_token` (8 chars): stored **plaintext** server-side — used for display in the UI and for log-safe identification. Does not authorize anything on its own.
- `long_token` (24 chars): stored as **SHA-256 hash** server-side. Never stored plaintext. The full key `hmn_{short}_{long}` is shown once at creation time — it cannot be recovered afterward.

**ApiKey struct:**

```rust
pub struct ApiKey {
    pub id: ApiKeyId,
    pub user_id: UserId,
    pub short_token: String,              // stored plaintext — for display
    pub long_token_hash: String,          // SHA-256 of long_token — for validation
    pub label: String,                    // human description ("Home NAS", "OPDS reader")
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked: bool,
}
```

API keys are long-lived and manually revocable — there is no TTL. Users revoke them through the Harmonia admin UI when a device is decommissioned or a key is compromised.

---

## Request Authentication

Three credential paths, processed in priority order:

| Priority | Method | Header / Parameter | Use case |
|----------|--------|--------------------|----------|
| 1 | Bearer token | `Authorization: Bearer <jwt>` | Web UI, Android app |
| 2 | API key | `X-Api-Key: hmn_{short}_{long}` | External tools, OPDS readers, automation |
| 3 | Query parameter token | `?token=<jwt>` | Streaming/media routes only |

**Why the query parameter path exists:** Browser elements (`<audio>`, `<img>`, `<video>`) cannot set custom headers. Media routes that serve binary content directly (audio streams, cover art, book files) must accept a JWT via query parameter. This is the same pattern used by the existing C# backend. Query parameter tokens carry the same JWT payload and are subject to the same expiry — they are not weaker credentials, just differently delivered.

**All three paths produce the same result:** an `AuthenticatedUser` struct passed to downstream handlers. The auth method is recorded but does not affect what the user can do.

---

## Renderer Authentication

Renderers (`harmonia render`) authenticate with a serve instance using a pairing flow.
Unlike user sessions, renderers are headless devices that need persistent, unattended auth.
The transport is [Syndesis](../serving/quic-streaming.md) (QUIC).

### Pairing Flow

1. User initiates pairing on the server (CLI or UI):
   `harmonia pair --name "Living Room Pi"`
2. Server generates a one-time pairing code (6-digit, 5-minute TTL)
3. Renderer is started with the code:
   `harmonia render --server harmonia.local --pair-code 847291`
4. Server validates the code and issues:
   - A permanent API key (stored in renderer's config)
   - The server's TLS certificate fingerprint (SHA-256)
5. Renderer stores both and uses them for all future connections

### Ongoing Auth

- Renderer presents API key in QUIC connection handshake
- Server validates key and checks renderer is not revoked
- TLS certificate is validated by pinned fingerprint (not CA chain)
- Self-signed certificates are expected and correct — no external CA needed

### Key Management

- Renderer API keys use the prefix `hmn_rnd_` to distinguish them from user API keys
- Server can revoke a renderer's API key at any time
- Revoked renderers are immediately disconnected
- Re-pairing generates a new key (old key is invalidated)
- API keys are stored as argon2id hashes on the server, plaintext on the renderer

### Security Model

- Renderers are trusted devices on the local network
- No user credentials on the renderer — it has a device key, not a user session
- The API key grants audio streaming and control only — no library management, no acquisition, no admin access

---

## axum Extractor Design

Two extractors in `harmonia-host` (or a dedicated auth middleware crate):

**`AuthenticatedUser`** — tries all three credential paths in priority order:

```rust
pub struct AuthenticatedUser {
    pub user_id: UserId,
    pub role: UserRole,
    pub auth_method: AuthMethod,  // Bearer | ApiKey | QueryParam
}

pub enum AuthMethod {
    Bearer,
    ApiKey,
    QueryParam,
}
```

On success: returns `AuthenticatedUser` with extracted identity.
On failure: returns `401 Unauthorized` with structured error body `{ "error": "...", "code": "UNAUTHORIZED", "correlation_id": "..." }`.

**`RequireAdmin`** — wraps `AuthenticatedUser` and checks `role == UserRole::Admin`:

```rust
pub struct RequireAdmin(pub AuthenticatedUser);
```

On success: passes through as `RequireAdmin(user)`.
On failure (role is Member): returns `403 Forbidden`.

Handler signatures use these extractors directly:

```rust
// Public route — any authenticated user
async fn get_library(user: AuthenticatedUser) -> impl IntoResponse { ... }

// Admin-only route
async fn create_user(admin: RequireAdmin) -> impl IntoResponse { ... }
```

---

## Anti-Patterns

**Do not implement per-library permissions.** Decided against. All-or-nothing access is the model. A member can see and request anything in the library.

**Do not store the JWT secret in `harmonia.toml`.** That file is committed to version control. JWT secrets belong in `secrets.toml` (gitignored) or in the `HARMONIA__EXOUSIA__JWT_SECRET` environment variable. Horismos must reject startup if the secret is the compiled-in default.

**Do not store refresh tokens or API key long tokens as plaintext.** Both are stored as SHA-256 hashes server-side. The plaintext is given to the client once and never stored.

**Do not use bcrypt for password hashing.** Use Argon2id. Bcrypt has known weaknesses against GPU-accelerated cracking. Argon2id is the OWASP recommendation.

**Do not implement device tracking or session limits.** No maximum concurrent sessions, no per-device token binding. Sessions are independent — any valid refresh token can refresh regardless of what device issued it.

**Do not use `role` in `Claims` for fine-grained decisions.** The role claim in the JWT is for UI display and coarse access control (admin vs member). Per-resource access is determined by the resource ownership stored server-side, not by JWT claims alone.
