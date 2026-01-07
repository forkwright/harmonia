// Unit tests for VoiceSearchHandler voice command parsing
package app.akroasis.audio

import android.os.Bundle
import app.akroasis.data.model.SearchResult
import app.akroasis.data.model.Track
import app.akroasis.data.repository.MusicRepository
import app.akroasis.data.repository.SearchRepository
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.*

class VoiceSearchHandlerTest {

    private lateinit var handler: VoiceSearchHandler
    private lateinit var mockSearchRepository: SearchRepository
    private lateinit var mockMusicRepository: MusicRepository

    private val testTrack = Track(
        id = "1",
        title = "Paranoid Android",
        artist = "Radiohead",
        album = "OK Computer",
        albumArtist = "Radiohead",
        trackNumber = 2,
        discNumber = 1,
        year = 1997,
        duration = 384000,
        bitrate = 320000,
        sampleRate = 44100,
        bitDepth = 16,
        format = "FLAC",
        fileSize = 15000000,
        filePath = "/music/radiohead/ok_computer/02_paranoid_android.flac",
        coverArtUrl = null,
        replayGainTrackGain = null,
        replayGainAlbumGain = null,
        createdAt = "2024-01-01T00:00:00Z",
        updatedAt = "2024-01-01T00:00:00Z"
    )

    private val testSearchResult = SearchResult(
        trackId = 1,
        title = "Paranoid Android",
        artist = "Radiohead",
        album = "OK Computer",
        trackNumber = 2,
        discNumber = 1,
        durationSeconds = 384,
        genre = "Alternative Rock",
        bitDepth = 16,
        dynamicRange = 12,
        lossless = true,
        sampleRate = 44100,
        format = "FLAC",
        relevanceScore = 1.0
    )

    @Before
    fun setup() {
        mockSearchRepository = mock()
        mockMusicRepository = mock()
        handler = VoiceSearchHandler(mockSearchRepository, mockMusicRepository)
    }

    @Test
    fun `structured search with title returns tracks`() = runTest {
        val extras = Bundle().apply {
            putString("android.intent.extra.title", "Paranoid Android")
        }

        whenever(mockSearchRepository.search(any(), any())).thenReturn(
            Result.success(listOf(testSearchResult))
        )
        whenever(mockMusicRepository.getTrack("1")).thenReturn(
            Result.success(testTrack)
        )

        val result = handler.handleVoiceSearch(null, extras)

        assertTrue(result is VoiceSearchResult.Success)
        val success = result as VoiceSearchResult.Success
        assertEquals(1, success.tracks.size)
        assertEquals("Paranoid Android", success.tracks[0].title)
    }

    @Test
    fun `structured search with title and artist combines query`() = runTest {
        val extras = Bundle().apply {
            putString("android.intent.extra.title", "Paranoid Android")
            putString("android.intent.extra.artist", "Radiohead")
        }

        whenever(mockSearchRepository.search(eq("Paranoid Android Radiohead"), any())).thenReturn(
            Result.success(listOf(testSearchResult))
        )
        whenever(mockMusicRepository.getTrack("1")).thenReturn(
            Result.success(testTrack)
        )

        val result = handler.handleVoiceSearch(null, extras)

        assertTrue(result is VoiceSearchResult.Success)
        verify(mockSearchRepository).search(eq("Paranoid Android Radiohead"), any())
    }

    @Test
    fun `structured search with album and artist searches both`() = runTest {
        val extras = Bundle().apply {
            putString("android.intent.extra.album", "OK Computer")
            putString("android.intent.extra.artist", "Radiohead")
        }

        whenever(mockSearchRepository.search(eq("OK Computer Radiohead"), any())).thenReturn(
            Result.success(listOf(testSearchResult))
        )
        whenever(mockMusicRepository.getTrack("1")).thenReturn(
            Result.success(testTrack)
        )

        val result = handler.handleVoiceSearch(null, extras)

        assertTrue(result is VoiceSearchResult.Success)
    }

    @Test
    fun `structured search with only artist searches artist`() = runTest {
        val extras = Bundle().apply {
            putString("android.intent.extra.artist", "Radiohead")
        }

        whenever(mockSearchRepository.search(eq("Radiohead"), any())).thenReturn(
            Result.success(listOf(testSearchResult))
        )
        whenever(mockMusicRepository.getTrack("1")).thenReturn(
            Result.success(testTrack)
        )

        val result = handler.handleVoiceSearch(null, extras)

        assertTrue(result is VoiceSearchResult.Success)
    }

    @Test
    fun `structured search with only album searches album`() = runTest {
        val extras = Bundle().apply {
            putString("android.intent.extra.album", "OK Computer")
        }

        whenever(mockSearchRepository.search(eq("OK Computer"), any())).thenReturn(
            Result.success(listOf(testSearchResult))
        )
        whenever(mockMusicRepository.getTrack("1")).thenReturn(
            Result.success(testTrack)
        )

        val result = handler.handleVoiceSearch(null, extras)

        assertTrue(result is VoiceSearchResult.Success)
    }

