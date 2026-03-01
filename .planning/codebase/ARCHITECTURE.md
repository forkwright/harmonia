# Architecture

**Analysis Date:** 2026-03-01

## Pattern Overview

**Overall:** Layered monorepo with backend (Mouseion) and multi-platform client (Akroasis) sharing API contracts. Backend uses vertical slicing by media type (Books, Music, TV, etc.).

**Key Characteristics:**
- Monorepo structure: two independently-deployable components
- Backend: layered .NET 10/C# with Dapper ORM and DryIoc DI
- Frontend: React 19/TypeScript (web), Kotlin/Jetpack Compose (Android)
- Feature-driven organization (media types as top-level concerns)
- Real-time messaging via SignalR
- SQLite default with PostgreSQL option

## Layers

**Mouseion.Host:**
- Purpose: Application bootstrap, configuration, entry point
- Location: `mouseion/src/Mouseion.Host/`
- Contains: `Program.cs`, environment detection, DI wiring
- Depends on: All Mouseion.* libraries
- Used by: Hosting runtime (.NET runtime, Docker)

**Mouseion.Api:**
- Purpose: HTTP API surface, controllers, request/response handling, middleware
- Location: `mouseion/src/Mouseion.Api/`
- Contains: Feature controllers (Audiobooks, Books, Artists, Albums, etc.), middleware, validation
- Depends on: Mouseion.Core, Mouseion.Common, Mouseion.SignalR
- Used by: Clients (Akroasis web/Android)

**Mouseion.Core:**
- Purpose: Business logic, entities, repositories, domain services
- Location: `mouseion/src/Mouseion.Core/`
- Contains: Domain entities per media type, repositories, service interfaces, business workflows
- Depends on: Mouseion.Common
- Used by: Mouseion.Api, Mouseion.Host

**Mouseion.Common:**
- Purpose: Shared utilities, DI helpers, HTTP client, environment configuration
- Location: `mouseion/src/Mouseion.Common/`
- Contains: Cache manager, disk provider, environment info, logger setup, HTTP utilities
- Depends on: Nothing (foundation)
- Used by: All Mouseion.* projects

**Mouseion.SignalR:**
- Purpose: Real-time messaging, broadcast notifications
- Location: `mouseion/src/Mouseion.SignalR/`
- Contains: SignalR hub implementations, message broadcasting
- Depends on: Mouseion.Core, Mouseion.Common
- Used by: Mouseion.Api, Mouseion.Host

**Akroasis (Web):**
- Purpose: Browser-based and desktop player (via Tauri 2)
- Location: `akroasis/web/`
- Contains: React components, state management (Zustand), API client, player logic
- Depends on: Mouseion API (backend)
- Used by: Users

**Akroasis (Android):**
- Purpose: Native Android player
- Location: `akroasis/android/`
- Contains: Kotlin/Compose UI, local persistence, scrobbling, playback control
- Depends on: Mouseion API (backend), Rust audio core (via JNI)
- Used by: Users

## Data Flow

**Media Acquisition Flow:**

1. Client (web/Android) requests library or searches metadata
2. Request hits Mouseion.Api controller (e.g., `AudiobooksController`)
3. Controller validates input (FluentValidation), calls Core service
4. Service queries repository via Dapper
5. Repository executes SQL against SQLite/PostgreSQL
6. Result mapped to DTO, returned to controller
7. Controller responds with JSON
8. If real-time update needed, SignalR broadcasts to connected clients

**Playback State Flow:**

1. Client sends play/pause command to backend
2. Mouseion.Api records playback position in History table
3. Backend broadcasts update via SignalR to other connected clients
4. Web/Android UI updates local state via Zustand/LiveData
5. Next sync: playback position pulled on app startup

**State Management:**

- **Backend:** Serilog + OpenTelemetry for instrumentation. Database as source of truth. In-memory cache (IMemoryCache) with 15-min TTL for metadata responses.
- **Frontend (Web):** Zustand stores for auth, player state, library cache
- **Frontend (Android):** Room database for local persistence, LiveData/Flow for reactivity

## Key Abstractions

**MediaItem (Polymorphic Base):**
- Purpose: Represents any playable/watchable media (Book, Audiobook, Track, Episode, Movie, etc.)
- Examples: `mouseion/src/Mouseion.Core/MediaItems/MediaItem.cs`, `mouseion/src/Mouseion.Core/Audiobooks/Audiobook.cs`, `mouseion/src/Mouseion.Core/Music/Track.cs`
- Pattern: Discriminated union via `MediaItemType` enum; entity-per-type inheritance in Core

**Repository Pattern (Generic Base):**
- Purpose: Type-safe data access with Dapper
- Examples: `mouseion/src/Mouseion.Core/Datastore/BasicRepository.cs`, `mouseion/src/Mouseion.Core/Books/BookRepository.cs`
- Pattern: `IBasicRepository<T>` generic interface; concrete repositories override for domain logic

**Service Layer:**
- Purpose: Orchestrate business logic across repositories
- Examples: `mouseion/src/Mouseion.Core/Audiobooks/AddAudiobookService.cs`, `mouseion/src/Mouseion.Core/Music/ArtistStatisticsService.cs`
- Pattern: Single responsibility per service; async/await throughout

**Controller Per Feature:**
- Purpose: REST endpoint group for a media type or domain
- Examples: `mouseion/src/Mouseion.Api/Audiobooks/AudiobooksController.cs`, `mouseion/src/Mouseion.Api/Music/ArtistsController.cs`
- Pattern: `[ApiController]` with `[Route("api/v3/[controller]")]`; validation via `FluentValidator`

## Entry Points

**Backend:**
- Location: `mouseion/src/Mouseion.Host/Program.cs`
- Triggers: Application startup (dotnet run or container entry)
- Responsibilities: Parse environment, register DI, configure Serilog, run migrations, start ASP.NET Core server on port 7878

**Frontend (Web):**
- Location: `akroasis/web/src/main.tsx`
- Triggers: Browser load
- Responsibilities: Hydrate React app, load auth state, establish API client connection

**Frontend (Android):**
- Location: `akroasis/android/app/src/main/java/app/akroasis/MainActivity.kt` (approx)
- Triggers: App launch
- Responsibilities: Initialize Compose UI, restore playback state, connect to backend

## Error Handling

**Strategy:** Exceptions bubble up to middleware (backend) or error boundary (frontend). Backend logs via Serilog. Clients show user-friendly toasts.

**Patterns:**
- Backend: Try-catch in services, log + rethrow or return error DTO
- Frontend: try-catch in async thunks (Zustand actions), set error state
- HTTP errors: 4xx for validation/auth, 5xx for server errors; clients parse and display

## Cross-Cutting Concerns

**Logging:** Serilog configured in `Mouseion.Common.Instrumentation.SerilogConfiguration`. Structured logging with context (e.g., `Log.ForContext<T>()`, `_logger.Information("Event {EventId}", id)`).

**Validation:** FluentValidation on all API requests. Fluent rules defined per DTO (e.g., `AudiobookUpdateValidator.cs`). Auto-wired via ASP.NET Core pipeline middleware.

**Authentication:** JWT bearer token validation in `Mouseion.Api.Security.AuthenticationMiddleware`. API key via `X-Api-Key` header as fallback. Claims extracted and available in controllers.

**Caching:** IMemoryCache on metadata responses (15-min sliding expiration). Repository methods check cache before DB query. Cache eviction on entity mutations.

---

*Architecture analysis: 2026-03-01*
