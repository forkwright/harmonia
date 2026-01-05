// Crossfeed DSP for stereo fatigue reduction
package app.akroasis.audio

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class CrossfeedEngine @Inject constructor() {

    private val _isEnabled = MutableStateFlow(false)
    val isEnabled: StateFlow<Boolean> = _isEnabled.asStateFlow()

    private val _crossfeedStrength = MutableStateFlow(0.3f)
    val crossfeedStrength: StateFlow<Float> = _crossfeedStrength.asStateFlow()

    private var sampleRate: Int = 44100

    fun enable() {
        _isEnabled.value = true
    }

    fun disable() {
        _isEnabled.value = false
    }

    fun setStrength(strength: Float) {
        _crossfeedStrength.value = strength.coerceIn(0f, 1f)
    }

    fun setSampleRate(rate: Int) {
        sampleRate = rate
    }

    fun processSamples(samples: ShortArray, channels: Int): ShortArray {
        if (!_isEnabled.value || channels != 2) {
            return samples
        }

        val processed = samples.copyOf()
        val strength = _crossfeedStrength.value

        for (i in 0 until samples.size step 2) {
            val left = samples[i].toFloat()
            val right = samples[i + 1].toFloat()

            processed[i] = (left * (1f - strength) + right * strength).toInt().toShort()
            processed[i + 1] = (right * (1f - strength) + left * strength).toInt().toShort()
        }

        return processed
    }

    fun release() {
        _isEnabled.value = false
    }

    companion object {
        const val STRENGTH_LOW = 0.15f
        const val STRENGTH_MEDIUM = 0.30f
        const val STRENGTH_HIGH = 0.50f
    }
}