    @Test
    fun `free-form query searches directly`() = runTest {
        whenever(mockSearchRepository.search(eq("radiohead paranoid"), any())).thenReturn(
            Result.success(listOf(testSearchResult))
        )
        whenever(mockMusicRepository.getTrack("1")).thenReturn(
            Result.success(testTrack)
        )

        val result = handler.handleVoiceSearch("radiohead paranoid", null)

        assertTrue(result is VoiceSearchResult.Success)
    }

    @Test
    fun `empty query returns recent tracks`() = runTest {
        whenever(mockMusicRepository.getRecentTracks(any())).thenReturn(
            listOf(testTrack)
        )

        val result = handler.handleVoiceSearch(null, null)

        assertTrue(result is VoiceSearchResult.Success)
        val success = result as VoiceSearchResult.Success
        assertEquals(1, success.tracks.size)
        verify(mockMusicRepository).getRecentTracks(20)
    }

    @Test
    fun `empty query with no recent tracks returns NoResults`() = runTest {
        whenever(mockMusicRepository.getRecentTracks(any())).thenReturn(emptyList())

        val result = handler.handleVoiceSearch(null, null)

        assertTrue(result is VoiceSearchResult.NoResults)
        val noResults = result as VoiceSearchResult.NoResults
        assertEquals("No recent tracks available", noResults.query)
    }

    @Test
    fun `search with no results returns NoResults`() = runTest {
        whenever(mockSearchRepository.search(any(), any())).thenReturn(
            Result.success(emptyList())
        )

        val result = handler.handleVoiceSearch("nonexistent artist", null)

        assertTrue(result is VoiceSearchResult.NoResults)
        val noResults = result as VoiceSearchResult.NoResults
        assertEquals("nonexistent artist", noResults.query)
    }

    @Test
    fun `search repository error returns Error result`() = runTest {
        whenever(mockSearchRepository.search(any(), any())).thenReturn(
            Result.failure(Exception("Network error"))
        )

        val result = handler.handleVoiceSearch("test query", null)

        assertTrue(result is VoiceSearchResult.Error)
        val error = result as VoiceSearchResult.Error
        assertTrue(error.message.contains("Network error"))
    }

    @Test
    fun `music repository error returns Error result`() = runTest {
        whenever(mockSearchRepository.search(any(), any())).thenReturn(
            Result.success(listOf(testSearchResult))
        )
        whenever(mockMusicRepository.getTrack(any())).thenReturn(
            Result.failure(Exception("Track load failed"))
        )

        val result = handler.handleVoiceSearch("test", null)

        // When all tracks fail to load, should return NoResults
        assertTrue(result is VoiceSearchResult.NoResults)
    }

    @Test
    fun `search limits results to MAX_QUEUE_SIZE`() = runTest {
        whenever(mockSearchRepository.search(any(), eq(50))).thenReturn(
            Result.success(listOf(testSearchResult))
        )
        whenever(mockMusicRepository.getTrack(any())).thenReturn(
            Result.success(testTrack)
        )

        handler.handleVoiceSearch("test", null)

        verify(mockSearchRepository).search(any(), eq(50))
    }

    @Test
    fun `multiple search results all converted to tracks`() = runTest {
        val searchResults = List(3) { index ->
            testSearchResult.copy(trackId = index + 1)
        }
        val tracks = List(3) { index ->
            testTrack.copy(id = (index + 1).toString())
        }

        whenever(mockSearchRepository.search(any(), any())).thenReturn(
            Result.success(searchResults)
        )
        tracks.forEach { track ->
            whenever(mockMusicRepository.getTrack(track.id)).thenReturn(
                Result.success(track)
            )
        }

        val result = handler.handleVoiceSearch("test", null)

        assertTrue(result is VoiceSearchResult.Success)
        val success = result as VoiceSearchResult.Success
        assertEquals(3, success.tracks.size)
    }

    @Test
    fun `partial track load returns available tracks`() = runTest {
        val searchResults = listOf(
            testSearchResult.copy(trackId = 1),
            testSearchResult.copy(trackId = 2)
        )

        whenever(mockSearchRepository.search(any(), any())).thenReturn(
            Result.success(searchResults)
        )
        whenever(mockMusicRepository.getTrack("1")).thenReturn(
            Result.success(testTrack.copy(id = "1"))
        )
        whenever(mockMusicRepository.getTrack("2")).thenReturn(
            Result.failure(Exception("Track 2 not found"))
        )

        val result = handler.handleVoiceSearch("test", null)

        assertTrue(result is VoiceSearchResult.Success)
        val success = result as VoiceSearchResult.Success
        assertEquals(1, success.tracks.size)
        assertEquals("1", success.tracks[0].id)
    }
}
