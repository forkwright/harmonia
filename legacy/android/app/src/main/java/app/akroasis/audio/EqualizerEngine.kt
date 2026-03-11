// Parametric equalizer using Android Equalizer API
package app.akroasis.audio

import android.media.audiofx.Equalizer
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class EqualizerEngine @Inject constructor() {

    private var equalizer: Equalizer? = null
    private var enabled: Boolean = false

    data class EqualizerPreset(
        val name: String,
        val bandLevels: List<Short>  // dB * 100 for each band
    )

    fun attachToSession(audioSessionId: Int) {
        release()
        equalizer = Equalizer(0, audioSessionId).apply {
            enabled = this@EqualizerEngine.enabled
        }
    }

    fun enable() {
        enabled = true
        equalizer?.enabled = true
    }

    fun disable() {
        enabled = false
        equalizer?.enabled = false
    }

    fun isEnabled(): Boolean = enabled

    fun getCurrentPreset(): EqualizerPreset? = null

    fun setBandLevel(band: Short, level: Short) {
        equalizer?.setBandLevel(band, level)
    }

    fun getBandLevel(band: Short): Short {
        return equalizer?.getBandLevel(band) ?: 0
    }

    fun getNumberOfBands(): Short {
        return equalizer?.numberOfBands ?: 5
    }

    fun getBandFreqRange(band: Short): IntArray? {
        return equalizer?.getBandFreqRange(band)
    }

    fun getCenterFreq(band: Short): Int? {
        return equalizer?.getCenterFreq(band)
    }

    fun getBandLevelRange(): ShortArray? {
        return equalizer?.bandLevelRange
    }

    fun applyPreset(preset: EqualizerPreset) {
        preset.bandLevels.forEachIndexed { index, level ->
            if (index < getNumberOfBands()) {
                setBandLevel(index.toShort(), level)
            }
        }
    }

    fun getCurrentLevels(): List<Short> {
        val numBands = getNumberOfBands()
        return (0 until numBands).map { getBandLevel(it.toShort()) }
    }

    fun release() {
        equalizer?.release()
        equalizer = null
    }

    companion object {
        val PRESET_FLAT = EqualizerPreset(
            name = "Flat",
            bandLevels = listOf(0, 0, 0, 0, 0)
        )

        val PRESET_ROCK = EqualizerPreset(
            name = "Rock",
            bandLevels = listOf(500, 300, -100, 100, 600)
        )

        val PRESET_JAZZ = EqualizerPreset(
            name = "Jazz",
            bandLevels = listOf(400, 200, 300, 100, 400)
        )

        val PRESET_CLASSICAL = EqualizerPreset(
            name = "Classical",
            bandLevels = listOf(500, 300, -200, 400, 500)
        )

        val PRESET_POP = EqualizerPreset(
            name = "Pop",
            bandLevels = listOf(-100, 300, 500, 300, -100)
        )

        val PRESET_BASS_BOOST = EqualizerPreset(
            name = "Bass Boost",
            bandLevels = listOf(700, 500, 200, 0, 0)
        )

        val ALL_PRESETS = listOf(
            PRESET_FLAT,
            PRESET_ROCK,
            PRESET_JAZZ,
            PRESET_CLASSICAL,
            PRESET_POP,
            PRESET_BASS_BOOST
        )
    }
}
