# Technology Stack

**Analysis Date:** 2026-03-01

## Languages

**Primary:**
- C# 10 - Backend (Mouseion), .NET 10 runtime
- TypeScript 5.9 - Web frontend (Akroasis), strict mode enforced
- Kotlin 1.9+ - Android app (Akroasis)
- Rust 2021 edition - Audio core (Akroasis shared), FLAC decoding and DSP

**Secondary:**
- JavaScript - Node build tooling, Tauri configuration
- XML - MSBuild project files, Android manifests

## Runtime

**Environment:**
- .NET 10 (backend)
- Node.js (web dev, build tooling) - version managed via npm
- JVM 17+ (Android build)
- Rust (audio core library)

**Package Managers:**
- npm (Node.js) - web application
- dotnet (NuGet) - backend packages
- Gradle (Kotlin DSL) - Android dependencies
- Cargo - Rust audio core

## Frameworks

**Core Backend:**
- ASP.NET Core 10 (`Microsoft.AspNetCore.App` framework reference)
- Purpose: HTTP API server, SignalR real-time messaging

**Web Frontend:**
- React 19.2 - UI framework
- React Router 7.13 - client-side routing
- Vite 7.3 - build tool and dev server
- Tauri 2.10 - desktop (Electron alternative)

**Mobile:**
- Jetpack Compose - Android UI (Kotlin)
- Hilt 2.50+ - dependency injection

**Styling:**
- TailwindCSS 3.4 - web styling
- Material 3 - Android design system

**State Management:**
- Zustand 5.0 - web client state
- StateFlow - Android

**Testing:**
- Vitest 4.0 - web unit/component tests (Vite-native)
- Jest DOM matchers via `@testing-library/jest-dom`
- JUnit + Mockito - Android unit tests
- Robolectric - Android instrumented tests
- xUnit - .NET tests (via `dotnet test`)

**Build/Dev:**
- TypeScript 5.9 - type checking (web)
- Tauri CLI 2.10 - desktop packaging
- Gradle - Android build
- MSBuild - .NET build system
- dotnet format - C# code formatting

## Key Dependencies

**Backend (Mouseion.Core):**
- Dapper 2.1 - SQL ORM (type-safe, query-first)
- FluentMigrator 7.2 - database schema migrations
- Npgsql 10.0 - PostgreSQL client
- Microsoft.Data.Sqlite 10.0 - SQLite support
- Polly 8.6 - resilience/retry policies for external APIs
- TagLibSharp 2.3 - audio file metadata extraction
- SixLabors.ImageSharp 3.1 - image processing
- MailKit 4.15 - email sending
- MimeKit 4.15 - MIME message handling
- System.ServiceModel.Syndication 10.0 - RSS/Atom feeds
- FluentValidation.AspNetCore 11.3 - request validation
- JWT libraries: System.IdentityModel.Tokens.Jwt 8.16, Microsoft.IdentityModel.Tokens 8.16

**Backend (Mouseion.Host):**
- AspNetCoreRateLimit 5.0 - API rate limiting
- OpenTelemetry 1.15 - observability (console, OTLP, Prometheus exporters)
- Swashbuckle.AspNetCore 7.2 - Swagger/OpenAPI documentation

**Web Frontend:**
- @dnd-kit 6.3+ - drag-and-drop UI primitives
- react-router-dom 7.13 - routing
- @testing-library/react 16.3 - component testing utilities
- msw 2.12 - Mock Service Worker for API mocking in dev
- vite-plugin-pwa 1.2 - Progressive Web App support
- eslint + typescript-eslint 8.56 - code quality
- happy-dom 20.7 - lightweight DOM for testing

**Android:**
- Retrofit 2 - HTTP client
- OkHttp 4 - HTTP engine with logging
- Room - local SQLite database (Jetpack)
- Coil - image loading with Compose support
- Readium (Kotlin Toolkit) - EPUB reader
- Timber - logging
- androidx.security.crypto - encrypted SharedPreferences
- reorderable - drag-and-drop lists
- desugar.jdk.libs - JDK backports for Android 29+

**Audio Core (Rust):**
- claxon 0.4 - pure Rust FLAC decoder
- rubato 0.15 - sample rate conversion
- dasp 0.11 - digital audio signal processing
- jni 0.21 - JNI bindings for Android (optional feature)
- criterion 0.5 - benchmarking
- thiserror 2.0 - error handling
- tracing 0.1 - structured logging

## Configuration

**Environment:**
- Backend loads config from environment variables and `local.properties` (Android)
- .env files: Not present in repo (secrets excluded)
- Backend: ASP.NET Core configuration system (appsettings.json per environment)
- Android: BuildConfig fields populated from local.properties or env vars

**Build:**
- `Mouseion.sln` - .NET solution file
- `tsconfig.json` + `tsconfig.app.json` + `tsconfig.node.json` - TypeScript config
- `gradle/` - Gradle wrapper and dependency versions
- `build.gradle.kts` - Android build script (Kotlin DSL)
- `vite.config.ts` - Vite configuration (web)
- `tailwind.config.ts` - Tailwind CSS configuration
- `eslint.config.js` - ESLint rules
- `.prettierrc` - code formatting (if present)

**Dependency Versions:**
- Android: `gradle/libs.versions.toml` (version catalog)
- Backend: `.csproj` files (NuGet packages)
- Web: `package.json` (npm packages)
- Audio: `Cargo.toml` (Rust crates)

## Platform Requirements

**Development:**
- .NET 10 SDK (backend)
- Node.js 18+ (web, build tools)
- JDK 17+ (Android)
- Rust 1.70+ (audio core)
- Android SDK 35 target (API level 35), minSdk 29
- Kotlin 1.9+

**Production:**
- Backend: .NET 10 runtime container or self-contained executable
- Web: Static bundle (Vite output) served via HTTP, or embedded in Tauri desktop app
- Android: APK/AAB for Google Play or side-load
- Desktop: Tauri 2 distributable (Windows, macOS, Linux)

**Databases:**
- PostgreSQL 12+ (primary production database for Mouseion)
- SQLite (fallback for Mouseion, local cache on Android)

---

*Stack analysis: 2026-03-01*
