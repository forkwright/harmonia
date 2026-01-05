// Search view model with debounced search logic
package app.akroasis.ui.search

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.akroasis.data.model.SearchResult
import app.akroasis.data.repository.SearchRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch
import javax.inject.Inject

sealed class SearchState {
    object Idle : SearchState()
    object Loading : SearchState()
    data class Success(val results: List<SearchResult>) : SearchState()
    data class Error(val message: String) : SearchState()
}

@OptIn(FlowPreview::class)
@HiltViewModel
class SearchViewModel @Inject constructor(
    private val searchRepository: SearchRepository,
    val bitPerfectCalculator: app.akroasis.audio.BitPerfectCalculator
) : ViewModel() {

    private val _searchQuery = MutableStateFlow("")
    val searchQuery: StateFlow<String> = _searchQuery.asStateFlow()

    private val _searchState = MutableStateFlow<SearchState>(SearchState.Idle)
    val searchState: StateFlow<SearchState> = _searchState.asStateFlow()

    init {
        // Debounced search: 300ms delay after user stops typing
        _searchQuery
            .debounce(300)
            .filter { it.length >= 2 } // Minimum 2 characters
            .distinctUntilChanged()
            .onEach { query ->
                performSearch(query)
            }
            .launchIn(viewModelScope)
    }

    fun updateSearchQuery(query: String) {
        _searchQuery.value = query
        if (query.isEmpty()) {
            _searchState.value = SearchState.Idle
        }
    }

    private fun performSearch(query: String) {
        viewModelScope.launch {
            _searchState.value = SearchState.Loading

            val result = searchRepository.search(query = query, limit = 50)

            _searchState.value = if (result.isSuccess) {
                val results = result.getOrElse { emptyList() }
                SearchState.Success(results)
            } else {
                SearchState.Error(result.exceptionOrNull()?.message ?: "Search failed")
            }
        }
    }

    fun clearSearch() {
        _searchQuery.value = ""
        _searchState.value = SearchState.Idle
    }
}
