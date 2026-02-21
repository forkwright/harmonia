// Android Auto media browser service
package app.akroasis.auto

import android.net.Uri
import android.os.Bundle
import android.support.v4.media.MediaBrowserCompat
import android.support.v4.media.MediaDescriptionCompat
import androidx.media.MediaBrowserServiceCompat
import app.akroasis.audio.MediaSessionManager
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
import dagger.hilt.android.AndroidEntryPoint
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import timber.log.Timber
import javax.inject.Inject

@AndroidEntryPoint
class AndroidAutoService : MediaBrowserServiceCompat() {

    @Inject
    lateinit var mediaSessionManager: MediaSessionManager

    @Inject
    lateinit var musicRepository: MusicRepository

    @Inject
    lateinit var filterRepository: FilterRepository

    @Inject
    lateinit var voiceSearchHandler: VoiceSearchHandler

    private val serviceScope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    companion object {
        private const val MEDIA_ROOT_ID = "akroasis_root"
        private const val RECENT_ROOT_ID = "recent"
        private const val ALBUMS_ROOT_ID = "albums"
        private const val ARTISTS_ROOT_ID = "artists"
        private const val PLAYLISTS_ROOT_ID = "playlists"
        private const val GENRES_ROOT_ID = "genres"
    }

    override fun onCreate() {
        super.onCreate()

        val sessionToken = mediaSessionManager.getSessionToken()
        sessionToken?.let {
            setSessionToken(it)
        }
    }

    override fun onGetRoot(
        clientPackageName: String,
        clientUid: Int,
        rootHints: Bundle?
    ): BrowserRoot {
        return BrowserRoot(MEDIA_ROOT_ID, null)
    }

    override fun onLoadChildren(
        parentId: String,
        result: Result<MutableList<MediaBrowserCompat.MediaItem>>
    ) {
        result.detach()

        serviceScope.launch {
            val mediaItems = when {
                parentId == MEDIA_ROOT_ID -> getRootItems()
                parentId == RECENT_ROOT_ID -> getRecentTracks()
                parentId == ALBUMS_ROOT_ID -> getAlbums()
                parentId == ARTISTS_ROOT_ID -> getArtists()
                parentId == PLAYLISTS_ROOT_ID -> getPlaylists()
                parentId == GENRES_ROOT_ID -> getGenres()
                parentId.startsWith("album_") -> getAlbumTracks(parentId.removePrefix("album_"))
                parentId.startsWith("artist_") -> getArtistTracks(parentId.removePrefix("artist_"))
                parentId.startsWith("playlist_") -> getPlaylistTracks(parentId.removePrefix("playlist_"))
                parentId.startsWith("genre_") -> getGenreTracks(parentId.removePrefix("genre_"))
                else -> emptyList()
            }

            result.sendResult(mediaItems.toMutableList())
        }
    }

    override fun onSearch(
        query: String,
        extras: Bundle?,
        result: Result<MutableList<MediaBrowserCompat.MediaItem>>
    ) {
        result.detach()

        serviceScope.launch {
            val mediaItems = when (val searchResult = voiceSearchHandler.handleVoiceSearch(query, extras)) {
                is VoiceSearchResult.Success -> searchResult.tracks.map { createPlayableItem(it) }
                is VoiceSearchResult.NoResults -> listOf(
                    createErrorItem("No results", "Nothing matched \"${searchResult.query}\"")
                )
                is VoiceSearchResult.Error -> {
                    Timber.e("Auto search error: ${searchResult.message}")
                    listOf(createErrorItem("Search unavailable", searchResult.message))
                }
            }

            result.sendResult(mediaItems.toMutableList())
        }
    }

    private fun getRootItems(): List<MediaBrowserCompat.MediaItem> {
        return listOf(
            createBrowsableItem(
                RECENT_ROOT_ID,
                "Recently Played",
                "Your recent tracks"
            ),
            createBrowsableItem(
                ALBUMS_ROOT_ID,
                "Albums",
                "Browse by album"
            ),
            createBrowsableItem(
                ARTISTS_ROOT_ID,
                "Artists",
                "Browse by artist"
            ),
            createBrowsableItem(
                GENRES_ROOT_ID,
                "Genres",
                "Browse by genre"
            ),
            createBrowsableItem(
                PLAYLISTS_ROOT_ID,
                "Playlists",
                "Your playlists"
            )
        )
    }

    private suspend fun getRecentTracks(): List<MediaBrowserCompat.MediaItem> {
        return try {
            val tracks = musicRepository.getRecentTracks(limit = 20)
            tracks.map { track -> createPlayableItem(track) }
        } catch (e: Exception) {
            Timber.e(e, "Failed to load recent tracks")
            listOf(createErrorItem("Unable to load", "Recent tracks unavailable"))
        }
    }

