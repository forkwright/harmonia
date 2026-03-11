// ViewModel for Focus filtering screen
package app.akroasis.ui.focus

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.akroasis.data.model.*
import app.akroasis.data.repository.FilterRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch
import javax.inject.Inject

sealed class FilterState {
    object Idle : FilterState()
    object Loading : FilterState()
    data class Success(val response: FilterResponse) : FilterState()
    data class Error(val message: String) : FilterState()
}

sealed class FacetsState {
    object Loading : FacetsState()
    data class Success(val facets: LibraryFacets) : FacetsState()
    data class Error(val message: String) : FacetsState()
}

@HiltViewModel
class FocusFilterViewModel @Inject constructor(
    private val filterRepository: FilterRepository
) : ViewModel() {

    // Filter rules managed by UI
    private val _filterRules = MutableStateFlow<List<FilterRule>>(emptyList())
    val filterRules: StateFlow<List<FilterRule>> = _filterRules.asStateFlow()

    private val _filterLogic = MutableStateFlow(FilterLogic.AND)
    val filterLogic: StateFlow<FilterLogic> = _filterLogic.asStateFlow()

    // Filter results
    private val _filterState = MutableStateFlow<FilterState>(FilterState.Idle)
    val filterState: StateFlow<FilterState> = _filterState.asStateFlow()

    // Facets for autocomplete
    private val _facetsState = MutableStateFlow<FacetsState>(FacetsState.Loading)
    val facetsState: StateFlow<FacetsState> = _facetsState.asStateFlow()

    init {
        loadFacets()
    }

    private fun loadFacets() {
        viewModelScope.launch {
            _facetsState.value = FacetsState.Loading
            val result = filterRepository.getLibraryFacets()
            _facetsState.value = if (result.isSuccess) {
                FacetsState.Success(result.getOrThrow())
            } else {
                FacetsState.Error(result.exceptionOrNull()?.message ?: "Failed to load facets")
            }
        }
    }

    fun addFilterRule(rule: FilterRule) {
        _filterRules.value = _filterRules.value + rule
    }

    fun removeFilterRule(rule: FilterRule) {
        _filterRules.value = _filterRules.value.filter { it != rule }
    }

    fun updateFilterRule(oldRule: FilterRule, newRule: FilterRule) {
        _filterRules.value = _filterRules.value.map {
            if (it == oldRule) newRule else it
        }
    }

    fun setFilterLogic(logic: FilterLogic) {
        _filterLogic.value = logic
    }

    fun applyFilter(page: Int = 1, pageSize: Int = 50) {
        if (_filterRules.value.isEmpty()) {
            _filterState.value = FilterState.Error("No filter rules defined")
            return
        }

        viewModelScope.launch {
            _filterState.value = FilterState.Loading

            val request = FilterRequest(
                conditions = _filterRules.value,
                logic = _filterLogic.value,
                page = page,
                pageSize = pageSize
            )

            val result = filterRepository.filterLibrary(request)
            _filterState.value = if (result.isSuccess) {
                FilterState.Success(result.getOrThrow())
            } else {
                FilterState.Error(result.exceptionOrNull()?.message ?: "Filter failed")
            }
        }
    }

    fun clearFilter() {
        _filterRules.value = emptyList()
        _filterState.value = FilterState.Idle
    }

    fun refreshFacets() {
        viewModelScope.launch {
            val result = filterRepository.getLibraryFacets(forceRefresh = true)
            _facetsState.value = if (result.isSuccess) {
                FacetsState.Success(result.getOrThrow())
            } else {
                FacetsState.Error(result.exceptionOrNull()?.message ?: "Failed to refresh facets")
            }
        }
    }
}
