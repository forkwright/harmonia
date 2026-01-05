// Android Auto media browser service
package app.akroasis.auto

import android.os.Bundle
import android.support.v4.media.MediaBrowserCompat
import android.support.v4.media.MediaDescriptionCompat
import androidx.media.MediaBrowserServiceCompat
import app.akroasis.audio.MediaSessionManager
import app.akroasis.data.model.Track
import app.akroasis.data.repository.MusicRepository
import dagger.hilt.android.AndroidEntryPoint
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import javax.inject.Inject

@AndroidEntryPoint
class AndroidAutoService : MediaBrowserServiceCompat() {

    @Inject
    lateinit var mediaSessionManager: MediaSessionManager

    @Inject
    lateinit var musicRepository: MusicRepository

    private val serviceScope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    companion object {
        private const val MEDIA_ROOT_ID = "akroasis_root"
        private const val RECENT_ROOT_ID = "recent"
        private const val ALBUMS_ROOT_ID = "albums"
        private const val ARTISTS_ROOT_ID = "artists"
        private const val PLAYLISTS_ROOT_ID = "playlists"
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
            val mediaItems = when (parentId) {
                MEDIA_ROOT_ID -> getRootItems()
                RECENT_ROOT_ID -> getRecentTracks()
                ALBUMS_ROOT_ID -> getAlbums()
                ARTISTS_ROOT_ID -> getArtists()
                PLAYLISTS_ROOT_ID -> getPlaylists()
                else -> {
                    if (parentId.startsWith("album_")) {
                        getAlbumTracks(parentId.removePrefix("album_"))
                    } else if (parentId.startsWith("artist_")) {
                        getArtistTracks(parentId.removePrefix("artist_"))
                    } else if (parentId.startsWith("playlist_")) {
                        getPlaylistTracks(parentId.removePrefix("playlist_"))
                    } else {
                        emptyList()
                    }
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
            emptyList()
        }
    }

    private suspend fun getAlbums(): List<MediaBrowserCompat.MediaItem> {
        return try {
            val albums = musicRepository.getAllAlbums()
            albums.map { album ->
                createBrowsableItem(
                    "album_${album.id}",
                    album.title,
                    album.artist
                )
            }
        } catch (e: Exception) {
            emptyList()
        }
    }

    private suspend fun getArtists(): List<MediaBrowserCompat.MediaItem> {
        return try {
            val artists = musicRepository.getAllArtists()
            artists.map { artist ->
                createBrowsableItem(
                    "artist_${artist.id}",
                    artist.name,
                    "${artist.albumCount} albums"
                )
            }
        } catch (e: Exception) {
            emptyList()
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
            emptyList()
        }
    }

    private suspend fun getAlbumTracks(albumId: String): List<MediaBrowserCompat.MediaItem> {
        return try {
            val tracks = musicRepository.getAlbumTracks(albumId)
            tracks.map { track -> createPlayableItem(track) }
        } catch (e: Exception) {
            emptyList()
        }
    }

    private suspend fun getArtistTracks(artistId: String): List<MediaBrowserCompat.MediaItem> {
        return try {
            val tracks = musicRepository.getArtistTracks(artistId)
            tracks.map { track -> createPlayableItem(track) }
        } catch (e: Exception) {
            emptyList()
        }
    }

    private suspend fun getPlaylistTracks(playlistId: String): List<MediaBrowserCompat.MediaItem> {
        return try {
            val tracks = musicRepository.getPlaylistTracks(playlistId)
            tracks.map { track -> createPlayableItem(track) }
        } catch (e: Exception) {
            emptyList()
        }
    }

    private fun createPlayableItem(track: Track): MediaBrowserCompat.MediaItem {
        val description = MediaDescriptionCompat.Builder()
            .setMediaId(track.id)
            .setTitle(track.title)
            .setSubtitle(track.artist)
            .setDescription(track.album)
            .build()

        return MediaBrowserCompat.MediaItem(
            description,
            MediaBrowserCompat.MediaItem.FLAG_PLAYABLE
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

    override fun onDestroy() {
        super.onDestroy()
        serviceScope.cancel()
    }
}
