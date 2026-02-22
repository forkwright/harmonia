# Spec 06: Authentication & Multi-User

**Status:** Feature-complete (all 5 phases implemented)
**Priority:** High
**Issues:** —

## Goal

Replace API-key-only authentication with full OIDC/OAuth 2.0 support and per-user state isolation. Multi-user households and shared servers are the norm for self-hosted media — Mouseion currently has no concept of user identity beyond a hardcoded "default" string in PlaybackSession and MediaProgress. This spec makes Mouseion multi-tenant: each user has their own progress, watchlists, quality preferences, and library views while sharing the underlying media collection.

## Phases

### Phase 1: User model and identity ✅
- [x] User entity — ID, username, display name, email, role, authentication method, created/updated timestamps
- [x] UserRole enum — Admin, User, ReadOnly (Admin manages server + all libraries, User manages own state, ReadOnly browses only)
- [x] Database migration: users table, seed default admin user from existing `AuthOptions.ApiKey`
- [x] Replace hardcoded `UserId = "default"` in MediaProgress and PlaybackSession with actual user FK
- [x] User CRUD API: GET/POST/PUT/DELETE /api/v3/users (admin only)
- [x] Current user endpoint: GET /api/v3/users/me

### Phase 2: Local authentication ✅
- [x] Username/password authentication with bcrypt/argon2 hashing
- [x] POST /api/v3/auth/login — returns JWT access token + refresh token
- [x] POST /api/v3/auth/refresh — rotate refresh token
- [x] POST /api/v3/auth/logout — revoke refresh token
- [x] JWT middleware: validate token on every API request, extract user identity
- [x] API key auth preserved as fallback (for automation, scripts, OPDS clients)
- [x] Rate limiting on auth endpoints (5 attempts per minute per IP)

### Phase 3: OIDC/OAuth 2.0 ✅
- [x] OIDC discovery endpoint configuration (issuer URL → auto-discover .well-known/openid-configuration)
- [x] OAuth 2.0 authorization code flow with PKCE
- [x] GET /api/v3/auth/oidc/authorize — redirect to provider
- [x] GET /api/v3/auth/oidc/callback — handle provider redirect, create/link user, issue JWT
- [x] Auto-provision users on first OIDC login (configurable: auto-create or require admin approval)
- [x] Map OIDC claims to Mouseion roles (configurable claim → role mapping)
- [x] Multiple provider support (e.g., Keycloak for family, Google for friends)
- [x] OIDC provider CRUD: GET/POST/PUT/DELETE /api/v3/auth/providers (admin only)

### Phase 4: Per-user state isolation ✅
- [x] UserPreferences entity — hidden media types, quality profile overrides (per-media-type), language, theme, notifications
- [x] Per-user Smart List subscriptions — opt-in with auto-add and notification preferences
- [x] GET/PUT /api/v3/users/me/preferences — manage own preferences
- [x] GET/POST/DELETE /api/v3/users/me/smartlists/{id} — smart list subscriptions
- [x] Admin can view any user's preferences: GET /api/v3/users/{id}/preferences
- [x] Migration 033

### Phase 5: Permissions and access control ✅
- [x] Resource-level permissions: MediaTypeAccess, RootFolderAccess, DownloadPermission
- [x] Default-open model: no permissions set = full access. Permissions are restrictions.
- [x] Admin bypass: UserRole.Admin skips all permission checks
- [x] AuthorizationService — centralized permission checks, API key management, audit logging
- [x] Scoped API keys — PBKDF2-hashed, prefix-based lookup, scope + expiration support
- [x] GET/POST/DELETE /api/v3/apikeys — manage own keys (raw key shown only at creation)
- [x] Audit log — all auth events tracked (login, permission changes, API key CRUD, session revocations)
- [x] GET /api/v3/admin/audit — recent entries (admin only)
- [x] GET /api/v3/admin/sessions — active sessions across all users
- [x] DELETE /api/v3/admin/sessions/{userId} — force logout
- [x] Migration 033

## Dependencies

- Phase 1 must land before any per-user feature in other specs (progress, Smart Lists, etc.)
- Phase 3 OIDC requires redirect URI configuration — affects deployment docs and reverse proxy setup
- Spec 01 Phase 5 (OPDS) needs API key auth via URL parameter — Phase 2 here provides that
- Spec 03 Phase 2 (Smart Lists) benefits from per-user scoping but can ship without it

## Notes

- MediaManager uses `authlib` (Python) for OIDC. C# equivalent: `Microsoft.AspNetCore.Authentication.OpenIdConnect` — first-party, well-maintained, integrates with ASP.NET middleware pipeline.
- Existing `AuthOptions` has `ApiKey`, `Enabled`, `Method`, `Required` — this framework extends rather than replaces it.
- JWT over session cookies: APIs are consumed by Akroasis (mobile), OPDS clients, and scripts — JWT is more portable than cookies. Store refresh tokens server-side.
- Self-hosted SSO is increasingly common: Authentik, Authelia, Keycloak. Supporting OIDC covers all of them with one implementation.
