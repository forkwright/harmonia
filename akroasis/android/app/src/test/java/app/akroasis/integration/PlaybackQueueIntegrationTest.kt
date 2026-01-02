package app.akroasis.integration

import app.akroasis.audio.PlaybackQueue
import app.akroasis.data.model.Track
import app.akroasis.data.persistence.PlaybackStateStore
import app.akroasis.data.preferences.PlaybackSpeedPreferences
import app.akroasis.ui.queue.ExportFormat
import app.akroasis.ui.queue.QueueExporter
import android.content.Context
import android.net.Uri
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.*
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

/**
 * Integration tests for queue management, history, and export features
 */
@OptIn(ExperimentalCoroutinesApi::class)
class PlaybackQueueIntegrationTest {

    private lateinit var queue: PlaybackQueue
    private lateinit var mockContext: Context
    private lateinit var queueExporter: QueueExporter

    private val testTracks = listOf(
        createTestTrack("1", "Song 1"),
        createTestTrack("2", "Song 2"),
        createTestTrack("3", "Song 3"),
        createTestTrack("4", "Song 4"),
        createTestTrack("5", "Song 5")
    )

    @Before
    fun setup() {
        queue = PlaybackQueue()
        mockContext = mock()
        queueExporter = QueueExporter(mockContext)
    }

    @Test
    fun `SCENARIO 1 - Queue with history allows undo after adding tracks`() = runTest {
        // Given - empty queue
        assertTrue(queue.getTracks().isEmpty())

        // When - add tracks and then undo
        queue.setQueue(testTracks)
        assertTrue(queue.canUndo)

        val undoSuccess = queue.undo()

        // Then - should revert to empty
        assertTrue(undoSuccess)
        assertTrue(queue.getTracks().isEmpty())
    }

    @Test
    fun `SCENARIO 2 - Queue history supports redo after undo`() = runTest {
        // Given - queue with tracks
        queue.setQueue(testTracks)

        // When - undo then redo
        queue.undo()
        assertTrue(queue.canRedo)

        val redoSuccess = queue.redo()

        // Then - should restore tracks
        assertTrue(redoSuccess)
        assertEquals(5, queue.getTracks().size)
    }

    @Test
    fun `SCENARIO 3 - Queue export includes all tracks in order`() = runTest {
        // Given - queue with tracks
        val outputStream = java.io.ByteArrayOutputStream()
        val mockUri: Uri = mock()
        whenever(mockContext.contentResolver).thenReturn(mock())
        whenever(mockContext.contentResolver.openOutputStream(mockUri))
            .thenReturn(outputStream)

        // When - export queue
        val result = queueExporter.exportQueue(testTracks, ExportFormat.M3U, mockUri)

        // Then - should succeed and include all tracks
        assertTrue(result.isSuccess)
        val content = outputStream.toString()
        assertTrue(content.contains("Song 1"))
        assertTrue(content.contains("Song 2"))
        assertTrue(content.contains("Song 3"))
        assertTrue(content.contains("Song 4"))
        assertTrue(content.contains("Song 5"))
    }

    @Test
    fun `SCENARIO 4 - Queue operations preserve current index`() = runTest {
        // Given - queue with multiple tracks
        queue.setQueue(testTracks)
        queue.skipTo(2) // Move to track 3

        // When - add more tracks
        val newTracks = testTracks + createTestTrack("6", "Song 6")
        queue.setQueue(newTracks)

        // Then - current track should still be accessible
        val current = queue.getCurrentTrack()
        assertNotNull(current)
        assertTrue(queue.getCurrentIndex() >= 0)
    }

    @Test
    fun `SCENARIO 5 - Queue history has limited size`() = runTest {
        // Given - empty queue
        queue.setQueue(emptyList())

        // When - make many changes (more than history limit of 50)
        repeat(60) { i ->
            queue.setQueue(listOf(createTestTrack(i.toString(), "Song $i")))
        }

        // Then - should be able to undo but not infinitely
        var undoCount = 0
        while (queue.canUndo && undoCount < 100) {
            queue.undo()
            undoCount++
        }

        // Should stop at history limit (50 states)
        assertTrue(undoCount <= 50)
    }

    @Test
    fun `SCENARIO 6 - Queue export M3U8 includes extended metadata`() = runTest {
        // Given
        val trackWithMetadata = testTracks[0].copy(
            coverArtUrl = "https://example.com/art.jpg",
            album = "Test Album"
        )
        val outputStream = java.io.ByteArrayOutputStream()
        val mockUri: Uri = mock()
        whenever(mockContext.contentResolver).thenReturn(mock())
        whenever(mockContext.contentResolver.openOutputStream(mockUri))
            .thenReturn(outputStream)

        // When - export as M3U8
        queueExporter.exportQueue(listOf(trackWithMetadata), ExportFormat.M3U8, mockUri)

        // Then - should include album and artwork
        val content = outputStream.toString()
        assertTrue(content.contains("#EXTALB:Test Album"))
        assertTrue(content.contains("#EXTIMG:https://example.com/art.jpg"))
    }

    @Test
    fun `SCENARIO 7 - Queue shuffle preserves all tracks`() = runTest {
        // Given - ordered queue
        queue.setQueue(testTracks)
        val originalSize = queue.getTracks().size

        // When - enable shuffle
        queue.shuffle()

        // Then - same tracks, potentially different order
        assertEquals(originalSize, queue.getTracks().size)
        testTracks.forEach { track ->
            assertTrue(queue.getTracks().any { it.id == track.id })
        }
    }

