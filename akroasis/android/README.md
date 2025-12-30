# Akroasis Android

Native Android application for bit-perfect audio playback.

## Requirements

- Android Studio Ladybug | 2024.2.1 or later
- JDK 17
- Android SDK 35
- Minimum Android version: Android 10 (API 29)
- Target Android version: Android 15 (API 35)

## Building

```bash
./gradlew build
```

## Running

```bash
./gradlew installDebug
```

## Architecture

- **UI**: Jetpack Compose (declarative UI)
- **DI**: Hilt (dependency injection)
- **Networking**: Retrofit + OkHttp
- **Database**: Room (offline cache)
- **Audio**: Rust core (akroasis-core) via JNI

## Project Structure

```
app/
├── src/main/
│   ├── java/app/akroasis/
│   │   ├── MainActivity.kt
│   │   ├── AkroasisApplication.kt
│   │   └── ui/theme/
│   ├── res/
│   └── AndroidManifest.xml
└── build.gradle.kts
```

## Development

Phase 0 setup complete. Next steps:
1. Install Android Studio and Gradle
2. Set up Rust toolchain for Android targets
3. Implement JNI bindings to akroasis-core
4. Build UI components (Phase 2)
