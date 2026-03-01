# Codebase Structure

**Analysis Date:** 2026-03-01

## Directory Layout

```
harmonia/
├── mouseion/                # Backend API server (.NET 10/C#)
│   ├── src/
│   │   ├── Mouseion.Api/           # Controllers, middleware, API surface
│   │   ├── Mouseion.Core/          # Business logic, entities, repositories
│   │   ├── Mouseion.Common/        # Shared utilities, DI, environment
│   │   ├── Mouseion.SignalR/       # Real-time messaging
│   │   └── Mouseion.Host/          # Application entry point, bootstrap
│   ├── tests/
│   │   ├── Mouseion.Api.Tests/
│   │   ├── Mouseion.Core.Tests/
│   │   └── Mouseion.Common.Tests/
│   ├── specs/                      # Development specifications, design docs
│   ├── deploy/                     # Deployment configs (Caddy, Docker)
│   ├── Mouseion.sln                # Solution file
│   └── README.md
│
├── akroasis/                # Multi-platform player (Kotlin, React, Rust)
│   ├── web/                        # React 19 web + Tauri desktop
│   │   ├── src/
│   │   │   ├── api/                # API client and types
│   │   │   ├── components/         # React components
│   │   │   ├── pages/              # Page-level components
│   │   │   ├── stores/             # Zustand state management
│   │   │   ├── hooks/              # Custom React hooks
│   │   │   ├── types/              # TypeScript types
│   │   │   ├── utils/              # Utilities
│   │   │   ├── services/           # API service layer
│   │   │   ├── audio/              # Audio playback logic
│   │   │   ├── mocks/              # MSW mocks for testing
│   │   │   └── main.tsx            # App entry point
│   │   ├── package.json
│   │   └── README.md
│   │
│   ├── android/                    # Kotlin/Jetpack Compose Android app
│   │   ├── app/src/
│   │   │   ├── main/java/app/akroasis/
│   │   │   │   ├── ui/             # Compose screens per feature
│   │   │   │   ├── data/           # Data layer (API, local DB, prefs)
│   │   │   │   ├── service/        # Android services
│   │   │   │   ├── di/             # Dependency injection
│   │   │   │   ├── audio/          # Audio playback (JNI interface)
│   │   │   │   └── MainActivity.kt # App entry point
│   │   └── build.gradle
│   │
│   └── README.md
│
├── docs/                   # Cross-cutting documentation
│   ├── ARCHITECTURE.md     # System design
│   └── SETUP.md            # Development setup
│
├── .github/                # GitHub Actions CI/CD
├── .planning/              # GSD planning documents
│   └── codebase/           # This directory
│
└── CLAUDE.md               # Monorepo coding guidelines
```

## Directory Purposes

**mouseion/src/Mouseion.Api:**
- Purpose: REST API controllers and HTTP handling for all media types
- Contains: Feature-organized controller classes (AudiobooksController, BooksController, ArtistsController, etc.), middleware, validation, API responses
- Key files: `Program.cs` (ASP.NET Core startup), controllers organized in `Audiobooks/`, `Books/`, `Music/` subdirectories

**mouseion/src/Mouseion.Core:**
- Purpose: Domain logic, data access, business rules
- Contains: Entity models, repository implementations, domain services, queries, validation rules
- Key files: `Datastore/BasicRepository.cs` (generic repository base), `MediaItems/MediaItem.cs` (base entity), per-feature folders (Audiobooks/, Books/, Music/), each containing entities and repositories

**mouseion/src/Mouseion.Common:**
- Purpose: Cross-cutting utilities and infrastructure
- Contains: Cache manager, disk provider, environment configuration, Serilog setup, HTTP client factories, DI helpers
- Key files: `Instrumentation/SerilogConfiguration.cs`, `Cache/CacheManager.cs`, `EnvironmentInfo/AppFolderInfo.cs`

**mouseion/src/Mouseion.SignalR:**
- Purpose: Real-time client notifications
- Contains: SignalR hub definitions, message broadcaster implementations
- Key files: `SignalRMessageBroadcaster.cs`, hub interface implementations

**mouseion/src/Mouseion.Host:**
- Purpose: Application bootstrap and configuration
- Contains: Program.cs with full DI registration, environment detection, migration triggers
- Key files: `Program.cs` (2000+ lines, registers all services)

**mouseion/tests/:**
- Purpose: Unit and integration test suites
- Contains: Test classes mirroring source structure (Mouseion.Api.Tests, Mouseion.Core.Tests, Mouseion.Common.Tests)
- Key files: `*.Tests.csproj` files, test classes using xUnit and Moq

**akroasis/web/src/:**
- Purpose: React web application and Tauri desktop wrapper
- Contains: Components (feature-organized), state management (Zustand stores), API client, player logic
- Key files: `main.tsx` (entry), `api/` (client code), `stores/` (state), `components/` (reusable UI)

