package app.akroasis.audio

import android.os.Bundle
import app.akroasis.data.model.Track
import app.akroasis.data.repository.MusicRepository
import app.akroasis.data.repository.SearchRepository
import timber.log.Timber
import javax.inject.Inject

sealed class VoiceSearchResult {
    data class Success(val tracks: List<Track>, val startIndex: Int = 0) : VoiceSearchResult()
    data class NoResults(val query: String) : VoiceSearchResult()
    data class Error(val message: String) : VoiceSearchResult()
}

class VoiceSearchHandler @Inject constructor(
    private val searchRepository: SearchRepository,
    private val musicRepository: MusicRepository
) {
    companion object {
        private const val EXTRA_MEDIA_ARTIST = "android.intent.extra.artist"
        private const val EXTRA_MEDIA_ALBUM = "android.intent.extra.album"
        private const val EXTRA_MEDIA_TITLE = "android.intent.extra.title"
        private const val EXTRA_MEDIA_FOCUS = "android.intent.extra.focus"

        private const val MAX_QUEUE_SIZE = 50
        private const val SEARCH_TIMEOUT_MS = 5000L
    }

    suspend fun handleVoiceSearch(query: String?, extras: Bundle?): VoiceSearchResult {
        Timber.d("Voice search: query='$query', extras=${extras?.keySet()?.joinToString()}")

        return try {
            when {
                // Priority 1: Structured extras (most specific)
                extras != null && hasStructuredData(extras) -> {
                    handleStructuredSearch(extras)
                }
                // Priority 2: Free-form query
                !query.isNullOrBlank() -> {
                    handleFreeFormSearch(query)
                }
                // Priority 3: Empty query fallback
                else -> {
                    handleEmptyQuery()
                }
            }
        } catch (e: Exception) {
            Timber.e(e, "Voice search error")
            VoiceSearchResult.Error(e.message ?: "Search failed")
        }
    }

    private fun hasStructuredData(extras: Bundle): Boolean {
        return extras.containsKey(EXTRA_MEDIA_TITLE) ||
               extras.containsKey(EXTRA_MEDIA_ALBUM) ||
               extras.containsKey(EXTRA_MEDIA_ARTIST)
    }

    private suspend fun handleStructuredSearch(extras: Bundle): VoiceSearchResult {
        val title = extras.getString(EXTRA_MEDIA_TITLE)
        val album = extras.getString(EXTRA_MEDIA_ALBUM)
        val artist = extras.getString(EXTRA_MEDIA_ARTIST)

        Timber.d("Structured search: title='$title', album='$album', artist='$artist'")

        return when {
            // "Play [title]" or "Play [title] by [artist]"
            !title.isNullOrBlank() -> {
                searchByTitle(title, artist, album)
            }
            // "Play [album] by [artist]"
            !album.isNullOrBlank() && !artist.isNullOrBlank() -> {
                searchByAlbumAndArtist(album, artist)
            }
            // "Play [album]"
            !album.isNullOrBlank() -> {
                searchByAlbum(album)
            }
            // "Play [artist]"
            !artist.isNullOrBlank() -> {
                searchByArtist(artist)
            }
            else -> {
                VoiceSearchResult.NoResults("No search criteria provided")
            }
        }
    }

    private suspend fun searchByTitle(title: String, artist: String?, album: String?): VoiceSearchResult {
        val searchQuery = buildString {
            append(title)
            if (!artist.isNullOrBlank()) append(" $artist")
            if (!album.isNullOrBlank()) append(" $album")
        }

        val results = searchRepository.search(searchQuery, limit = MAX_QUEUE_SIZE).getOrElse {
            Timber.e(it, "Search failed")
            return VoiceSearchResult.Error(it.message ?: "Search failed")
        }

        if (results.isEmpty()) {
            return VoiceSearchResult.NoResults(searchQuery)
        }

        // Convert SearchResults to Tracks
        val tracks = results.mapNotNull { result ->
            musicRepository.getTrack(result.trackId.toString()).getOrNull()
        }

        if (tracks.isEmpty()) {
            return VoiceSearchResult.NoResults(searchQuery)
        }

        Timber.d("Found ${tracks.size} tracks for title '$title'")
        return VoiceSearchResult.Success(tracks)
    }

    private suspend fun searchByAlbumAndArtist(album: String, artist: String): VoiceSearchResult {
        // Try to get specific album tracks
        val searchQuery = "$album $artist"
        val results = searchRepository.search(searchQuery, limit = MAX_QUEUE_SIZE).getOrElse {
            Timber.e(it, "Search failed")
            return VoiceSearchResult.Error(it.message ?: "Search failed")
        }

        if (results.isEmpty()) {
            return VoiceSearchResult.NoResults(searchQuery)
        }

        // Get full track objects
        val tracks = results.mapNotNull { result ->
            musicRepository.getTrack(result.trackId.toString()).getOrNull()
        }

        if (tracks.isEmpty()) {
            return VoiceSearchResult.NoResults(searchQuery)
        }

        Timber.d("Found ${tracks.size} tracks for album '$album' by '$artist'")
        return VoiceSearchResult.Success(tracks)
    }

    private suspend fun searchByAlbum(album: String): VoiceSearchResult {
        val results = searchRepository.search(album, limit = MAX_QUEUE_SIZE).getOrElse {
            Timber.e(it, "Search failed")
            return VoiceSearchResult.Error(it.message ?: "Search failed")
        }

        if (results.isEmpty()) {
            return VoiceSearchResult.NoResults(album)
        }

        val tracks = results.mapNotNull { result ->
            musicRepository.getTrack(result.trackId.toString()).getOrNull()
        }

        if (tracks.isEmpty()) {
            return VoiceSearchResult.NoResults(album)
        }

        Timber.d("Found ${tracks.size} tracks for album '$album'")
        return VoiceSearchResult.Success(tracks)
    }

    private suspend fun searchByArtist(artist: String): VoiceSearchResult {
        val results = searchRepository.search(artist, limit = MAX_QUEUE_SIZE).getOrElse {
            Timber.e(it, "Search failed")
            return VoiceSearchResult.Error(it.message ?: "Search failed")
        }

        if (results.isEmpty()) {
            return VoiceSearchResult.NoResults(artist)
        }

        val tracks = results.mapNotNull { result ->
            musicRepository.getTrack(result.trackId.toString()).getOrNull()
        }

        if (tracks.isEmpty()) {
            return VoiceSearchResult.NoResults(artist)
        }

        Timber.d("Found ${tracks.size} tracks for artist '$artist'")
        return VoiceSearchResult.Success(tracks)
    }

    private suspend fun handleFreeFormSearch(query: String): VoiceSearchResult {
        Timber.d("Free-form search: '$query'")

        val results = searchRepository.search(query, limit = MAX_QUEUE_SIZE).getOrElse {
            Timber.e(it, "Search failed")
            return VoiceSearchResult.Error(it.message ?: "Search failed")
        }

        if (results.isEmpty()) {
            return VoiceSearchResult.NoResults(query)
        }

        val tracks = results.mapNotNull { result ->
            musicRepository.getTrack(result.trackId.toString()).getOrNull()
        }

        if (tracks.isEmpty()) {
            return VoiceSearchResult.NoResults(query)
        }

        Timber.d("Free-form search found ${tracks.size} tracks")
        return VoiceSearchResult.Success(tracks)
    }

    private suspend fun handleEmptyQuery(): VoiceSearchResult {
        Timber.d("Empty query - playing recent tracks")

        // Get recently added or played tracks as fallback
        val recentTracks = musicRepository.getRecentTracks(limit = 20)

        if (recentTracks.isEmpty()) {
            return VoiceSearchResult.NoResults("No recent tracks available")
        }

        Timber.d("Playing ${recentTracks.size} recent tracks")
        return VoiceSearchResult.Success(recentTracks)
    }
}
