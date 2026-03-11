// Audio settings preferences storage
package app.akroasis.data.preferences

import android.content.Context
import android.content.SharedPreferences
import app.akroasis.audio.ReplayGainProcessor
import app.akroasis.data.model.EqualizerPreset
import dagger.hilt.android.qualifiers.ApplicationContext
import org.json.JSONArray
import org.json.JSONObject
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class AudioPreferences @Inject constructor(
    @ApplicationContext context: Context
) {
    private val prefs: SharedPreferences = context.getSharedPreferences(
        "akroasis_audio_prefs",
        Context.MODE_PRIVATE
    )

    companion object {
        private const val KEY_REPLAY_GAIN_MODE = "replay_gain_mode"
        private const val KEY_EQUALIZER_ENABLED = "equalizer_enabled"
        private const val KEY_EQUALIZER_PRESET = "equalizer_preset"
        private const val KEY_GAPLESS_ENABLED = "gapless_enabled"
        private const val KEY_CROSSFADE_DURATION = "crossfade_duration"
        private const val KEY_PLAYBACK_SPEED = "playback_speed"
        private const val KEY_CUSTOM_EQ_PRESETS = "custom_eq_presets"
    }

    var replayGainMode: ReplayGainProcessor.Mode
        get() {
            val modeName = prefs.getString(KEY_REPLAY_GAIN_MODE, ReplayGainProcessor.Mode.OFF.name) ?: ReplayGainProcessor.Mode.OFF.name
            return try {
                ReplayGainProcessor.Mode.valueOf(modeName)
            } catch (e: IllegalArgumentException) {
                Timber.w("Invalid replay gain mode: $modeName")
                ReplayGainProcessor.Mode.OFF
            }
        }
        set(value) {
            prefs.edit().putString(KEY_REPLAY_GAIN_MODE, value.name).apply()
        }

    var equalizerEnabled: Boolean
        get() = prefs.getBoolean(KEY_EQUALIZER_ENABLED, false)
        set(value) {
            prefs.edit().putBoolean(KEY_EQUALIZER_ENABLED, value).apply()
        }

    var equalizerPreset: String
        get() = prefs.getString(KEY_EQUALIZER_PRESET, "Flat") ?: "Flat"
        set(value) {
            prefs.edit().putString(KEY_EQUALIZER_PRESET, value).apply()
        }

    var gaplessEnabled: Boolean
        get() = prefs.getBoolean(KEY_GAPLESS_ENABLED, true)
        set(value) {
            prefs.edit().putBoolean(KEY_GAPLESS_ENABLED, value).apply()
        }

    var crossfadeDuration: Int
        get() = prefs.getInt(KEY_CROSSFADE_DURATION, 0)
        set(value) {
            prefs.edit().putInt(KEY_CROSSFADE_DURATION, value).apply()
        }

    var playbackSpeed: Float
        get() = prefs.getFloat(KEY_PLAYBACK_SPEED, 1.0f)
        set(value) {
            prefs.edit().putFloat(KEY_PLAYBACK_SPEED, value).apply()
        }

    fun saveCustomEqualizerPreset(preset: EqualizerPreset) {
        val existingPresets = getCustomEqualizerPresets().toMutableList()
        existingPresets.removeAll { it.name == preset.name }
        existingPresets.add(preset)

        val jsonArray = JSONArray()
        existingPresets.forEach { p ->
            val jsonObject = JSONObject()
            jsonObject.put("name", p.name)
            val levelsArray = JSONArray()
            p.bandLevels.forEach { level -> levelsArray.put(level.toInt()) }
            jsonObject.put("bandLevels", levelsArray)
            jsonArray.put(jsonObject)
        }

        prefs.edit().putString(KEY_CUSTOM_EQ_PRESETS, jsonArray.toString()).apply()
    }

    fun getCustomEqualizerPresets(): List<EqualizerPreset> {
        val presetsJson = prefs.getString(KEY_CUSTOM_EQ_PRESETS, null) ?: return emptyList()
        val presets = mutableListOf<EqualizerPreset>()

        try {
            val jsonArray = JSONArray(presetsJson)
            for (i in 0 until jsonArray.length()) {
                val jsonObject = jsonArray.getJSONObject(i)
                val name = jsonObject.getString("name")
                val levelsArray = jsonObject.getJSONArray("bandLevels")
                val bandLevels = mutableListOf<Short>()
                for (j in 0 until levelsArray.length()) {
                    bandLevels.add(levelsArray.getInt(j).toShort())
                }
                presets.add(EqualizerPreset(name, bandLevels, isBuiltIn = false))
            }
        } catch (e: Exception) {
            Timber.e(e, "Error parsing custom EQ presets")
        }

        return presets
    }

    fun deleteCustomEqualizerPreset(name: String) {
        val existingPresets = getCustomEqualizerPresets().toMutableList()
        existingPresets.removeAll { it.name == name }

        val jsonArray = JSONArray()
        existingPresets.forEach { p ->
            val jsonObject = JSONObject()
            jsonObject.put("name", p.name)
            val levelsArray = JSONArray()
            p.bandLevels.forEach { level -> levelsArray.put(level.toInt()) }
            jsonObject.put("bandLevels", levelsArray)
            jsonArray.put(jsonObject)
        }

        prefs.edit().putString(KEY_CUSTOM_EQ_PRESETS, jsonArray.toString()).apply()
    }
}
