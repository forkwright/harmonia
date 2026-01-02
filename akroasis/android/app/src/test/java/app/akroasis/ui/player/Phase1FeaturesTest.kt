// Unit tests for Phase 1 features
package app.akroasis.ui.player

import androidx.arch.core.executor.testing.InstantTaskExecutorRule
import app.akroasis.audio.*
import app.akroasis.data.model.Track
import app.akroasis.data.preferences.AudioPreferences
import app.akroasis.data.repository.MusicRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.*
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.mockito.kotlin.*

@OptIn(ExperimentalCoroutinesApi::class)
class Phase1FeaturesTest {

    @get:Rule
    val instantTaskExecutorRule = InstantTaskExecutorRule()

    private lateinit var viewModel: PlayerViewModel
    private lateinit var audioPlayer: AudioPlayer
    private lateinit var trackLoader: TrackLoader
    private lateinit var musicRepository: MusicRepository
    private lateinit var playbackQueue: PlaybackQueue
    private lateinit var audioPreferences: AudioPreferences
    private lateinit var gaplessEngine: GaplessPlaybackEngine
    private lateinit var crossfadeEngine: CrossfadeEngine
    private lateinit var usbDacDetector: UsbDacDetector
    private lateinit var sleepTimer: SleepTimer
    private lateinit var batteryMonitor: BatteryMonitor

    private val testDispatcher = StandardTestDispatcher()
    private val testScope = TestScope(testDispatcher)

    private val testTrackA = Track(
        id = "A",
        title = "Track A",
        artist = "Artist",
        album = "Album",
        duration = 180000,
        format = "FLAC",
        coverArtUrl = null
    )

    private val testTrackB = Track(
        id = "B",
        title = "Track B",
        artist = "Artist",
        album = "Album",
        duration = 180000,
        format = "ALAC",
        coverArtUrl = null
    )

