// Gapless playback with dual AudioTrack architecture
package app.akroasis.audio

import android.app.ActivityManager
import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFormat
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
    private val equalizerEngine: EqualizerEngine
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

        val track = createAndConfigureTrack(decodedAudio, playbackSpeed)

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
                val nextTrack = createAndConfigureTrack(decodedAudio, playbackSpeed)

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

    private fun createAndConfigureTrack(
        decodedAudio: DecodedAudio,
        playbackSpeed: Float
    ): AudioTrack? {
        currentSampleRate = decodedAudio.sampleRate
        currentChannels = decodedAudio.channels

        val channelConfig = if (decodedAudio.channels == 1) {
            AudioFormat.CHANNEL_OUT_MONO
        } else {
            AudioFormat.CHANNEL_OUT_STEREO
        }

        val audioFormat = AudioFormat.Builder()
            .setSampleRate(decodedAudio.sampleRate)
            .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
            .setChannelMask(channelConfig)
            .build()

        val audioAttributes = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_MEDIA)
            .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
            .apply {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                    setAllowedCapturePolicy(AudioAttributes.ALLOW_CAPTURE_BY_NONE)
                }
            }
            .build()

        val bufferSize = decodedAudio.samples.size * 2
        val memoryThresholdBytes = calculateMemoryThreshold()

        return try {
            val track = if (bufferSize < memoryThresholdBytes) {
                AudioTrack.Builder()
                    .setAudioAttributes(audioAttributes)
                    .setAudioFormat(audioFormat)
                    .setBufferSizeInBytes(bufferSize)
                    .setTransferMode(AudioTrack.MODE_STATIC)
                    .apply {
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                            setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
                        }
                    }
                    .build()
                    .also { it.write(decodedAudio.samples, 0, decodedAudio.samples.size) }
            } else {
                val minBufferSize = AudioTrack.getMinBufferSize(
                    decodedAudio.sampleRate,
                    channelConfig,
                    AudioFormat.ENCODING_PCM_16BIT
                )
                AudioTrack.Builder()
                    .setAudioAttributes(audioAttributes)
                    .setAudioFormat(audioFormat)
                    .setBufferSizeInBytes(minBufferSize * 4)
                    .setTransferMode(AudioTrack.MODE_STREAM)
                    .apply {
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                            setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
                        }
                    }
                    .build()
                    .also { writeStreamingData(it, decodedAudio.samples) }
            }

            if (playbackSpeed != 1.0f && Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                val params = track.playbackParams
                params.speed = playbackSpeed
                track.playbackParams = params
            }

            track
        } catch (e: Exception) {
            null
        }
    }

    private fun writeStreamingData(track: AudioTrack, samples: ByteArray) {
        scope.launch(Dispatchers.IO) {
            val chunkSize = 4096
            var offset = 0
            while (offset < samples.size) {
                val length = minOf(chunkSize, samples.size - offset)
                track.write(samples, offset, length)
                offset += length
            }
        }
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

    private fun calculateMemoryThreshold(): Long {
        val activityManager = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
        val memoryClassMB = activityManager.memoryClass
        val thresholdMB = (memoryClassMB * 0.2).toLong()
        return thresholdMB * 1024 * 1024
    }
}

data class GapMeasurement(
    val gapMs: Float,
    val timestamp: Long
)
