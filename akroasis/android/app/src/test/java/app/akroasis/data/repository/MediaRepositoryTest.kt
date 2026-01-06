package app.akroasis.data.repository

import app.akroasis.data.api.*
import app.akroasis.data.model.MediaProgress
import app.akroasis.data.model.MediaType
import kotlinx.coroutines.test.runTest
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
}
