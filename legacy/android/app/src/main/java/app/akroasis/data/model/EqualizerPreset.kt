// Equalizer preset data model
package app.akroasis.data.model

data class EqualizerPreset(
    val name: String,
    val bandLevels: List<Short>,
    val isBuiltIn: Boolean = false
) {
    companion object {
        const val PRESET_FLAT = "Flat"
        const val PRESET_ROCK = "Rock"
        const val PRESET_JAZZ = "Jazz"
        const val PRESET_CLASSICAL = "Classical"
        const val PRESET_POP = "Pop"
        const val PRESET_BASS_BOOST = "Bass Boost"
    }
}
