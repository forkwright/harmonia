// Converts AutoEQ parametric profiles to fixed-band equalizer settings
package app.akroasis.audio

import app.akroasis.data.model.AutoEQProfile
import app.akroasis.data.model.FilterType
import app.akroasis.data.model.ParametricBand
import kotlin.math.*

object AutoEQConverter {

    fun convertToFixedBands(
        profile: AutoEQProfile,
        centerFrequencies: List<Int>,
        bandLevelRange: ShortArray
    ): List<Short> {
        val minLevel = bandLevelRange[0].toFloat()
        val maxLevel = bandLevelRange[1].toFloat()

        return centerFrequencies.map { centerFreq ->
            var totalGain = 0f

            profile.parametricEq.forEach { band ->
                val gainContribution = calculateGainContribution(band, centerFreq.toFloat())
                totalGain += gainContribution
            }

            val gainInMillibels = (totalGain * 100).coerceIn(minLevel, maxLevel)
            gainInMillibels.toInt().toShort()
        }
    }

    private fun calculateGainContribution(band: ParametricBand, frequency: Float): Float {
        return when (band.type) {
            FilterType.PEAKING -> calculatePeakingGain(band, frequency)
            FilterType.LOW_SHELF -> calculateLowShelfGain(band, frequency)
            FilterType.HIGH_SHELF -> calculateHighShelfGain(band, frequency)
            FilterType.LOW_PASS, FilterType.HIGH_PASS -> 0f
        }
    }

    private fun calculatePeakingGain(band: ParametricBand, frequency: Float): Float {
        val w0 = 2 * PI * band.frequency
        val w = 2 * PI * frequency

        val distance = abs(ln(w / w0).toFloat())
        val bandwidth = ln(2f) / (2 * band.q)

        return if (distance < bandwidth * 3) {
            val attenuation = exp(-distance / bandwidth).toFloat()
            band.gain * attenuation
        } else {
            0f
        }
    }

    private fun calculateLowShelfGain(band: ParametricBand, frequency: Float): Float {
        if (frequency >= band.frequency) {
            return band.gain
        }

        val ratio = frequency / band.frequency
        val slope = band.q.coerceIn(0.1f, 1.0f)
        val attenuation = (ratio.pow(slope * 2)).coerceIn(0f, 1f)

        return band.gain * attenuation
    }

    private fun calculateHighShelfGain(band: ParametricBand, frequency: Float): Float {
        if (frequency <= band.frequency) {
            return band.gain
        }

        val ratio = band.frequency / frequency
        val slope = band.q.coerceIn(0.1f, 1.0f)
        val attenuation = (ratio.pow(slope * 2)).coerceIn(0f, 1f)

        return band.gain * attenuation
    }
}
