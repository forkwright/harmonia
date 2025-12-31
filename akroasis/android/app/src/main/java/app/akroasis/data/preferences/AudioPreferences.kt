// Audio settings preferences storage
package app.akroasis.data.preferences

import android.content.Context
import android.content.SharedPreferences
import app.akroasis.audio.ReplayGainProcessor
import dagger.hilt.android.qualifiers.ApplicationContext
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
    }

    var replayGainMode: ReplayGainProcessor.Mode
        get() = ReplayGainProcessor.Mode.valueOf(
            prefs.getString(KEY_REPLAY_GAIN_MODE, ReplayGainProcessor.Mode.OFF.name)
                ?: ReplayGainProcessor.Mode.OFF.name
        )
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
}
