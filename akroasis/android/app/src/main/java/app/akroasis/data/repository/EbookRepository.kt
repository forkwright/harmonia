// Ebook repository for fetching ebook data
package app.akroasis.data.repository

import app.akroasis.data.api.EbookDto
import app.akroasis.data.api.MouseionApi
import app.akroasis.data.model.MediaItem
import app.akroasis.data.network.RetryPolicy
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class EbookRepository @Inject constructor(
    private val api: MouseionApi
) {
    suspend fun getEbooks(
        page: Int = 1,
        pageSize: Int = 50
    ): Result<List<MediaItem.Ebook>> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.getEbooks(page, pageSize)
                response.body()?.let { ebooks ->
                    Result.success(ebooks.map { it.toEbook() })
                } ?: Result.failure(Exception("Failed to fetch ebooks: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getEbook(id: String): Result<MediaItem.Ebook> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.getEbook(id)
                response.body()?.let { ebook ->
                    Result.success(ebook.toEbook())
                } ?: Result.failure(Exception("Failed to fetch ebook: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    fun getEpubUrl(id: String): String {
        return "api/v3/stream/$id"
    }

    private fun EbookDto.toEbook(): MediaItem.Ebook {
        return MediaItem.Ebook(
            id = id,
            title = title,
            author = author,
            seriesName = seriesName,
            seriesNumber = seriesNumber,
            pageCount = pageCount,
            publishDate = publishDate,
            coverArtUrl = coverArtUrl,
            duration = null, // Reading time estimate can be calculated later
            format = format,
            fileSize = fileSize,
            filePath = filePath,
            createdAt = createdAt,
            updatedAt = updatedAt
        )
    }
}
