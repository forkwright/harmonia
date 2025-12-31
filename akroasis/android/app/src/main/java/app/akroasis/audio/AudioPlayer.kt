// Bit-perfect audio playback engine
package app.akroasis.audio

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.os.Build
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import java.nio.ByteBuffer
import java.nio.ByteOrder

class AudioPlayer(
    private val context: Context,
    private val equalizerEngine: EqualizerEngine
) {
    private var audioTrack: AudioTrack? = null
    private var currentAudio: DecodedAudio? = null
    private var sampleRate: Int = 0
    private var channels: Int = 0

    private val _playbackState = MutableStateFlow<PlaybackState>(PlaybackState.Stopped)
    val playbackState: StateFlow<PlaybackState> = _playbackState.asStateFlow()

    private val _position = MutableStateFlow(0L)
    val position: StateFlow<Long> = _position.asStateFlow()

    private val _playbackSpeed = MutableStateFlow(1.0f)
    val playbackSpeed: StateFlow<Float> = _playbackSpeed.asStateFlow()

    private val _audioFormat = MutableStateFlow<AudioFormatInfo?>(null)
    val audioFormat: StateFlow<AudioFormatInfo?> = _audioFormat.asStateFlow()

    private var scope: CoroutineScope? = null
    private var positionUpdateJob: Job? = null

    private val audioFocusManager = AudioFocusManager(context).apply {
        onFocusLost = { pause() }
        onFocusGained = { resume() }
    }

    fun init() {
        if (scope == null) {
            scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
        }
    }

    fun release() {
        stopPositionTracking()
        stop()
        equalizerEngine.release()
        scope?.cancel()
        scope = null
    }

    fun play(decodedAudio: DecodedAudio) {
        if (!audioFocusManager.requestAudioFocus()) {
            return
        }

        stop()

        currentAudio = decodedAudio
        sampleRate = decodedAudio.sampleRate
        channels = decodedAudio.channels

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
        val memoryThresholdBytes = 10 * 1024 * 1024  // 10MB

        try {
            val track = if (bufferSize < memoryThresholdBytes) {
                // Small files: use MODE_STATIC for better performance
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
                // Large files: use MODE_STREAM to avoid loading entire file into memory
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

            // Apply playback speed if not default
            if (_playbackSpeed.value != 1.0f && Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                val params = track.playbackParams
                params.speed = _playbackSpeed.value
                track.playbackParams = params
            }

            track.play()
            audioTrack = track

            // Attach equalizer to audio session
            equalizerEngine.attachToSession(track.audioSessionId)

            _playbackState.value = PlaybackState.Playing
            _audioFormat.value = AudioFormatInfo(
                sampleRate = decodedAudio.sampleRate,
                bitDepth = 16,
                channels = decodedAudio.channels
            )
            startPositionTracking()
        } catch (e: Exception) {
            audioTrack?.release()
            audioTrack = null
            _playbackState.value = PlaybackState.Stopped
            throw e
        }
    }

    private fun writeStreamingData(track: AudioTrack, samples: ShortArray) {
        scope?.launch(Dispatchers.IO) {
            val chunkSize = 4096
            var offset = 0
            while (offset < samples.size && _playbackState.value != PlaybackState.Stopped) {
                val length = minOf(chunkSize, samples.size - offset)
                track.write(samples, offset, length)
                offset += length
            }
        }
    }

    fun pause() {
        audioTrack?.pause()
        _playbackState.value = PlaybackState.Paused
        stopPositionTracking()
    }

    fun resume() {
        audioTrack?.play()
        _playbackState.value = PlaybackState.Playing
        startPositionTracking()
    }

    fun stop() {
        stopPositionTracking()
        audioTrack?.apply {
            stop()
            release()
        }
        audioTrack = null
        currentAudio = null
        _playbackState.value = PlaybackState.Stopped
        _position.value = 0
        audioFocusManager.abandonAudioFocus()
    }

    fun seekTo(positionMs: Long) {
        val track = audioTrack ?: return
        val audio = currentAudio ?: return

        val positionFrames = (positionMs * sampleRate / 1000).toInt()
        val positionSamples = positionFrames * channels

        if (positionSamples in 0..audio.samples.size) {
            track.setPlaybackHeadPosition(positionFrames)
            _position.value = positionMs
        }
    }

    fun setPlaybackSpeed(speed: Float) {
        if (speed !in 0.5f..2.0f) return

        _playbackSpeed.value = speed
        audioTrack?.let { track ->
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                val params = track.playbackParams
                params.speed = speed
                track.playbackParams = params
            }
        }
    }

    fun getCurrentPositionMs(): Long {
        val track = audioTrack ?: return 0L
        if (_playbackState.value == PlaybackState.Stopped) return 0L

        val headPosition = track.playbackHeadPosition
        return (headPosition * 1000L) / sampleRate
    }

    private fun startPositionTracking() {
        positionUpdateJob?.cancel()
        positionUpdateJob = scope?.launch {
            while (isActive && _playbackState.value == PlaybackState.Playing) {
                _position.value = getCurrentPositionMs()
                delay(100)

                if (audioTrack?.playState == AudioTrack.PLAYSTATE_STOPPED) {
                    _playbackState.value = PlaybackState.Stopped
                    _position.value = 0
                    break
                }
            }
        }
    }

    private fun stopPositionTracking() {
        positionUpdateJob?.cancel()
        positionUpdateJob = null
    }

    // Equalizer controls
    fun enableEqualizer() {
        equalizerEngine.enable()
    }

    fun disableEqualizer() {
        equalizerEngine.disable()
    }

    fun setEqualizerBandLevel(band: Short, level: Short) {
        equalizerEngine.setBandLevel(band, level)
    }

    fun getEqualizerBandLevel(band: Short): Short {
        return equalizerEngine.getBandLevel(band)
    }

    fun getNumberOfBands(): Short {
        return equalizerEngine.getNumberOfBands()
    }

    fun applyEqualizerPreset(preset: EqualizerEngine.EqualizerPreset) {
        equalizerEngine.applyPreset(preset)
    }

    fun getEqualizerCurrentLevels(): List<Short> {
        return equalizerEngine.getCurrentLevels()
    }

    fun getEqualizerBandFreqRange(band: Short): IntArray? {
        return equalizerEngine.getBandFreqRange(band)
    }

    fun getEqualizerCenterFreq(band: Short): Int? {
        return equalizerEngine.getCenterFreq(band)
    }

    fun getEqualizerBandLevelRange(): ShortArray? {
        return equalizerEngine.getBandLevelRange()
    }
}

sealed class PlaybackState {
    data object Stopped : PlaybackState()
    data object Playing : PlaybackState()
    data object Paused : PlaybackState()
    data object Buffering : PlaybackState()
}

data class AudioFormatInfo(
    val sampleRate: Int,
    val bitDepth: Int,
    val channels: Int
)
