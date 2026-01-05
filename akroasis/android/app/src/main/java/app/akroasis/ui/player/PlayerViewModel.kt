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
import timber.log.Timber
import javax.inject.Inject

@HiltViewModel
class PlayerViewModel @Inject constructor(
    private val audioPlayer: AudioPlayer,
    private val trackLoader: TrackLoader,
    private val musicRepository: MusicRepository,
    private val playbackQueue: PlaybackQueue,
    private val audioPreferences: app.akroasis.data.preferences.AudioPreferences,
    private val playbackSpeedPreferences: app.akroasis.data.preferences.PlaybackSpeedPreferences,
    private val gaplessEngine: app.akroasis.audio.GaplessPlaybackEngine,
    private val crossfadeEngine: app.akroasis.audio.CrossfadeEngine,
    private val usbDacDetector: app.akroasis.audio.UsbDacDetector,
    private val sleepTimer: app.akroasis.audio.SleepTimer,
    private val batteryMonitor: app.akroasis.audio.BatteryMonitor,
    val crossfeedEngine: app.akroasis.audio.CrossfeedEngine,
    val headroomManager: app.akroasis.audio.HeadroomManager,
    private val autoEQRepository: app.akroasis.data.repository.AutoEQRepository,
    private val queueExporter: app.akroasis.ui.queue.QueueExporter,
    private val mediaSessionManager: app.akroasis.audio.MediaSessionManager,
    private val notificationManager: app.akroasis.audio.PlaybackNotificationManager,
    private val playbackStateStore: app.akroasis.data.persistence.PlaybackStateStore,
    private val scrobbleManager: app.akroasis.scrobble.ScrobbleManager
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
    val pipelineState: StateFlow<app.akroasis.audio.AudioPipelineState?> = audioPlayer.pipelineState
    val connectedDacs: StateFlow<List<app.akroasis.audio.UsbDacInfo>> = usbDacDetector.connectedDacs
    val preferredDac: StateFlow<app.akroasis.audio.UsbDacInfo?> = usbDacDetector.preferredDac
    val sleepTimerActive: StateFlow<Boolean> = sleepTimer.isActive
    val sleepTimerRemaining: StateFlow<Long> = sleepTimer.remainingTimeMs

    private val _equalizerEnabled = MutableStateFlow(false)
    val equalizerEnabled: StateFlow<Boolean> = _equalizerEnabled.asStateFlow()

    val gaplessEnabled: StateFlow<Boolean> = gaplessEngine.isGaplessEnabled

    val batteryLevel: StateFlow<Int> = batteryMonitor.batteryLevel
    val isLowBattery: StateFlow<Boolean> = batteryMonitor.isLowBattery
    val isCharging: StateFlow<Boolean> = batteryMonitor.isCharging

    private var batteryAwareModeEnabled = true
    private var effectsDisabledByBattery = false

    private val _abTestingMode = MutableStateFlow(false)
    val abTestingMode: StateFlow<Boolean> = _abTestingMode.asStateFlow()

    private val _abTestingCurrentVersion = MutableStateFlow("A")
    val abTestingCurrentVersion: StateFlow<String> = _abTestingCurrentVersion.asStateFlow()

    private var abTrackA: Track? = null
    private var abTrackB: Track? = null
    private var abPositionWhenSwitched = 0L

    private var fadeOutJob: Job? = null
    @Volatile
    private var isFadingOut = false

    init {
        // Initialize media session for notification controls
        mediaSessionManager.initialize(
            onPlayPause = { playPause() },
            onSkipToNext = { skipToNext() },
            onSkipToPrevious = { skipToPrevious() },
            onSeekTo = { position -> seekTo(position) },
            onStop = { stop() }
        )

        // Start USB DAC monitoring
        usbDacDetector.startMonitoring()

        // Load saved preferences
        _equalizerEnabled.value = audioPreferences.equalizerEnabled
        if (audioPreferences.equalizerEnabled) {
            audioPlayer.enableEqualizer()
        }
        audioPlayer.setPlaybackSpeed(audioPreferences.playbackSpeed)

        if (audioPreferences.gaplessEnabled) {
            gaplessEngine.enableGapless()
        }

        // Setup sleep timer expiration callback
        sleepTimer.onTimerExpired = {
            viewModelScope.launch {
                fadeOutAndStop()
            }
        }

        // Monitor battery and disable effects when low
        viewModelScope.launch {
            batteryMonitor.isLowBattery.collect { isLow ->
                if (batteryAwareModeEnabled && isLow && !batteryMonitor.isCharging.value) {
                    if (_equalizerEnabled.value) {
                        disableEqualizer()
                        effectsDisabledByBattery = true
                    }
                } else if (effectsDisabledByBattery && !isLow) {
                    if (audioPreferences.equalizerEnabled) {
                        enableEqualizer()
                        effectsDisabledByBattery = false
                    }
                }
            }
        }

        viewModelScope.launch {
            audioPlayer.playbackState.collect { state ->
                val previousState = _uiState.value.playbackState
                _uiState.value = _uiState.value.copy(playbackState = state)

                mediaSessionManager.updatePlaybackState(
                    state = state,
                    position = _uiState.value.position,
                    speed = audioPlayer.playbackSpeed.value
                )

                playbackQueue.currentTrack?.let { track ->
                    val isPlaying = state is PlaybackState.Playing
                    if (isPlaying || state is PlaybackState.Paused) {
                        notificationManager.showNotification(track, isPlaying)
                    } else if (state is PlaybackState.Stopped) {
                        notificationManager.hideNotification()
                    }
                }

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

                // Update scrobble progress
                playbackQueue.currentTrack?.let { track ->
                    scrobbleManager.onPlaybackProgress(track, position, _uiState.value.duration)
                }
            }
        }

        // Restore previous playback state
        restorePlaybackState()

        // Save state periodically
        viewModelScope.launch {
            while (isActive) {
                delay(5000)
                savePlaybackState()
            }
        }
    }

    private fun restorePlaybackState() {
        viewModelScope.launch {
            val savedState = playbackStateStore.restoreState() ?: return@launch

            val currentTime = System.currentTimeMillis()
            val stateSavedRecently = (currentTime - savedState.timestamp) < 3600000

            if (stateSavedRecently && savedState.queue.isNotEmpty()) {
                playbackQueue.setQueue(savedState.queue, savedState.currentIndex)

                if (savedState.shuffleEnabled) {
                    playbackQueue.toggleShuffle()
                }

                savedState.currentTrack?.let { track ->
                    _uiState.value = _uiState.value.copy(
                        trackTitle = track.title,
                        trackArtist = track.artist,
                        trackAlbum = track.album,
                        duration = track.duration,
                        coverArtUrl = track.coverArtUrl,
                        position = savedState.position
                    )

                    mediaSessionManager.updateMetadata(track, track.coverArtUrl)

                    // Notify scrobble manager of restored track to prevent double-scrobbling
                    val currentSpeed = audioPreferences.playbackSpeed
                    scrobbleManager.onTrackStarted(track, currentSpeed)
                }
            }
        }
    }

    private fun savePlaybackState() {
        val currentTrack = playbackQueue.currentTrack ?: return

        val state = app.akroasis.data.persistence.PlaybackStateStore.PlaybackState(
            currentTrack = currentTrack,
            position = _uiState.value.position,
            queue = playbackQueue.tracks.value,
            currentIndex = playbackQueue.currentIndex.value,
            shuffleEnabled = playbackQueue.shuffleEnabled.value,
            repeatMode = playbackQueue.repeatMode.value.name,
            playbackSpeed = audioPlayer.playbackSpeed.value,
            timestamp = System.currentTimeMillis()
        )

        playbackStateStore.saveState(state)
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

    suspend fun removeFromQueue(index: Int) {
        playbackQueue.removeFromQueue(index)
    }

    suspend fun moveTrackInQueue(fromIndex: Int, toIndex: Int) {
        playbackQueue.moveTrack(fromIndex, toIndex)
    }

    fun undoQueueChange() {
        viewModelScope.launch {
            if (playbackQueue.undo()) {
                // Clear gapless preload to prevent orphaned AudioTrack leak
                gaplessEngine.clearPreload()
            }
        }
    }

    fun redoQueueChange() {
        viewModelScope.launch {
            if (playbackQueue.redo()) {
                // Clear gapless preload to prevent orphaned AudioTrack leak
                gaplessEngine.clearPreload()
            }
        }
    }

    val canUndoQueue: Boolean
        get() = playbackQueue.canUndo

    val canRedoQueue: Boolean
        get() = playbackQueue.canRedo

    suspend fun exportQueue(
        format: app.akroasis.ui.queue.ExportFormat,
        outputUri: android.net.Uri
    ): Result<Unit> {
        return queueExporter.exportQueue(playbackQueue.tracks.value, format, outputUri)
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

            mediaSessionManager.updateMetadata(track, track.coverArtUrl)

            val decodedResult = trackLoader.loadAndDecodeTrack(track.id)
            val decodedAudio = decodedResult.getOrNull()
            if (decodedAudio == null) {
                _uiState.value = _uiState.value.copy(
                    playbackState = PlaybackState.Stopped,
                    errorMessage = "Failed to decode track: ${decodedResult.exceptionOrNull()?.message}"
                )
                return@launch
            }

            // Restore saved playback speed for this track
            // TODO: Add albumId support when available in Track model
            val savedSpeed = playbackSpeedPreferences.getSpeedForTrack(track.id, track.album)
            audioPlayer.setPlaybackSpeed(savedSpeed)

            audioPlayer.play(decodedAudio)
            scrobbleManager.onTrackStarted(track, savedSpeed)
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
            else -> { /* No action needed for other states */ }
        }
    }

    fun stop() {
        audioPlayer.stop()
        stopPositionUpdates()
        scrobbleManager.onTrackStopped()
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

    fun setPlaybackSpeedForTrack(speed: Float, saveForTrack: Boolean = false) {
        audioPlayer.setPlaybackSpeed(speed)
        audioPreferences.playbackSpeed = speed

        if (saveForTrack) {
            playbackQueue.currentTrack?.let { track ->
                viewModelScope.launch {
                    playbackSpeedPreferences.setSpeedForTrack(track.id, speed)
                }
            }
        }
    }

    fun setPlaybackSpeedForAlbum(speed: Float) {
        audioPlayer.setPlaybackSpeed(speed)
        audioPreferences.playbackSpeed = speed

        playbackQueue.currentTrack?.let { track ->
            viewModelScope.launch {
                // TODO: Add albumId support when available in Track model
                playbackSpeedPreferences.setSpeedForAlbum(track.album, speed)
            }
        }
    }

    // Equalizer controls
    fun enableEqualizer() {
        audioPlayer.enableEqualizer()
        audioPreferences.equalizerEnabled = true
        _equalizerEnabled.value = true
    }

    fun disableEqualizer() {
        audioPlayer.disableEqualizer()
        audioPreferences.equalizerEnabled = false
        _equalizerEnabled.value = false
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

    fun saveEqualizerPreset(name: String, bandLevels: List<Short>) {
        val preset = app.akroasis.data.model.EqualizerPreset(
            name = name,
            bandLevels = bandLevels,
            isBuiltIn = false
        )
        audioPreferences.saveCustomEqualizerPreset(preset)
    }

    fun getCustomEqualizerPresets(): List<app.akroasis.data.model.EqualizerPreset> {
        return audioPreferences.getCustomEqualizerPresets()
    }

    fun loadEqualizerPreset(preset: app.akroasis.data.model.EqualizerPreset) {
        preset.bandLevels.forEachIndexed { index, level ->
            setEqualizerBandLevel(index.toShort(), level)
        }
    }

    fun deleteEqualizerPreset(name: String) {
        audioPreferences.deleteCustomEqualizerPreset(name)
    }

    fun getAvailableAutoEQProfiles(): List<app.akroasis.data.model.AutoEQProfile> {
        return autoEQRepository.getAvailableProfiles()
    }

    fun searchAutoEQProfiles(query: String): List<app.akroasis.data.model.AutoEQProfile> {
        return autoEQRepository.searchProfiles(query)
    }

    fun applyAutoEQProfile(profile: app.akroasis.data.model.AutoEQProfile) {
        val bandLevelRange = getEqualizerBandLevelRange() ?: return
        val numBands = getNumberOfBands().toInt()

        val centerFrequencies = (0 until numBands).mapNotNull { band ->
            getEqualizerCenterFreq(band.toShort())
        }

        val bandLevels = app.akroasis.audio.AutoEQConverter.convertToFixedBands(
            profile = profile,
            centerFrequencies = centerFrequencies,
            bandLevelRange = bandLevelRange
        )

        bandLevels.forEachIndexed { index, level ->
            setEqualizerBandLevel(index.toShort(), level)
        }

        if (!_equalizerEnabled.value) {
            enableEqualizer()
        }
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

    // Crossfeed (stereo DSP) controls
    fun enableCrossfeed() {
        crossfeedEngine.enable()
    }

    fun disableCrossfeed() {
        crossfeedEngine.disable()
    }

    fun setCrossfeedStrength(strength: Float) {
        crossfeedEngine.setStrength(strength)
    }

    // Headroom management controls
    fun enableHeadroom() {
        headroomManager.enable()
    }

    fun disableHeadroom() {
        headroomManager.disable()
    }

    fun setHeadroom(db: Float) {
        headroomManager.setHeadroom(db)
    }

    fun resetClippingIndicator() {
        headroomManager.resetClippingIndicator()
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

    // Battery-aware playback controls
    fun setBatteryAwareMode(enabled: Boolean) {
        batteryAwareModeEnabled = enabled
    }

    fun getBatteryImpactEstimate(): String {
        return batteryMonitor.estimateBatteryImpact(
            equalizerEnabled = _equalizerEnabled.value,
            playbackSpeed = audioPlayer.playbackSpeed.value,
            usbDacConnected = usbDacDetector.preferredDac.value != null
        )
    }

    // A/B testing mode
    fun startABTest(trackA: Track, trackB: Track) {
        abTrackA = trackA
        abTrackB = trackB
        _abTestingMode.value = true
        _abTestingCurrentVersion.value = "A"
        playTrack(trackA)
    }

    fun switchABVersion() {
        if (!_abTestingMode.value) return

        abPositionWhenSwitched = _uiState.value.position

        val nextTrack = if (_abTestingCurrentVersion.value == "A") {
            _abTestingCurrentVersion.value = "B"
            abTrackB
        } else {
            _abTestingCurrentVersion.value = "A"
            abTrackA
        }

        nextTrack?.let { track ->
            playTrack(track)
            viewModelScope.launch {
                delay(100)
                seekTo(abPositionWhenSwitched)
            }
        }
    }

    fun exitABTest() {
        _abTestingMode.value = false
        abTrackA = null
        abTrackB = null
        abPositionWhenSwitched = 0L
        _abTestingCurrentVersion.value = "A"
    }

    // Sleep timer controls
    fun startSleepTimer(durationMs: Long) {
        sleepTimer.start(durationMs)
    }

    fun startSleepTimerEndOfTrack() {
        val remainingTrackTime = _uiState.value.duration - _uiState.value.position
        if (remainingTrackTime > 0) {
            sleepTimer.start(remainingTrackTime)
        }
    }

    fun cancelSleepTimer() {
        sleepTimer.cancel()
        fadeOutJob?.cancel()
        fadeOutJob = null
        isFadingOut = false
    }

    fun extendSleepTimer(durationMs: Long) {
        val currentRemaining = sleepTimer.remainingTimeMs.value
        sleepTimer.start(currentRemaining + durationMs)
    }

    private suspend fun fadeOutAndStop() {
        if (isFadingOut) {
            Timber.d("Fadeout already in progress, skipping duplicate request")
            return
        }

        isFadingOut = true
        fadeOutJob?.cancel()
        fadeOutJob = viewModelScope.launch {
            try {
                val fadeDurationMs = 3000L
                val steps = 30
                val stepDuration = fadeDurationMs / steps
                val initialSpeed = audioPlayer.playbackSpeed.value

                for (i in steps downTo 0) {
                    val volumeFactor = i.toFloat() / steps
                    audioPlayer.setPlaybackSpeed(initialSpeed * volumeFactor.coerceAtLeast(0.1f))
                    delay(stepDuration)
                }

                audioPlayer.stop()
                audioPlayer.setPlaybackSpeed(initialSpeed)
            } finally {
                isFadingOut = false
            }
        }
    }

    // Scrobbling controls
    suspend fun authenticateLastFm(username: String, password: String): app.akroasis.scrobble.lastfm.LastFmClient.AuthResult {
        return scrobbleManager.authenticateLastFm(username, password)
    }

    fun disconnectLastFm() {
        scrobbleManager.disconnectLastFm()
    }

    fun isLastFmConnected(): Boolean {
        return scrobbleManager.isLastFmConnected()
    }

    fun getLastFmUsername(): String? {
        return scrobbleManager.getLastFmUsername()
    }

    val scrobbleState: StateFlow<app.akroasis.scrobble.ScrobbleManager.ScrobbleState> = scrobbleManager.scrobbleState

    suspend fun authenticateListenBrainz(token: String): app.akroasis.scrobble.listenbrainz.ListenBrainzClient.SubmitResult {
        return scrobbleManager.authenticateListenBrainz(token)
    }

    fun disconnectListenBrainz() {
        scrobbleManager.disconnectListenBrainz()
    }

    fun isListenBrainzConnected(): Boolean {
        return scrobbleManager.isListenBrainzConnected()
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
        try {
            savePlaybackState()
        } catch (e: Exception) {
            Timber.e(e, "Failed to save playback state on cleanup")
        } finally {
            // Ensure all resources are released even if savePlaybackState() fails
            try {
                stopPositionUpdates()
                usbDacDetector.stopMonitoring()
                gaplessEngine.release()
                crossfadeEngine.release()
                crossfeedEngine.release()
                sleepTimer.release()
                batteryMonitor.release()
                mediaSessionManager.release()
                notificationManager.hideNotification()
                fadeOutJob?.cancel()
            } catch (e: Exception) {
                Timber.e(e, "Error during resource cleanup")
            }
        }
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
