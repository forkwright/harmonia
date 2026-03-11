// Scrobbling preferences and session storage
package app.akroasis.data.preferences

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ScrobblePreferences @Inject constructor(
    @ApplicationContext context: Context
) {
    private val masterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()

    // Encrypted storage for sensitive tokens
    private val prefs: SharedPreferences = EncryptedSharedPreferences.create(
        context,
        "scrobble_prefs_encrypted",
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )

    // Migrate from old plaintext storage if needed
    init {
        migrateFromPlaintextIfNeeded(context)
    }

    private fun migrateFromPlaintextIfNeeded(context: Context) {
        val oldPrefs = context.getSharedPreferences("scrobble_prefs", Context.MODE_PRIVATE)

        // Check if migration needed (old prefs exist and new ones don't)
        if (oldPrefs.contains(KEY_LASTFM_SESSION_KEY) && !prefs.contains(KEY_LASTFM_SESSION_KEY)) {
            // Migrate Last.fm tokens
            oldPrefs.getString(KEY_LASTFM_SESSION_KEY, null)?.let { sessionKey ->
                prefs.edit().putString(KEY_LASTFM_SESSION_KEY, sessionKey).apply()
            }
            oldPrefs.getString(KEY_LASTFM_USERNAME, null)?.let { username ->
                prefs.edit().putString(KEY_LASTFM_USERNAME, username).apply()
            }
            val lastFmEnabled = oldPrefs.getBoolean(KEY_LASTFM_ENABLED, false)
            prefs.edit().putBoolean(KEY_LASTFM_ENABLED, lastFmEnabled).apply()
        }

        if (oldPrefs.contains(KEY_LISTENBRAINZ_TOKEN) && !prefs.contains(KEY_LISTENBRAINZ_TOKEN)) {
            // Migrate ListenBrainz tokens
            oldPrefs.getString(KEY_LISTENBRAINZ_TOKEN, null)?.let { token ->
                prefs.edit().putString(KEY_LISTENBRAINZ_TOKEN, token).apply()
            }
            oldPrefs.getString(KEY_LISTENBRAINZ_USERNAME, null)?.let { username ->
                prefs.edit().putString(KEY_LISTENBRAINZ_USERNAME, username).apply()
            }
            val lbEnabled = oldPrefs.getBoolean(KEY_LISTENBRAINZ_ENABLED, false)
            prefs.edit().putBoolean(KEY_LISTENBRAINZ_ENABLED, lbEnabled).apply()
        }

        // Migrate settings (non-sensitive, but migrate for consistency)
        if (oldPrefs.contains(KEY_SCROBBLE_PERCENTAGE) && !prefs.contains(KEY_SCROBBLE_PERCENTAGE)) {
            val percentage = oldPrefs.getInt(KEY_SCROBBLE_PERCENTAGE, 50)
            prefs.edit().putInt(KEY_SCROBBLE_PERCENTAGE, percentage).apply()

            val minDuration = oldPrefs.getInt(KEY_SCROBBLE_MIN_DURATION, 30)
            prefs.edit().putInt(KEY_SCROBBLE_MIN_DURATION, minDuration).apply()
        }

        // Clear old plaintext storage after migration
        if (oldPrefs.all.isNotEmpty()) {
            oldPrefs.edit().clear().apply()
        }
    }

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
