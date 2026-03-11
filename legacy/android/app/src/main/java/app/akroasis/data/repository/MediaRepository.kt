// Unified media repository for Continue feed, progress tracking, and session management
package app.akroasis.data.repository

import app.akroasis.data.api.ContinueItemDto
import app.akroasis.data.api.MediaItemDto
import app.akroasis.data.api.MouseionApi
import app.akroasis.data.api.ProgressUpdateRequest
import app.akroasis.data.api.SessionDto
import app.akroasis.data.api.StartSessionRequest
import app.akroasis.data.api.UpdateSessionRequest
import app.akroasis.data.model.MediaItem
import app.akroasis.data.model.MediaProgress
import app.akroasis.data.model.MediaType
import app.akroasis.data.model.toMediaItem
import app.akroasis.data.network.RetryPolicy
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class MediaRepository @Inject constructor(
    private val api: MouseionApi,
    private val musicRepository: MusicRepository,
    private val audiobookRepository: AudiobookRepository,
    private val ebookRepository: EbookRepository
) {
    suspend fun getContinueFeed(
        userId: String = "default",
        limit: Int = 20
    ): Result<List<ContinueItem>> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.getContinueFeed(userId, limit)
                response.body()?.let { items ->
                    Result.success(items.map { it.toContinueItem() })
                } ?: Result.failure(Exception("Failed to fetch continue feed: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun updateProgress(
        mediaId: String,
        mediaType: MediaType,
        positionMs: Long,
        durationMs: Long?,
        userId: String = "default"
    ): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            val request = ProgressUpdateRequest(
                mediaItemId = mediaId,
                mediaType = mediaType.name.lowercase(),
                positionMs = positionMs,
                totalDurationMs = durationMs,
                userId = userId
            )
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.updateProgress(request)
                if (response.isSuccessful) {
                    Result.success(Unit)
                } else {
                    Result.failure(Exception("Failed to update progress: ${response.code()}"))
                }
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getProgress(
        mediaId: String,
        userId: String = "default"
    ): Result<MediaProgress?> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.getProgress(mediaId, userId)
                response.body()?.let { progress ->
                    Result.success(progress)
                } ?: Result.success(null)
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun deleteProgress(
        mediaId: String,
        userId: String = "default"
    ): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.deleteProgress(mediaId, userId)
                if (response.isSuccessful) {
                    Result.success(Unit)
                } else {
                    Result.failure(Exception("Failed to delete progress: ${response.code()}"))
                }
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun startSession(
        mediaId: String,
        mediaType: MediaType,
        userId: String = "default"
    ): Result<Session> = withContext(Dispatchers.IO) {
        try {
            val request = StartSessionRequest(
                mediaItemId = mediaId,
                mediaType = mediaType.name.lowercase(),
                userId = userId
            )
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.startSession(request)
                response.body()?.let { session ->
                    Result.success(session.toSession())
                } ?: Result.failure(Exception("Failed to start session: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun endSession(
        sessionId: String,
        durationMs: Long
    ): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            val request = UpdateSessionRequest(
                endedAt = System.currentTimeMillis(),
                durationMs = durationMs
            )
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.updateSession(sessionId, request)
                if (response.isSuccessful) {
                    Result.success(Unit)
                } else {
                    Result.failure(Exception("Failed to end session: ${response.code()}"))
                }
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getSessions(
        userId: String = "default",
        activeOnly: Boolean = false,
        limit: Int = 100
    ): Result<List<Session>> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.getSessions(userId, activeOnly, limit)
                response.body()?.let { sessions ->
                    Result.success(sessions.map { it.toSession() })
                } ?: Result.failure(Exception("Failed to fetch sessions: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun deleteSession(sessionId: String): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.deleteSession(sessionId)
                if (response.isSuccessful) {
                    Result.success(Unit)
                } else {
                    Result.failure(Exception("Failed to delete session: ${response.code()}"))
                }
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    private fun ContinueItemDto.toContinueItem(): ContinueItem {
        val mediaItem = when (mediaItem.mediaType) {
            "music" -> {
                // For music items, we need to fetch the full Track from MusicRepository
                // For now, create a simplified MediaItem.Music from the DTO
                MediaItem.Music(
                    id = mediaItem.id,
                    title = mediaItem.title,
                    artist = mediaItem.artist ?: "Unknown Artist",
                    album = mediaItem.album ?: "Unknown Album",
                    albumArtist = null,
                    trackNumber = null,
                    discNumber = null,
                    year = null,
                    duration = mediaItem.duration ?: 0,
                    bitrate = null,
                    sampleRate = null,
                    bitDepth = null,
                    format = "unknown",
                    fileSize = 0,
                    filePath = "",
                    coverArtUrl = mediaItem.coverArtUrl,
                    replayGainTrackGain = null,
                    replayGainAlbumGain = null,
                    createdAt = "",
                    updatedAt = ""
                )
            }
            "audiobook" -> {
                MediaItem.Audiobook(
                    id = mediaItem.id,
                    title = mediaItem.title,
                    author = mediaItem.author ?: "Unknown Author",
                    narrator = null,
                    seriesName = null,
                    seriesNumber = null,
                    chapters = emptyList(),
                    duration = mediaItem.duration ?: 0,
                    coverArtUrl = mediaItem.coverArtUrl,
                    totalChapters = 0,
                    format = "unknown",
                    fileSize = 0,
                    filePath = "",
                    createdAt = "",
                    updatedAt = ""
                )
            }
            "ebook" -> {
                MediaItem.Ebook(
                    id = mediaItem.id,
                    title = mediaItem.title,
                    author = mediaItem.author ?: "Unknown Author",
                    seriesName = null,
                    seriesNumber = null,
                    pageCount = null,
                    publishDate = null,
                    coverArtUrl = mediaItem.coverArtUrl,
                    duration = null,
                    format = "unknown",
                    fileSize = 0,
                    filePath = "",
                    createdAt = "",
                    updatedAt = ""
                )
            }
            else -> throw IllegalArgumentException("Unknown media type: ${mediaItem.mediaType}")
        }
        return ContinueItem(mediaItem, progress)
    }

    private fun SessionDto.toSession(): Session {
        return Session(
            id = id,
            userId = userId,
            mediaItemId = mediaItemId,
            mediaType = MediaType.valueOf(mediaType.uppercase()),
            startedAt = startedAt,
            endedAt = endedAt,
            durationMs = durationMs,
            isActive = isActive
        )
    }
}

data class ContinueItem(
    val mediaItem: MediaItem,
    val progress: MediaProgress
)

data class Session(
    val id: String,
    val userId: String,
    val mediaItemId: String,
    val mediaType: MediaType,
    val startedAt: Long,
    val endedAt: Long?,
    val durationMs: Long,
    val isActive: Boolean
)
