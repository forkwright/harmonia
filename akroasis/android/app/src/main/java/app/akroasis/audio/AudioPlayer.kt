// Bit-perfect audio playback engine
package app.akroasis.audio

import android.app.ActivityManager
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
import timber.log.Timber

class AudioPlayer(
    private val context: Context,
    private val equalizerEngine: EqualizerEngine,
    private val usbDacDetector: UsbDacDetector
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

    private val _pipelineState = MutableStateFlow<AudioPipelineState?>(null)
    val pipelineState: StateFlow<AudioPipelineState?> = _pipelineState.asStateFlow()

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
        Timber.d("Playing audio: ${decodedAudio.sampleRate}Hz, ${decodedAudio.bitDepth}-bit, ${decodedAudio.channels}ch")
        if (!audioFocusManager.requestAudioFocus()) {
            Timber.w("Failed to acquire audio focus")
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

        val encoding = when (decodedAudio.bitDepth) {
            16 -> AudioFormat.ENCODING_PCM_16BIT
            24 -> if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                AudioFormat.ENCODING_PCM_24BIT_PACKED
            } else {
                AudioFormat.ENCODING_PCM_16BIT
            }
            32 -> if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
                AudioFormat.ENCODING_PCM_32BIT
            } else {
                AudioFormat.ENCODING_PCM_16BIT
            }
            else -> AudioFormat.ENCODING_PCM_16BIT
        }

        val audioFormat = AudioFormat.Builder()
            .setSampleRate(decodedAudio.sampleRate)
            .setEncoding(encoding)
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

        val convertedSamples = convertSamplesForAudioTrack(decodedAudio.samples, decodedAudio.bitDepth, encoding)

        val bufferSize = convertedSamples.size
        val memoryThresholdBytes = calculateMemoryThreshold()

        try {
            val track = if (bufferSize < memoryThresholdBytes) {
                Timber.d("Using MODE_STATIC (${bufferSize / 1024}KB < ${memoryThresholdBytes / 1024 / 1024}MB threshold)")
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
                    .also { it.write(convertedSamples, 0, convertedSamples.size) }
            } else {
                // Large files: use MODE_STREAM to avoid loading entire file into memory
                val minBufferSize = AudioTrack.getMinBufferSize(
                    decodedAudio.sampleRate,
                    channelConfig,
                    encoding
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
                    .also { writeStreamingData(it, convertedSamples) }
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
                bitDepth = decodedAudio.bitDepth,
                channels = decodedAudio.channels
            )
            updatePipelineState(decodedAudio, track)
            startPositionTracking()
        } catch (e: Exception) {
            audioTrack?.release()
            audioTrack = null
            _playbackState.value = PlaybackState.Stopped
            throw e
        }
    }

    private fun writeStreamingData(track: AudioTrack, samples: ByteArray) {
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

    private fun convertSamplesForAudioTrack(samples: ByteArray, sourceBitDepth: Int, targetEncoding: Int): ByteArray {
        val sourceBytesPerSample = 4

        return when (targetEncoding) {
            AudioFormat.ENCODING_PCM_16BIT -> {
                val numSamples = samples.size / sourceBytesPerSample
                val output = ByteArray(numSamples * 2)

                for (i in 0 until numSamples) {
                    val offset = i * sourceBytesPerSample
                    val sample32 = (samples[offset].toInt() and 0xFF) or
                            ((samples[offset + 1].toInt() and 0xFF) shl 8) or
                            ((samples[offset + 2].toInt() and 0xFF) shl 16) or
                            ((samples[offset + 3].toInt() and 0xFF) shl 24)

                    val sample16 = when (sourceBitDepth) {
                        16 -> sample32.toShort()
                        24 -> (sample32 shr 8).toShort()
                        32 -> (sample32 shr 16).toShort()
                        else -> sample32.toShort()
                    }

                    output[i * 2] = (sample16.toInt() and 0xFF).toByte()
                    output[i * 2 + 1] = ((sample16.toInt() shr 8) and 0xFF).toByte()
                }
                output
            }

            AudioFormat.ENCODING_PCM_24BIT_PACKED -> {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                    val numSamples = samples.size / sourceBytesPerSample
                    val output = ByteArray(numSamples * 3)

                    for (i in 0 until numSamples) {
                        val offset = i * sourceBytesPerSample
                        val sample32 = (samples[offset].toInt() and 0xFF) or
                                ((samples[offset + 1].toInt() and 0xFF) shl 8) or
                                ((samples[offset + 2].toInt() and 0xFF) shl 16) or
                                ((samples[offset + 3].toInt() and 0xFF) shl 24)

                        val sample24 = when (sourceBitDepth) {
                            16 -> sample32 shl 8
                            24 -> sample32
                            32 -> sample32 shr 8
                            else -> sample32
                        }

                        output[i * 3] = (sample24 and 0xFF).toByte()
                        output[i * 3 + 1] = ((sample24 shr 8) and 0xFF).toByte()
                        output[i * 3 + 2] = ((sample24 shr 16) and 0xFF).toByte()
                    }
                    output
                } else {
                    convertSamplesForAudioTrack(samples, sourceBitDepth, AudioFormat.ENCODING_PCM_16BIT)
                }
            }

            AudioFormat.ENCODING_PCM_32BIT -> {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
                    samples
                } else {
                    convertSamplesForAudioTrack(samples, sourceBitDepth, AudioFormat.ENCODING_PCM_16BIT)
                }
            }

            else -> convertSamplesForAudioTrack(samples, sourceBitDepth, AudioFormat.ENCODING_PCM_16BIT)
        }
    }

    fun pause() {
        Timber.d("Pausing playback")
        audioTrack?.pause()
        _playbackState.value = PlaybackState.Paused
        stopPositionTracking()
    }

    fun resume() {
        Timber.d("Resuming playback")
        audioTrack?.play()
        _playbackState.value = PlaybackState.Playing
        startPositionTracking()
    }

    fun stop() {
        Timber.d("Stopping playback")
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
        refreshPipelineState()
    }

    fun disableEqualizer() {
        equalizerEngine.disable()
        refreshPipelineState()
    }

    fun refreshPipelineState() {
        val audio = currentAudio ?: return
        val track = audioTrack ?: return
        updatePipelineState(audio, track)
    }

    fun setEqualizerBandLevel(band: Short, level: Short) {
        equalizerEngine.setBandLevel(band, level)
        refreshPipelineState()
    }

    fun getEqualizerBandLevel(band: Short): Short {
        return equalizerEngine.getBandLevel(band)
    }

    fun getNumberOfBands(): Short {
        return equalizerEngine.getNumberOfBands()
    }

    fun applyEqualizerPreset(preset: EqualizerEngine.EqualizerPreset) {
        equalizerEngine.applyPreset(preset)
        refreshPipelineState()
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

    private fun updatePipelineState(decodedAudio: DecodedAudio, track: AudioTrack) {
        val dspChain = buildList {
            if (equalizerEngine.isEnabled()) {
                val preset = equalizerEngine.getCurrentPreset()?.name ?: "Custom"
                add(DspComponent.Equalizer(preset))
            }
        }

        _pipelineState.value = AudioPipelineState(
            inputFormat = AudioFormatInfo(
                sampleRate = decodedAudio.sampleRate,
                bitDepth = decodedAudio.bitDepth,
                channels = decodedAudio.channels
            ),
            outputFormat = AudioFormatInfo(
                sampleRate = track.sampleRate,
                bitDepth = 16,
                channels = if (track.channelCount == 1) 1 else 2
            ),
            audioPath = determineAudioPath(),
            dspChain = dspChain,
            gaplessActive = false
        )
    }

    private fun determineAudioPath(): AudioPath {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            usbDacDetector.preferredDac.value?.let { AudioPath.UsbDac(it) } ?: AudioPath.BitPerfect
        } else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            AudioPath.BitPerfect
        } else {
            AudioPath.Transparent
        }
    }

    private fun calculateMemoryThreshold(): Long {
        val activityManager = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
        val memoryClassMB = activityManager.memoryClass
        val thresholdMB = (memoryClassMB * 0.2).toLong()
        return thresholdMB * 1024 * 1024
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