**akroasis/android/app/src/main/java/app/akroasis/:**
- Purpose: Android application with Jetpack Compose UI
- Contains: Compose screens (organized in `ui/` by feature), data layer (API client, Room DB, preferences), services, DI
- Key files: `MainActivity.kt`, `data/api/` (Retrofit client), `ui/` (feature screens), `di/` (Hilt modules)

## Key File Locations

**Entry Points:**
- `mouseion/src/Mouseion.Host/Program.cs`: Backend server startup, DI wiring, migrations
- `akroasis/web/src/main.tsx`: Web app initialization, root component render
- `akroasis/android/app/src/main/java/app/akroasis/MainActivity.kt`: Android app entry

**Configuration:**
- `mouseion/src/Mouseion.Host/Program.cs`: ASP.NET Core, database, logging configuration
- `akroasis/web/vite.config.ts`: Vite bundler, Tauri, Tailwind config
- `akroasis/android/build.gradle`: Gradle build configuration

**Core Logic:**
- `mouseion/src/Mouseion.Core/`: All domain logic (repositories, services, entities)
- `mouseion/src/Mouseion.Api/[Feature]/`: Controllers exposing domain logic as REST endpoints
- `akroasis/web/src/stores/`: Zustand stores managing player/library state
- `akroasis/android/app/src/main/java/app/akroasis/data/repository/`: Repository pattern for local/remote data

**Testing:**
- `mouseion/tests/Mouseion.*.Tests/`: xUnit test projects with Moq mocks
- `akroasis/web/src/test/`: Vitest unit tests, React Testing Library components
- `akroasis/android/app/src/test/`: JUnit tests with Mockito

## Naming Conventions

**Files:**
- C# classes: PascalCase matching class name (e.g., `AudiobooksController.cs`, `AddAudiobookService.cs`)
- React components: PascalCase (e.g., `PlayerControls.tsx`, `LibraryGrid.tsx`)
- Kotlin classes: PascalCase (e.g., `AudiobookViewModel.kt`, `PlayerService.kt`)
- Test files: `[ClassName].Tests.cs` (C#), `[componentName].test.tsx` (React), `[ClassName]Test.kt` (Kotlin)

**Directories:**
- Feature folders: PascalCase plural (e.g., `Audiobooks/`, `Books/`, `Artists/`)
- Utility folders: lowercase (e.g., `utils/`, `hooks/`, `services/`)
- Data layer: Verb-noun pattern (e.g., `data/repository/`, `data/api/`)

## Where to Add New Code

**New Media Type Feature (e.g., Podcasts):**
- Core domain: `mouseion/src/Mouseion.Core/Podcasts/` (entities, repository, services)
- API: `mouseion/src/Mouseion.Api/Podcasts/` (PodcastsController.cs, DTOs)
- Tests: `mouseion/tests/Mouseion.Core.Tests/Podcasts/` (service tests), `mouseion/tests/Mouseion.Api.Tests/Podcasts/` (endpoint tests)

**New Web Component:**
- Implementation: `akroasis/web/src/components/[Feature]/ComponentName.tsx`
- State: `akroasis/web/src/stores/[feature]Store.ts` (if stateful)
- Tests: `akroasis/web/src/components/[Feature]/ComponentName.test.tsx`

**New Android Screen:**
- Implementation: `akroasis/android/app/src/main/java/app/akroasis/ui/[feature]/ScreenName.kt`
- ViewModel: `akroasis/android/app/src/main/java/app/akroasis/ui/[feature]/ScreenNameViewModel.kt`
- Tests: `akroasis/android/app/src/test/java/app/akroasis/ui/[feature]/ScreenNameTest.kt`

**Shared Utilities:**
- Backend utilities: `mouseion/src/Mouseion.Common/` (new subfolder if needed)
- Web utilities: `akroasis/web/src/utils/` (new file or subfolder)
- Android utilities: `akroasis/android/app/src/main/java/app/akroasis/` (consider `util/` subfolder)

## Special Directories

**mouseion/deploy/:**
- Purpose: Deployment artifacts (Docker, Caddy reverse proxy configs)
- Generated: No (committed)
- Committed: Yes

**mouseion/specs/:**
- Purpose: Design specifications, architecture decisions, feature specs
- Generated: No (committed)
- Committed: Yes

**akroasis/web/dist/ & akroasis/web/target/:**
- Purpose: Build outputs
- Generated: Yes (build system)
- Committed: No

**akroasis/android/app/build/:**
- Purpose: Gradle build artifacts
- Generated: Yes (Gradle)
- Committed: No

---

*Structure analysis: 2026-03-01*
