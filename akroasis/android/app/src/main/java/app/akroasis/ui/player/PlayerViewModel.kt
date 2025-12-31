// Music player UI state and playback control
package app.akroasis.ui.player

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.akroasis.audio.AudioPlayer
import app.akroasis.audio.PlaybackQueue
import app.akroasis.audio.PlaybackState
import app.akroasis.audio.RepeatMode
import app.akroasis.audio.TrackLoader
import app.akroasis.data.model.Track
import app.akroasis.data.repository.MusicRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import javax.inject.Inject

@HiltViewModel
class PlayerViewModel @Inject constructor(
    private val audioPlayer: AudioPlayer,
    private val trackLoader: TrackLoader,
    private val musicRepository: MusicRepository,
    private val playbackQueue: PlaybackQueue,
    private val audioPreferences: app.akroasis.data.preferences.AudioPreferences,
    private val gaplessEngine: app.akroasis.audio.GaplessPlaybackEngine,
    private val crossfadeEngine: app.akroasis.audio.CrossfadeEngine,
    private val usbDacDetector: app.akroasis.audio.UsbDacDetector
) : ViewModel() {

    private val _uiState = MutableStateFlow(PlayerUiState())
    val uiState: StateFlow<PlayerUiState> = _uiState.asStateFlow()

    private var positionUpdateJob: Job? = null
    private var wasPlayingBeforeCompletion = false

    val queue: StateFlow<List<Track>> = playbackQueue.tracks
    val currentIndex: StateFlow<Int> = playbackQueue.currentIndex
    val shuffleEnabled: StateFlow<Boolean> = playbackQueue.shuffleEnabled
    val repeatMode: StateFlow<RepeatMode> = playbackQueue.repeatMode
    val playbackSpeed: StateFlow<Float> = audioPlayer.playbackSpeed
    val audioFormat: StateFlow<app.akroasis.audio.AudioFormatInfo?> = audioPlayer.audioFormat
    val connectedDacs: StateFlow<List<app.akroasis.audio.UsbDacInfo>> = usbDacDetector.connectedDacs
    val preferredDac: StateFlow<app.akroasis.audio.UsbDacInfo?> = usbDacDetector.preferredDac

    init {
        // Start USB DAC monitoring
        usbDacDetector.startMonitoring()

        // Load saved preferences
        if (audioPreferences.equalizerEnabled) {
            audioPlayer.enableEqualizer()
        }
        audioPlayer.setPlaybackSpeed(audioPreferences.playbackSpeed)

        if (audioPreferences.gaplessEnabled) {
            gaplessEngine.enableGapless()
        }

        viewModelScope.launch {
            audioPlayer.playbackState.collect { state ->
                val previousState = _uiState.value.playbackState
                _uiState.value = _uiState.value.copy(playbackState = state)

                when (state) {
                    is PlaybackState.Playing -> startPositionUpdates()
                    is PlaybackState.Stopped -> {
                        stopPositionUpdates()
                        if (previousState is PlaybackState.Playing && wasPlayingBeforeCompletion) {
                            wasPlayingBeforeCompletion = false
                            skipToNext()
                        }
                    }
                    else -> stopPositionUpdates()
                }
            }
        }

        viewModelScope.launch {
            audioPlayer.position.collect { position ->
                _uiState.value = _uiState.value.copy(position = position)

                if (_uiState.value.duration > 0 &&
                    position >= _uiState.value.duration - 500 &&
                    _uiState.value.playbackState is PlaybackState.Playing
                ) {
                    wasPlayingBeforeCompletion = true
                }
            }
        }
    }

    fun playTracks(tracks: List<Track>, startIndex: Int = 0) {
        viewModelScope.launch {
            playbackQueue.setQueue(tracks, startIndex)
            playbackQueue.currentTrack?.let { track ->
                loadAndPlayTrack(track)
            }
        }
    }

    fun playTrack(track: Track) {
        viewModelScope.launch {
            playbackQueue.setQueue(listOf(track), 0)
            loadAndPlayTrack(track)
        }
    }

    fun skipToNext() {
        viewModelScope.launch {
            playbackQueue.skipToNext()?.let { track ->
                loadAndPlayTrack(track)
            }
        }
    }

    fun skipToPrevious() {
        viewModelScope.launch {
            if (_uiState.value.position > 3000) {
                seekTo(0)
            } else {
                playbackQueue.skipToPrevious()?.let { track ->
                    loadAndPlayTrack(track)
                }
            }
        }
    }

    fun skipToIndex(index: Int) {
        viewModelScope.launch {
            playbackQueue.skipToIndex(index)?.let { track ->
                loadAndPlayTrack(track)
            }
        }
    }

    fun toggleShuffle() {
        viewModelScope.launch {
            playbackQueue.toggleShuffle()
        }
    }

    fun cycleRepeatMode() {
        playbackQueue.cycleRepeatMode()
    }

    fun addToQueue(track: Track) {
        viewModelScope.launch {
            playbackQueue.addToQueue(track)
        }
    }

    fun addNextInQueue(track: Track) {
        viewModelScope.launch {
            playbackQueue.addNextInQueue(track)
        }
    }

    private fun loadAndPlayTrack(track: Track) {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(
                playbackState = PlaybackState.Buffering,
                errorMessage = null,
                trackTitle = track.title,
                trackArtist = track.artist,
                trackAlbum = track.album,
                duration = track.duration,
                coverArtUrl = track.coverArtUrl
            )

            val decodedResult = trackLoader.loadAndDecodeTrack(track.id)
            val decodedAudio = decodedResult.getOrNull()
            if (decodedAudio == null) {
                _uiState.value = _uiState.value.copy(
                    playbackState = PlaybackState.Stopped,
                    errorMessage = "Failed to decode track: ${decodedResult.exceptionOrNull()?.message}"
                )
                return@launch
            }

            audioPlayer.play(decodedAudio)
        }
    }

    fun playPause() {
        when (_uiState.value.playbackState) {
            is PlaybackState.Playing -> audioPlayer.pause()
            is PlaybackState.Paused -> audioPlayer.resume()
            is PlaybackState.Stopped -> {
                playbackQueue.currentTrack?.let { track ->
                    loadAndPlayTrack(track)
                }
            }
            else -> {}
        }
    }

    fun stop() {
        audioPlayer.stop()
        stopPositionUpdates()
    }

    fun seekTo(positionMs: Long) {
        audioPlayer.seekTo(positionMs)
    }

    fun retryLoad() {
        playbackQueue.currentTrack?.let { track ->
            loadAndPlayTrack(track)
        }
    }

    fun setPlaybackSpeed(speed: Float) {
        audioPlayer.setPlaybackSpeed(speed)
        audioPreferences.playbackSpeed = speed
    }

    // Equalizer controls
    fun enableEqualizer() {
        audioPlayer.enableEqualizer()
        audioPreferences.equalizerEnabled = true
    }

    fun disableEqualizer() {
        audioPlayer.disableEqualizer()
        audioPreferences.equalizerEnabled = false
    }

    fun applyEqualizerPreset(preset: app.akroasis.audio.EqualizerEngine.EqualizerPreset) {
        audioPlayer.applyEqualizerPreset(preset)
        audioPreferences.equalizerPreset = preset.name
    }

    fun setEqualizerBandLevel(band: Short, level: Short) {
        audioPlayer.setEqualizerBandLevel(band, level)
    }

    fun getEqualizerBandLevel(band: Short): Short {
        return audioPlayer.getEqualizerBandLevel(band)
    }

    fun getNumberOfBands(): Short {
        return audioPlayer.getNumberOfBands()
    }

    fun getEqualizerCurrentLevels(): List<Short> {
        return audioPlayer.getEqualizerCurrentLevels()
    }

    fun getEqualizerBandFreqRange(band: Short): IntArray? {
        return audioPlayer.getEqualizerBandFreqRange(band)
    }

    fun getEqualizerCenterFreq(band: Short): Int? {
        return audioPlayer.getEqualizerCenterFreq(band)
    }

    fun getEqualizerBandLevelRange(): ShortArray? {
        return audioPlayer.getEqualizerBandLevelRange()
    }

    // Gapless playback controls
    fun enableGapless() {
        audioPreferences.gaplessEnabled = true
    }

    fun disableGapless() {
        audioPreferences.gaplessEnabled = false
    }

    // Crossfade controls
    fun setCrossfadeDuration(durationMs: Int) {
        audioPreferences.crossfadeDuration = durationMs
    }

    fun getCrossfadeDuration(): Int {
        return audioPreferences.crossfadeDuration
    }

    // USB DAC controls
    fun setPreferredDac(dac: app.akroasis.audio.UsbDacInfo?) {
        usbDacDetector.setPreferredDac(dac)
    }

    fun scanForUsbDacs() {
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.M) {
            usbDacDetector.scanForUsbDacs()
        }
    }

    private fun startPositionUpdates() {
        positionUpdateJob?.cancel()
        positionUpdateJob = viewModelScope.launch {
            while (isActive) {
                delay(100)
            }
        }
    }

    private fun stopPositionUpdates() {
        positionUpdateJob?.cancel()
        positionUpdateJob = null
    }

    override fun onCleared() {
        super.onCleared()
        stopPositionUpdates()
        usbDacDetector.stopMonitoring()
        gaplessEngine.release()
        crossfadeEngine.release()
    }
}

data class PlayerUiState(
    val playbackState: PlaybackState = PlaybackState.Stopped,
    val trackTitle: String = "No Track Playing",
    val trackArtist: String = "",
    val trackAlbum: String = "",
    val position: Long = 0,
    val duration: Long = 0,
    val coverArtUrl: String? = null,
    val errorMessage: String? = null
)
