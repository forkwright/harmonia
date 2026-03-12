# Desktop Architecture

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| React state management | Zustand (client) + TanStack Query (server) | Zustand for UI state and auth token; TanStack Query for API caching, pagination, and background refresh |
| Tauri IPC pattern | HTTP via `api` client for server calls; Tauri `invoke` for local-only ops | Server communication stays in the HTTP client layer. IPC is reserved for config (server URL) and OS integration. |
| Desktop build matrix | Architecture-specific (x86_64, aarch64) | Universal binary deferred — separate binaries are simpler for initial release and CI |
| Signal path visualization | Canvas-based spectrum | Deferred to P3-11; Canvas avoids WebGL dependency overhead for the initial player |

## Structure

```
desktop/src/
├── api/          — HTTP client wrapping fetch + Tauri invoke for base URL
├── components/   — Shared layout components
├── features/     — Feature-based folders (library, player, …)
│   └── library/  — Album/artist/track browser (P3-10)
│       ├── store.ts           — Zustand: auth token, sort preference
│       ├── hooks.ts           — TanStack Query: infinite queries per media type
│       ├── AlbumsPage.tsx     — Virtualized album grid
│       ├── AlbumCard.tsx      — Album card (cover placeholder, title, year, type)
│       ├── TracksPage.tsx     — Virtualized track list with codec badge
│       ├── AudiobooksPage.tsx — Virtualized audiobook grid
│       └── SortFilterBar.tsx  — Sort controls
├── hooks/        — Shared hooks (useServer)
├── pages/        — Route-level components (Settings, redirect Home)
└── types/        — API response types aligned with Paroche
```

## Communication Pattern

All media API calls go through `src/api/client.ts`:
- `api.get<T>(path, token)` / `api.post<T>(path, body, token)` → base HTTP methods
- Typed wrappers (`listReleaseGroups`, `listTracks`, `listAudiobooks`) enforce response shape

Tauri `invoke` is used only for:
- `get_server_url` — read stored server URL
- `set_server_url` — write server URL
- `health_check` — TCP reachability probe

## Auth

Bearer JWT is stored in Zustand (`persist` → localStorage). Users set it once in Settings.
TanStack Query hooks are `enabled: token.length > 0` — no fetches until authenticated.

See `docs/architecture/binary-modes.md` for the `harmonia desktop` execution mode context.
