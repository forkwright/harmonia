// Scrobbling preferences and session storage
package app.akroasis.data.preferences

import android.content.Context
import android.content.SharedPreferences
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ScrobblePreferences @Inject constructor(
    @ApplicationContext context: Context
) {
    private val prefs: SharedPreferences = context.getSharedPreferences(
        "scrobble_prefs",
        Context.MODE_PRIVATE
    )

    companion object {
        private const val KEY_LASTFM_ENABLED = "lastfm_enabled"
        private const val KEY_LASTFM_SESSION_KEY = "lastfm_session_key"
        private const val KEY_LASTFM_USERNAME = "lastfm_username"

        private const val KEY_LISTENBRAINZ_ENABLED = "listenbrainz_enabled"
        private const val KEY_LISTENBRAINZ_TOKEN = "listenbrainz_token"
        private const val KEY_LISTENBRAINZ_USERNAME = "listenbrainz_username"

        private const val KEY_SCROBBLE_PERCENTAGE = "scrobble_percentage"
        private const val KEY_SCROBBLE_MIN_DURATION = "scrobble_min_duration"
    }

    // Last.fm settings
    var lastFmEnabled: Boolean
        get() = prefs.getBoolean(KEY_LASTFM_ENABLED, false)
        set(value) = prefs.edit().putBoolean(KEY_LASTFM_ENABLED, value).apply()

    var lastFmSessionKey: String?
        get() = prefs.getString(KEY_LASTFM_SESSION_KEY, null)
        set(value) = prefs.edit().putString(KEY_LASTFM_SESSION_KEY, value).apply()

    var lastFmUsername: String?
        get() = prefs.getString(KEY_LASTFM_USERNAME, null)
        set(value) = prefs.edit().putString(KEY_LASTFM_USERNAME, value).apply()

    // ListenBrainz settings
    var listenBrainzEnabled: Boolean
        get() = prefs.getBoolean(KEY_LISTENBRAINZ_ENABLED, false)
        set(value) = prefs.edit().putBoolean(KEY_LISTENBRAINZ_ENABLED, value).apply()

    var listenBrainzToken: String?
        get() = prefs.getString(KEY_LISTENBRAINZ_TOKEN, null)
        set(value) = prefs.edit().putString(KEY_LISTENBRAINZ_TOKEN, value).apply()

    var listenBrainzUsername: String?
        get() = prefs.getString(KEY_LISTENBRAINZ_USERNAME, null)
        set(value) = prefs.edit().putString(KEY_LISTENBRAINZ_USERNAME, value).apply()

    // Scrobbling behavior
    var scrobblePercentage: Int
        get() = prefs.getInt(KEY_SCROBBLE_PERCENTAGE, 50)
        set(value) = prefs.edit().putInt(KEY_SCROBBLE_PERCENTAGE, value.coerceIn(0, 100)).apply()

    var scrobbleMinDuration: Int
        get() = prefs.getInt(KEY_SCROBBLE_MIN_DURATION, 30)
        set(value) = prefs.edit().putInt(KEY_SCROBBLE_MIN_DURATION, value.coerceAtLeast(0)).apply()

    fun clearLastFmSession() {
        prefs.edit()
            .remove(KEY_LASTFM_SESSION_KEY)
            .remove(KEY_LASTFM_USERNAME)
            .putBoolean(KEY_LASTFM_ENABLED, false)
            .apply()
    }

    fun clearListenBrainzSession() {
        prefs.edit()
            .remove(KEY_LISTENBRAINZ_TOKEN)
            .remove(KEY_LISTENBRAINZ_USERNAME)
            .putBoolean(KEY_LISTENBRAINZ_ENABLED, false)
            .apply()
    }
}
