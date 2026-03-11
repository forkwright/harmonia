// Test double for AndroidAutoService that exposes internal logic without Android framework
package app.akroasis.auto

import android.net.Uri
import android.support.v4.media.MediaBrowserCompat
import android.support.v4.media.MediaDescriptionCompat
import app.akroasis.audio.VoiceSearchHandler
import app.akroasis.audio.VoiceSearchResult
import app.akroasis.data.model.Album
import app.akroasis.data.model.Artist
import app.akroasis.data.model.FilterField
import app.akroasis.data.model.FilterLogic
import app.akroasis.data.model.FilterOperator
import app.akroasis.data.model.FilterRequest
import app.akroasis.data.model.FilterRule
import app.akroasis.data.model.Track
import app.akroasis.data.repository.FilterRepository
import app.akroasis.data.repository.MusicRepository
import timber.log.Timber

class TestableAndroidAutoService(
    private val musicRepository: MusicRepository,
    private val filterRepository: FilterRepository,
    private val voiceSearchHandler: VoiceSearchHandler
) {
    fun getRootItemsPublic(): List<MediaBrowserCompat.MediaItem> {
        return listOf(
            createBrowsableItem("recent", "Recently Played", "Your recent tracks"),
            createBrowsableItem("albums", "Albums", "Browse by album"),
            createBrowsableItem("artists", "Artists", "Browse by artist"),
            createBrowsableItem("genres", "Genres", "Browse by genre"),
            createBrowsableItem("playlists", "Playlists", "Your playlists")
        )
    }

    suspend fun getRecentTracksPublic(): List<MediaBrowserCompat.MediaItem> {
        return try {
            val tracks = musicRepository.getRecentTracks(limit = 20)
            tracks.map { createPlayableItemPublic(it) }
        } catch (e: Exception) {
            listOf(createErrorItem("Unable to load", "Recent tracks unavailable"))
        }
    }

    suspend fun getAlbumsPublic(): List<MediaBrowserCompat.MediaItem> {
        return try {
            val albums = musicRepository.getAllAlbums()
            albums.map { createBrowsableAlbumItemPublic(it) }
        } catch (e: Exception) {
            listOf(createErrorItem("Unable to load", "Albums unavailable"))
        }
    }

    suspend fun getArtistsPublic(): List<MediaBrowserCompat.MediaItem> {
        return try {
            val artists = musicRepository.getAllArtists()
            artists.map { createBrowsableArtistItemPublic(it) }
        } catch (e: Exception) {
            listOf(createErrorItem("Unable to load", "Artists unavailable"))
        }
    }

    suspend fun getGenresPublic(): List<MediaBrowserCompat.MediaItem> {
        return try {
            val facets = filterRepository.getLibraryFacets().getOrThrow()
            facets.genres.sorted().map { genre ->
                createBrowsableItem("genre_${genre}", genre, "Browse $genre tracks")
            }
        } catch (e: Exception) {
            listOf(createErrorItem("Unable to load", "Genres unavailable"))
        }
    }

    suspend fun getAlbumTracksPublic(albumId: String): List<MediaBrowserCompat.MediaItem> {
        return try {
            val tracks = musicRepository.getAlbumTracks(albumId)
            tracks.map { createPlayableItemPublic(it) }
        } catch (e: Exception) {
            listOf(createErrorItem("Unable to load", "Album tracks unavailable"))
        }
    }

    suspend fun getArtistTracksPublic(artistId: String): List<MediaBrowserCompat.MediaItem> {
        return try {
            val tracks = musicRepository.getArtistTracks(artistId)
            tracks.map { createPlayableItemPublic(it) }
        } catch (e: Exception) {
            listOf(createErrorItem("Unable to load", "Artist tracks unavailable"))
        }
    }

    suspend fun getGenreTracksPublic(genre: String): List<MediaBrowserCompat.MediaItem> {
        return try {
            val request = FilterRequest(
                conditions = listOf(
                    FilterRule(
                        field = FilterField.GENRE,
                        operator = FilterOperator.EQUALS,
                        value = genre
                    )
                ),
                logic = FilterLogic.AND,
                pageSize = 100
            )
            val response = filterRepository.filterLibrary(request).getOrThrow()
            response.tracks.map { createPlayableItemPublic(it) }
        } catch (e: Exception) {
            listOf(createErrorItem("Unable to load", "Genre tracks unavailable"))
        }
    }

    suspend fun handleSearchPublic(query: String, extras: android.os.Bundle?): List<MediaBrowserCompat.MediaItem> {
        return when (val result = voiceSearchHandler.handleVoiceSearch(query, extras)) {
            is VoiceSearchResult.Success -> result.tracks.map { createPlayableItemPublic(it) }
            is VoiceSearchResult.NoResults -> listOf(
                createErrorItem("No results", "Nothing matched \"${result.query}\"")
            )
            is VoiceSearchResult.Error -> {
                Timber.e("Auto search error: ${result.message}")
                listOf(createErrorItem("Search unavailable", result.message))
            }
        }
    }

    fun createPlayableItemPublic(track: Track): MediaBrowserCompat.MediaItem {
        val builder = MediaDescriptionCompat.Builder()
            .setMediaId(track.id)
            .setTitle(track.title)
            .setSubtitle(track.artist)
            .setDescription(track.album)

        track.coverArtUrl?.let { builder.setIconUri(Uri.parse(it)) }

        return MediaBrowserCompat.MediaItem(
            builder.build(),
            MediaBrowserCompat.MediaItem.FLAG_PLAYABLE
        )
    }

    fun createBrowsableAlbumItemPublic(album: Album): MediaBrowserCompat.MediaItem {
        val builder = MediaDescriptionCompat.Builder()
            .setMediaId("album_${album.id}")
            .setTitle(album.title)
            .setSubtitle(album.artist)

        album.coverArtUrl?.let { builder.setIconUri(Uri.parse(it)) }

        return MediaBrowserCompat.MediaItem(
            builder.build(),
            MediaBrowserCompat.MediaItem.FLAG_BROWSABLE
        )
    }

    fun createBrowsableArtistItemPublic(artist: Artist): MediaBrowserCompat.MediaItem {
        val builder = MediaDescriptionCompat.Builder()
            .setMediaId("artist_${artist.id}")
            .setTitle(artist.name)
            .setSubtitle("${artist.albumCount} albums")

        artist.imageUrl?.let { builder.setIconUri(Uri.parse(it)) }

        return MediaBrowserCompat.MediaItem(
            builder.build(),
            MediaBrowserCompat.MediaItem.FLAG_BROWSABLE
        )
    }

    private fun createBrowsableItem(
        id: String,
        title: String,
        subtitle: String
    ): MediaBrowserCompat.MediaItem {
        val description = MediaDescriptionCompat.Builder()
            .setMediaId(id)
            .setTitle(title)
            .setSubtitle(subtitle)
            .build()

        return MediaBrowserCompat.MediaItem(
            description,
            MediaBrowserCompat.MediaItem.FLAG_BROWSABLE
        )
    }

    private fun createErrorItem(title: String, detail: String): MediaBrowserCompat.MediaItem {
        val description = MediaDescriptionCompat.Builder()
            .setMediaId("error_${System.currentTimeMillis()}")
            .setTitle(title)
            .setSubtitle(detail)
            .build()

        return MediaBrowserCompat.MediaItem(
            description,
            MediaBrowserCompat.MediaItem.FLAG_BROWSABLE
        )
    }
}
