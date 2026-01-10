// Bit-perfect audio playback engine
package app.akroasis.audio

import android.app.ActivityManager
import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.media.audiofx.Visualizer
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
    private val usbDacDetector: UsbDacDetector,
    private val levelMatcher: LevelMatcher
) {
    private var audioTrack: AudioTrack? = null
    private var currentAudio: DecodedAudio? = null
    private var currentSourceCodec: String? = null
    private var sampleRate: Int = 0
    private var channels: Int = 0
    private var visualizer: Visualizer? = null
    private var currentAbVersion: String = "A"

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

    fun getLevelMatcher(): LevelMatcher = levelMatcher

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
        stopRmsMeasurement()
        stop()
        equalizerEngine.release()
        scope?.cancel()
        scope = null
    }

    fun setSourceCodec(codec: String?) {
        currentSourceCodec = codec
    }

    fun play(decodedAudio: DecodedAudio) {
        Timber.d("Playing audio: ${decodedAudio.sampleRate}Hz, ${decodedAudio.bitDepth}-bit, ${decodedAudio.channels}ch (source: $currentSourceCodec)")
        if (!audioFocusManager.requestAudioFocus()) {
            Timber.w("Failed to acquire audio focus")
            return
        }

        stop()
        currentAudio = decodedAudio
        sampleRate = decodedAudio.sampleRate
        channels = decodedAudio.channels

        val channelConfig = getChannelConfig(decodedAudio.channels)
        val encoding = getEncoding(decodedAudio.bitDepth)
        val audioFormat = buildAudioFormat(decodedAudio.sampleRate, encoding, channelConfig)
        val audioAttributes = buildAudioAttributes()
        val convertedSamples = convertSamplesForAudioTrack(decodedAudio.samples, decodedAudio.bitDepth, encoding)

        try {
            val track = createAudioTrack(audioAttributes, audioFormat, convertedSamples, decodedAudio, channelConfig, encoding)
            applyPlaybackSpeed(track)
            track.play()
            audioTrack = track

            equalizerEngine.attachToSession(track.audioSessionId)
            startRmsMeasurement(track.audioSessionId)

            _playbackState.value = PlaybackState.Playing
            updatePipelineState(decodedAudio, track)
            _audioFormat.value = _pipelineState.value?.inputFormat
            startPositionTracking()
        } catch (e: Exception) {
            audioTrack?.release()
            audioTrack = null
            _playbackState.value = PlaybackState.Stopped
            throw e
        }
    }

    private fun getChannelConfig(channelCount: Int): Int {
        return if (channelCount == 1) AudioFormat.CHANNEL_OUT_MONO else AudioFormat.CHANNEL_OUT_STEREO
    }

    private fun getEncoding(bitDepth: Int): Int {
        return when (bitDepth) {
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
    }

    private fun buildAudioFormat(sampleRate: Int, encoding: Int, channelConfig: Int): AudioFormat {
        return AudioFormat.Builder()
            .setSampleRate(sampleRate)
            .setEncoding(encoding)
            .setChannelMask(channelConfig)
            .build()
    }

    private fun buildAudioAttributes(): AudioAttributes {
        return AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_MEDIA)
            .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
            .apply {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                    setAllowedCapturePolicy(AudioAttributes.ALLOW_CAPTURE_BY_NONE)
                }
            }
            .build()
    }

    private fun createAudioTrack(
        attributes: AudioAttributes,
        format: AudioFormat,
        samples: ByteArray,
        decodedAudio: DecodedAudio,
        channelConfig: Int,
        encoding: Int
    ): AudioTrack {
        val bufferSize = samples.size
        val memoryThresholdBytes = calculateMemoryThreshold()

        return if (bufferSize < memoryThresholdBytes) {
            createStaticAudioTrack(attributes, format, samples)
        } else {
            createStreamingAudioTrack(attributes, format, samples, decodedAudio, channelConfig, encoding)
        }
    }

    private fun createStaticAudioTrack(attributes: AudioAttributes, format: AudioFormat, samples: ByteArray): AudioTrack {
        Timber.d("Using MODE_STATIC (${samples.size / 1024}KB)")
        return AudioTrack.Builder()
            .setAudioAttributes(attributes)
            .setAudioFormat(format)
            .setBufferSizeInBytes(samples.size)
            .setTransferMode(AudioTrack.MODE_STATIC)
            .apply {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                    setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
                }
            }
            .build()
            .also { it.write(samples, 0, samples.size) }
    }

    private fun createStreamingAudioTrack(
        attributes: AudioAttributes,
        format: AudioFormat,
        samples: ByteArray,
        decodedAudio: DecodedAudio,
        channelConfig: Int,
        encoding: Int
    ): AudioTrack {
        Timber.d("Using MODE_STREAM (${samples.size / 1024}KB)")
        val minBufferSize = AudioTrack.getMinBufferSize(decodedAudio.sampleRate, channelConfig, encoding)
        return AudioTrack.Builder()
            .setAudioAttributes(attributes)
            .setAudioFormat(format)
            .setBufferSizeInBytes(minBufferSize * 4)
            .setTransferMode(AudioTrack.MODE_STREAM)
            .apply {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                    setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
                }
            }
            .build()
            .also { writeStreamingData(it, samples) }
    }

    private fun applyPlaybackSpeed(track: AudioTrack) {
        if (_playbackSpeed.value != 1.0f && Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            val params = track.playbackParams
            params.speed = _playbackSpeed.value
            track.playbackParams = params
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
        return when (targetEncoding) {
            AudioFormat.ENCODING_PCM_16BIT -> convertTo16Bit(samples, sourceBitDepth)
            AudioFormat.ENCODING_PCM_24BIT_PACKED -> convertTo24BitPacked(samples, sourceBitDepth)
            AudioFormat.ENCODING_PCM_32BIT -> convertTo32Bit(samples)
            else -> convertTo16Bit(samples, sourceBitDepth)
        }
    }

    private fun convertTo16Bit(samples: ByteArray, sourceBitDepth: Int): ByteArray {
        val sourceBytesPerSample = 4
        val numSamples = samples.size / sourceBytesPerSample
        val output = ByteArray(numSamples * 2)

        for (i in 0 until numSamples) {
            val sample32 = readSample32(samples, i * sourceBytesPerSample)
            val sample16 = convertSampleTo16Bit(sample32, sourceBitDepth)
            writeSample16(output, i * 2, sample16)
        }
        return output
    }

    private fun convertTo24BitPacked(samples: ByteArray, sourceBitDepth: Int): ByteArray {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
            return convertTo16Bit(samples, sourceBitDepth)
        }

        val sourceBytesPerSample = 4
        val numSamples = samples.size / sourceBytesPerSample
        val output = ByteArray(numSamples * 3)

        for (i in 0 until numSamples) {
            val sample32 = readSample32(samples, i * sourceBytesPerSample)
            val sample24 = convertSampleTo24Bit(sample32, sourceBitDepth)
            writeSample24(output, i * 3, sample24)
        }
        return output
    }

    private fun convertTo32Bit(samples: ByteArray): ByteArray {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) samples else convertTo16Bit(samples, 32)
    }

    private fun readSample32(samples: ByteArray, offset: Int): Int {
        return (samples[offset].toInt() and 0xFF) or
                ((samples[offset + 1].toInt() and 0xFF) shl 8) or
                ((samples[offset + 2].toInt() and 0xFF) shl 16) or
                ((samples[offset + 3].toInt() and 0xFF) shl 24)
    }

    private fun convertSampleTo16Bit(sample32: Int, sourceBitDepth: Int): Short {
        return when (sourceBitDepth) {
            16 -> sample32.toShort()
            24 -> (sample32 shr 8).toShort()
            32 -> (sample32 shr 16).toShort()
            else -> sample32.toShort()
        }
    }

    private fun convertSampleTo24Bit(sample32: Int, sourceBitDepth: Int): Int {
        return when (sourceBitDepth) {
            16 -> sample32 shl 8
            24 -> sample32
            32 -> sample32 shr 8
            else -> sample32
        }
    }

    private fun writeSample16(output: ByteArray, offset: Int, sample: Short) {
        output[offset] = (sample.toInt() and 0xFF).toByte()
        output[offset + 1] = ((sample.toInt() shr 8) and 0xFF).toByte()
    }

    private fun writeSample24(output: ByteArray, offset: Int, sample: Int) {
        output[offset] = (sample and 0xFF).toByte()
        output[offset + 1] = ((sample shr 8) and 0xFF).toByte()
        output[offset + 2] = ((sample shr 16) and 0xFF).toByte()
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
        stopRmsMeasurement()
        audioTrack?.apply {
            stop()
            release()
        }
        audioTrack = null
        currentAudio = null
        currentSourceCodec = null
        _playbackState.value = PlaybackState.Stopped
        _position.value = 0
        audioFocusManager.abandonAudioFocus()
        levelMatcher.reset()
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

    fun getEqualizerBandLevel(band: Short): Short = equalizerEngine.getBandLevel(band)
    fun getNumberOfBands(): Short = equalizerEngine.getNumberOfBands()

    fun applyEqualizerPreset(preset: EqualizerEngine.EqualizerPreset) {
        equalizerEngine.applyPreset(preset)
        refreshPipelineState()
    }

    fun getEqualizerCurrentLevels(): List<Short> = equalizerEngine.getCurrentLevels()
    fun getEqualizerBandFreqRange(band: Short): IntArray? = equalizerEngine.getBandFreqRange(band)
    fun getEqualizerCenterFreq(band: Short): Int? = equalizerEngine.getCenterFreq(band)
    fun getEqualizerBandLevelRange(): ShortArray? = equalizerEngine.getBandLevelRange()

    private fun updatePipelineState(decodedAudio: DecodedAudio, track: AudioTrack) {
        val dspChain = buildList {
            if (equalizerEngine.isEnabled()) {
                val preset = equalizerEngine.getCurrentPreset()?.name ?: "Custom"
                add(DspComponent.Equalizer(preset))
            }
        }

        _pipelineState.value = AudioPipelineState(
            sourceCodec = currentSourceCodec,
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
        return when {
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE ->
                usbDacDetector.preferredDac.value?.let { AudioPath.UsbDac(it) } ?: AudioPath.BitPerfect
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> AudioPath.BitPerfect
            else -> AudioPath.Transparent
        }
    }

    fun setAbVersion(version: String) { currentAbVersion = version }
    fun enableLevelMatching() { levelMatcher.setMatchingEnabled(true) }
    fun disableLevelMatching() { levelMatcher.setMatchingEnabled(false) }
    fun setManualGain(db: Float) { levelMatcher.setManualGain(db) }

    private fun startRmsMeasurement(audioSessionId: Int) {
        try {
            visualizer?.release()
            visualizer = createVisualizer(audioSessionId)
        } catch (e: Exception) {
            Timber.e(e, "Failed to start RMS measurement")
            visualizer = null
        }
    }

    private fun createVisualizer(audioSessionId: Int): Visualizer {
        return Visualizer(audioSessionId).apply {
            captureSize = Visualizer.getCaptureSizeRange()[0]
            setDataCaptureListener(
                createVisualizerListener(),
                Visualizer.getMaxCaptureRate(),
                true,
                false
            )
            enabled = true
        }
    }

    private fun createVisualizerListener(): Visualizer.OnDataCaptureListener {
        return object : Visualizer.OnDataCaptureListener {
            override fun onWaveFormDataCapture(visualizer: Visualizer?, waveform: ByteArray?, samplingRate: Int) {
                waveform?.let { processWaveformData(it) }
            }
            override fun onFftDataCapture(visualizer: Visualizer?, fft: ByteArray?, samplingRate: Int) {}
        }
    }

    private fun processWaveformData(waveform: ByteArray) {
        val rms = calculateRms(waveform)
        val rmsDb = if (rms > 0) 20 * kotlin.math.log10(rms / 128.0) else -96.0
        levelMatcher.updateLevel(currentAbVersion, rmsDb.toFloat())
    }

    private fun calculateRms(waveform: ByteArray): Double {
        var sum = 0.0
        for (byte in waveform) {
            val sample = byte.toDouble()
            sum += sample * sample
        }
        return kotlin.math.sqrt(sum / waveform.size)
    }

    private fun stopRmsMeasurement() {
        visualizer?.enabled = false
        visualizer?.release()
        visualizer = null
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
