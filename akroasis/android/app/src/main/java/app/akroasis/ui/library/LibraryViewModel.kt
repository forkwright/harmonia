package app.akroasis.ui.library

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.akroasis.data.model.Album
import app.akroasis.data.model.Artist
import app.akroasis.data.model.Track
import app.akroasis.data.repository.MusicRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

private const val ERROR_UNKNOWN = "Unknown error"

@HiltViewModel
class LibraryViewModel @Inject constructor(
    private val musicRepository: MusicRepository
) : ViewModel() {

    private val _artistsState = MutableStateFlow<LibraryState<List<Artist>>>(LibraryState.Loading)
    val artistsState: StateFlow<LibraryState<List<Artist>>> = _artistsState.asStateFlow()

    private val _albumsState = MutableStateFlow<LibraryState<List<Album>>>(LibraryState.Loading)
    val albumsState: StateFlow<LibraryState<List<Album>>> = _albumsState.asStateFlow()

    private val _tracksState = MutableStateFlow<LibraryState<List<Track>>>(LibraryState.Loading)
    val tracksState: StateFlow<LibraryState<List<Track>>> = _tracksState.asStateFlow()

    init {
        loadArtists()
    }

    fun loadArtists() {
        viewModelScope.launch {
            _artistsState.value = LibraryState.Loading
            val result = musicRepository.getArtists()
            _artistsState.value = if (result.isSuccess) {
                LibraryState.Success(result.getOrElse { emptyList() })
            } else {
                LibraryState.Error(result.exceptionOrNull()?.message ?: ERROR_UNKNOWN)
            }
        }
    }

    fun loadAlbums(artistId: String? = null) {
        viewModelScope.launch {
            _albumsState.value = LibraryState.Loading
            val result = musicRepository.getAlbums(artistId = artistId)
            _albumsState.value = if (result.isSuccess) {
                LibraryState.Success(result.getOrElse { emptyList() })
            } else {
                LibraryState.Error(result.exceptionOrNull()?.message ?: ERROR_UNKNOWN)
            }
        }
    }

    fun loadTracks(albumId: String? = null, artistId: String? = null) {
        viewModelScope.launch {
            _tracksState.value = LibraryState.Loading
            val result = musicRepository.getTracks(albumId = albumId, artistId = artistId)
            _tracksState.value = if (result.isSuccess) {
                LibraryState.Success(result.getOrElse { emptyList() })
            } else {
                LibraryState.Error(result.exceptionOrNull()?.message ?: ERROR_UNKNOWN)
            }
        }
    }
}

sealed class LibraryState<out T> {
    data object Loading : LibraryState<Nothing>()
    data class Success<T>(val data: T) : LibraryState<T>()
    data class Error(val message: String) : LibraryState<Nothing>()
}
