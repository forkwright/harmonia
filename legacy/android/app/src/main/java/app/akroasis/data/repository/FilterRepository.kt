// Filter repository with facets caching
package app.akroasis.data.repository

import app.akroasis.data.api.MouseionApi
import app.akroasis.data.model.FilterRequest
import app.akroasis.data.model.FilterResponse
import app.akroasis.data.model.LibraryFacets
import app.akroasis.data.network.RetryPolicy
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class FilterRepository @Inject constructor(
    private val api: MouseionApi
) {
    companion object {
        private const val FACETS_CACHE_TTL_MS = 5 * 60 * 1000L  // 5 minutes
    }

    // Cached facets (refreshed every 5 minutes or on library scan)
    private var cachedFacets: LibraryFacets? = null
    private var facetsCachedAt: Long = 0L

    /**
     * Get library facets for autocomplete in filter UI
     * Caches for 5 minutes to reduce API calls
     */
    suspend fun getLibraryFacets(
        forceRefresh: Boolean = false
    ): Result<LibraryFacets> = withContext(Dispatchers.IO) {
        try {
            // Check cache
            val now = System.currentTimeMillis()
            if (!forceRefresh && cachedFacets != null &&
                (now - facetsCachedAt) < FACETS_CACHE_TTL_MS
            ) {
                return@withContext Result.success(cachedFacets!!)
            }

            // Fetch from API with retry
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.getLibraryFacets()
                response.body()?.let { facets ->
                    cachedFacets = facets
                    facetsCachedAt = now
                    Result.success(facets)
                } ?: Result.failure(Exception("Failed to fetch facets: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Apply filter rules to library
     * No caching (results change frequently based on rules)
     */
    suspend fun filterLibrary(
        request: FilterRequest
    ): Result<FilterResponse> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.filterLibrary(request)
                response.body()?.let { filterResponse ->
                    Result.success(filterResponse)
                } ?: Result.failure(Exception("Filter failed: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Clear cached facets (call after library scan)
     */
    fun invalidateFacetsCache() {
        cachedFacets = null
        facetsCachedAt = 0L
    }
}
