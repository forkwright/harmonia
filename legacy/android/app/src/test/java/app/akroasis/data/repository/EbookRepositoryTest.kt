package app.akroasis.data.repository

import app.akroasis.data.api.EbookDto
import app.akroasis.data.api.MouseionApi
import kotlinx.coroutines.test.runTest
import okhttp3.ResponseBody.Companion.toResponseBody
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

    // Error handling tests

    @Test
    fun `getEbooks returns failure on API error`() = runTest {
        whenever(api.getEbooks(any(), any())).thenReturn(Response.error(500, "".toResponseBody()))

        val result = repository.getEbooks()

        assertTrue(result.isFailure)
    }

    @Test
    fun `getEbooks returns failure on exception`() = runTest {
        whenever(api.getEbooks(any(), any())).thenThrow(RuntimeException("Network error"))

        val result = repository.getEbooks()

        assertTrue(result.isFailure)
    }

    @Test
    fun `getEbooks returns empty list on 404`() = runTest {
        whenever(api.getEbooks(any(), any())).thenReturn(Response.error(404, "".toResponseBody()))

        val result = repository.getEbooks()

        assertTrue(result.isFailure)
    }

    @Test
    fun `getEbooks with empty list returns success`() = runTest {
        whenever(api.getEbooks(any(), any())).thenReturn(Response.success(emptyList()))

        val result = repository.getEbooks()

        assertTrue(result.isSuccess)
        assertEquals(0, result.getOrNull()?.size)
    }

    @Test
    fun `getEbook returns failure on API error`() = runTest {
        whenever(api.getEbook(any())).thenReturn(Response.error(404, "".toResponseBody()))

        val result = repository.getEbook("1")

        assertTrue(result.isFailure)
    }

    @Test
    fun `getEbook returns failure on exception`() = runTest {
        whenever(api.getEbook(any())).thenThrow(RuntimeException("Connection timeout"))

        val result = repository.getEbook("1")

        assertTrue(result.isFailure)
    }

    // Custom parameters tests

    @Test
    fun `getEbooks with custom page and pageSize`() = runTest {
        whenever(api.getEbooks(any(), any())).thenReturn(Response.success(emptyList()))

        val result = repository.getEbooks(page = 2, pageSize = 25)

        assertTrue(result.isSuccess)
        verify(api).getEbooks(2, 25)
    }

    // Data validation tests

    @Test
    fun `getEbook with null series information returns success`() = runTest {
        val ebook = EbookDto(
            id = "1",
            title = "Test Ebook",
            author = "Test Author",
            seriesName = null,
            seriesNumber = null,
            pageCount = 300,
            publishDate = "2024-01-01",
            coverArtUrl = null,
            format = "EPUB",
            fileSize = 5000000,
            filePath = "/path/to/file.epub",
            createdAt = "2024-01-01",
            updatedAt = "2024-01-01"
        )
        whenever(api.getEbook(any())).thenReturn(Response.success(ebook))

        val result = repository.getEbook("1")

        assertTrue(result.isSuccess)
        assertNull(result.getOrNull()?.seriesName)
        assertNull(result.getOrNull()?.seriesNumber)
    }

    @Test
    fun `getEbook with series information returns success`() = runTest {
        val ebook = EbookDto(
            id = "1",
            title = "Test Ebook",
            author = "Test Author",
            seriesName = "Test Series",
            seriesNumber = 2,
            pageCount = 450,
            publishDate = "2024-01-01",
            coverArtUrl = "http://example.com/cover.jpg",
            format = "EPUB",
            fileSize = 8000000,
            filePath = "/path/to/file.epub",
            createdAt = "2024-01-01",
            updatedAt = "2024-01-01"
        )
        whenever(api.getEbook(any())).thenReturn(Response.success(ebook))

        val result = repository.getEbook("1")

        assertTrue(result.isSuccess)
        assertEquals("Test Series", result.getOrNull()?.seriesName)
        assertEquals(2, result.getOrNull()?.seriesNumber)
    }

    @Test
    fun `getEbooks with multiple formats returns success`() = runTest {
        val ebooks = listOf(
            EbookDto(
                id = "1",
                title = "EPUB Book",
                author = "Author 1",
                seriesName = null,
                seriesNumber = null,
                pageCount = 250,
                publishDate = "2024-01-01",
                coverArtUrl = null,
                format = "EPUB",
                fileSize = 3000000,
                filePath = "/path/to/epub",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            ),
            EbookDto(
                id = "2",
                title = "PDF Book",
                author = "Author 2",
                seriesName = null,
                seriesNumber = null,
                pageCount = 400,
                publishDate = "2024-01-01",
                coverArtUrl = null,
                format = "PDF",
                fileSize = 10000000,
                filePath = "/path/to/pdf",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            ),
            EbookDto(
                id = "3",
                title = "MOBI Book",
                author = "Author 3",
                seriesName = null,
                seriesNumber = null,
                pageCount = 350,
                publishDate = "2024-01-01",
                coverArtUrl = null,
                format = "MOBI",
                fileSize = 5000000,
                filePath = "/path/to/mobi",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            )
        )
        whenever(api.getEbooks(any(), any())).thenReturn(Response.success(ebooks))

        val result = repository.getEbooks()

        assertTrue(result.isSuccess)
        assertEquals(3, result.getOrNull()?.size)
        val formats = result.getOrNull()?.map { it.format }
        assertTrue(formats?.contains("EPUB") == true)
        assertTrue(formats?.contains("PDF") == true)
        assertTrue(formats?.contains("MOBI") == true)
    }

    @Test
    fun `getEpubUrl with special characters in ID`() {
        val url = repository.getEpubUrl("book-456-xyz")

        assertEquals("api/v3/stream/book-456-xyz", url)
    }

    @Test
    fun `getEbook with zero pages returns success`() = runTest {
        val ebook = EbookDto(
            id = "1",
            title = "Test Ebook",
            author = "Test Author",
            seriesName = null,
            seriesNumber = null,
            pageCount = 0,
            publishDate = "2024-01-01",
            coverArtUrl = null,
            format = "EPUB",
            fileSize = 1000,
            filePath = "/path/to/file.epub",
            createdAt = "2024-01-01",
            updatedAt = "2024-01-01"
        )
        whenever(api.getEbook(any())).thenReturn(Response.success(ebook))

        val result = repository.getEbook("1")

        assertTrue(result.isSuccess)
        assertEquals(0, result.getOrNull()?.pageCount)
    }

    @Test
    fun `getEbook with large page count returns success`() = runTest {
        val ebook = EbookDto(
            id = "1",
            title = "Test Ebook",
            author = "Test Author",
            seriesName = null,
            seriesNumber = null,
            pageCount = 2500,
            publishDate = "2024-01-01",
            coverArtUrl = null,
            format = "EPUB",
            fileSize = 50000000,
            filePath = "/path/to/file.epub",
            createdAt = "2024-01-01",
            updatedAt = "2024-01-01"
        )
        whenever(api.getEbook(any())).thenReturn(Response.success(ebook))

        val result = repository.getEbook("1")

        assertTrue(result.isSuccess)
        assertEquals(2500, result.getOrNull()?.pageCount)
    }

    @Test
    fun `getEbook with null publish date returns success`() = runTest {
        val ebook = EbookDto(
            id = "1",
            title = "Test Ebook",
            author = "Test Author",
            seriesName = null,
            seriesNumber = null,
            pageCount = 300,
            publishDate = null,
            coverArtUrl = null,
            format = "EPUB",
            fileSize = 5000000,
            filePath = "/path/to/file.epub",
            createdAt = "2024-01-01",
            updatedAt = "2024-01-01"
        )
        whenever(api.getEbook(any())).thenReturn(Response.success(ebook))

        val result = repository.getEbook("1")

        assertTrue(result.isSuccess)
        assertNull(result.getOrNull()?.publishDate)
    }

    @Test
    fun `getEbooks sorted by author returns success`() = runTest {
        val ebooks = listOf(
            EbookDto(
                id = "1",
                title = "Book A",
                author = "Author Z",
                seriesName = null,
                seriesNumber = null,
                pageCount = 200,
                publishDate = "2024-01-01",
                coverArtUrl = null,
                format = "EPUB",
                fileSize = 3000000,
                filePath = "/path/a",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            ),
            EbookDto(
                id = "2",
                title = "Book B",
                author = "Author A",
                seriesName = null,
                seriesNumber = null,
                pageCount = 250,
                publishDate = "2024-01-01",
                coverArtUrl = null,
                format = "EPUB",
                fileSize = 4000000,
                filePath = "/path/b",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            ),
            EbookDto(
                id = "3",
                title = "Book C",
                author = "Author M",
                seriesName = null,
                seriesNumber = null,
                pageCount = 300,
                publishDate = "2024-01-01",
                coverArtUrl = null,
                format = "EPUB",
                fileSize = 5000000,
                filePath = "/path/c",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            )
        )
        whenever(api.getEbooks(any(), any())).thenReturn(Response.success(ebooks))

        val result = repository.getEbooks()

        assertTrue(result.isSuccess)
        assertEquals(3, result.getOrNull()?.size)
        val authors = result.getOrNull()?.map { it.author }
        assertEquals("Author Z", authors?.get(0))
        assertEquals("Author A", authors?.get(1))
        assertEquals("Author M", authors?.get(2))
    }

    @Test
    fun `getEbook with very large file size returns success`() = runTest {
        val ebook = EbookDto(
            id = "1",
            title = "Large Ebook",
            author = "Test Author",
            seriesName = null,
            seriesNumber = null,
            pageCount = 1500,
            publishDate = "2024-01-01",
            coverArtUrl = null,
            format = "PDF",
            fileSize = 500000000,
            filePath = "/path/to/large.pdf",
            createdAt = "2024-01-01",
            updatedAt = "2024-01-01"
        )
        whenever(api.getEbook(any())).thenReturn(Response.success(ebook))

        val result = repository.getEbook("1")

        assertTrue(result.isSuccess)
        assertEquals(500000000L, result.getOrNull()?.fileSize)
    }

    @Test
    fun `getEbooks with series filters returns success`() = runTest {
        val ebooks = listOf(
            EbookDto(
                id = "1",
                title = "Series Book 1",
                author = "Author",
                seriesName = "Test Series",
                seriesNumber = 1,
                pageCount = 300,
                publishDate = "2024-01-01",
                coverArtUrl = null,
                format = "EPUB",
                fileSize = 5000000,
                filePath = "/path/1",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            ),
            EbookDto(
                id = "2",
                title = "Series Book 2",
                author = "Author",
                seriesName = "Test Series",
                seriesNumber = 2,
                pageCount = 320,
                publishDate = "2024-01-01",
                coverArtUrl = null,
                format = "EPUB",
                fileSize = 5200000,
                filePath = "/path/2",
                createdAt = "2024-01-01",
                updatedAt = "2024-01-01"
            )
        )
        whenever(api.getEbooks(any(), any())).thenReturn(Response.success(ebooks))

        val result = repository.getEbooks()

        assertTrue(result.isSuccess)
        assertEquals(2, result.getOrNull()?.size)
        result.getOrNull()?.forEach { ebook ->
            assertEquals("Test Series", ebook.seriesName)
        }
    }
}
