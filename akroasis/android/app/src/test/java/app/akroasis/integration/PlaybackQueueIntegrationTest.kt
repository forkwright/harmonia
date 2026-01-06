package app.akroasis.integration

import app.akroasis.audio.PlaybackQueue
import app.akroasis.data.model.Track
import app.akroasis.ui.queue.ExportFormat
import app.akroasis.ui.queue.QueueExporter
import android.content.Context
import android.net.Uri
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.*

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
        assertTrue(queue.tracks.value.isEmpty())

        // When - add tracks and then undo
        queue.setQueue(testTracks)
        assertTrue(queue.canUndo)

        val undoSuccess = queue.undo()

        // Then - should revert to empty
        assertTrue(undoSuccess)
        assertTrue(queue.tracks.value.isEmpty())
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
        assertEquals(5, queue.tracks.value.size)
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
        queue.skipToIndex(2) // Move to track 3

        // When - add more tracks
        val newTracks = testTracks + createTestTrack("6", "Song 6")
        queue.setQueue(newTracks)

        // Then - current track should still be accessible
        val current = queue.currentTrack
        assertNotNull(current)
        assertTrue(queue.currentIndex.value >= 0)
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
        val originalSize = queue.tracks.value.size

        // When - enable shuffle
        queue.toggleShuffle()

        // Then - same tracks, potentially different order
        assertEquals(originalSize, queue.tracks.value.size)
        testTracks.forEach { track ->
            assertTrue(queue.tracks.value.any { it.id == track.id })
        }
    }

    @Test
    fun `SCENARIO 8 - Queue clear removes all tracks`() = runTest {
        // Given - queue with tracks
        queue.setQueue(testTracks)
        assertEquals(5, queue.tracks.value.size)

        // When - clear queue
        queue.clear()

        // Then - should be empty
        assertTrue(queue.tracks.value.isEmpty())
    }

    @Test
    fun `SCENARIO 9 - Queue move changes track order`() = runTest {
        // Given - queue with tracks
        queue.setQueue(testTracks)
        val firstTrack = queue.tracks.value[0]

        // When - move first track to end
        queue.moveTrack(0, 4)

        // Then - track should be at new position
        assertEquals(firstTrack.id, queue.tracks.value[4].id)
    }

    @Test
    fun `SCENARIO 10 - Queue remove preserves remaining tracks`() = runTest {
        // Given - queue with 5 tracks
        queue.setQueue(testTracks)

        // When - remove track at index 2
        queue.removeFromQueue(2)

        // Then - should have 4 tracks left
        assertEquals(4, queue.tracks.value.size)
        // Track IDs 1, 2, 4, 5 should remain (3 removed)
        val ids = queue.tracks.value.map { it.id }
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
        queue.removeFromQueue(2)
        assertEquals(4, queue.tracks.value.size)

        queue.undo()

        // Then - track should be restored
        assertEquals(5, queue.tracks.value.size)
        assertTrue(queue.tracks.value.any { it.id == "3" })
    }

    @Test
    fun `SCENARIO 12 - Queue next returns correct track`() = runTest {
        // Given - queue at first track
        queue.setQueue(testTracks, startIndex = 0)

        // When - skip to next track
        val nextTrack = queue.skipToNext()

        // Then - should be second track
        assertNotNull(nextTrack)
        assertEquals("2", nextTrack?.id)
    }

    @Test
    fun `SCENARIO 13 - Queue previous returns correct track`() = runTest {
        // Given - queue at second track
        queue.setQueue(testTracks, startIndex = 1)

        // When - skip to previous track
        val previousTrack = queue.skipToPrevious()

        // Then - should be first track
        assertNotNull(previousTrack)
        assertEquals("1", previousTrack?.id)
    }

    @Test
    fun `SCENARIO 14 - Queue skip to updates current index`() = runTest {
        // Given - queue with tracks
        queue.setQueue(testTracks)

        // When - skip to index 3
        queue.skipToIndex(3)

        // Then - current index should be 3
        assertEquals(3, queue.currentIndex.value)
        assertEquals("4", queue.currentTrack?.id)
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
        queue.removeFromQueue(0)
        queue.undo() // Now can redo

        assertTrue(queue.canRedo)

        // When - make new change
        queue.removeFromQueue(1)

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
        assertEquals(0, queue.tracks.value.size)
        assertNull(queue.currentTrack)
        assertNull(queue.skipToNext())
        assertNull(queue.skipToPrevious())
    }

    @Test
    fun `SCENARIO 18 - Multiple undo operations work correctly`() = runTest {
        // Given - series of queue changes
        queue.setQueue(testTracks.take(2))
        queue.addToQueue(testTracks[2])
        queue.addToQueue(testTracks[3])
        queue.addToQueue(testTracks[4])

        // When - undo multiple times
        queue.undo() // Remove track 5
        queue.undo() // Remove track 4
        queue.undo() // Remove track 3

        // Then - should have only first 2 tracks
        assertEquals(2, queue.tracks.value.size)
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
        coverArtUrl = null,
        sampleRate = 44100,
        bitDepth = 16,
        format = "FLAC",
        bitrate = 1000,
        fileSize = 5000000,
        createdAt = "2024-01-01T00:00:00Z",
        updatedAt = "2024-01-01T00:00:00Z"
    )
}
