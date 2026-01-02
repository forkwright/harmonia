package app.akroasis.ui.queue

import android.content.ContentResolver
import android.content.Context
import android.net.Uri
import app.akroasis.data.model.Track
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.*
import java.io.ByteArrayOutputStream
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class QueueExporterTest {

    private lateinit var exporter: QueueExporter
    private lateinit var mockContext: Context
    private lateinit var mockContentResolver: ContentResolver
    private lateinit var mockUri: Uri

    private val testTrack1 = Track(
        id = "1",
        title = "Test Song",
        artist = "Test Artist",
        album = "Test Album",
        albumArtist = "Test Album Artist",
        duration = 180000, // 3 minutes
        filePath = "/music/test_song.flac",
        trackNumber = 1,
        discNumber = 1,
        year = 2024,
        genre = "Rock",
        coverArtUrl = "https://example.com/cover.jpg",
        sampleRate = 44100,
        bitDepth = 16,
        channels = 2,
        codec = "FLAC",
        bitrate = 1000
    )

    private val testTrack2 = Track(
        id = "2",
        title = "Another Song",
        artist = "Another Artist",
        album = "Another Album",
        albumArtist = null,
        duration = 240000, // 4 minutes
        filePath = "/music/another_song.mp3",
        trackNumber = 2,
        discNumber = 1,
        year = 2023,
        genre = "Pop",
        coverArtUrl = null,
        sampleRate = 44100,
        bitDepth = 16,
        channels = 2,
        codec = "MP3",
        bitrate = 320
    )

    @Before
    fun setup() {
        mockContext = mock()
        mockContentResolver = mock()
        mockUri = mock()

        whenever(mockContext.contentResolver).thenReturn(mockContentResolver)

        exporter = QueueExporter(mockContext)
    }

    @Test
    fun `exportQueue M3U format generates correct header`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        val result = exporter.exportQueue(listOf(testTrack1), ExportFormat.M3U, mockUri)

        // Then
        assertTrue(result.isSuccess)
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.startsWith("#EXTM3U"))
    }

    @Test
    fun `exportQueue M3U format includes track duration`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        exporter.exportQueue(listOf(testTrack1), ExportFormat.M3U, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.contains("#EXTINF:180,Test Artist - Test Song"))
    }

    @Test
    fun `exportQueue M3U format includes file path`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        exporter.exportQueue(listOf(testTrack1), ExportFormat.M3U, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.contains("/music/test_song.flac"))
    }

    @Test
    fun `exportQueue M3U format handles multiple tracks`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)
        val tracks = listOf(testTrack1, testTrack2)

        // When
        exporter.exportQueue(tracks, ExportFormat.M3U, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.contains("Test Artist - Test Song"))
        assertTrue(content.contains("Another Artist - Another Song"))
        assertTrue(content.contains("/music/test_song.flac"))
        assertTrue(content.contains("/music/another_song.mp3"))
    }

    @Test
    fun `exportQueue M3U8 format includes album metadata`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        exporter.exportQueue(listOf(testTrack1), ExportFormat.M3U8, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.contains("#EXTALB:Test Album"))
    }

    @Test
    fun `exportQueue M3U8 format includes cover art if available`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        exporter.exportQueue(listOf(testTrack1), ExportFormat.M3U8, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.contains("#EXTIMG:https://example.com/cover.jpg"))
    }

    @Test
    fun `exportQueue M3U8 format omits cover art if null`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        exporter.exportQueue(listOf(testTrack2), ExportFormat.M3U8, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertFalse(content.contains("#EXTIMG:"))
    }

    @Test
    fun `exportQueue PLS format includes playlist header`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        exporter.exportQueue(listOf(testTrack1), ExportFormat.PLS, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.startsWith("[playlist]"))
    }

    @Test
    fun `exportQueue PLS format includes number of entries`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)
        val tracks = listOf(testTrack1, testTrack2)

        // When
        exporter.exportQueue(tracks, ExportFormat.PLS, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.contains("NumberOfEntries=2"))
    }

    @Test
    fun `exportQueue PLS format uses 1-based indexing`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        exporter.exportQueue(listOf(testTrack1), ExportFormat.PLS, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.contains("File1="))
        assertTrue(content.contains("Title1="))
        assertTrue(content.contains("Length1="))
    }

    @Test
    fun `exportQueue PLS format includes version`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        exporter.exportQueue(listOf(testTrack1), ExportFormat.PLS, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.contains("Version=2"))
    }

    @Test
    fun `exportQueue PLS format includes all track fields`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        exporter.exportQueue(listOf(testTrack1), ExportFormat.PLS, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.contains("File1=/music/test_song.flac"))
        assertTrue(content.contains("Title1=Test Artist - Test Song"))
        assertTrue(content.contains("Length1=180"))
    }

    @Test
    fun `exportQueue returns failure when output stream is null`() = runTest {
        // Given
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(null)

        // When
        val result = exporter.exportQueue(listOf(testTrack1), ExportFormat.M3U, mockUri)

        // Then
        assertTrue(result.isFailure)
        assertEquals("Could not open output stream", result.exceptionOrNull()?.message)
    }

    @Test
    fun `exportQueue returns failure on exception`() = runTest {
        // Given
        whenever(mockContentResolver.openOutputStream(mockUri))
            .thenThrow(RuntimeException("IO Error"))

        // When
        val result = exporter.exportQueue(listOf(testTrack1), ExportFormat.M3U, mockUri)

        // Then
        assertTrue(result.isFailure)
        assertEquals("IO Error", result.exceptionOrNull()?.message)
    }

    @Test
    fun `exportQueue handles empty track list`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        val result = exporter.exportQueue(emptyList(), ExportFormat.M3U, mockUri)

        // Then
        assertTrue(result.isSuccess)
        val content = outputStream.toString(Charsets.UTF_8)
        assertEquals("#EXTM3U\n", content)
    }

    @Test
    fun `exportQueue PLS handles empty track list`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        val result = exporter.exportQueue(emptyList(), ExportFormat.PLS, mockUri)

        // Then
        assertTrue(result.isSuccess)
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.contains("NumberOfEntries=0"))
    }

    @Test
    fun `ExportFormat M3U has correct extension`() {
        // Then
        assertEquals("m3u", ExportFormat.M3U.extension)
    }

    @Test
    fun `ExportFormat M3U8 has correct extension`() {
        // Then
        assertEquals("m3u8", ExportFormat.M3U8.extension)
    }

    @Test
    fun `ExportFormat PLS has correct extension`() {
        // Then
        assertEquals("pls", ExportFormat.PLS.extension)
    }

    @Test
    fun `ExportFormat M3U has correct MIME type`() {
        // Then
        assertEquals("audio/x-mpegurl", ExportFormat.M3U.mimeType)
    }

    @Test
    fun `ExportFormat M3U8 has correct MIME type`() {
        // Then
        assertEquals("application/vnd.apple.mpegurl", ExportFormat.M3U8.mimeType)
    }

    @Test
    fun `ExportFormat PLS has correct MIME type`() {
        // Then
        assertEquals("audio/x-scpls", ExportFormat.PLS.mimeType)
    }

    @Test
    fun `exportQueue converts duration from milliseconds to seconds`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)
        val track = testTrack1.copy(duration = 123456) // 123.456 seconds

        // When
        exporter.exportQueue(listOf(track), ExportFormat.M3U, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.contains("#EXTINF:123,"))
    }

    @Test
    fun `exportQueue M3U format has correct line endings`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        exporter.exportQueue(listOf(testTrack1), ExportFormat.M3U, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        val lines = content.lines()
        assertTrue(lines.size >= 3) // Header + EXTINF + path
    }

    @Test
    fun `exportQueue writes UTF-8 encoded content`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)
        val trackWithUnicode = testTrack1.copy(
            title = "Test Café ☕",
            artist = "Artiste Français"
        )

        // When
        exporter.exportQueue(listOf(trackWithUnicode), ExportFormat.M3U, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        assertTrue(content.contains("Test Café ☕"))
        assertTrue(content.contains("Artiste Français"))
    }

    @Test
    fun `exportQueue closes output stream after writing`() = runTest {
        // Given
        val outputStream = mock<ByteArrayOutputStream>()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)

        // When
        exporter.exportQueue(listOf(testTrack1), ExportFormat.M3U, mockUri)

        // Then
        verify(outputStream).close()
    }

    @Test
    fun `exportQueue M3U8 handles tracks with and without cover art`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)
        val tracks = listOf(testTrack1, testTrack2) // track1 has cover, track2 doesn't

        // When
        exporter.exportQueue(tracks, ExportFormat.M3U8, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        val extimgCount = content.split("#EXTIMG:").size - 1
        assertEquals(1, extimgCount) // Only track1 has cover art
    }

    @Test
    fun `exportQueue PLS format orders entries sequentially`() = runTest {
        // Given
        val outputStream = ByteArrayOutputStream()
        whenever(mockContentResolver.openOutputStream(mockUri)).thenReturn(outputStream)
        val tracks = listOf(testTrack1, testTrack2)

        // When
        exporter.exportQueue(tracks, ExportFormat.PLS, mockUri)

        // Then
        val content = outputStream.toString(Charsets.UTF_8)
        val file1Index = content.indexOf("File1=")
        val file2Index = content.indexOf("File2=")
        assertTrue(file1Index < file2Index)
    }
}
