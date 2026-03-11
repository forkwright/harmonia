# Mouseion Bug Audit — 2026-02-22

## Status: All Issues Resolved ✅

Comprehensive audit performed on 2026-02-22. All 10 identified issues have been fixed.

## Fixes Applied

| # | Severity | Issue | Fix | PR |
|---|----------|-------|-----|-----|
| 1 | 🔴 Critical | 5 controllers missing `[Authorize]` | Added `[Authorize]` to Streaming, Analytics, ImportList, ImportListExclusion, ImportWizard controllers | #213 |
| 2 | 🔴 Critical | JWT secret regenerated on restart | Auto-persist to `.jwt-secret` in data directory | #213 |
| 3 | 🔴 Critical | DryIoc only registered last IDebridClient | Changed to `RegisterMany` for all 3 providers | #213 |
| 4 | 🟡 Medium | Null dereference in GetActiveSessionsAsync | Added proper null guard on GroupBy | #213 |
| 5 | 🟡 Medium | int.Parse without Enum.IsDefined in permissions | Added validation before cast | #213 |
| 6 | 🟡 Medium | OPDS endpoints had no auth | ApiKeyAuthFilter validates `?apikey=` query param | #215 |
| 7 | 🟡 Medium | Webhook endpoints had no auth | WebhookSecretFilter validates `X-Webhook-Secret` header | #215 |
| 8 | 🟢 Low | 6 housekeeping tasks referenced non-existent tables | Rewritten to use MediaItems (unified schema) | #215 |
| 9 | 🟢 Low | No admin seeding on fresh install | `POST /api/v3/setup` — only works when no users exist | #209 |
| 10 | 🟢 Low | Webhook secret not discoverable | `GET /api/v3/webhooks/secret` — admin-only endpoint | #215 |

## Infrastructure Fixes (PRs #205-212)

| PR | Fix |
|----|-----|
| #205 | SQLite migration compat (duplicate indexes, FKs, SearchHistory) |
| #206 | Housekeeping table names (Movies→MediaItems) |
| #207 | Resilient background services (catch all exceptions) |
| #208 | Duplicate SystemController route |
| #209 | Setup endpoint for initial admin creation |
| #210-211 | Missing OIDC columns in Users table |
| #212 | Computed property leakage in BasicRepository |

## Verified Clean

| Check | Status |
|-------|--------|
| All interface methods exist | ✅ |
| SQL queries parameterized | ✅ |
| No async void | ✅ |
| No missing connection disposal | ✅ |
| No HttpContext in singletons | ✅ |
| No race conditions in repos | ✅ |
| No hardcoded secrets | ✅ |
| Notification column injection defended | ✅ |
| All controllers have auth | ✅ |
| JWT secret persisted | ✅ |
| Webhook secret persisted | ✅ |
| All housekeeping tasks use correct tables | ✅ |
