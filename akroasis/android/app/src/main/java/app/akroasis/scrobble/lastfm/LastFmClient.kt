// Last.fm API client with scrobbling support
package app.akroasis.scrobble.lastfm

import app.akroasis.BuildConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.FormBody
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import timber.log.Timber
import java.security.MessageDigest
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class LastFmClient @Inject constructor(
    private val httpClient: OkHttpClient
) {
    companion object {
        private const val API_BASE_URL = "https://ws.audioscrobbler.com/2.0/"
        private val API_KEY = BuildConfig.LASTFM_API_KEY
        private val API_SECRET = BuildConfig.LASTFM_API_SECRET
    }

    private var sessionKey: String? = null

    data class AuthResult(
        val success: Boolean,
        val sessionKey: String?,
        val error: String?
    )

    data class ScrobbleResult(
        val success: Boolean,
        val error: String?
    )

    suspend fun authenticate(username: String, password: String): AuthResult = withContext(Dispatchers.IO) {
        try {
            Timber.d("Authenticating Last.fm user: $username")
            val authToken = getMobileSessionToken(username, password)

            if (authToken != null) {
                sessionKey = authToken
                Timber.i("Last.fm authentication successful: $username")
                AuthResult(success = true, sessionKey = authToken, error = null)
            } else {
                Timber.w("Last.fm authentication failed: $username")
                AuthResult(success = false, sessionKey = null, error = "Authentication failed")
            }
        } catch (e: Exception) {
            Timber.e(e, "Last.fm authentication error: $username")
            AuthResult(success = false, sessionKey = null, error = e.message)
        }
    }

    private fun getMobileSessionToken(username: String, password: String): String? {
        val params = sortedMapOf(
            "method" to "auth.getMobileSession",
            "username" to username,
            "password" to password,
            "api_key" to API_KEY
        )

        val signature = generateSignature(params)

        val formBody = FormBody.Builder()
            .add("method", "auth.getMobileSession")
            .add("username", username)
            .add("password", password)
            .add("api_key", API_KEY)
            .add("api_sig", signature)
            .add("format", "json")
            .build()

        val request = Request.Builder()
            .url(API_BASE_URL)
            .post(formBody)
            .build()

        httpClient.newCall(request).execute().use { response ->
            if (!response.isSuccessful) return null

            val json = JSONObject(response.body?.string() ?: return null)
            return json.optJSONObject("session")?.optString("key")
        }
    }

    suspend fun updateNowPlaying(
        track: String,
        artist: String,
        album: String? = null,
        duration: Int? = null
    ): ScrobbleResult = withContext(Dispatchers.IO) {
        val key = sessionKey ?: return@withContext ScrobbleResult(false, "Not authenticated")

        try {
            Timber.d("Last.fm Now Playing: $artist - $track")
            val params = sortedMapOf(
                "method" to "track.updateNowPlaying",
                "track" to track,
                "artist" to artist,
                "api_key" to API_KEY,
                "sk" to key
            )

            album?.let { params["album"] = it }
            duration?.let { params["duration"] = it.toString() }

            val signature = generateSignature(params)

            val formBodyBuilder = FormBody.Builder()
            params.forEach { (k, v) -> formBodyBuilder.add(k, v) }
            formBodyBuilder.add("api_sig", signature)
            formBodyBuilder.add("format", "json")

            val request = Request.Builder()
                .url(API_BASE_URL)
                .post(formBodyBuilder.build())
                .build()

            httpClient.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    return@withContext ScrobbleResult(false, "HTTP ${response.code}")
                }

                val json = JSONObject(response.body?.string() ?: "")
                if (json.has("error")) {
                    return@withContext ScrobbleResult(false, json.optString("message"))
                }

                ScrobbleResult(true, null)
            }
        } catch (e: Exception) {
            ScrobbleResult(false, e.message)
        }
    }

    suspend fun scrobble(
        track: String,
        artist: String,
        timestamp: Long,
        album: String? = null,
        duration: Int? = null
    ): ScrobbleResult = withContext(Dispatchers.IO) {
        val key = sessionKey ?: return@withContext ScrobbleResult(false, "Not authenticated")

        try {
            Timber.d("Last.fm scrobbling: $artist - $track")
            val params = sortedMapOf(
                "method" to "track.scrobble",
                "track" to track,
                "artist" to artist,
                "timestamp" to timestamp.toString(),
                "api_key" to API_KEY,
                "sk" to key
            )

            album?.let { params["album"] = it }
            duration?.let { params["duration"] = it.toString() }

            val signature = generateSignature(params)

            val formBodyBuilder = FormBody.Builder()
            params.forEach { (k, v) -> formBodyBuilder.add(k, v) }
            formBodyBuilder.add("api_sig", signature)
            formBodyBuilder.add("format", "json")

            val request = Request.Builder()
                .url(API_BASE_URL)
                .post(formBodyBuilder.build())
                .build()

            httpClient.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    return@withContext ScrobbleResult(false, "HTTP ${response.code}")
                }

                val json = JSONObject(response.body?.string() ?: "")
                if (json.has("error")) {
                    val error = json.optString("message")
                    Timber.w("Last.fm scrobble failed: $error")
                    return@withContext ScrobbleResult(false, error)
                }

                Timber.i("Last.fm scrobbled: $artist - $track")
                ScrobbleResult(true, null)
            }
        } catch (e: Exception) {
            Timber.e(e, "Last.fm scrobble error")
            ScrobbleResult(false, e.message)
        }
    }

    fun setSessionKey(key: String) {
        sessionKey = key
    }

    fun clearSession() {
        sessionKey = null
    }

    fun isAuthenticated(): Boolean = sessionKey != null

    private fun generateSignature(params: Map<String, String>): String {
        val signatureString = params.toSortedMap().entries.joinToString("") { "${it.key}${it.value}" } + API_SECRET
        return md5(signatureString)
    }

    // SonarCloud: MD5 is acceptable here - required by Last.fm API specification
    // This is NOT used for password storage or cryptographic security
    @Suppress("kotlin:S4790")
    private fun md5(input: String): String {
        val md = MessageDigest.getInstance("MD5")
        val digest = md.digest(input.toByteArray())
        return digest.joinToString("") { "%02x".format(it) }
    }
}
