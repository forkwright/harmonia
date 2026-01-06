// Gapless playback with dual AudioTrack architecture
package app.akroasis.audio

import android.content.Context
import android.media.AudioTrack
import android.os.Build
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class GaplessPlaybackEngine @Inject constructor(
    @ApplicationContext private val context: Context,
    private val equalizerEngine: EqualizerEngine,
    private val audioTrackFactory: AudioTrackFactory
) {
    private var primaryTrack: AudioTrack? = null
    private var secondaryTrack: AudioTrack? = null
    private var isPrimaryActive = true

    @Volatile
    private var preloadReady = false
    private val preloadMutex = Mutex()

    private var currentSampleRate: Int = 0
    private var currentChannels: Int = 0

    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())

    private val _isGaplessEnabled = MutableStateFlow(true)
    val isGaplessEnabled: StateFlow<Boolean> = _isGaplessEnabled.asStateFlow()

    private val _currentTrackIndex = MutableStateFlow(0)
    val currentTrackIndex: StateFlow<Int> = _currentTrackIndex.asStateFlow()

    private val _gapMeasurements = MutableStateFlow<List<GapMeasurement>>(emptyList())
    val gapMeasurements: StateFlow<List<GapMeasurement>> = _gapMeasurements.asStateFlow()

    fun enableGapless() {
        Timber.d("Gapless playback enabled")
        _isGaplessEnabled.value = true
    }

    fun disableGapless() {
        Timber.d("Gapless playback disabled")
        _isGaplessEnabled.value = false
        releaseSecondaryTrack()
    }

    fun playTrack(decodedAudio: DecodedAudio, playbackSpeed: Float = 1.0f): AudioTrack? {
        Timber.d("Playing track on ${if (isPrimaryActive) "primary" else "secondary"} (${decodedAudio.sampleRate}Hz, ${decodedAudio.bitDepth}-bit)")
        preloadReady = false  // Playing new track invalidates any preload
        val activeTrack = if (isPrimaryActive) primaryTrack else secondaryTrack

        activeTrack?.apply {
            stop()
            release()
        }

        currentSampleRate = decodedAudio.sampleRate
        currentChannels = decodedAudio.channels
        val track = audioTrackFactory.createAudioTrack(decodedAudio, playbackSpeed)

        if (isPrimaryActive) {
            primaryTrack = track
        } else {
            secondaryTrack = track
        }

        track?.let {
            equalizerEngine.attachToSession(it.audioSessionId)
            it.play()
        }

        _currentTrackIndex.value += 1
        return track
    }

    fun preloadNextTrack(decodedAudio: DecodedAudio, playbackSpeed: Float = 1.0f) {
        if (!_isGaplessEnabled.value) return

        preloadReady = false
        Timber.d("Preloading next track on ${if (isPrimaryActive) "secondary" else "primary"}")
        scope.launch {
            preloadMutex.withLock {
                val nextTrack = audioTrackFactory.createAudioTrack(decodedAudio, playbackSpeed)

                if (isPrimaryActive) {
                    secondaryTrack?.release()
                    secondaryTrack = nextTrack
                } else {
                    primaryTrack?.release()
                    primaryTrack = nextTrack
                }
                preloadReady = true
                Timber.d("Preload complete, ready for gapless switch")
            }
        }
    }

    fun switchToPreloadedTrack() {
        if (!_isGaplessEnabled.value) return

        // Use tryLock to avoid blocking - if preload is still in progress, skip gapless
        if (!preloadMutex.tryLock()) {
            Timber.w("Gapless switch requested but preload in progress - skipping gapless transition")
            return
        }

        try {
            if (!preloadReady) {
                Timber.w("Gapless switch requested but preload not ready - skipping gapless transition")
                return
            }

            val nextTrack = if (isPrimaryActive) secondaryTrack else primaryTrack
            if (nextTrack == null) {
                Timber.w("Gapless switch requested but next track is null - preload may have failed")
                return
            }

            Timber.i("Gapless switch: ${if (isPrimaryActive) "primary" else "secondary"} → ${if (isPrimaryActive) "secondary" else "primary"}")
            val switchStartTime = System.nanoTime()
            val currentTrack = if (isPrimaryActive) primaryTrack else secondaryTrack

            currentTrack?.stop()

            equalizerEngine.attachToSession(nextTrack.audioSessionId)
            nextTrack.play()

            val gapMs = (System.nanoTime() - switchStartTime) / 1_000_000f

            _gapMeasurements.value += GapMeasurement(
                gapMs = gapMs,
                timestamp = System.currentTimeMillis()
            )

            isPrimaryActive = !isPrimaryActive
            _currentTrackIndex.value += 1
            preloadReady = false
        } finally {
            preloadMutex.unlock()
        }
    }

    fun clearGapMeasurements() {
        _gapMeasurements.value = emptyList()
    }

    fun clearPreload() {
        preloadReady = false
        releaseSecondaryTrack()
    }

    fun getActiveTrack(): AudioTrack? {
        return if (isPrimaryActive) primaryTrack else secondaryTrack
    }

    fun pause() {
        getActiveTrack()?.pause()
    }

    fun resume() {
        getActiveTrack()?.play()
    }

    fun stop() {
        primaryTrack?.apply {
            stop()
            release()
        }
        secondaryTrack?.apply {
            stop()
            release()
        }
        primaryTrack = null
        secondaryTrack = null
        isPrimaryActive = true
    }

    fun seekTo(positionFrames: Int) {
        getActiveTrack()?.setPlaybackHeadPosition(positionFrames)
    }

    fun setPlaybackSpeed(speed: Float) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            getActiveTrack()?.let { track ->
                val params = track.playbackParams
                params.speed = speed
                track.playbackParams = params
            }
        }
    }

    fun release() {
        stop()
        // Cancel all coroutines asynchronously (no blocking)
        scope.cancel()
    }

    private fun releaseSecondaryTrack() {
        val inactiveTrack = if (isPrimaryActive) secondaryTrack else primaryTrack
        inactiveTrack?.apply {
            stop()
            release()
        }
        if (isPrimaryActive) {
            secondaryTrack = null
        } else {
            primaryTrack = null
        }
    }
}

data class GapMeasurement(
    val gapMs: Float,
    val timestamp: Long
)
