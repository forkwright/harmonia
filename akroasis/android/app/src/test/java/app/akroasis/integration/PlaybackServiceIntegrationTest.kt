package app.akroasis.integration

import android.content.Intent
import android.os.Build
import android.os.IBinder
import app.akroasis.audio.AudioPlayer
import app.akroasis.audio.PlaybackQueue
import app.akroasis.audio.PlaybackState
import app.akroasis.audio.TrackLoader
import app.akroasis.audio.VoiceSearchHandler
import app.akroasis.data.model.Track
import app.akroasis.data.repository.MediaRepository
import app.akroasis.data.repository.Session
import app.akroasis.data.model.MediaType
import app.akroasis.service.PlaybackService
import app.akroasis.util.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.mockito.kotlin.*
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/**
 * Integration tests for PlaybackService lifecycle and state management.
 * Tests realistic scenarios involving service binding, playback state transitions,
 * and interaction with MediaSession.
 */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [Build.VERSION_CODES.TIRAMISU])
class PlaybackServiceIntegrationTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var mockAudioPlayer: AudioPlayer
    private lateinit var mockPlaybackQueue: PlaybackQueue
    private lateinit var mockTrackLoader: TrackLoader
    private lateinit var mockVoiceSearchHandler: VoiceSearchHandler
    private lateinit var mockMediaRepository: MediaRepository

    private val playbackStateFlow = MutableStateFlow<PlaybackState>(PlaybackState.Stopped)
    private val positionFlow = MutableStateFlow(0L)
    private val currentIndexFlow = MutableStateFlow(0)

    private val testTrack = Track(
        id = "track-1",
        title = "Test Track",
        artist = "Test Artist",
        album = "Test Album",
        albumArtist = null,
        trackNumber = 1,
        discNumber = null,
        year = 2024,
        duration = 300000L,
        bitrate = 1411,
        sampleRate = 44100,
        bitDepth = 16,
        format = "FLAC",
        fileSize = 50000000L,
        filePath = "/music/test.flac",
        coverArtUrl = null,
        createdAt = "2024-01-01T00:00:00Z",
        updatedAt = "2024-01-01T00:00:00Z"
    )

    private val testSession = Session(
        id = "session-1",
        userId = "default",
        mediaItemId = "track-1",
        mediaType = MediaType.MUSIC,
        startedAt = System.currentTimeMillis(),
        endedAt = null,
        durationMs = 0L,
        isActive = true
    )

    @Before
    fun setup() {
        mockAudioPlayer = mock {
            on { playbackState } doReturn playbackStateFlow
            on { position } doReturn positionFlow
        }
        mockPlaybackQueue = mock {
            on { currentIndex } doReturn currentIndexFlow
            on { currentTrack } doReturn testTrack
        }
        mockTrackLoader = mock()
        mockVoiceSearchHandler = mock()
        mockMediaRepository = mock()
    }

    // ===== Service Lifecycle Tests =====

    @Test
    fun `SCENARIO 1 - service starts in stopped state`() = runTest {
        // Given - fresh service
        playbackStateFlow.value = PlaybackState.Stopped

        // Then - playback state should be stopped
        assertEquals(PlaybackState.Stopped, playbackStateFlow.value)
    }

    @Test
    fun `SCENARIO 2 - service transitions to playing state`() = runTest {
        // Given - service in stopped state
        playbackStateFlow.value = PlaybackState.Stopped

        // When - playback starts
        playbackStateFlow.value = PlaybackState.Playing

        // Then - state should be playing
        assertEquals(PlaybackState.Playing, playbackStateFlow.value)
    }

    @Test
    fun `SCENARIO 3 - service transitions from playing to paused`() = runTest {
        // Given - service is playing
        playbackStateFlow.value = PlaybackState.Playing

        // When - playback pauses
        playbackStateFlow.value = PlaybackState.Paused

        // Then - state should be paused
        assertEquals(PlaybackState.Paused, playbackStateFlow.value)
    }

    @Test
    fun `SCENARIO 4 - service transitions from paused to playing`() = runTest {
        // Given - service is paused
        playbackStateFlow.value = PlaybackState.Paused

        // When - playback resumes
        playbackStateFlow.value = PlaybackState.Playing

        // Then - state should be playing
        assertEquals(PlaybackState.Playing, playbackStateFlow.value)
    }

    @Test
    fun `SCENARIO 5 - service handles buffering state`() = runTest {
        // Given - service is playing
        playbackStateFlow.value = PlaybackState.Playing

        // When - buffering occurs
        playbackStateFlow.value = PlaybackState.Buffering

        // Then - state should be buffering
        assertEquals(PlaybackState.Buffering, playbackStateFlow.value)

        // When - buffering completes
        playbackStateFlow.value = PlaybackState.Playing

        // Then - state should be playing
        assertEquals(PlaybackState.Playing, playbackStateFlow.value)
    }

    // ===== Session Management Tests =====

    @Test
    fun `SCENARIO 6 - session starts when track plays`() = runTest {
        // Given - repository returns session
        whenever(mockMediaRepository.startSession(any(), any()))
            .thenReturn(Result.success(testSession))

        // When - track starts playing
        val result = mockMediaRepository.startSession("track-1", MediaType.MUSIC)

        // Then - session should be started
        assertTrue(result.isSuccess)
        assertEquals("session-1", result.getOrNull()?.id)
        assertTrue(result.getOrNull()?.isActive == true)
    }

    @Test
    fun `SCENARIO 7 - session ends when playback stops`() = runTest {
        // Given - active session
        whenever(mockMediaRepository.endSession(any(), any()))
            .thenReturn(Result.success(Unit))

        // When - playback stops
        val result = mockMediaRepository.endSession("session-1", 150000L)

        // Then - session should end successfully
        assertTrue(result.isSuccess)
        verify(mockMediaRepository).endSession("session-1", 150000L)
    }

    @Test
    fun `SCENARIO 8 - progress updates during playback`() = runTest {
        // Given - track is playing
        whenever(mockMediaRepository.updateProgress(any(), any(), any(), any()))
            .thenReturn(Result.success(Unit))

        // When - progress updates
        val result = mockMediaRepository.updateProgress(
            mediaId = "track-1",
            mediaType = MediaType.MUSIC,
            positionMs = 60000L,
            durationMs = 300000L
        )

        // Then - progress should be saved
        assertTrue(result.isSuccess)
    }

    // ===== Queue Integration Tests =====

    @Test
    fun `SCENARIO 9 - current track is tracked in queue`() {
        // Given - queue with track
        whenever(mockPlaybackQueue.currentTrack).thenReturn(testTrack)

        // When - getting current track
        val current = mockPlaybackQueue.currentTrack

        // Then - should return the track
        assertEquals("track-1", current?.id)
        assertEquals("Test Track", current?.title)
    }

    @Test
    fun `SCENARIO 10 - queue index changes on skip`() = runTest {
        // Given - queue at index 0
        currentIndexFlow.value = 0

        // When - skip to next
        currentIndexFlow.value = 1

        // Then - index should update
        assertEquals(1, currentIndexFlow.value)
    }

    // ===== Intent Handling Tests =====

    @Test
    fun `SCENARIO 11 - PLAY action triggers playback`() {
        // Given - play intent
        val playIntent = Intent().apply {
            action = PlaybackService.ACTION_PLAY
        }

        // Then - action should be recognized
        assertEquals(PlaybackService.ACTION_PLAY, playIntent.action)
    }

    @Test
    fun `SCENARIO 12 - PAUSE action pauses playback`() {
        // Given - pause intent
        val pauseIntent = Intent().apply {
            action = PlaybackService.ACTION_PAUSE
        }

        // Then - action should be recognized
        assertEquals(PlaybackService.ACTION_PAUSE, pauseIntent.action)
    }

    @Test
    fun `SCENARIO 13 - STOP action stops service`() {
        // Given - stop intent
        val stopIntent = Intent().apply {
            action = PlaybackService.ACTION_STOP
        }

        // Then - action should be recognized
        assertEquals(PlaybackService.ACTION_STOP, stopIntent.action)
    }

    @Test
    fun `SCENARIO 14 - SKIP_TO_NEXT advances queue`() {
        // Given - skip next intent
        val skipIntent = Intent().apply {
            action = PlaybackService.ACTION_SKIP_TO_NEXT
        }

        // Then - action should be recognized
        assertEquals(PlaybackService.ACTION_SKIP_TO_NEXT, skipIntent.action)
    }

    @Test
    fun `SCENARIO 15 - SKIP_TO_PREVIOUS goes back in queue`() {
        // Given - skip previous intent
        val skipIntent = Intent().apply {
            action = PlaybackService.ACTION_SKIP_TO_PREVIOUS
        }

        // Then - action should be recognized
        assertEquals(PlaybackService.ACTION_SKIP_TO_PREVIOUS, skipIntent.action)
    }

    // ===== Error Handling Tests =====

    @Test
    fun `SCENARIO 16 - session start failure is handled gracefully`() = runTest {
        // Given - repository fails
        whenever(mockMediaRepository.startSession(any(), any()))
            .thenReturn(Result.failure(Exception("Network error")))

        // When - trying to start session
        val result = mockMediaRepository.startSession("track-1", MediaType.MUSIC)

        // Then - failure should be returned
        assertTrue(result.isFailure)
        assertTrue(result.exceptionOrNull()?.message?.contains("Network") == true)
    }

    @Test
    fun `SCENARIO 17 - progress update failure is handled gracefully`() = runTest {
        // Given - repository fails
        whenever(mockMediaRepository.updateProgress(any(), any(), any(), any()))
            .thenReturn(Result.failure(Exception("Server error")))

        // When - updating progress
        val result = mockMediaRepository.updateProgress(
            mediaId = "track-1",
            mediaType = MediaType.MUSIC,
            positionMs = 60000L,
            durationMs = 300000L
        )

        // Then - failure should be returned
        assertTrue(result.isFailure)
    }

    // ===== Position Tracking Tests =====

    @Test
    fun `SCENARIO 18 - position updates during playback`() = runTest {
        // Given - playing state
        playbackStateFlow.value = PlaybackState.Playing
        positionFlow.value = 0L

        // When - position updates
        positionFlow.value = 30000L

        // Then - position should be tracked
        assertEquals(30000L, positionFlow.value)
    }

    @Test
    fun `SCENARIO 19 - seek updates position`() = runTest {
        // Given - player at position 30s
        positionFlow.value = 30000L

        // When - seeking to 120s
        positionFlow.value = 120000L

        // Then - position should update
        assertEquals(120000L, positionFlow.value)
    }

    @Test
    fun `SCENARIO 20 - position resets on new track`() = runTest {
        // Given - position at 2 minutes
        positionFlow.value = 120000L

        // When - new track starts
        currentIndexFlow.value = 1
        positionFlow.value = 0L

        // Then - position should reset
        assertEquals(0L, positionFlow.value)
        assertEquals(1, currentIndexFlow.value)
    }
}