    @Test
    fun `SCENARIO 8 - Queue clear removes all tracks`() = runTest {
        // Given - queue with tracks
        queue.setQueue(testTracks)
        assertEquals(5, queue.getTracks().size)

        // When - clear queue
        queue.clear()

        // Then - should be empty
        assertTrue(queue.getTracks().isEmpty())
    }

    @Test
    fun `SCENARIO 9 - Queue move changes track order`() = runTest {
        // Given - queue with tracks
        queue.setQueue(testTracks)
        val firstTrack = queue.getTracks()[0]

        // When - move first track to end
        queue.move(0, 4)

        // Then - track should be at new position
        assertEquals(firstTrack.id, queue.getTracks()[4].id)
    }

    @Test
    fun `SCENARIO 10 - Queue remove preserves remaining tracks`() = runTest {
        // Given - queue with 5 tracks
        queue.setQueue(testTracks)

        // When - remove track at index 2
        queue.removeAt(2)

        // Then - should have 4 tracks left
        assertEquals(4, queue.getTracks().size)
        // Track IDs 1, 2, 4, 5 should remain (3 removed)
        val ids = queue.getTracks().map { it.id }
        assertTrue("1" in ids)
        assertTrue("2" in ids)
        assertFalse("3" in ids)
        assertTrue("4" in ids)
        assertTrue("5" in ids)
    }

    @Test
    fun `SCENARIO 11 - Undo after remove restores track`() = runTest {
        // Given - queue with tracks
        queue.setQueue(testTracks)

        // When - remove a track then undo
        queue.removeAt(2)
        assertEquals(4, queue.getTracks().size)

        queue.undo()

        // Then - track should be restored
        assertEquals(5, queue.getTracks().size)
        assertTrue(queue.getTracks().any { it.id == "3" })
    }

    @Test
    fun `SCENARIO 12 - Queue next returns correct track`() = runTest {
        // Given - queue at first track
        queue.setQueue(testTracks)
        queue.skipTo(0)

        // When - get next track
        val nextTrack = queue.getNextTrack()

        // Then - should be second track
        assertNotNull(nextTrack)
        assertEquals("2", nextTrack.id)
    }

    @Test
    fun `SCENARIO 13 - Queue previous returns correct track`() = runTest {
        // Given - queue at second track
        queue.setQueue(testTracks)
        queue.skipTo(1)

        // When - get previous track
        val previousTrack = queue.getPreviousTrack()

        // Then - should be first track
        assertNotNull(previousTrack)
        assertEquals("1", previousTrack.id)
    }

    @Test
    fun `SCENARIO 14 - Queue skip to updates current index`() = runTest {
        // Given - queue with tracks
        queue.setQueue(testTracks)

        // When - skip to index 3
        queue.skipTo(3)

        // Then - current index should be 3
        assertEquals(3, queue.getCurrentIndex())
        assertEquals("4", queue.getCurrentTrack()?.id)
    }

    @Test
    fun `SCENARIO 15 - Queue export PLS format has correct structure`() = runTest {
        // Given
        val outputStream = java.io.ByteArrayOutputStream()
        val mockUri: Uri = mock()
        whenever(mockContext.contentResolver).thenReturn(mock())
        whenever(mockContext.contentResolver.openOutputStream(mockUri))
            .thenReturn(outputStream)

        // When - export as PLS
        queueExporter.exportQueue(testTracks, ExportFormat.PLS, mockUri)

        // Then - should have PLS structure
        val content = outputStream.toString()
        assertTrue(content.startsWith("[playlist]"))
        assertTrue(content.contains("NumberOfEntries=5"))
        assertTrue(content.contains("File1="))
        assertTrue(content.contains("Title1="))
        assertTrue(content.contains("Length1="))
        assertTrue(content.contains("Version=2"))
    }

    @Test
    fun `SCENARIO 16 - Queue history cleared after new action`() = runTest {
        // Given - queue with undo/redo history
        queue.setQueue(testTracks)
        queue.removeAt(0)
        queue.undo() // Now can redo

        assertTrue(queue.canRedo)

        // When - make new change
        queue.removeAt(1)

        // Then - redo should be cleared
        assertFalse(queue.canRedo)
    }

    @Test
    fun `SCENARIO 17 - Queue handles empty state correctly`() = runTest {
        // Given - empty queue
        queue.clear()

        // When/Then - operations on empty queue
        assertFalse(queue.canUndo)
        assertFalse(queue.canRedo)
        assertEquals(0, queue.getTracks().size)
        assertNull(queue.getCurrentTrack())
        assertNull(queue.getNextTrack())
        assertNull(queue.getPreviousTrack())
    }

    @Test
    fun `SCENARIO 18 - Multiple undo operations work correctly`() = runTest {
        // Given - series of queue changes
        queue.setQueue(testTracks.take(2))
        queue.addTrack(testTracks[2])
        queue.addTrack(testTracks[3])
        queue.addTrack(testTracks[4])

        // When - undo multiple times
        queue.undo() // Remove track 5
        queue.undo() // Remove track 4
        queue.undo() // Remove track 3

        // Then - should have only first 2 tracks
        assertEquals(2, queue.getTracks().size)
    }

    private fun createTestTrack(id: String, title: String) = Track(
        id = id,
        title = title,
        artist = "Test Artist",
        album = "Test Album",
        albumArtist = null,
        duration = 180000,
        filePath = "/music/$id.flac",
        trackNumber = id.toInt(),
        discNumber = 1,
        year = 2024,
        genre = "Rock",
        coverArtUrl = null,
        sampleRate = 44100,
        bitDepth = 16,
        channels = 2,
        codec = "FLAC",
        bitrate = 1000
    )
}
