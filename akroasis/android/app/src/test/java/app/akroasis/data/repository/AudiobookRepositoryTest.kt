package app.akroasis.data.repository

import app.akroasis.data.api.AudiobookDto
import app.akroasis.data.api.MouseionApi
import app.akroasis.data.model.Chapter
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.*
import retrofit2.Response

class AudiobookRepositoryTest {

    private lateinit var repository: AudiobookRepository
    private lateinit var api: MouseionApi

    @Before
    fun setup() {
        api = mock()
        repository = AudiobookRepository(api)
    }

    @Test
    fun `getAudiobooks returns success with valid response`() = runTest {
        val audiobooks = listOf(
            AudiobookDto(
                id = "1",
                title = "Test Audiobook",
                author = "Test Author",
                narrator = "Test Narrator",
                seriesName = null,
                seriesNumber = null,
                duration = 3600000,
                coverArtUrl = "http://example.com/cover.jpg",
                totalChapters = 10,
                format = "MP3",
                fileSize = 1000000,
                filePath = "/path/to/file",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            )
        )
        whenever(api.getAudiobooks(any(), any())).thenReturn(Response.success(audiobooks))

        val result = repository.getAudiobooks()

        assertTrue(result.isSuccess)
        assertEquals(1, result.getOrNull()?.size)
        assertEquals("Test Audiobook", result.getOrNull()?.first()?.title)
    }

    @Test
    fun `getAudiobook returns success with valid response`() = runTest {
        val audiobook = AudiobookDto(
            id = "1",
            title = "Test Audiobook",
            author = "Test Author",
            narrator = "Test Narrator",
            seriesName = null,
            seriesNumber = null,
            duration = 3600000,
            coverArtUrl = "http://example.com/cover.jpg",
            totalChapters = 10,
            format = "MP3",
            fileSize = 1000000,
            filePath = "/path/to/file",
            createdAt = "2024-01-01",
            updatedAt = "2024-01-01"
        )
        whenever(api.getAudiobook(any())).thenReturn(Response.success(audiobook))

        val result = repository.getAudiobook("1")

        assertTrue(result.isSuccess)
        assertEquals("Test Audiobook", result.getOrNull()?.title)
    }

    @Test
    fun `getChapters returns success with valid response`() = runTest {
        val chapters = listOf(
            Chapter(index = 0, title = "Chapter 1", startTimeMs = 0, endTimeMs = 600000),
            Chapter(index = 1, title = "Chapter 2", startTimeMs = 600000, endTimeMs = 1200000)
        )
        whenever(api.getChapters(any())).thenReturn(Response.success(chapters))

        val result = repository.getChapters("file-1")

        assertTrue(result.isSuccess)
        assertEquals(2, result.getOrNull()?.size)
        assertEquals("Chapter 1", result.getOrNull()?.first()?.title)
    }

    @Test
    fun `streamAudiobook returns correct URL`() {
        val url = repository.streamAudiobook("123")

        assertEquals("api/v3/stream/123", url)
    }
}
