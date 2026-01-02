// Playback speed persistence
package app.akroasis.data.local

import androidx.room.Entity
import androidx.room.PrimaryKey

enum class ContentType {
    TRACK,
    ALBUM,
    AUDIOBOOK
}

@Entity(tableName = "playback_speeds")
data class PlaybackSpeedRecord(
    @PrimaryKey val contentId: String,
    val speed: Float,
    val contentType: ContentType,
    val lastUpdated: Long = System.currentTimeMillis()
)
