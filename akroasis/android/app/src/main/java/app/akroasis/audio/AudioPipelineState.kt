// Audio pipeline state visualization
package app.akroasis.audio

sealed class AudioPath {
    data object BitPerfect : AudioPath() {
        override fun describe() = "Bit-Perfect (Android 14+)"
    }

    data object Transparent : AudioPath() {
        override fun describe() = "Transparent (192kHz/32-bit)"
    }

    data class UsbDac(val dacInfo: UsbDacInfo) : AudioPath() {
        override fun describe() = "USB DAC: ${dacInfo.productName}"
    }

    abstract fun describe(): String
}

sealed class DspComponent {
    data class Equalizer(val preset: String) : DspComponent() {
        override fun label() = "EQ"
        override fun value() = preset
    }

    data class ReplayGain(val adjustmentDb: Float) : DspComponent() {
        override fun label() = "ReplayGain"
        override fun value() = "${adjustmentDb.format(1)}dB"
    }

    data class Crossfade(val durationMs: Int) : DspComponent() {
        override fun label() = "Crossfade"
        override fun value() = "${durationMs / 1000}s"
    }

    abstract fun label(): String
    abstract fun value(): String
}

data class AudioPipelineState(
    val inputFormat: AudioFormatInfo,
    val outputFormat: AudioFormatInfo,
    val audioPath: AudioPath,
    val dspChain: List<DspComponent>,
    val gaplessActive: Boolean
)

private fun Float.format(decimals: Int): String = "%.${decimals}f".format(this)
