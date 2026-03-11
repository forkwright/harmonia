// Media progress tracking for audiobooks, ebooks, and music
package app.akroasis.data.local

import androidx.room.Entity
import androidx.room.PrimaryKey
import app.akroasis.data.model.MediaProgress
import app.akroasis.data.model.MediaType

@Entity(tableName = "media_progress")
data class MediaProgressEntity(
    @PrimaryKey val mediaItemId: String,
    val mediaType: String,
    val positionMs: Long,
    val totalDurationMs: Long?,
    val percentComplete: Float,
    val lastPlayedAt: Long,
    val isComplete: Boolean
) {
    fun toMediaProgress(): MediaProgress {
        return MediaProgress(
            mediaItemId = mediaItemId,
            mediaType = MediaType.valueOf(mediaType.uppercase()),
            positionMs = positionMs,
            totalDurationMs = totalDurationMs,
            percentComplete = percentComplete,
            lastPlayedAt = lastPlayedAt,
            isComplete = isComplete
        )
    }

    companion object {
        fun fromMediaProgress(progress: MediaProgress): MediaProgressEntity {
            return MediaProgressEntity(
                mediaItemId = progress.mediaItemId,
                mediaType = progress.mediaType.name.lowercase(),
                positionMs = progress.positionMs,
                totalDurationMs = progress.totalDurationMs,
                percentComplete = progress.percentComplete,
                lastPlayedAt = progress.lastPlayedAt,
                isComplete = progress.isComplete
            )
        }
    }
}
