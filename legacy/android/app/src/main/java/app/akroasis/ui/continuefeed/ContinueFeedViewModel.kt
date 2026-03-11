// Continue feed for in-progress media across all types
package app.akroasis.ui.continuefeed

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.akroasis.data.repository.ContinueItem
import app.akroasis.data.repository.MediaRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import timber.log.Timber
import javax.inject.Inject

@HiltViewModel
class ContinueFeedViewModel @Inject constructor(
    private val mediaRepository: MediaRepository
) : ViewModel() {

    companion object {
        private const val ERROR_MESSAGE_DEFAULT = "Unknown error"
    }

    private val _uiState = MutableStateFlow<ContinueFeedUiState>(ContinueFeedUiState.Loading)
    val uiState: StateFlow<ContinueFeedUiState> = _uiState.asStateFlow()

    private val _isRefreshing = MutableStateFlow(false)
    val isRefreshing: StateFlow<Boolean> = _isRefreshing.asStateFlow()

    init {
        loadContinueFeed()
    }

    fun loadContinueFeed(userId: String = "default", limit: Int = 20) {
        viewModelScope.launch {
            _uiState.value = ContinueFeedUiState.Loading

            try {
                val result = mediaRepository.getContinueFeed(userId, limit)

                if (result.isSuccess) {
                    val items = result.getOrNull() ?: emptyList()
                    _uiState.value = if (items.isEmpty()) {
                        ContinueFeedUiState.Empty
                    } else {
                        ContinueFeedUiState.Success(items)
                    }
                    Timber.d("Loaded ${items.size} continue items")
                } else {
                    val error = result.exceptionOrNull()?.message ?: ERROR_MESSAGE_DEFAULT
                    _uiState.value = ContinueFeedUiState.Error(error)
                    Timber.e("Failed to load continue feed: $error")
                }
            } catch (e: Exception) {
                _uiState.value = ContinueFeedUiState.Error(e.message ?: ERROR_MESSAGE_DEFAULT)
                Timber.e(e, "Error loading continue feed")
            }
        }
    }

    fun refresh(userId: String = "default", limit: Int = 20) {
        viewModelScope.launch {
            _isRefreshing.value = true

            try {
                val result = mediaRepository.getContinueFeed(userId, limit)

                if (result.isSuccess) {
                    val items = result.getOrNull() ?: emptyList()
                    _uiState.value = if (items.isEmpty()) {
                        ContinueFeedUiState.Empty
                    } else {
                        ContinueFeedUiState.Success(items)
                    }
                    Timber.d("Refreshed ${items.size} continue items")
                } else {
                    val error = result.exceptionOrNull()?.message ?: ERROR_MESSAGE_DEFAULT
                    _uiState.value = ContinueFeedUiState.Error(error)
                    Timber.e("Failed to refresh continue feed: $error")
                }
            } catch (e: Exception) {
                Timber.e(e, "Error refreshing continue feed")
            } finally {
                _isRefreshing.value = false
            }
        }
    }

    fun deleteProgress(continueItem: ContinueItem, userId: String = "default") {
        viewModelScope.launch {
            try {
                val result = mediaRepository.deleteProgress(continueItem.mediaItem.id, userId)

                if (result.isSuccess) {
                    // Remove item from current list
                    val currentState = _uiState.value
                    if (currentState is ContinueFeedUiState.Success) {
                        val updatedItems = currentState.items.filterNot {
                            it.mediaItem.id == continueItem.mediaItem.id
                        }
                        _uiState.value = if (updatedItems.isEmpty()) {
                            ContinueFeedUiState.Empty
                        } else {
                            ContinueFeedUiState.Success(updatedItems)
                        }
                    }
                    Timber.d("Deleted progress for ${continueItem.mediaItem.title}")
                } else {
                    Timber.e("Failed to delete progress: ${result.exceptionOrNull()?.message}")
                }
            } catch (e: Exception) {
                Timber.e(e, "Error deleting progress")
            }
        }
    }

    fun retry() {
        loadContinueFeed()
    }
}

sealed class ContinueFeedUiState {
    object Loading : ContinueFeedUiState()
    object Empty : ContinueFeedUiState()
    data class Success(val items: List<ContinueItem>) : ContinueFeedUiState()
    data class Error(val message: String) : ContinueFeedUiState()
}
