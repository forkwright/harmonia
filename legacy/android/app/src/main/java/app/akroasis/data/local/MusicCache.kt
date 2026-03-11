// Room entities for caching music metadata
package app.akroasis.data.local

import androidx.room.Entity
import androidx.room.PrimaryKey
import app.akroasis.data.model.Track

@Entity(tableName = "track_cache")
data class TrackCacheEntity(
    @PrimaryKey
    val id: String,
    val title: String,
    val artist: String,
    val album: String,
    val duration: Long,
    val format: String,
    val coverArtUrl: String?,
    val cachedAt: Long
) {
    fun toTrack(): Track {
        return Track(
            id = id,
            title = title,
            artist = artist,
            album = album,
            albumArtist = null,
            trackNumber = null,
            discNumber = null,
            year = null,
            duration = duration,
            bitrate = null,
            sampleRate = null,
            bitDepth = null,
            format = format,
            fileSize = 0,
            filePath = "",
            coverArtUrl = coverArtUrl,
            createdAt = "",
            updatedAt = ""
        )
    }

    companion object {
        fun fromTrack(track: Track): TrackCacheEntity {
            return TrackCacheEntity(
                id = track.id,
                title = track.title,
                artist = track.artist,
                album = track.album,
                duration = track.duration,
                format = track.format,
                coverArtUrl = track.coverArtUrl,
                cachedAt = System.currentTimeMillis()
            )
        }
    }
}
