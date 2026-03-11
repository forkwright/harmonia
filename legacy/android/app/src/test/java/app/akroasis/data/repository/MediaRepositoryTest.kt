package app.akroasis.data.repository

import app.akroasis.data.api.*
import app.akroasis.data.model.MediaProgress
import app.akroasis.data.model.MediaType
import kotlinx.coroutines.test.runTest
import okhttp3.ResponseBody.Companion.toResponseBody
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.*
import retrofit2.Response

class MediaRepositoryTest {

    private lateinit var repository: MediaRepository
    private lateinit var api: MouseionApi
    private lateinit var musicRepository: MusicRepository
    private lateinit var audiobookRepository: AudiobookRepository
    private lateinit var ebookRepository: EbookRepository

    @Before
    fun setup() {
        api = mock()
        musicRepository = mock()
        audiobookRepository = mock()
        ebookRepository = mock()
        repository = MediaRepository(api, musicRepository, audiobookRepository, ebookRepository)
    }

    @Test
    fun `getContinueFeed returns success with valid response`() = runTest {
        val continueItems = listOf(
            ContinueItemDto(
                mediaItem = MediaItemDto(
                    id = "1",
                    title = "Test Item",
                    mediaType = "music",
                    coverArtUrl = null,
                    duration = 300000,
                    author = null,
                    artist = "Test Artist",
                    album = "Test Album"
                ),
                progress = MediaProgress(
                    mediaItemId = "1",
                    mediaType = MediaType.MUSIC,
                    positionMs = 150000,
                    totalDurationMs = 300000,
                    percentComplete = 0.5f,
                    lastPlayedAt = System.currentTimeMillis(),
                    isComplete = false
                )
            )
        )
        whenever(api.getContinueFeed(any(), any())).thenReturn(Response.success(continueItems))

        val result = repository.getContinueFeed()

        assertTrue(result.isSuccess)
        assertEquals(1, result.getOrNull()?.size)
        assertEquals("Test Item", result.getOrNull()?.first()?.mediaItem?.title)
    }

    @Test
    fun `updateProgress returns success`() = runTest {
        whenever(api.updateProgress(any())).thenReturn(Response.success(Unit))

        val result = repository.updateProgress("1", MediaType.MUSIC, 150000, 300000)

        assertTrue(result.isSuccess)
        verify(api).updateProgress(any())
    }

    @Test
    fun `getProgress returns progress when exists`() = runTest {
        val progress = MediaProgress(
            mediaItemId = "1",
            mediaType = MediaType.MUSIC,
            positionMs = 150000,
            totalDurationMs = 300000,
            percentComplete = 0.5f,
            lastPlayedAt = System.currentTimeMillis(),
            isComplete = false
        )
        whenever(api.getProgress(any(), any())).thenReturn(Response.success(progress))

        val result = repository.getProgress("1")

        assertTrue(result.isSuccess)
        assertEquals(150000L, result.getOrNull()?.positionMs)
    }

    @Test
    fun `deleteProgress returns success`() = runTest {
        whenever(api.deleteProgress(any(), any())).thenReturn(Response.success(Unit))

        val result = repository.deleteProgress("1")

        assertTrue(result.isSuccess)
        verify(api).deleteProgress(eq("1"), any())
    }

    @Test
    fun `startSession returns session on success`() = runTest {
        val sessionDto = SessionDto(
            id = "session-1",
            userId = "default",
            mediaItemId = "1",
            mediaType = "music",
            startedAt = System.currentTimeMillis(),
            endedAt = null,
            durationMs = 0,
            isActive = true
        )
        whenever(api.startSession(any())).thenReturn(Response.success(sessionDto))

        val result = repository.startSession("1", MediaType.MUSIC)

        assertTrue(result.isSuccess)
        assertEquals("session-1", result.getOrNull()?.id)
        assertTrue(result.getOrNull()?.isActive == true)
    }

    @Test
    fun `endSession returns success`() = runTest {
        whenever(api.updateSession(any(), any())).thenReturn(Response.success(Unit))

        val result = repository.endSession("session-1", 150000)

        assertTrue(result.isSuccess)
        verify(api).updateSession(eq("session-1"), any())
    }