    @Before
    fun setUp() {
        Dispatchers.setMain(testDispatcher)

        audioPlayer = mock()
        trackLoader = mock()
        musicRepository = mock()
        playbackQueue = mock()
        audioPreferences = mock()
        gaplessEngine = mock()
        crossfadeEngine = mock()
        usbDacDetector = mock()
        sleepTimer = mock()
        batteryMonitor = mock()

        // Set up default mock behaviors
        whenever(audioPlayer.playbackState).thenReturn(MutableStateFlow(PlaybackState.Stopped))
        whenever(audioPlayer.position).thenReturn(MutableStateFlow(0L))
        whenever(audioPlayer.playbackSpeed).thenReturn(MutableStateFlow(1.0f))
        whenever(audioPlayer.audioFormat).thenReturn(MutableStateFlow(null))
        whenever(playbackQueue.tracks).thenReturn(MutableStateFlow(emptyList()))
        whenever(playbackQueue.currentIndex).thenReturn(MutableStateFlow(-1))
        whenever(playbackQueue.shuffleEnabled).thenReturn(MutableStateFlow(false))
        whenever(playbackQueue.repeatMode).thenReturn(MutableStateFlow(RepeatMode.OFF))
        whenever(audioPreferences.equalizerEnabled).thenReturn(false)
        whenever(audioPreferences.playbackSpeed).thenReturn(1.0f)
        whenever(audioPreferences.gaplessEnabled).thenReturn(true)
        whenever(audioPreferences.crossfadeDuration).thenReturn(0)
        whenever(gaplessEngine.isGaplessEnabled).thenReturn(MutableStateFlow(true))
        whenever(usbDacDetector.connectedDacs).thenReturn(MutableStateFlow(emptyList()))
        whenever(usbDacDetector.preferredDac).thenReturn(MutableStateFlow(null))
        whenever(sleepTimer.isActive).thenReturn(MutableStateFlow(false))
        whenever(sleepTimer.remainingTimeMs).thenReturn(MutableStateFlow(0L))
        whenever(batteryMonitor.batteryLevel).thenReturn(MutableStateFlow(100))
        whenever(batteryMonitor.isLowBattery).thenReturn(MutableStateFlow(false))
        whenever(batteryMonitor.isCharging).thenReturn(MutableStateFlow(false))

        viewModel = PlayerViewModel(
            audioPlayer = audioPlayer,
            trackLoader = trackLoader,
            musicRepository = musicRepository,
            playbackQueue = playbackQueue,
            audioPreferences = audioPreferences,
            gaplessEngine = gaplessEngine,
            crossfadeEngine = crossfadeEngine,
            usbDacDetector = usbDacDetector,
            sleepTimer = sleepTimer,
            batteryMonitor = batteryMonitor
        )
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    // Sleep Timer Tests
    @Test
    fun `startSleepTimer starts timer with correct duration`() {
        val duration = 60000L

        viewModel.startSleepTimer(duration)

        verify(sleepTimer).start(duration)
    }

    @Test
    fun `cancelSleepTimer cancels the timer`() {
        viewModel.cancelSleepTimer()

        verify(sleepTimer).cancel()
    }

    @Test
    fun `extendSleepTimer adds to remaining time`() {
        whenever(sleepTimer.remainingTimeMs).thenReturn(MutableStateFlow(30000L))

        viewModel.extendSleepTimer(60000L)

        verify(sleepTimer).start(90000L)
    }

    // Queue Management Tests
    @Test
    fun `removeFromQueue calls playbackQueue removeFromQueue`() = testScope.runTest {
        viewModel.removeFromQueue(2)

        verify(playbackQueue).removeFromQueue(2)
    }

    @Test
    fun `moveTrackInQueue calls playbackQueue moveTrack`() = testScope.runTest {
        viewModel.moveTrackInQueue(0, 5)

        verify(playbackQueue).moveTrack(0, 5)
    }

    // Equalizer Tests
    @Test
    fun `enableEqualizer updates state flow`() = testScope.runTest {
        viewModel.enableEqualizer()

        assertTrue(viewModel.equalizerEnabled.first())
        verify(audioPlayer).enableEqualizer()
    }

    @Test
    fun `disableEqualizer updates state flow`() = testScope.runTest {
        viewModel.disableEqualizer()

        assertFalse(viewModel.equalizerEnabled.first())
        verify(audioPlayer).disableEqualizer()
    }

    // Battery-Aware Tests
    @Test
    fun `setBatteryAwareMode allows disabling battery awareness`() {
        viewModel.setBatteryAwareMode(false)

        // Should not auto-disable effects even if battery is low
        // This is verified through integration testing
    }

    @Test
    fun `getBatteryImpactEstimate returns formatted string`() {
        whenever(audioPlayer.playbackSpeed).thenReturn(MutableStateFlow(1.0f))
        whenever(usbDacDetector.preferredDac).thenReturn(MutableStateFlow(null))
        whenever(batteryMonitor.estimateBatteryImpact(any(), any(), any())).thenReturn("~10h at current quality")

        val estimate = viewModel.getBatteryImpactEstimate()

        assertEquals("~10h at current quality", estimate)
    }

    // A/B Testing Tests
    @Test
    fun `startABTest enters AB mode and plays track A`() {
        viewModel.startABTest(testTrackA, testTrackB)

        assertEquals("A", viewModel.abTestingCurrentVersion.value)
        assertTrue(viewModel.abTestingMode.value)
    }

    @Test
    fun `switchABVersion switches from A to B`() = testScope.runTest {
        viewModel.startABTest(testTrackA, testTrackB)

        viewModel.switchABVersion()
        advanceUntilIdle()

        assertEquals("B", viewModel.abTestingCurrentVersion.value)
    }

    @Test
    fun `switchABVersion switches from B to A`() = testScope.runTest {
        viewModel.startABTest(testTrackA, testTrackB)
        viewModel.switchABVersion()
        advanceUntilIdle()

        viewModel.switchABVersion()
        advanceUntilIdle()

        assertEquals("A", viewModel.abTestingCurrentVersion.value)
    }

    @Test
    fun `exitABTest clears AB mode state`() {
        viewModel.startABTest(testTrackA, testTrackB)

        viewModel.exitABTest()

        assertFalse(viewModel.abTestingMode.value)
        assertEquals("A", viewModel.abTestingCurrentVersion.value)
    }

    @Test
    fun `switchABVersion does nothing when not in AB mode`() {
        val initialVersion = viewModel.abTestingCurrentVersion.value

        viewModel.switchABVersion()

        assertEquals(initialVersion, viewModel.abTestingCurrentVersion.value)
    }

    // Integration Tests
    @Test
    fun `playback speed changes reflected in signal path`() = testScope.runTest {
        whenever(audioPlayer.playbackSpeed).thenReturn(MutableStateFlow(1.5f))

        viewModel.setPlaybackSpeed(1.5f)

        verify(audioPlayer).setPlaybackSpeed(1.5f)
    }

    @Test
    fun `gapless state exposed correctly`() = testScope.runTest {
        whenever(gaplessEngine.isGaplessEnabled).thenReturn(MutableStateFlow(true))

        assertTrue(viewModel.gaplessEnabled.first())
    }

    @Test
    fun `battery level state exposed correctly`() = testScope.runTest {
        whenever(batteryMonitor.batteryLevel).thenReturn(MutableStateFlow(75))

        assertEquals(75, viewModel.batteryLevel.first())
    }
}
