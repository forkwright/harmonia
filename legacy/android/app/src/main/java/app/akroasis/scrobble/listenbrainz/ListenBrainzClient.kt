// ListenBrainz API client for scrobbling
package app.akroasis.scrobble.listenbrainz

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ListenBrainzClient @Inject constructor(
    private val httpClient: OkHttpClient
) {
    companion object {
        private const val API_BASE_URL = "https://api.listenbrainz.org/1/"
    }

    private var userToken: String? = null

    data class SubmitResult(
        val success: Boolean,
        val error: String?
    )

    suspend fun submitPlayingNow(
        track: String,
        artist: String,
        album: String? = null
    ): SubmitResult = withContext(Dispatchers.IO) {
        val token = userToken ?: return@withContext SubmitResult(false, "Not authenticated")

        try {
            val listenData = JSONObject().apply {
                put("track_metadata", JSONObject().apply {
                    put("track_name", track)
                    put("artist_name", artist)
                    album?.let { put("release_name", it) }
                })
            }

            val payload = JSONObject().apply {
                put("listen_type", "playing_now")
                put("payload", JSONArray().put(listenData))
            }

            val request = Request.Builder()
                .url("${API_BASE_URL}submit-listens")
                .post(payload.toString().toRequestBody("application/json".toMediaType()))
                .header("Authorization", "Token $token")
                .build()

            httpClient.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    return@withContext SubmitResult(false, "HTTP ${response.code}")
                }

                val json = JSONObject(response.body?.string() ?: "")
                if (json.optString("status") == "ok") {
                    SubmitResult(true, null)
                } else {
                    SubmitResult(false, json.optString("error", "Unknown error"))
                }
            }
        } catch (e: Exception) {
            SubmitResult(false, e.message)
        }
    }

    suspend fun submitListen(
        track: String,
        artist: String,
        timestamp: Long,
        album: String? = null
    ): SubmitResult = withContext(Dispatchers.IO) {
        val token = userToken ?: return@withContext SubmitResult(false, "Not authenticated")

        try {
            val listenData = JSONObject().apply {
                put("listened_at", timestamp)
                put("track_metadata", JSONObject().apply {
                    put("track_name", track)
                    put("artist_name", artist)
                    album?.let { put("release_name", it) }
                })
            }

            val payload = JSONObject().apply {
                put("listen_type", "single")
                put("payload", JSONArray().put(listenData))
            }

            val request = Request.Builder()
                .url("${API_BASE_URL}submit-listens")
                .post(payload.toString().toRequestBody("application/json".toMediaType()))
                .header("Authorization", "Token $token")
                .build()

            httpClient.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    return@withContext SubmitResult(false, "HTTP ${response.code}")
                }

                val json = JSONObject(response.body?.string() ?: "")
                if (json.optString("status") == "ok") {
                    SubmitResult(true, null)
                } else {
                    SubmitResult(false, json.optString("error", "Unknown error"))
                }
            }
        } catch (e: Exception) {
            SubmitResult(false, e.message)
        }
    }

    suspend fun validateToken(token: String): SubmitResult = withContext(Dispatchers.IO) {
        try {
            val request = Request.Builder()
                .url("${API_BASE_URL}validate-token")
                .get()
                .header("Authorization", "Token $token")
                .build()

            httpClient.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    return@withContext SubmitResult(false, "HTTP ${response.code}")
                }

                val json = JSONObject(response.body?.string() ?: "")
                if (json.optBoolean("valid", false)) {
                    userToken = token
                    SubmitResult(true, null)
                } else {
                    SubmitResult(false, "Invalid token")
                }
            }
        } catch (e: Exception) {
            SubmitResult(false, e.message)
        }
    }

    fun setToken(token: String) {
        userToken = token
    }

    fun clearToken() {
        userToken = null
    }

    fun isAuthenticated(): Boolean = userToken != null
}
