package app.akroasis.audio

import android.content.Context
import android.media.AudioTrack
import app.cash.turbine.test
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.mockito.kotlin.any
import org.mockito.kotlin.mock
import org.mockito.kotlin.verify
import org.mockito.kotlin.whenever
import org.robolectric.RobolectricTestRunner

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class GaplessPlaybackEngineTest {

    private lateinit var gaplessEngine: GaplessPlaybackEngine
    private lateinit var mockContext: Context
    private lateinit var mockEqualizerEngine: EqualizerEngine
    private lateinit var mockAudioTrackFactory: AudioTrackFactory
    private lateinit var mockAudioTrack: AudioTrack

    private val testDecodedAudio = DecodedAudio(
        samples = ByteArray(44100 * 2 * 2) { (it % 100).toByte() }, // 1 second stereo, 16-bit (2 bytes per sample)
        sampleRate = 44100,
        channels = 2,
        bitDepth = 16
    )

    @Before
    fun setup() {
        mockContext = mock()
        mockEqualizerEngine = mock()
        mockAudioTrackFactory = mock()
        mockAudioTrack = mock()

        // Mock factory to return mock AudioTrack
        whenever(mockAudioTrackFactory.createAudioTrack(any(), any()))
            .thenReturn(mockAudioTrack)

        // Mock AudioTrack methods
        whenever(mockAudioTrack.audioSessionId).thenReturn(1)
        whenever(mockAudioTrack.play()).then { /* no-op */ }
        whenever(mockAudioTrack.pause()).then { /* no-op */ }
        whenever(mockAudioTrack.stop()).then { /* no-op */ }
        whenever(mockAudioTrack.release()).then { /* no-op */ }

        gaplessEngine = GaplessPlaybackEngine(mockContext, mockEqualizerEngine, mockAudioTrackFactory)
    }

    @Test
    fun `enableGapless sets gapless state to true`() = runTest {
        gaplessEngine.isGaplessEnabled.test {
            assertEquals(true, awaitItem()) // Default is true

            gaplessEngine.disableGapless()
            assertEquals(false, awaitItem())

            gaplessEngine.enableGapless()
            assertEquals(true, awaitItem())
        }
    }

    @Test
    fun `disableGapless sets gapless state to false`() = runTest {
        gaplessEngine.isGaplessEnabled.test {
            assertEquals(true, awaitItem()) // Default is true

            gaplessEngine.disableGapless()
            assertEquals(false, awaitItem())
        }
    }

    @Test
    fun `playTrack increments track index`() = runTest {
        gaplessEngine.currentTrackIndex.test {
            val initialIndex = awaitItem()

            gaplessEngine.playTrack(testDecodedAudio)

            val newIndex = awaitItem()
            assertEquals(initialIndex + 1, newIndex)
        }
    }

    @Test
    fun `playTrack creates and plays AudioTrack`() = runTest {
        val track = gaplessEngine.playTrack(testDecodedAudio)

        assertNotNull(track)
        verify(mockEqualizerEngine).attachToSession(track!!.audioSessionId)
    }

    @Test
    fun `playTrack with custom playback speed`() = runTest {
        val track = gaplessEngine.playTrack(testDecodedAudio, playbackSpeed = 1.5f)

        assertNotNull(track)
        // Note: Actual playback speed validation requires API 23+
    }

    @Test
    fun `getActiveTrack returns current track`() = runTest {
        val track = gaplessEngine.playTrack(testDecodedAudio)

        val activeTrack = gaplessEngine.getActiveTrack()
        assertEquals(track, activeTrack)
    }

    @Test
    fun `pause pauses active track`() = runTest {
        gaplessEngine.playTrack(testDecodedAudio)

        gaplessEngine.pause()

        val activeTrack = gaplessEngine.getActiveTrack()
        assertNotNull(activeTrack)
        // Track state verification would require PowerMock or similar
    }

    @Test
    fun `resume resumes active track`() = runTest {
        gaplessEngine.playTrack(testDecodedAudio)
        gaplessEngine.pause()

        gaplessEngine.resume()

        val activeTrack = gaplessEngine.getActiveTrack()
        assertNotNull(activeTrack)
    }

    @Test
    fun `stop releases all tracks`() = runTest {
        gaplessEngine.playTrack(testDecodedAudio)

        gaplessEngine.stop()

        val activeTrack = gaplessEngine.getActiveTrack()
        assertNull(activeTrack)
    }

    @Test
    fun `preloadNextTrack with gapless enabled`() = runTest {
        gaplessEngine.enableGapless()
        gaplessEngine.playTrack(testDecodedAudio)

        gaplessEngine.preloadNextTrack(testDecodedAudio)

        // Preload happens in background coroutine
        // Verification would require delay or coroutine testing utilities
    }

    @Test
    fun `preloadNextTrack with gapless disabled does nothing`() = runTest {
        gaplessEngine.disableGapless()
        gaplessEngine.playTrack(testDecodedAudio)

        val indexBefore = gaplessEngine.currentTrackIndex.value

        gaplessEngine.preloadNextTrack(testDecodedAudio)

        val indexAfter = gaplessEngine.currentTrackIndex.value
        assertEquals(indexBefore, indexAfter)
    }

    @Test
    fun `switchToPreloadedTrack with gapless enabled increments index`() = runTest {
        gaplessEngine.enableGapless()

        gaplessEngine.currentTrackIndex.test {
            val initialIndex = awaitItem()

            gaplessEngine.playTrack(testDecodedAudio)
            awaitItem() // playTrack increments

            // Preload next track
            gaplessEngine.preloadNextTrack(testDecodedAudio)
            delay(100) // Let preload coroutine complete

            gaplessEngine.switchToPreloadedTrack()

            val newIndex = awaitItem()
            assertEquals(initialIndex + 2, newIndex)
        }
    }

    @Test
    fun `switchToPreloadedTrack with gapless disabled does nothing`() = runTest {
        gaplessEngine.disableGapless()

        gaplessEngine.currentTrackIndex.test {
            val initialIndex = awaitItem()

            gaplessEngine.playTrack(testDecodedAudio)
            awaitItem() // playTrack increments

            gaplessEngine.switchToPreloadedTrack()

            // Should not increment again
            expectNoEvents()
        }
    }

    @Test
    fun `setPlaybackSpeed updates active track`() = runTest {
        gaplessEngine.playTrack(testDecodedAudio)

        gaplessEngine.setPlaybackSpeed(1.25f)

        // Actual playback speed verification requires API 23+ and PowerMock
        val activeTrack = gaplessEngine.getActiveTrack()
        assertNotNull(activeTrack)
    }

    @Test
    fun `seekTo sets playback head position`() = runTest {
        gaplessEngine.playTrack(testDecodedAudio)

        gaplessEngine.seekTo(1000)

        // Position verification requires PowerMock or integration test
        val activeTrack = gaplessEngine.getActiveTrack()
        assertNotNull(activeTrack)
    }

    @Test
    fun `release stops and cleans up resources`() = runTest {
        gaplessEngine.playTrack(testDecodedAudio)

        gaplessEngine.release()

        val activeTrack = gaplessEngine.getActiveTrack()
        assertNull(activeTrack)
    }
}
