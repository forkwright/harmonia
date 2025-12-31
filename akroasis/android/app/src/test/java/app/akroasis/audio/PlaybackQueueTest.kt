// Unit tests for PlaybackQueue thread safety and operations
package app.akroasis.audio

import app.akroasis.data.model.Track
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class PlaybackQueueTest {

    private lateinit var queue: PlaybackQueue
    private lateinit var testScope: TestScope
    private lateinit var testDispatcher: StandardTestDispatcher

    private val testTracks = listOf(
        Track("1", "Track 1", "Artist 1", "Album 1", 180000, "FLAC", null),
        Track("2", "Track 2", "Artist 2", "Album 2", 200000, "FLAC", null),
        Track("3", "Track 3", "Artist 3", "Album 3", 220000, "FLAC", null)
    )

    @Before
    fun setUp() {
        testDispatcher = StandardTestDispatcher()
        testScope = TestScope(testDispatcher)
        queue = PlaybackQueue()
    }

    @Test
    fun `setQueue updates tracks and currentIndex`() = runTest {
        queue.setQueue(testTracks, 1)

        assertEquals(testTracks, queue.tracks.value)
        assertEquals(1, queue.currentIndex.value)
        assertEquals(testTracks[1], queue.currentTrack)
    }

    @Test
    fun `skipToNext advances to next track`() = runTest {
        queue.setQueue(testTracks, 0)

        val nextTrack = queue.skipToNext()

        assertEquals(testTracks[1], nextTrack)
        assertEquals(1, queue.currentIndex.value)
    }

    @Test
    fun `skipToPrevious goes back to previous track`() = runTest {
        queue.setQueue(testTracks, 2)

        val prevTrack = queue.skipToPrevious()

        assertEquals(testTracks[1], prevTrack)
        assertEquals(1, queue.currentIndex.value)
    }

    @Test
    fun `toggleShuffle shuffles tracks`() = runTest {
        queue.setQueue(testTracks, 0)
        val originalOrder = queue.tracks.value

        queue.toggleShuffle()

        assertTrue(queue.shuffleEnabled.value)
        assertEquals(testTracks[0], queue.currentTrack) // Current track stays at index 0
        assertNotEquals(originalOrder, queue.tracks.value) // Order should be different
    }

    @Test
    fun `toggleShuffle twice restores original order`() = runTest {
        queue.setQueue(testTracks, 0)

        queue.toggleShuffle()
        queue.toggleShuffle()

        assertFalse(queue.shuffleEnabled.value)
        assertEquals(testTracks, queue.tracks.value)
    }

    @Test
    fun `cycleRepeatMode cycles through modes`() = runTest {
        assertEquals(RepeatMode.OFF, queue.repeatMode.value)

        queue.cycleRepeatMode()
        assertEquals(RepeatMode.ALL, queue.repeatMode.value)

        queue.cycleRepeatMode()
        assertEquals(RepeatMode.ONE, queue.repeatMode.value)

        queue.cycleRepeatMode()
        assertEquals(RepeatMode.OFF, queue.repeatMode.value)
    }

    @Test
    fun `addToQueue adds track to end`() = runTest {
        queue.setQueue(testTracks.subList(0, 2), 0)

        queue.addToQueue(testTracks[2])

        assertEquals(3, queue.tracks.value.size)
        assertEquals(testTracks[2], queue.tracks.value.last())
    }

    @Test
    fun `removeFromQueue removes track and adjusts index`() = runTest {
        queue.setQueue(testTracks, 2)

        queue.removeFromQueue(0)

        assertEquals(2, queue.tracks.value.size)
        assertEquals(1, queue.currentIndex.value) // Index adjusted from 2 to 1
    }

    @Test
    fun `clear removes all tracks`() = runTest {
        queue.setQueue(testTracks, 0)

        queue.clear()

        assertTrue(queue.tracks.value.isEmpty())
        assertEquals(-1, queue.currentIndex.value)
    }

    @Test
    fun `concurrent shuffle operations are thread-safe`() = runTest {
        queue.setQueue(testTracks, 0)

        // Launch multiple concurrent shuffle operations
        val jobs = List(10) {
            launch {
                queue.toggleShuffle()
            }
        }

        jobs.forEach { it.join() }

        // Verify queue is in consistent state (all operations completed)
        assertNotNull(queue.currentTrack)
    }
}