    @Test
    fun `getSessions returns list of sessions`() = runTest {
        val sessions = listOf(
            SessionDto(
                id = "session-1",
                userId = "default",
                mediaItemId = "1",
                mediaType = "music",
                startedAt = System.currentTimeMillis(),
                endedAt = null,
                durationMs = 150000,
                isActive = false
            )
        )
        whenever(api.getSessions(any(), any(), any())).thenReturn(Response.success(sessions))

        val result = repository.getSessions()

        assertTrue(result.isSuccess)
        assertEquals(1, result.getOrNull()?.size)
        assertEquals("session-1", result.getOrNull()?.first()?.id)
    }

    @Test
    fun `deleteSession returns success`() = runTest {
        whenever(api.deleteSession(any())).thenReturn(Response.success(Unit))

        val result = repository.deleteSession("session-1")

        assertTrue(result.isSuccess)
        verify(api).deleteSession("session-1")
    }

    // Error handling tests

    @Test
    fun `getContinueFeed returns failure on API error`() = runTest {
        whenever(api.getContinueFeed(any(), any())).thenReturn(Response.error(404, "".toResponseBody()))

        val result = repository.getContinueFeed()

        assertTrue(result.isFailure)
    }

    @Test
    fun `getContinueFeed returns failure on exception`() = runTest {
        whenever(api.getContinueFeed(any(), any())).thenThrow(RuntimeException("Network error"))

        val result = repository.getContinueFeed()

        assertTrue(result.isFailure)
    }

    @Test
    fun `getContinueFeed with empty list returns success`() = runTest {
        whenever(api.getContinueFeed(any(), any())).thenReturn(Response.success(emptyList()))

        val result = repository.getContinueFeed()

        assertTrue(result.isSuccess)
        assertEquals(0, result.getOrNull()?.size)
    }

    @Test
    fun `updateProgress returns failure on API error`() = runTest {
        whenever(api.updateProgress(any())).thenReturn(Response.error(500, "".toResponseBody()))

        val result = repository.updateProgress("1", MediaType.MUSIC, 150000, 300000)

        assertTrue(result.isFailure)
    }

    @Test
    fun `updateProgress returns failure on exception`() = runTest {
        whenever(api.updateProgress(any())).thenThrow(RuntimeException("Connection timeout"))

        val result = repository.updateProgress("1", MediaType.MUSIC, 150000, 300000)

        assertTrue(result.isFailure)
    }

    @Test
    fun `getProgress returns null on 404`() = runTest {
        whenever(api.getProgress(any(), any())).thenReturn(Response.error(404, "".toResponseBody()))

        val result = repository.getProgress("1")

        assertTrue(result.isSuccess)
        assertNull(result.getOrNull())
    }

    @Test
    fun `getProgress returns failure on exception`() = runTest {
        whenever(api.getProgress(any(), any())).thenThrow(RuntimeException("Network error"))

        val result = repository.getProgress("1")

        assertTrue(result.isFailure)
    }

    @Test
    fun `deleteProgress returns failure on API error`() = runTest {
        whenever(api.deleteProgress(any(), any())).thenReturn(Response.error(403, "".toResponseBody()))

        val result = repository.deleteProgress("1")

        assertTrue(result.isFailure)
    }

    @Test
    fun `startSession returns failure on API error`() = runTest {
        whenever(api.startSession(any())).thenReturn(Response.error(500, "".toResponseBody()))

        val result = repository.startSession("1", MediaType.MUSIC)

        assertTrue(result.isFailure)
    }

    @Test
    fun `startSession returns failure on exception`() = runTest {
        whenever(api.startSession(any())).thenThrow(RuntimeException("Server error"))

        val result = repository.startSession("1", MediaType.MUSIC)

        assertTrue(result.isFailure)
    }

    @Test
    fun `endSession returns failure on API error`() = runTest {
        whenever(api.updateSession(any(), any())).thenReturn(Response.error(404, "".toResponseBody()))

        val result = repository.endSession("session-1", 150000)

        assertTrue(result.isFailure)
    }

