// ReplayGain loudness normalization
package app.akroasis.audio

import kotlin.math.pow

class ReplayGainProcessor {

    enum class Mode {
        OFF,
        TRACK,
        ALBUM
    }

    fun applyGain(samples: ShortArray, gainDb: Float): ShortArray {
        if (gainDb == 0f) return samples

        val linearGain = dbToLinear(gainDb)
        val processed = ShortArray(samples.size)

        for (i in samples.indices) {
            val amplified = (samples[i] * linearGain).toInt()
            // Clamp to prevent overflow
            processed[i] = amplified.coerceIn(Short.MIN_VALUE.toInt(), Short.MAX_VALUE.toInt()).toShort()
        }

        return processed
    }

    private fun dbToLinear(db: Float): Float {
        return 10f.pow(db / 20f)
    }

    companion object {
        const val REFERENCE_LOUDNESS_DB = -18.0f  // EBU R128 standard
    }
}
