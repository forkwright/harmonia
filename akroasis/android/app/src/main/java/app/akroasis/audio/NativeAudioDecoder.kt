// JNI bridge to Rust FLAC decoder
package app.akroasis.audio

import timber.log.Timber

object NativeAudioDecoder {
    private var isLoaded = false
    private var loadError: String? = null

    init {
        try {
            System.loadLibrary("akroasis_core")
            isLoaded = true
            Timber.i("Successfully loaded akroasis_core")
        } catch (e: UnsatisfiedLinkError) {
            isLoaded = false
            loadError = e.message
            Timber.e(e, "Failed to load akroasis_core")
        }
    }

    fun isAvailable(): Boolean = isLoaded
    fun getLoadError(): String? = loadError

    private fun ensureLoaded() {
        if (!isLoaded) {
            throw IllegalStateException("Native library not loaded: $loadError")
        }
    }

    external fun createFlacDecoder(): Long
    external fun destroyFlacDecoder(decoderPtr: Long)
    external fun decodeFlac(decoderPtr: Long, data: ByteArray): ByteArray?
    external fun getSampleRate(decoderPtr: Long): Int
    external fun getChannels(decoderPtr: Long): Int
    external fun getBitDepth(decoderPtr: Long): Int
}

class FlacDecoder : AutoCloseable {
    @Volatile
    private var nativePtr: Long = 0
    private val lock = Any()
    private var isClosed = false

    init {
        if (!NativeAudioDecoder.isAvailable()) {
            throw IllegalStateException(
                "Cannot create FLAC decoder: Native library not available. " +
                "Error: ${NativeAudioDecoder.getLoadError()}"
            )
        }
        nativePtr = NativeAudioDecoder.createFlacDecoder()
        if (nativePtr == 0L) {
            throw IllegalStateException("Native decoder creation failed")
        }
    }

    fun decode(data: ByteArray): DecodedAudio? = synchronized(lock) {
        check(!(isClosed || nativePtr == 0L)) { "Decoder already closed" }

        val samples = NativeAudioDecoder.decodeFlac(nativePtr, data) ?: return null
        val sampleRate = NativeAudioDecoder.getSampleRate(nativePtr)
        val channels = NativeAudioDecoder.getChannels(nativePtr)
        val bitDepth = NativeAudioDecoder.getBitDepth(nativePtr)

        return DecodedAudio(samples, sampleRate, channels, bitDepth)
    }

    override fun close() = synchronized(lock) {
        if (!isClosed && nativePtr != 0L) {
            NativeAudioDecoder.destroyFlacDecoder(nativePtr)
            nativePtr = 0
            isClosed = true
        }
    }
}

data class DecodedAudio(
    val samples: ByteArray,
    val sampleRate: Int,
    val channels: Int,
    val bitDepth: Int
) {
    val durationMs: Long
        get() {
            val bytesPerSample = 4
            val numSamples = samples.size / bytesPerSample / channels
            return (numSamples * 1000L) / sampleRate
        }
}
