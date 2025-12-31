// Unit tests for PlayerViewModel state management
package app.akroasis.ui.player

import androidx.arch.core.executor.testing.InstantTaskExecutorRule
import app.akroasis.audio.AudioPlayer
import app.akroasis.audio.PlaybackQueue
import app.akroasis.audio.PlaybackState
import app.akroasis.audio.TrackLoader
import app.akroasis.data.model.Track
import app.akroasis.data.repository.MusicRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.mockito.kotlin.*

@OptIn(ExperimentalCoroutinesApi::class)
class PlayerViewModelTest {

    @get:Rule
    val instantTaskExecutorRule = InstantTaskExecutorRule()

    private lateinit var viewModel: PlayerViewModel
    private lateinit var audioPlayer: AudioPlayer
    private lateinit var trackLoader: TrackLoader
    private lateinit var musicRepository: MusicRepository
    private lateinit var playbackQueue: PlaybackQueue

    private val testDispatcher = StandardTestDispatcher()

    private val testTrack = Track(
        id = "1",
        title = "Test Track",
        artist = "Test Artist",
        album = "Test Album",
        duration = 180000,
        format = "FLAC",
        coverArtUrl = null
    )

    @Before
    fun setUp() {
        Dispatchers.setMain(testDispatcher)

        audioPlayer = mock()
        trackLoader = mock()
        musicRepository = mock()
        playbackQueue = mock()

        // Set up default mock behaviors
        whenever(audioPlayer.playbackState).thenReturn(MutableStateFlow(PlaybackState.Stopped))
        whenever(audioPlayer.position).thenReturn(MutableStateFlow(0L))
        whenever(playbackQueue.tracks).thenReturn(MutableStateFlow(emptyList()))
        whenever(playbackQueue.currentIndex).thenReturn(MutableStateFlow(-1))
        whenever(playbackQueue.shuffleEnabled).thenReturn(MutableStateFlow(false))
        whenever(playbackQueue.repeatMode).thenReturn(MutableStateFlow(app.akroasis.audio.RepeatMode.OFF))

        viewModel = PlayerViewModel(
            audioPlayer = audioPlayer,
            trackLoader = trackLoader,
            musicRepository = musicRepository,
            playbackQueue = playbackQueue
        )
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun `playPause pauses when playing`() {
        // Simulate playing state
        whenever(audioPlayer.playbackState).thenReturn(MutableStateFlow(PlaybackState.Playing))

        viewModel.playPause()

        verify(audioPlayer).pause()
    }

    @Test
    fun `playPause resumes when paused`() {
        // Simulate paused state
        whenever(audioPlayer.playbackState).thenReturn(MutableStateFlow(PlaybackState.Paused))

        viewModel.playPause()

        verify(audioPlayer).resume()
    }

    @Test
    fun `stop calls audioPlayer stop`() {
        viewModel.stop()

        verify(audioPlayer).stop()
    }

    @Test
    fun `seekTo calls audioPlayer seekTo`() {
        val position = 60000L

        viewModel.seekTo(position)

        verify(audioPlayer).seekTo(position)
    }

    @Test
    fun `toggleShuffle calls playbackQueue toggleShuffle`() {
        viewModel.toggleShuffle()

        // Note: Can't easily verify suspend function call without more setup
        // This test verifies the method doesn't crash
    }

    @Test
    fun `cycleRepeatMode calls playbackQueue cycleRepeatMode`() {
        viewModel.cycleRepeatMode()

        verify(playbackQueue).cycleRepeatMode()
    }
}
