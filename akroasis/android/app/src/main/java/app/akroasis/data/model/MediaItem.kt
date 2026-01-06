// Unified media item for audiobooks, ebooks, and music
package app.akroasis.data.model

import com.google.gson.annotations.SerializedName

sealed class MediaItem {
    abstract val id: String
    abstract val title: String
    abstract val coverArtUrl: String?
    abstract val duration: Long?

    data class Music(
        override val id: String,
        override val title: String,
        val artist: String,
        val album: String,
        val albumArtist: String?,
        val trackNumber: Int?,
        val discNumber: Int?,
        val year: Int?,
        override val duration: Long,
        val bitrate: Int?,
        val sampleRate: Int?,
        val bitDepth: Int?,
        val format: String,
        val fileSize: Long,
        val filePath: String,
        override val coverArtUrl: String?,
        val replayGainTrackGain: Float? = null,
        val replayGainAlbumGain: Float? = null,
        val createdAt: String,
        val updatedAt: String
    ) : MediaItem()

    data class Audiobook(
        override val id: String,
        override val title: String,
        val author: String,
        val narrator: String?,
        val seriesName: String?,
        val seriesNumber: Int?,
        val chapters: List<Chapter>,
        override val duration: Long,
        override val coverArtUrl: String?,
        val totalChapters: Int,
        val format: String,
        val fileSize: Long,
        val filePath: String,
        val createdAt: String,
        val updatedAt: String
    ) : MediaItem()

    data class Ebook(
        override val id: String,
        override val title: String,
        val author: String,
        val seriesName: String?,
        val seriesNumber: Int?,
        val pageCount: Int?,
        val publishDate: String?,
        override val coverArtUrl: String?,
        override val duration: Long? = null,
        val format: String,
        val fileSize: Long,
        val filePath: String,
        val createdAt: String,
        val updatedAt: String
    ) : MediaItem()
}

data class Chapter(
    @SerializedName("index")
    val index: Int,

    @SerializedName("title")
    val title: String,

    @SerializedName("startTimeMs")
    val startTimeMs: Long,

    @SerializedName("endTimeMs")
    val endTimeMs: Long
)

data class MediaProgress(
    @SerializedName("mediaItemId")
    val mediaItemId: String,

    @SerializedName("mediaType")
    val mediaType: MediaType,

    @SerializedName("positionMs")
    val positionMs: Long,

    @SerializedName("totalDurationMs")
    val totalDurationMs: Long?,

    @SerializedName("percentComplete")
    val percentComplete: Float,

    @SerializedName("lastPlayedAt")
    val lastPlayedAt: Long,

    @SerializedName("isComplete")
    val isComplete: Boolean
)

enum class MediaType {
    @SerializedName("music")
    MUSIC,

    @SerializedName("audiobook")
    AUDIOBOOK,

    @SerializedName("ebook")
    EBOOK
}

// Extension to convert Track to MediaItem.Music
fun Track.toMediaItem(): MediaItem.Music = MediaItem.Music(
    id = id,
    title = title,
    artist = artist,
    album = album,
    albumArtist = albumArtist,
    trackNumber = trackNumber,
    discNumber = discNumber,
    year = year,
    duration = duration,
    bitrate = bitrate,
    sampleRate = sampleRate,
    bitDepth = bitDepth,
    format = format,
    fileSize = fileSize,
    filePath = filePath,
    coverArtUrl = coverArtUrl,
    replayGainTrackGain = replayGainTrackGain,
    replayGainAlbumGain = replayGainAlbumGain,
    createdAt = createdAt,
    updatedAt = updatedAt
)

// Extension to convert MediaItem.Music back to Track (for backwards compatibility)
fun MediaItem.Music.toTrack(): Track = Track(
    id = id,
    title = title,
    artist = artist,
    album = album,
    albumArtist = albumArtist,
    trackNumber = trackNumber,
    discNumber = discNumber,
    year = year,
    duration = duration,
    bitrate = bitrate,
    sampleRate = sampleRate,
    bitDepth = bitDepth,
    format = format,
    fileSize = fileSize,
    filePath = filePath,
    coverArtUrl = coverArtUrl,
    replayGainTrackGain = replayGainTrackGain,
    replayGainAlbumGain = replayGainAlbumGain,
    createdAt = createdAt,
    updatedAt = updatedAt
)