    private suspend fun getAlbums(): List<MediaBrowserCompat.MediaItem> {
        return try {
            val albums = musicRepository.getAllAlbums()
            albums.map { album -> createBrowsableAlbumItem(album) }
        } catch (e: Exception) {
            Timber.e(e, "Failed to load albums")
            listOf(createErrorItem("Unable to load", "Albums unavailable"))
        }
    }

    private suspend fun getArtists(): List<MediaBrowserCompat.MediaItem> {
        return try {
            val artists = musicRepository.getAllArtists()
            artists.map { artist -> createBrowsableArtistItem(artist) }
        } catch (e: Exception) {
            Timber.e(e, "Failed to load artists")
            listOf(createErrorItem("Unable to load", "Artists unavailable"))
        }
    }

    private suspend fun getPlaylists(): List<MediaBrowserCompat.MediaItem> {
        return try {
            val playlists = musicRepository.getAllPlaylists()
            playlists.map { playlist ->
                createBrowsableItem(
                    "playlist_${playlist.id}",
                    playlist.name,
                    "${playlist.trackCount} tracks"
                )
            }
        } catch (e: Exception) {
            Timber.e(e, "Failed to load playlists")
            listOf(createErrorItem("Unable to load", "Playlists unavailable"))
        }
    }

    private suspend fun getGenres(): List<MediaBrowserCompat.MediaItem> {
        return try {
            val facets = filterRepository.getLibraryFacets().getOrThrow()
            facets.genres.sorted().map { genre ->
                createBrowsableItem(
                    "genre_${genre}",
                    genre,
                    "Browse $genre tracks"
                )
            }
        } catch (e: Exception) {
            Timber.e(e, "Failed to load genres")
            listOf(createErrorItem("Unable to load", "Genres unavailable"))
        }
    }

    private suspend fun getAlbumTracks(albumId: String): List<MediaBrowserCompat.MediaItem> {
        return try {
            val tracks = musicRepository.getAlbumTracks(albumId)
            tracks.map { track -> createPlayableItem(track) }
        } catch (e: Exception) {
            Timber.e(e, "Failed to load tracks for album $albumId")
            listOf(createErrorItem("Unable to load", "Album tracks unavailable"))
        }
    }

    private suspend fun getArtistTracks(artistId: String): List<MediaBrowserCompat.MediaItem> {
        return try {
            val tracks = musicRepository.getArtistTracks(artistId)
            tracks.map { track -> createPlayableItem(track) }
        } catch (e: Exception) {
            Timber.e(e, "Failed to load tracks for artist $artistId")
            listOf(createErrorItem("Unable to load", "Artist tracks unavailable"))
        }
    }

    private suspend fun getPlaylistTracks(playlistId: String): List<MediaBrowserCompat.MediaItem> {
        return try {
            val tracks = musicRepository.getPlaylistTracks(playlistId)
            tracks.map { track -> createPlayableItem(track) }
        } catch (e: Exception) {
            Timber.e(e, "Failed to load tracks for playlist $playlistId")
            listOf(createErrorItem("Unable to load", "Playlist tracks unavailable"))
        }
    }

    private suspend fun getGenreTracks(genre: String): List<MediaBrowserCompat.MediaItem> {
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
            response.tracks.map { track -> createPlayableItem(track) }
        } catch (e: Exception) {
            Timber.e(e, "Failed to load tracks for genre $genre")
            listOf(createErrorItem("Unable to load", "Genre tracks unavailable"))
        }
    }

    private fun createPlayableItem(track: Track): MediaBrowserCompat.MediaItem {
        val builder = MediaDescriptionCompat.Builder()
            .setMediaId(track.id)
            .setTitle(track.title)
            .setSubtitle(track.artist)
            .setDescription(track.album)

        // TODO: artwork URLs may require auth headers; load via Coil in the host app instead
        track.coverArtUrl?.let { builder.setIconUri(Uri.parse(it)) }

        return MediaBrowserCompat.MediaItem(
            builder.build(),
            MediaBrowserCompat.MediaItem.FLAG_PLAYABLE
        )
    }

    private fun createBrowsableAlbumItem(album: Album): MediaBrowserCompat.MediaItem {
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

    private fun createBrowsableArtistItem(artist: Artist): MediaBrowserCompat.MediaItem {
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

    override fun onDestroy() {
        super.onDestroy()
        serviceScope.cancel()
    }
}
