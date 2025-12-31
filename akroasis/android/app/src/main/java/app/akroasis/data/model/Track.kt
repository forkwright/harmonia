// Track data model from Mouseion API
package app.akroasis.data.model

import com.google.gson.annotations.SerializedName

data class Track(
    @SerializedName("id")
    val id: String,

    @SerializedName("title")
    val title: String,

    @SerializedName("artist")
    val artist: String,

    @SerializedName("album")
    val album: String,

    @SerializedName("albumArtist")
    val albumArtist: String?,

    @SerializedName("trackNumber")
    val trackNumber: Int?,

    @SerializedName("discNumber")
    val discNumber: Int?,

    @SerializedName("year")
    val year: Int?,

    @SerializedName("duration")
    val duration: Long,

    @SerializedName("bitrate")
    val bitrate: Int?,

    @SerializedName("sampleRate")
    val sampleRate: Int?,

    @SerializedName("bitDepth")
    val bitDepth: Int?,

    @SerializedName("format")
    val format: String,

    @SerializedName("fileSize")
    val fileSize: Long,

    @SerializedName("filePath")
    val filePath: String,

    @SerializedName("coverArtUrl")
    val coverArtUrl: String?,

    @SerializedName("replayGainTrackGain")
    val replayGainTrackGain: Float? = null,

    @SerializedName("replayGainAlbumGain")
    val replayGainAlbumGain: Float? = null,

    @SerializedName("createdAt")
    val createdAt: String,

    @SerializedName("updatedAt")
    val updatedAt: String
)

data class Album(
    @SerializedName("id")
    val id: String,

    @SerializedName("title")
    val title: String,

    @SerializedName("artist")
    val artist: String,

    @SerializedName("albumArtist")
    val albumArtist: String?,

    @SerializedName("year")
    val year: Int?,

    @SerializedName("trackCount")
    val trackCount: Int,

    @SerializedName("coverArtUrl")
    val coverArtUrl: String?,

    @SerializedName("createdAt")
    val createdAt: String,

    @SerializedName("updatedAt")
    val updatedAt: String
)

data class Artist(
    @SerializedName("id")
    val id: String,

    @SerializedName("name")
    val name: String,

    @SerializedName("albumCount")
    val albumCount: Int,

    @SerializedName("trackCount")
    val trackCount: Int,

    @SerializedName("imageUrl")
    val imageUrl: String?,

    @SerializedName("createdAt")
    val createdAt: String,

    @SerializedName("updatedAt")
    val updatedAt: String
)