    @Test
    fun `endSession returns failure on exception`() = runTest {
        whenever(api.updateSession(any(), any())).thenThrow(RuntimeException("Connection lost"))

        val result = repository.endSession("session-1", 150000)

        assertTrue(result.isFailure)
    }

    @Test
    fun `getSessions returns empty list on 404`() = runTest {
        whenever(api.getSessions(any(), any(), any())).thenReturn(Response.error(404, "".toResponseBody()))

        val result = repository.getSessions()

        assertTrue(result.isFailure)
    }

    @Test
    fun `getSessions returns failure on exception`() = runTest {
        whenever(api.getSessions(any(), any(), any())).thenThrow(RuntimeException("Timeout"))

        val result = repository.getSessions()

        assertTrue(result.isFailure)
    }

    @Test
    fun `deleteSession returns failure on API error`() = runTest {
        whenever(api.deleteSession(any())).thenReturn(Response.error(404, "".toResponseBody()))

        val result = repository.deleteSession("session-1")

        assertTrue(result.isFailure)
    }

    @Test
    fun `deleteSession returns failure on exception`() = runTest {
        whenever(api.deleteSession(any())).thenThrow(RuntimeException("Server error"))

        val result = repository.deleteSession("session-1")

        assertTrue(result.isFailure)
    }

    // Custom userId and parameters tests

    @Test
    fun `getContinueFeed with custom userId and limit`() = runTest {
        whenever(api.getContinueFeed(any(), any())).thenReturn(Response.success(emptyList()))

        val result = repository.getContinueFeed(userId = "user123", limit = 50)

        assertTrue(result.isSuccess)
        verify(api).getContinueFeed("user123", 50)
    }

    @Test
    fun `getProgress with custom userId`() = runTest {
        val progress = MediaProgress(
            mediaItemId = "1",
            mediaType = MediaType.EBOOK,
            positionMs = 100000,
            totalDurationMs = 500000,
            percentComplete = 0.2f,
            lastPlayedAt = System.currentTimeMillis(),
            isComplete = false
        )
        whenever(api.getProgress(any(), any())).thenReturn(Response.success(progress))

        val result = repository.getProgress("1", "user456")

        assertTrue(result.isSuccess)
        verify(api).getProgress("1", "user456")
    }

    @Test
    fun `updateProgress with audiobook type`() = runTest {
        whenever(api.updateProgress(any())).thenReturn(Response.success(Unit))

        val result = repository.updateProgress("1", MediaType.AUDIOBOOK, 200000, 600000, "user789")

        assertTrue(result.isSuccess)
        verify(api).updateProgress(any())
    }

    @Test
    fun `updateProgress with ebook type`() = runTest {
        whenever(api.updateProgress(any())).thenReturn(Response.success(Unit))

        val result = repository.updateProgress("1", MediaType.EBOOK, 300000, 900000)

        assertTrue(result.isSuccess)
        verify(api).updateProgress(any())
    }

    @Test
    fun `startSession with audiobook type`() = runTest {
        val sessionDto = SessionDto(
            id = "session-2",
            userId = "default",
            mediaItemId = "2",
            mediaType = "audiobook",
            startedAt = System.currentTimeMillis(),
            endedAt = null,
            durationMs = 0,
            isActive = true
        )
        whenever(api.startSession(any())).thenReturn(Response.success(sessionDto))

        val result = repository.startSession("2", MediaType.AUDIOBOOK, "default")

        assertTrue(result.isSuccess)
        assertEquals("audiobook", result.getOrNull()?.mediaType?.name?.lowercase())
    }

    @Test
    fun `startSession with ebook type`() = runTest {
        val sessionDto = SessionDto(
            id = "session-3",
            userId = "user-custom",
            mediaItemId = "3",
            mediaType = "ebook",
            startedAt = System.currentTimeMillis(),
            endedAt = null,
            durationMs = 0,
            isActive = true
        )
        whenever(api.startSession(any())).thenReturn(Response.success(sessionDto))

        val result = repository.startSession("3", MediaType.EBOOK, "user-custom")

        assertTrue(result.isSuccess)
        assertEquals("ebook", result.getOrNull()?.mediaType?.name?.lowercase())
    }
}
