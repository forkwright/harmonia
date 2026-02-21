// Unit tests for AndroidAutoService browse hierarchy, search, artwork, and error handling
package app.akroasis.auto

import android.net.Uri
import android.os.Bundle
import app.akroasis.audio.VoiceSearchHandler
import app.akroasis.audio.VoiceSearchResult
import app.akroasis.data.model.Album
import app.akroasis.data.model.Artist
import app.akroasis.data.model.FilterResponse
import app.akroasis.data.model.LibraryFacets
import app.akroasis.data.model.DynamicRangeRange
import app.akroasis.data.model.Track
import app.akroasis.data.repository.FilterRepository
import app.akroasis.data.repository.MusicRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.mockito.kotlin.*
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
@OptIn(ExperimentalCoroutinesApi::class)
class AndroidAutoServiceTest {

    private lateinit var service: TestableAndroidAutoService
    private lateinit var mockMusicRepository: MusicRepository
    private lateinit var mockFilterRepository: FilterRepository
    private lateinit var mockVoiceSearchHandler: VoiceSearchHandler

    private val testDispatcher = StandardTestDispatcher()

    private val testTrack = Track(
        id = "42",
        title = "National Anthem",
        artist = "Radiohead",
        album = "Kid A",
        albumArtist = "Radiohead",
        trackNumber = 5,
        discNumber = 1,
        year = 2000,
        duration = 369000,
        bitrate = 1411,
        sampleRate = 44100,
        bitDepth = 16,
        format = "FLAC",
        fileSize = 25000000,
        filePath = "/music/radiohead/kid_a/05_national_anthem.flac",
        coverArtUrl = "https://mouseion.local/art/kid_a.jpg",
        replayGainTrackGain = null,
        replayGainAlbumGain = null,
        createdAt = "2024-01-01T00:00:00Z",
        updatedAt = "2024-01-01T00:00:00Z"
    )

    private val testTrackNoCover = testTrack.copy(id = "43", coverArtUrl = null)

    private val testAlbum = Album(
        id = "7",
        title = "Kid A",
        artist = "Radiohead",
        albumArtist = "Radiohead",
        year = 2000,
        trackCount = 10,
        coverArtUrl = "https://mouseion.local/art/kid_a.jpg",
        createdAt = "2024-01-01T00:00:00Z",
        updatedAt = "2024-01-01T00:00:00Z"
    )

    private val testAlbumNoCover = testAlbum.copy(id = "8", coverArtUrl = null)

    private val testArtist = Artist(
        id = "3",
        name = "Radiohead",
        albumCount = 9,
        trackCount = 120,
        imageUrl = "https://mouseion.local/art/radiohead.jpg",
        createdAt = "2024-01-01T00:00:00Z",
        updatedAt = "2024-01-01T00:00:00Z"
    )

    private val testArtistNoImage = testArtist.copy(id = "4", imageUrl = null)

    private val testFacets = LibraryFacets(
        formats = listOf("FLAC", "WAV"),
        sampleRates = listOf(44100, 96000),
        bitDepths = listOf(16, 24),
        genres = listOf("Electronic", "Rock", "Jazz"),
        years = listOf(1990, 2000, 2010),
        dynamicRangeRange = DynamicRangeRange(min = 5, max = 20),
        codecList = listOf("FLAC", "PCM")
    )

