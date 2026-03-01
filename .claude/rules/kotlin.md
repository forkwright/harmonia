# Kotlin Rules

Rules for Akroasis Android — media player on Kotlin, Jetpack Compose, Hilt.

---

## Build & Validate

```bash
cd android && ./gradlew build && ./gradlew test
```

Both must pass before any PR.

---

## Architecture

MVVM with unidirectional data flow:

```
app/src/main/java/app/akroasis/
├── data/           — repositories, data sources, Room DAOs
├── domain/         — use cases, domain models
├── ui/             — screens, composables, ViewModels
├── di/             — Hilt modules
└── util/           — shared utilities
```

- ViewModel exposes `StateFlow<UiState>`, never `LiveData`
- UI observes state, emits events
- Repository is the single source of truth for data
- Use cases encapsulate business logic between ViewModel and Repository

---

## Dependency Injection

**Hilt** for all DI.

- `@HiltViewModel` on all ViewModels
- `@Inject constructor` — no manual instantiation
- Module bindings in `di/` package
- `@Singleton` for app-scoped, `@ViewModelScoped` for ViewModel-scoped

Compliant:
```kotlin
@HiltViewModel
class PlayerViewModel @Inject constructor(
    private val playbackRepository: PlaybackRepository,
    private val savedStateHandle: SavedStateHandle,
) : ViewModel() {
    // ...
}
```

---

## State Management

**StateFlow** for reactive state. No LiveData in new code.

- `MutableStateFlow` private, expose as `StateFlow`
- `stateIn` with `SharingStarted.WhileSubscribed(5000)` for flow collection
- Sealed class/interface for UI state

Compliant:
```kotlin
sealed interface PlayerUiState {
    data object Loading : PlayerUiState
    data class Playing(val track: Track, val progress: Float) : PlayerUiState
    data class Error(val message: String) : PlayerUiState
}

private val _uiState = MutableStateFlow<PlayerUiState>(PlayerUiState.Loading)
val uiState: StateFlow<PlayerUiState> = _uiState.asStateFlow()
```

---

## Compose

- **State hoisting** — composables are stateless by default, state lives in ViewModel
- `remember` / `rememberSaveable` for local ephemeral state only
- `@Preview` on all reusable composables
- Material 3 design system
- Extract reusable composables into separate files

Compliant:
```kotlin
@Composable
fun TrackItem(
    track: Track,
    isPlaying: Boolean,
    onTrackClick: (Track) -> Unit,
    modifier: Modifier = Modifier,
) {
    // stateless — all state passed in
}
```

---

## Coroutines

- `viewModelScope` for ViewModel coroutines
- `Dispatchers.IO` for I/O operations, never `Dispatchers.Main` for blocking work
- `withContext` for dispatcher switching
- Structured concurrency — never `GlobalScope`

Compliant:
```kotlin
fun loadAlbum(id: Int) {
    viewModelScope.launch {
        _uiState.value = AlbumUiState.Loading
        val result = withContext(Dispatchers.IO) {
            albumRepository.getAlbum(id)
        }
        _uiState.value = result.fold(
            onSuccess = { AlbumUiState.Loaded(it) },
            onFailure = { AlbumUiState.Error(it.message ?: "Unknown error") },
        )
    }
}
```

---

## Room

- DAOs return `Flow<T>` for observable queries
- Entity classes are data classes
- Migrations for every schema change — no `fallbackToDestructiveMigration`
- Type converters for complex types

---

## Error Handling

- `Result<T>` for operations that can fail
- `sealed class` / `sealed interface` for error hierarchies
- Never bare `catch (Exception e)` — catch specific types
- `runCatching { }` for concise error handling where appropriate

---

## Naming

| Element | Convention | Example |
|---------|-----------|---------|
| Classes, interfaces | `PascalCase` | `PlayerViewModel` |
| Functions, properties | `camelCase` | `loadAlbum` |
| Constants | `UPPER_SNAKE` or `PascalCase` | `MAX_RETRY_COUNT` |
| Packages | `lowercase` | `app.akroasis.ui` |
| Composables | `PascalCase` | `TrackItem` |
| State flows | `_camelCase` (private), `camelCase` (public) | `_uiState` / `uiState` |

---

## What NOT to Do

- Don't use LiveData in new code — use StateFlow
- Don't use GlobalScope — use structured concurrency
- Don't use `fallbackToDestructiveMigration` in Room
- Don't mix android/ and web/ changes in the same PR
- Don't add dependencies without justification
