// JNI bridge to Rust FLAC decoder
package app.akroasis.audio

object NativeAudioDecoder {
    init {
        System.loadLibrary("akroasis_core")
    }

    external fun createFlacDecoder(): Long
    external fun destroyFlacDecoder(decoderPtr: Long)
    external fun decodeFlac(decoderPtr: Long, data: ByteArray): ByteArray?
    external fun getSampleRate(decoderPtr: Long): Int
    external fun getChannels(decoderPtr: Long): Int
    external fun getBitDepth(decoderPtr: Long): Int
}

class FlacDecoder : AutoCloseable {
    private var nativePtr: Long = 0

    init {
        nativePtr = NativeAudioDecoder.createFlacDecoder()
        if (nativePtr == 0L) {
            throw IllegalStateException("Failed to create FLAC decoder")
        }
    }

    fun decode(data: ByteArray): DecodedAudio? {
        if (nativePtr == 0L) {
            throw IllegalStateException("Decoder already closed")
        }

        val samples = NativeAudioDecoder.decodeFlac(nativePtr, data) ?: return null

        if (nativePtr == 0L) {
            throw IllegalStateException("Decoder closed during decode")
        }

        val sampleRate = NativeAudioDecoder.getSampleRate(nativePtr)
        val channels = NativeAudioDecoder.getChannels(nativePtr)
        val bitDepth = NativeAudioDecoder.getBitDepth(nativePtr)

        return DecodedAudio(
            samples = samples,
            sampleRate = sampleRate,
            channels = channels,
            bitDepth = bitDepth
        )
    }

    override fun close() {
        if (nativePtr != 0L) {
            NativeAudioDecoder.destroyFlacDecoder(nativePtr)
            nativePtr = 0
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
        get() = (samples.size / (channels * 2) * 1000L) / sampleRate
}
