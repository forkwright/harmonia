// Search repository for full-text music search
package app.akroasis.data.repository

import app.akroasis.data.api.MouseionApi
import app.akroasis.data.model.SearchResult
import app.akroasis.data.network.RetryPolicy
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class SearchRepository @Inject constructor(
    private val api: MouseionApi
) {
    suspend fun search(
        query: String,
        limit: Int = 50
    ): Result<List<SearchResult>> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.search(query = query, limit = limit)
                response.body()?.let { results ->
                    Result.success(results)
                } ?: Result.failure(Exception("Search failed: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
