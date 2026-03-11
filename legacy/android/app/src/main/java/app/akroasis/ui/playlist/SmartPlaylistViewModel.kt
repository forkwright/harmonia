// ViewModel for smart playlist management
package app.akroasis.ui.playlist

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.akroasis.data.local.SmartPlaylistEntity
import app.akroasis.data.model.FilterRequest
import app.akroasis.data.repository.SmartPlaylistRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch
import javax.inject.Inject

sealed class SmartPlaylistUiState {
    object Idle : SmartPlaylistUiState()
    object Loading : SmartPlaylistUiState()
    data class Error(val message: String) : SmartPlaylistUiState()
    object Success : SmartPlaylistUiState()
}

@HiltViewModel
class SmartPlaylistViewModel @Inject constructor(
    private val repository: SmartPlaylistRepository
) : ViewModel() {

    private val _uiState = MutableStateFlow<SmartPlaylistUiState>(SmartPlaylistUiState.Idle)
    val uiState: StateFlow<SmartPlaylistUiState> = _uiState.asStateFlow()

    val playlists: StateFlow<List<SmartPlaylistEntity>> = repository.getAllPlaylists()
        .stateIn(
            scope = viewModelScope,
            started = SharingStarted.WhileSubscribed(5000),
            initialValue = emptyList()
        )

    init {
        syncFromServer()
    }

    fun syncFromServer() {
        viewModelScope.launch {
            _uiState.value = SmartPlaylistUiState.Loading
            repository.syncFromServer()
                .onSuccess {
                    _uiState.value = SmartPlaylistUiState.Success
                }
                .onFailure { error ->
                    _uiState.value = SmartPlaylistUiState.Error(
                        error.message ?: "Failed to sync playlists"
                    )
                }
        }
    }

    fun createPlaylist(name: String, filterRequest: FilterRequest) {
        viewModelScope.launch {
            _uiState.value = SmartPlaylistUiState.Loading
            repository.createPlaylist(name, filterRequest)
                .onSuccess {
                    _uiState.value = SmartPlaylistUiState.Success
                }
                .onFailure { error ->
                    _uiState.value = SmartPlaylistUiState.Error(
                        error.message ?: "Failed to create playlist"
                    )
                }
        }
    }

    fun updatePlaylist(id: String, name: String, filterRequest: FilterRequest) {
        viewModelScope.launch {
            _uiState.value = SmartPlaylistUiState.Loading
            repository.updatePlaylist(id, name, filterRequest)
                .onSuccess {
                    _uiState.value = SmartPlaylistUiState.Success
                }
                .onFailure { error ->
                    _uiState.value = SmartPlaylistUiState.Error(
                        error.message ?: "Failed to update playlist"
                    )
                }
        }
    }

    fun deletePlaylist(id: String) {
        viewModelScope.launch {
            _uiState.value = SmartPlaylistUiState.Loading
            repository.deletePlaylist(id)
                .onSuccess {
                    _uiState.value = SmartPlaylistUiState.Success
                }
                .onFailure { error ->
                    _uiState.value = SmartPlaylistUiState.Error(
                        error.message ?: "Failed to delete playlist"
                    )
                }
        }
    }

    fun refreshPlaylist(id: String) {
        viewModelScope.launch {
            _uiState.value = SmartPlaylistUiState.Loading
            repository.refreshPlaylist(id)
                .onSuccess {
                    _uiState.value = SmartPlaylistUiState.Success
                }
                .onFailure { error ->
                    _uiState.value = SmartPlaylistUiState.Error(
                        error.message ?: "Failed to refresh playlist"
                    )
                }
        }
    }
}
