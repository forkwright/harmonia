package app.akroasis.data.repository

import app.akroasis.data.api.EbookDto
import app.akroasis.data.api.MouseionApi
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.*
import retrofit2.Response

class EbookRepositoryTest {

    private lateinit var repository: EbookRepository
    private lateinit var api: MouseionApi

    @Before
    fun setup() {
        api = mock()
        repository = EbookRepository(api)
    }

    @Test
    fun `getEbooks returns success with valid response`() = runTest {
        val ebooks = listOf(
            EbookDto(
                id = "1",
                title = "Test Ebook",
                author = "Test Author",
                seriesName = null,
                seriesNumber = null,
                pageCount = 300,
                publishDate = "2024-01-01",
                coverArtUrl = "http://example.com/cover.jpg",
                format = "EPUB",
                fileSize = 5000000,
                filePath = "/path/to/file.epub",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            )
        )
        whenever(api.getEbooks(any(), any())).thenReturn(Response.success(ebooks))

        val result = repository.getEbooks()

        assertTrue(result.isSuccess)
        assertEquals(1, result.getOrNull()?.size)
        assertEquals("Test Ebook", result.getOrNull()?.first()?.title)
    }

    @Test
    fun `getEbook returns success with valid response`() = runTest {
        val ebook = EbookDto(
            id = "1",
            title = "Test Ebook",
            author = "Test Author",
            seriesName = null,
            seriesNumber = null,
            pageCount = 300,
            publishDate = "2024-01-01",
            coverArtUrl = "http://example.com/cover.jpg",
            format = "EPUB",
            fileSize = 5000000,
            filePath = "/path/to/file.epub",
            createdAt = "2024-01-01",
            updatedAt = "2024-01-01"
        )
        whenever(api.getEbook(any())).thenReturn(Response.success(ebook))

        val result = repository.getEbook("1")

        assertTrue(result.isSuccess)
        assertEquals("Test Ebook", result.getOrNull()?.title)
        assertEquals(300, result.getOrNull()?.pageCount)
    }

    @Test
    fun `getEpubUrl returns correct URL`() {
        val url = repository.getEpubUrl("123")

        assertEquals("api/v3/stream/123", url)
    }
}
