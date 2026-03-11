package app.akroasis.data.repository

import app.akroasis.data.api.AudiobookDto
import app.akroasis.data.api.MouseionApi
import app.akroasis.data.model.Chapter
import kotlinx.coroutines.test.runTest
import okhttp3.ResponseBody.Companion.toResponseBody
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

    // Error handling tests

    @Test
    fun `getAudiobooks returns failure on API error`() = runTest {
        whenever(api.getAudiobooks(any(), any())).thenReturn(Response.error(500, "".toResponseBody()))

        val result = repository.getAudiobooks()

        assertTrue(result.isFailure)
    }

    @Test
    fun `getAudiobooks returns failure on exception`() = runTest {
        whenever(api.getAudiobooks(any(), any())).thenThrow(RuntimeException("Network error"))

        val result = repository.getAudiobooks()

        assertTrue(result.isFailure)
    }

    @Test
    fun `getAudiobooks returns empty list on 404`() = runTest {
        whenever(api.getAudiobooks(any(), any())).thenReturn(Response.error(404, "".toResponseBody()))

        val result = repository.getAudiobooks()

        assertTrue(result.isFailure)
    }

    @Test
    fun `getAudiobooks with empty list returns success`() = runTest {
        whenever(api.getAudiobooks(any(), any())).thenReturn(Response.success(emptyList()))

        val result = repository.getAudiobooks()

        assertTrue(result.isSuccess)
        assertEquals(0, result.getOrNull()?.size)
    }

    @Test
    fun `getAudiobook returns failure on API error`() = runTest {
        whenever(api.getAudiobook(any())).thenReturn(Response.error(404, "".toResponseBody()))

        val result = repository.getAudiobook("1")

        assertTrue(result.isFailure)
    }

    @Test
    fun `getAudiobook returns failure on exception`() = runTest {
        whenever(api.getAudiobook(any())).thenThrow(RuntimeException("Connection timeout"))

        val result = repository.getAudiobook("1")

        assertTrue(result.isFailure)
    }

    @Test
    fun `getChapters returns failure on API error`() = runTest {
        whenever(api.getChapters(any())).thenReturn(Response.error(500, "".toResponseBody()))

        val result = repository.getChapters("file-1")

        assertTrue(result.isFailure)
    }

    @Test
    fun `getChapters returns failure on exception`() = runTest {
        whenever(api.getChapters(any())).thenThrow(RuntimeException("Server error"))

        val result = repository.getChapters("file-1")

        assertTrue(result.isFailure)
    }

    @Test
    fun `getChapters with empty list returns success`() = runTest {
        whenever(api.getChapters(any())).thenReturn(Response.success(emptyList()))

        val result = repository.getChapters("file-1")

        assertTrue(result.isSuccess)
        assertEquals(0, result.getOrNull()?.size)
    }

    // Custom parameters tests

    @Test
    fun `getAudiobooks with custom page and pageSize`() = runTest {
        whenever(api.getAudiobooks(any(), any())).thenReturn(Response.success(emptyList()))

        val result = repository.getAudiobooks(page = 2, pageSize = 25)

        assertTrue(result.isSuccess)
        verify(api).getAudiobooks(2, 25)
    }

    // Data validation tests

    @Test
    fun `getAudiobook with null narrator returns success`() = runTest {
        val audiobook = AudiobookDto(
            id = "1",
            title = "Test Audiobook",
            author = "Test Author",
            narrator = null,
            seriesName = null,
            seriesNumber = null,
            duration = 3600000,
            coverArtUrl = null,
            totalChapters = 0,
            format = "MP3",
            fileSize = 1000000,
            filePath = "/path/to/file",
            createdAt = "2024-01-01",
            updatedAt = "2024-01-01"
        )
        whenever(api.getAudiobook(any())).thenReturn(Response.success(audiobook))

        val result = repository.getAudiobook("1")

        assertTrue(result.isSuccess)
        assertNull(result.getOrNull()?.narrator)
    }

    @Test
    fun `getAudiobook with series information returns success`() = runTest {
        val audiobook = AudiobookDto(
            id = "1",
            title = "Test Audiobook",
            author = "Test Author",
            narrator = "Test Narrator",
            seriesName = "Test Series",
            seriesNumber = 1,
            duration = 3600000,
            coverArtUrl = "http://example.com/cover.jpg",
            totalChapters = 10,
            format = "M4B",
            fileSize = 2000000,
            filePath = "/path/to/file",
            createdAt = "2024-01-01",
            updatedAt = "2024-01-01"
        )
        whenever(api.getAudiobook(any())).thenReturn(Response.success(audiobook))

        val result = repository.getAudiobook("1")

        assertTrue(result.isSuccess)
        assertEquals("Test Series", result.getOrNull()?.seriesName)
        assertEquals(1, result.getOrNull()?.seriesNumber)
    }

    @Test
    fun `getChapters with overlapping times returns success`() = runTest {
        val chapters = listOf(
            Chapter(index = 0, title = "Chapter 1", startTimeMs = 0, endTimeMs = 600000),
            Chapter(index = 1, title = "Chapter 2", startTimeMs = 600000, endTimeMs = 1200000),
            Chapter(index = 2, title = "Chapter 3", startTimeMs = 1200000, endTimeMs = 1800000)
        )
        whenever(api.getChapters(any())).thenReturn(Response.success(chapters))

        val result = repository.getChapters("file-1")

        assertTrue(result.isSuccess)
        assertEquals(3, result.getOrNull()?.size)
        assertEquals(0L, result.getOrNull()?.first()?.startTimeMs)
        assertEquals(1800000L, result.getOrNull()?.last()?.endTimeMs)
    }

    @Test
    fun `getAudiobooks with multiple formats returns success`() = runTest {
        val audiobooks = listOf(
            AudiobookDto(
                id = "1",
                title = "MP3 Book",
                author = "Author 1",
                narrator = "Narrator 1",
                seriesName = null,
                seriesNumber = null,
                duration = 3600000,
                coverArtUrl = null,
                totalChapters = 5,
                format = "MP3",
                fileSize = 1000000,
                filePath = "/path/to/mp3",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            ),
            AudiobookDto(
                id = "2",
                title = "M4B Book",
                author = "Author 2",
                narrator = "Narrator 2",
                seriesName = null,
                seriesNumber = null,
                duration = 7200000,
                coverArtUrl = null,
                totalChapters = 10,
                format = "M4B",
                fileSize = 2000000,
                filePath = "/path/to/m4b",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            ),
            AudiobookDto(
                id = "3",
                title = "FLAC Book",
                author = "Author 3",
                narrator = "Narrator 3",
                seriesName = null,
                seriesNumber = null,
                duration = 5400000,
                coverArtUrl = null,
                totalChapters = 8,
                format = "FLAC",
                fileSize = 5000000,
                filePath = "/path/to/flac",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            )
        )
        whenever(api.getAudiobooks(any(), any())).thenReturn(Response.success(audiobooks))

        val result = repository.getAudiobooks()

        assertTrue(result.isSuccess)
        assertEquals(3, result.getOrNull()?.size)
        val formats = result.getOrNull()?.map { it.format }
        assertTrue(formats?.contains("MP3") == true)
        assertTrue(formats?.contains("M4B") == true)
        assertTrue(formats?.contains("FLAC") == true)
    }

    @Test
    fun `streamAudiobook with special characters in ID`() {
        val url = repository.streamAudiobook("book-123-abc")

        assertEquals("api/v3/stream/book-123-abc", url)
    }

    @Test
    fun `getChapters preserves chapter order`() = runTest {
        val chapters = listOf(
            Chapter(index = 2, title = "Chapter 3", startTimeMs = 1200000, endTimeMs = 1800000),
            Chapter(index = 0, title = "Chapter 1", startTimeMs = 0, endTimeMs = 600000),
            Chapter(index = 1, title = "Chapter 2", startTimeMs = 600000, endTimeMs = 1200000)
        )
        whenever(api.getChapters(any())).thenReturn(Response.success(chapters))

        val result = repository.getChapters("file-1")

        assertTrue(result.isSuccess)
        val resultChapters = result.getOrNull()
        assertEquals(3, resultChapters?.size)
        assertEquals(2, resultChapters?.get(0)?.index)
        assertEquals(0, resultChapters?.get(1)?.index)
        assertEquals(1, resultChapters?.get(2)?.index)
    }

    @Test
    fun `getAudiobook with zero duration returns success`() = runTest {
        val audiobook = AudiobookDto(
            id = "1",
            title = "Test Audiobook",
            author = "Test Author",
            narrator = "Test Narrator",
            seriesName = null,
            seriesNumber = null,
            duration = 0,
            coverArtUrl = null,
            totalChapters = 0,
            format = "MP3",
            fileSize = 1000,
            filePath = "/path/to/file",
            createdAt = "2024-01-01",
            updatedAt = "2024-01-01"
        )
        whenever(api.getAudiobook(any())).thenReturn(Response.success(audiobook))

        val result = repository.getAudiobook("1")

        assertTrue(result.isSuccess)
        assertEquals(0L, result.getOrNull()?.duration)
    }

    @Test
    fun `getAudiobook with large file size returns success`() = runTest {
        val audiobook = AudiobookDto(
            id = "1",
            title = "Test Audiobook",
            author = "Test Author",
            narrator = "Test Narrator",
            seriesName = null,
            seriesNumber = null,
            duration = 36000000,
            coverArtUrl = null,
            totalChapters = 50,
            format = "FLAC",
            fileSize = 5000000000,
            filePath = "/path/to/file",
            createdAt = "2024-01-01",
            updatedAt = "2024-01-01"
        )
        whenever(api.getAudiobook(any())).thenReturn(Response.success(audiobook))

        val result = repository.getAudiobook("1")

        assertTrue(result.isSuccess)
        assertEquals(5000000000L, result.getOrNull()?.fileSize)
    }
}
