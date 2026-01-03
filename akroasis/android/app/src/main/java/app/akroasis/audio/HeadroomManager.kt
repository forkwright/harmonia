// Headroom management and clipping prevention
package app.akroasis.audio

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import javax.inject.Inject
import javax.inject.Singleton
import kotlin.math.abs
import kotlin.math.pow

@Singleton
class HeadroomManager @Inject constructor() {

    private val _isEnabled = MutableStateFlow(true)
    val isEnabled: StateFlow<Boolean> = _isEnabled.asStateFlow()

    private val _headroomDb = MutableStateFlow(-3.0f)
    val headroomDb: StateFlow<Float> = _headroomDb.asStateFlow()

    private val _clippingDetected = MutableStateFlow(false)
    val clippingDetected: StateFlow<Boolean> = _clippingDetected.asStateFlow()

    private val _peakLevel = MutableStateFlow(0f)
    val peakLevel: StateFlow<Float> = _peakLevel.asStateFlow()

    private var consecutiveClips = 0
    private val clippingThreshold = Short.MAX_VALUE * 0.99f

    fun enable() {
        _isEnabled.value = true
    }

    fun disable() {
        _isEnabled.value = false
    }

    fun setHeadroom(db: Float) {
        _headroomDb.value = db.coerceIn(-12f, 0f)
    }

    fun processSamples(samples: ShortArray): ShortArray {
        if (!_isEnabled.value) {
            return samples
        }

        val headroomMultiplier = dbToLinear(_headroomDb.value)
        val processed = ShortArray(samples.size)
        var maxPeak = 0f
        var clipped = false

        for (i in samples.indices) {
            val sample = samples[i].toFloat()
            val processedVal = sample * headroomMultiplier

            val clampedValue = processedVal.coerceIn(
                Short.MIN_VALUE.toFloat(),
                Short.MAX_VALUE.toFloat()
            )

            if (abs(processedVal) > clippingThreshold) {
                clipped = true
                consecutiveClips++
            }

            processed[i] = clampedValue.toInt().toShort()
            maxPeak = maxOf(maxPeak, abs(clampedValue) / Short.MAX_VALUE)
        }

        _peakLevel.value = maxPeak
        _clippingDetected.value = clipped

        if (!clipped) {
            consecutiveClips = 0
        }

        if (consecutiveClips > 100) {
            _clippingDetected.value = true
        }

        return processed
    }

    fun getRecommendedHeadroom(
        equalizerEnabled: Boolean,
        crossfeedEnabled: Boolean,
        maxEqGain: Float = 0f
    ): Float {
        var recommended = 0f

        if (equalizerEnabled && maxEqGain > 0f) {
            recommended -= maxEqGain
        }

        if (crossfeedEnabled) {
            recommended -= 3f
        }

        return recommended.coerceIn(-12f, 0f)
    }

    fun resetClippingIndicator() {
        _clippingDetected.value = false
        consecutiveClips = 0
    }

    private fun dbToLinear(db: Float): Float {
        return 10f.pow(db / 20f)
    }

    companion object {
        const val HEADROOM_SAFE = -6.0f
        const val HEADROOM_MODERATE = -3.0f
        const val HEADROOM_MINIMAL = -1.0f
        const val HEADROOM_NONE = 0.0f
    }
}
