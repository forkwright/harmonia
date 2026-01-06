// Audiobook repository for fetching audiobook data and chapters
package app.akroasis.data.repository

import app.akroasis.data.api.AudiobookDto
import app.akroasis.data.api.MouseionApi
import app.akroasis.data.model.Chapter
import app.akroasis.data.model.MediaItem
import app.akroasis.data.network.RetryPolicy
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class AudiobookRepository @Inject constructor(
    private val api: MouseionApi
) {
    suspend fun getAudiobooks(
        page: Int = 1,
        pageSize: Int = 50
    ): Result<List<MediaItem.Audiobook>> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.getAudiobooks(page, pageSize)
                response.body()?.let { audiobooks ->
                    Result.success(audiobooks.map { it.toAudiobook() })
                } ?: Result.failure(Exception("Failed to fetch audiobooks: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getAudiobook(id: String): Result<MediaItem.Audiobook> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.getAudiobook(id)
                response.body()?.let { audiobook ->
                    Result.success(audiobook.toAudiobook())
                } ?: Result.failure(Exception("Failed to fetch audiobook: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getChapters(mediaFileId: String): Result<List<Chapter>> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.getChapters(mediaFileId)
                response.body()?.let { chapters ->
                    Result.success(chapters)
                } ?: Result.failure(Exception("Failed to fetch chapters: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    fun streamAudiobook(id: String): String {
        return "api/v3/stream/$id"
    }

    private fun AudiobookDto.toAudiobook(): MediaItem.Audiobook {
        return MediaItem.Audiobook(
            id = id,
            title = title,
            author = author,
            narrator = narrator,
            seriesName = seriesName,
            seriesNumber = seriesNumber,
            chapters = emptyList(), // Chapters loaded separately via getChapters()
            duration = duration,
            coverArtUrl = coverArtUrl,
            totalChapters = totalChapters,
            format = format,
            fileSize = fileSize,
            filePath = filePath,
            createdAt = createdAt,
            updatedAt = updatedAt
        )
    }
}