    @Before
    fun setup() {
        Dispatchers.setMain(testDispatcher)
        mockMusicRepository = mock()
        mockFilterRepository = mock()
        mockVoiceSearchHandler = mock()

        service = TestableAndroidAutoService(
            musicRepository = mockMusicRepository,
            filterRepository = mockFilterRepository,
            voiceSearchHandler = mockVoiceSearchHandler
        )
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    // --- Root hierarchy ---

    @Test
    fun `root items include genres category`() = runTest {
        val items = service.getRootItemsPublic()

        val ids = items.map { it.mediaId }
        assertTrue(ids.contains("genres"))
    }

    @Test
    fun `root items contain all five categories`() = runTest {
        val items = service.getRootItemsPublic()

        assertEquals(5, items.size)
        val ids = items.map { it.mediaId }
        assertTrue(ids.contains("recent"))
        assertTrue(ids.contains("albums"))
        assertTrue(ids.contains("artists"))
        assertTrue(ids.contains("genres"))
        assertTrue(ids.contains("playlists"))
    }

    @Test
    fun `all root items are browsable`() = runTest {
        val items = service.getRootItemsPublic()

        items.forEach { item ->
            assertTrue(
                "Expected ${item.mediaId} to be browsable",
                item.isBrowsable
            )
        }
    }

    // --- Artwork ---

    @Test
    fun `playable item includes iconUri when coverArtUrl present`() = runTest {
        val item = service.createPlayableItemPublic(testTrack)

        val iconUri = item.description.iconUri
        assertNotNull(iconUri)
        assertEquals(Uri.parse("https://mouseion.local/art/kid_a.jpg"), iconUri)
    }

    @Test
    fun `playable item has no iconUri when coverArtUrl is null`() = runTest {
        val item = service.createPlayableItemPublic(testTrackNoCover)

        assertNull(item.description.iconUri)
    }

    @Test
    fun `album browsable item includes iconUri when coverArtUrl present`() = runTest {
        val item = service.createBrowsableAlbumItemPublic(testAlbum)

        val iconUri = item.description.iconUri
        assertNotNull(iconUri)
        assertEquals(Uri.parse("https://mouseion.local/art/kid_a.jpg"), iconUri)
    }

    @Test
    fun `album browsable item has no iconUri when coverArtUrl is null`() = runTest {
        val item = service.createBrowsableAlbumItemPublic(testAlbumNoCover)

        assertNull(item.description.iconUri)
    }

    @Test
    fun `artist browsable item includes iconUri when imageUrl present`() = runTest {
        val item = service.createBrowsableArtistItemPublic(testArtist)

        val iconUri = item.description.iconUri
        assertNotNull(iconUri)
        assertEquals(Uri.parse("https://mouseion.local/art/radiohead.jpg"), iconUri)
    }

    @Test
    fun `artist browsable item has no iconUri when imageUrl is null`() = runTest {
        val item = service.createBrowsableArtistItemPublic(testArtistNoImage)

        assertNull(item.description.iconUri)
    }

    // --- Genres ---

    @Test
    fun `genres list returns sorted genre items`() = runTest {
        whenever(mockFilterRepository.getLibraryFacets()).thenReturn(
            Result.success(testFacets)
        )

        val items = service.getGenresPublic()

        assertEquals(3, items.size)
        // Sorted alphabetically
        assertEquals("Electronic", items[0].description.title)
        assertEquals("Jazz", items[1].description.title)
        assertEquals("Rock", items[2].description.title)
    }

    @Test
    fun `genre items are browsable with genre_ prefix`() = runTest {
        whenever(mockFilterRepository.getLibraryFacets()).thenReturn(
            Result.success(testFacets)
        )

        val items = service.getGenresPublic()

        assertTrue(items.all { it.isBrowsable })
        assertEquals("genre_Electronic", items[0].mediaId)
    }

    @Test
    fun `genre tracks returned as playable items`() = runTest {
        val filterResponse = FilterResponse(
            tracks = listOf(testTrack),
            totalCount = 1,
            page = 1,
            pageSize = 100,
            summary = null
        )
        whenever(mockFilterRepository.filterLibrary(any())).thenReturn(
            Result.success(filterResponse)
        )

        val items = service.getGenreTracksPublic("Electronic")

        assertEquals(1, items.size)
        assertTrue(items[0].isPlayable)
        assertEquals("42", items[0].mediaId)
    }

    @Test
    fun `genre filter request targets correct genre`() = runTest {
        val filterResponse = FilterResponse(
            tracks = emptyList(),
            totalCount = 0,
            page = 1,
            pageSize = 100,
            summary = null
        )
        whenever(mockFilterRepository.filterLibrary(any())).thenReturn(
            Result.success(filterResponse)
        )

        service.getGenreTracksPublic("Jazz")

        val captor = argumentCaptor<app.akroasis.data.model.FilterRequest>()
        verify(mockFilterRepository).filterLibrary(captor.capture())

        val request = captor.firstValue
        assertEquals(1, request.conditions.size)
        assertEquals(app.akroasis.data.model.FilterField.GENRE, request.conditions[0].field)
        assertEquals("Jazz", request.conditions[0].value)
    }

    // --- Search ---

    @Test
    fun `search delegates to VoiceSearchHandler with query and extras`() = runTest {
        val extras = Bundle()
        whenever(mockVoiceSearchHandler.handleVoiceSearch(any(), any())).thenReturn(
            VoiceSearchResult.Success(listOf(testTrack))
        )

        service.handleSearchPublic("radiohead", extras)

        verify(mockVoiceSearchHandler).handleVoiceSearch("radiohead", extras)
    }

    @Test
    fun `search success returns playable items`() = runTest {
        whenever(mockVoiceSearchHandler.handleVoiceSearch(any(), anyOrNull())).thenReturn(
            VoiceSearchResult.Success(listOf(testTrack))
        )

        val items = service.handleSearchPublic("radiohead", null)

        assertEquals(1, items.size)
        assertTrue(items[0].isPlayable)
        assertEquals("42", items[0].mediaId)
    }

    @Test
    fun `search no-results returns error item`() = runTest {
        whenever(mockVoiceSearchHandler.handleVoiceSearch(any(), anyOrNull())).thenReturn(
            VoiceSearchResult.NoResults("unknown artist")
        )

        val items = service.handleSearchPublic("unknown artist", null)

        assertEquals(1, items.size)
        assertEquals("No results", items[0].description.title.toString())
    }

    @Test
    fun `search error returns error item with message`() = runTest {
        whenever(mockVoiceSearchHandler.handleVoiceSearch(any(), anyOrNull())).thenReturn(
            VoiceSearchResult.Error("Network timeout")
        )

        val items = service.handleSearchPublic("test", null)

        assertEquals(1, items.size)
        assertEquals("Search unavailable", items[0].description.title.toString())
        assertEquals("Network timeout", items[0].description.subtitle.toString())
    }

    // --- Error handling ---

    @Test
    fun `recent tracks failure returns error item`() = runTest {
        whenever(mockMusicRepository.getRecentTracks(any())).thenThrow(RuntimeException("API down"))

        val items = service.getRecentTracksPublic()

        assertEquals(1, items.size)
        assertEquals("Unable to load", items[0].description.title.toString())
    }

    @Test
    fun `albums failure returns error item`() = runTest {
        whenever(mockMusicRepository.getAllAlbums()).thenThrow(RuntimeException("API down"))

        val items = service.getAlbumsPublic()

        assertEquals(1, items.size)
        assertEquals("Unable to load", items[0].description.title.toString())
    }

    @Test
    fun `artists failure returns error item`() = runTest {
        whenever(mockMusicRepository.getAllArtists()).thenThrow(RuntimeException("API down"))

        val items = service.getArtistsPublic()

        assertEquals(1, items.size)
        assertEquals("Unable to load", items[0].description.title.toString())
    }

    @Test
    fun `genres failure returns error item`() = runTest {
        whenever(mockFilterRepository.getLibraryFacets()).thenReturn(
            Result.failure(RuntimeException("Facets unavailable"))
        )

        val items = service.getGenresPublic()

        assertEquals(1, items.size)
        assertEquals("Unable to load", items[0].description.title.toString())
    }

    @Test
    fun `album tracks failure returns error item`() = runTest {
        whenever(mockMusicRepository.getAlbumTracks(any())).thenThrow(RuntimeException("Not found"))

        val items = service.getAlbumTracksPublic("7")

        assertEquals(1, items.size)
        assertEquals("Unable to load", items[0].description.title.toString())
    }

    @Test
    fun `artist tracks failure returns error item`() = runTest {
        whenever(mockMusicRepository.getArtistTracks(any())).thenThrow(RuntimeException("Not found"))

        val items = service.getArtistTracksPublic("3")

        assertEquals(1, items.size)
        assertEquals("Unable to load", items[0].description.title.toString())
    }

    @Test
    fun `genre tracks failure returns error item`() = runTest {
        whenever(mockFilterRepository.filterLibrary(any())).thenReturn(
            Result.failure(RuntimeException("Filter failed"))
        )

        val items = service.getGenreTracksPublic("Electronic")

        assertEquals(1, items.size)
        assertEquals("Unable to load", items[0].description.title.toString())
    }

    @Test
    fun `error item is browsable not playable`() = runTest {
        whenever(mockMusicRepository.getAllAlbums()).thenThrow(RuntimeException("down"))

        val items = service.getAlbumsPublic()

        assertEquals(1, items.size)
        assertTrue(items[0].isBrowsable)
        assertFalse(items[0].isPlayable)
    }
}
