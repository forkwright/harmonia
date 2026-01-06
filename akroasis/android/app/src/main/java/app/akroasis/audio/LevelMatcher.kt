package app.akroasis.audio

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import javax.inject.Inject
import kotlin.math.pow

/**
 * Provides RMS-based level matching for A/B audio comparisons.
 *
 * Measures real-time RMS levels for two versions (A and B) and calculates
 * gain compensation to match B's level to A's level, eliminating "louder = better" bias.
 */
class LevelMatcher @Inject constructor() {
    companion object {
        private const val WINDOW_SIZE_MS = 500
        private const val MAX_GAIN_DB = 12f
        private const val MIN_GAIN_DB = -12f
    }

    private val _levelA = MutableStateFlow(0f)
    val levelA: StateFlow<Float> = _levelA.asStateFlow()

    private val _levelB = MutableStateFlow(0f)
    val levelB: StateFlow<Float> = _levelB.asStateFlow()

    private val _gainCompensation = MutableStateFlow(0f)
    val gainCompensation: StateFlow<Float> = _gainCompensation.asStateFlow()

    private val _matchingEnabled = MutableStateFlow(true)
    val matchingEnabled: StateFlow<Boolean> = _matchingEnabled.asStateFlow()

    /**
     * Update RMS level for a version.
     *
     * @param version "A" or "B"
     * @param rmsDb RMS level in dB
     */
    fun updateLevel(version: String, rmsDb: Float) {
        when (version) {
            "A" -> _levelA.value = rmsDb
            "B" -> _levelB.value = rmsDb
        }

        if (_matchingEnabled.value) {
            calculateGainCompensation()
        }
    }

    /**
     * Calculate gain compensation to match B to A's level.
     *
     * Gain is clamped to ±12dB to prevent excessive amplification.
     */
    private fun calculateGainCompensation() {
        val delta = _levelA.value - _levelB.value
        _gainCompensation.value = delta.coerceIn(MIN_GAIN_DB, MAX_GAIN_DB)
    }

    /**
     * Apply gain compensation to audio samples.
     *
     * @param samples Audio samples to process
     * @param version "A" or "B" - only B gets compensation
     * @return Processed samples
     */
    fun applyGainCompensation(samples: ShortArray, version: String): ShortArray {
        if (!_matchingEnabled.value || version != "B") {
            return samples
        }

        val gain = dbToLinear(_gainCompensation.value)
        return samples.map { sample ->
            (sample * gain).coerceIn(
                Short.MIN_VALUE.toFloat(),
                Short.MAX_VALUE.toFloat()
            ).toInt().toShort()
        }.toShortArray()
    }

    /**
     * Reset all levels and gain compensation.
     */
    fun reset() {
        _levelA.value = 0f
        _levelB.value = 0f
        _gainCompensation.value = 0f
    }

    /**
     * Enable or disable automatic level matching.
     *
     * @param enabled True to enable matching, false for manual control
     */
    fun setMatchingEnabled(enabled: Boolean) {
        _matchingEnabled.value = enabled
        if (!enabled) {
            _gainCompensation.value = 0f
        } else {
            calculateGainCompensation()
        }
    }

    /**
     * Set manual gain adjustment (when matching is disabled).
     *
     * @param db Gain in dB (±12dB range)
     */
    fun setManualGain(db: Float) {
        if (!_matchingEnabled.value) {
            _gainCompensation.value = db.coerceIn(MIN_GAIN_DB, MAX_GAIN_DB)
        }
    }

    /**
     * Convert dB to linear gain factor.
     */
    private fun dbToLinear(db: Float): Float = 10f.pow(db / 20f)
}
