// Playback speed preference manager
package app.akroasis.data.preferences

import app.akroasis.data.local.ContentType
import app.akroasis.data.local.PlaybackSpeedDao
import app.akroasis.data.local.PlaybackSpeedRecord
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class PlaybackSpeedPreferences @Inject constructor(
    private val dao: PlaybackSpeedDao
) {
    /**
     * Get playback speed with priority: Track > Album > Default (1.0x)
     */
    suspend fun getSpeedForTrack(trackId: String, albumId: String): Float {
        // Check track-specific speed first
        dao.getSpeed(trackId)?.let { return it.speed }

        // Fall back to album speed
        dao.getSpeed(albumId)?.let { return it.speed }

        // Default speed
        return 1.0f
    }

    /**
     * Set speed for specific track
     */
    suspend fun setSpeedForTrack(trackId: String, speed: Float) {
        dao.setSpeed(
            PlaybackSpeedRecord(
                contentId = trackId,
                speed = speed,
                contentType = ContentType.TRACK
            )
        )
    }

    /**
     * Set speed for entire album
     */
    suspend fun setSpeedForAlbum(albumId: String, speed: Float) {
        dao.setSpeed(
            PlaybackSpeedRecord(
                contentId = albumId,
                speed = speed,
                contentType = ContentType.ALBUM
            )
        )
    }

    /**
     * Set speed for audiobook
     */
    suspend fun setSpeedForAudiobook(audiobookId: String, speed: Float) {
        dao.setSpeed(
            PlaybackSpeedRecord(
                contentId = audiobookId,
                speed = speed,
                contentType = ContentType.AUDIOBOOK
            )
        )
    }

    /**
     * Remove speed setting for specific content
     */
    suspend fun clearSpeed(contentId: String) {
        dao.deleteSpeed(contentId)
    }

    /**
     * Clear all speed settings
     */
    suspend fun clearAll() {
        dao.clearAll()
    }
}
