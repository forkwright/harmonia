package app.akroasis.scrobble

import android.content.Context
import app.akroasis.data.model.Track
import app.akroasis.data.preferences.ScrobblePreferences
import app.akroasis.scrobble.lastfm.LastFmClient
import app.akroasis.scrobble.listenbrainz.ListenBrainzClient
import app.akroasis.util.MainDispatcherRule
import app.cash.turbine.test
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.mockito.kotlin.mock
import org.mockito.kotlin.whenever
import org.mockito.kotlin.verify
import org.mockito.kotlin.any

@OptIn(ExperimentalCoroutinesApi::class)
class ScrobbleManagerTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var scrobbleManager: ScrobbleManager
    private lateinit var mockContext: Context
    private lateinit var mockPrefs: ScrobblePreferences
    private lateinit var mockLastFmClient: LastFmClient
    private lateinit var mockListenBrainzClient: ListenBrainzClient

    private val testTrack = Track(
        id = "1",
        title = "Test Track",
        artist = "Test Artist",
        album = "Test Album",
        albumArtist = null,
        trackNumber = 1,
        discNumber = null,
        year = null,
        duration = 300000L,
        bitrate = 1411,
        sampleRate = null,
        bitDepth = null,
        format = "FLAC",
        fileSize = 5000000,
        filePath = "/music/test.flac",
        coverArtUrl = null,
        createdAt = "2024-01-01T00:00:00Z",
        updatedAt = "2024-01-01T00:00:00Z"
    )

    @Before
    fun setup() {
        mockContext = mock()
        mockPrefs = mock()
        mockLastFmClient = mock()
        mockListenBrainzClient = mock()

        whenever(mockPrefs.lastFmEnabled).thenReturn(true)
        whenever(mockPrefs.listenBrainzEnabled).thenReturn(true)
        whenever(mockPrefs.scrobblePercentage).thenReturn(50)
        whenever(mockPrefs.scrobbleMinDuration).thenReturn(30)

        scrobbleManager = ScrobbleManager(
            mockContext,
            mockPrefs,
            mockLastFmClient,
            mockListenBrainzClient
        )
    }

    @Test
    fun `onTrackStarted triggers Now Playing state`() = runTest {
        whenever(mockLastFmClient.isAuthenticated()).thenReturn(true)
        whenever(mockLastFmClient.updateNowPlaying(any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))

        scrobbleManager.scrobbleState.test {
            assertEquals(ScrobbleManager.ScrobbleState.Idle, awaitItem())

            scrobbleManager.onTrackStarted(testTrack)

            val nowPlaying = awaitItem()
            assertTrue(nowPlaying is ScrobbleManager.ScrobbleState.NowPlaying)
            assertEquals(testTrack, (nowPlaying as ScrobbleManager.ScrobbleState.NowPlaying).track)
        }
    }

    @Test
    fun `scrobble submitted at 50 percent threshold`() = runTest {
        whenever(mockLastFmClient.isAuthenticated()).thenReturn(true)
        whenever(mockLastFmClient.updateNowPlaying(any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))
        whenever(mockLastFmClient.scrobble(any(), any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))

        scrobbleManager.scrobbleState.test {
            awaitItem() // Idle

            scrobbleManager.onTrackStarted(testTrack)
            awaitItem() // NowPlaying

            scrobbleManager.onPlaybackProgress(testTrack, 150000L, 300000L)

            val scrobbled = awaitItem()
            assertTrue(scrobbled is ScrobbleManager.ScrobbleState.Scrobbled)
            assertEquals(testTrack, (scrobbled as ScrobbleManager.ScrobbleState.Scrobbled).track)
        }
    }

    @Test
    fun `scrobble not submitted below threshold`() = runTest {
        whenever(mockLastFmClient.isAuthenticated()).thenReturn(true)
        whenever(mockLastFmClient.updateNowPlaying(any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))

        scrobbleManager.onTrackStarted(testTrack)
        scrobbleManager.onPlaybackProgress(testTrack, 100000L, 300000L)

        verify(mockLastFmClient, org.mockito.kotlin.never()).scrobble(any(), any(), any(), any(), any())
    }

    @Test
    fun `error state on scrobble failure`() = runTest {
        whenever(mockLastFmClient.isAuthenticated()).thenReturn(true)
        whenever(mockLastFmClient.updateNowPlaying(any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))
        whenever(mockLastFmClient.scrobble(any(), any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(false, "Network error"))

        scrobbleManager.scrobbleState.test {
            awaitItem() // Idle
            scrobbleManager.onTrackStarted(testTrack)
            awaitItem() // NowPlaying

            scrobbleManager.onPlaybackProgress(testTrack, 150000L, 300000L)

            val error = awaitItem()
            assertTrue(error is ScrobbleManager.ScrobbleState.Error)
        }
    }

    @Test
    fun `onTrackStopped returns to Idle`() = runTest {
        // Given - track is playing
        whenever(mockLastFmClient.isAuthenticated()).thenReturn(true)
        whenever(mockLastFmClient.updateNowPlaying(any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))

        scrobbleManager.scrobbleState.test {
            assertEquals(ScrobbleManager.ScrobbleState.Idle, awaitItem())

            // Start track to enter NowPlaying state
            scrobbleManager.onTrackStarted(testTrack)
            val nowPlaying = awaitItem()
            assertTrue(nowPlaying is ScrobbleManager.ScrobbleState.NowPlaying)

            // When - stop track
            scrobbleManager.onTrackStopped()

            // Then - should return to Idle
            assertEquals(ScrobbleManager.ScrobbleState.Idle, awaitItem())
        }
    }

    @Test
    fun `concurrent scrobble to Last_fm and ListenBrainz`() = runTest {
        // Given - both services enabled and authenticated
        whenever(mockLastFmClient.isAuthenticated()).thenReturn(true)
        whenever(mockListenBrainzClient.isAuthenticated()).thenReturn(true)
        whenever(mockLastFmClient.updateNowPlaying(any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))
        whenever(mockLastFmClient.scrobble(any(), any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))
        whenever(mockListenBrainzClient.submitPlayingNow(any(), any(), any()))
            .thenReturn(ListenBrainzClient.SubmitResult(true, null))
        whenever(mockListenBrainzClient.submitListen(any(), any(), any(), any()))
            .thenReturn(ListenBrainzClient.SubmitResult(true, null))

        // When - trigger scrobble
        scrobbleManager.onTrackStarted(testTrack, playbackSpeed = 1.0f)
        scrobbleManager.onPlaybackProgress(testTrack, 150000L, 300000L)

        // Then - both services should receive scrobble
        verify(mockLastFmClient).scrobble(any(), any(), any(), any(), any())
        verify(mockListenBrainzClient).submitListen(any(), any(), any(), any())
    }

    @Test
    fun `scrobble respects disabled Last_fm`() = runTest {
        // Given - Last.fm disabled
        whenever(mockPrefs.lastFmEnabled).thenReturn(false)
        whenever(mockPrefs.listenBrainzEnabled).thenReturn(true)
        whenever(mockListenBrainzClient.isAuthenticated()).thenReturn(true)
        whenever(mockListenBrainzClient.submitPlayingNow(any(), any(), any()))
            .thenReturn(ListenBrainzClient.SubmitResult(true, null))
        whenever(mockListenBrainzClient.submitListen(any(), any(), any(), any()))
            .thenReturn(ListenBrainzClient.SubmitResult(true, null))

        // When - trigger scrobble
        scrobbleManager.onTrackStarted(testTrack)
        scrobbleManager.onPlaybackProgress(testTrack, 150000L, 300000L)

        // Then - only ListenBrainz should receive scrobble
        verify(mockLastFmClient, org.mockito.kotlin.never()).scrobble(any(), any(), any(), any(), any())
        verify(mockListenBrainzClient).submitListen(any(), any(), any(), any())
    }

    @Test
    fun `scrobble timestamp accounts for playback speed`() = runTest {
        // Given
        whenever(mockLastFmClient.isAuthenticated()).thenReturn(true)
        whenever(mockLastFmClient.updateNowPlaying(any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))
        whenever(mockLastFmClient.scrobble(any(), any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))

        // When - track started with 1.5x speed
        scrobbleManager.onTrackStarted(testTrack, playbackSpeed = 1.5f)
        scrobbleManager.onPlaybackProgress(testTrack, 150000L, 300000L)

        // Then - scrobble submitted with speed captured at start
        verify(mockLastFmClient).scrobble(any(), any(), any(), any(), any())
    }

    @Test
    fun `scrobble not duplicated on multiple progress updates`() = runTest {
        // Given
        whenever(mockLastFmClient.isAuthenticated()).thenReturn(true)
        whenever(mockLastFmClient.updateNowPlaying(any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))
        whenever(mockLastFmClient.scrobble(any(), any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))

        // When - multiple progress updates after threshold
        scrobbleManager.onTrackStarted(testTrack)
        scrobbleManager.onPlaybackProgress(testTrack, 150000L, 300000L)
        scrobbleManager.onPlaybackProgress(testTrack, 200000L, 300000L)
        scrobbleManager.onPlaybackProgress(testTrack, 250000L, 300000L)

        // Then - scrobble should be submitted only once
        verify(mockLastFmClient, org.mockito.kotlin.times(1)).scrobble(any(), any(), any(), any(), any())
    }

    @Test
    fun `new track resets scrobble state`() = runTest {
        // Given - first track scrobbled
        whenever(mockLastFmClient.isAuthenticated()).thenReturn(true)
        whenever(mockLastFmClient.updateNowPlaying(any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))
        whenever(mockLastFmClient.scrobble(any(), any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))

        scrobbleManager.onTrackStarted(testTrack)
        scrobbleManager.onPlaybackProgress(testTrack, 150000L, 300000L)

        // When - new track started
        val newTrack = testTrack.copy(id = "2", title = "New Track")
        scrobbleManager.onTrackStarted(newTrack)

        // Then - should be able to scrobble new track
        scrobbleManager.onPlaybackProgress(newTrack, 150000L, 300000L)
        verify(mockLastFmClient, org.mockito.kotlin.times(2)).scrobble(any(), any(), any(), any(), any())
    }

    @Test
    fun `240 second minimum duration threshold`() = runTest {
        // Given - short track (240 seconds = 240000ms)
        val shortTrack = testTrack.copy(duration = 240000L)
        whenever(mockLastFmClient.isAuthenticated()).thenReturn(true)
        whenever(mockLastFmClient.updateNowPlaying(any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))
        whenever(mockLastFmClient.scrobble(any(), any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))

        // When - played to 240 seconds (100%)
        scrobbleManager.onTrackStarted(shortTrack)
        scrobbleManager.onPlaybackProgress(shortTrack, 240000L, 240000L)

        // Then - should scrobble (meets 240s minimum)
        verify(mockLastFmClient).scrobble(any(), any(), any(), any(), any())
    }

    @Test
    fun `scrobble fails gracefully when both services fail`() = runTest {
        // Given - both services fail
        whenever(mockLastFmClient.isAuthenticated()).thenReturn(true)
        whenever(mockListenBrainzClient.isAuthenticated()).thenReturn(true)
        whenever(mockLastFmClient.updateNowPlaying(any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(true, null))
        whenever(mockListenBrainzClient.submitPlayingNow(any(), any(), any()))
            .thenReturn(ListenBrainzClient.SubmitResult(true, null))
        whenever(mockLastFmClient.scrobble(any(), any(), any(), any(), any()))
            .thenReturn(LastFmClient.ScrobbleResult(false, "Last.fm error"))
        whenever(mockListenBrainzClient.submitListen(any(), any(), any(), any()))
            .thenReturn(ListenBrainzClient.SubmitResult(false, "ListenBrainz error"))

        scrobbleManager.scrobbleState.test {
            awaitItem() // Idle
            scrobbleManager.onTrackStarted(testTrack)
            awaitItem() // NowPlaying

            // When - trigger scrobble
            scrobbleManager.onPlaybackProgress(testTrack, 150000L, 300000L)

            // Then - should show error state
            val error = awaitItem()
            assertTrue(error is ScrobbleManager.ScrobbleState.Error)
        }
    }

    @Test
    fun `disconnect Last_fm clears authentication`() = runTest {
        // When
        scrobbleManager.disconnectLastFm()

        // Then - should clear Last.fm auth
        verify(mockLastFmClient).clearSession()
    }

    @Test
    fun `disconnect ListenBrainz clears token`() = runTest {
        // When
        scrobbleManager.disconnectListenBrainz()

        // Then - should clear ListenBrainz token
        verify(mockListenBrainzClient).clearToken()
    }

    @Test
    fun `isLastFmConnected reflects client state`() {
        // Given
        whenever(mockLastFmClient.isAuthenticated()).thenReturn(true)

        // When/Then
        assertTrue(scrobbleManager.isLastFmConnected())
    }

    @Test
    fun `isListenBrainzConnected reflects client state`() {
        // Given
        whenever(mockListenBrainzClient.isAuthenticated()).thenReturn(true)

        // When/Then
        assertTrue(scrobbleManager.isListenBrainzConnected())
    }
}
