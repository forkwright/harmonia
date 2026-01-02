// Persists playback state for quick resume
package app.akroasis.data.persistence

import android.content.Context
import android.content.SharedPreferences
import app.akroasis.data.model.Track
import dagger.hilt.android.qualifiers.ApplicationContext
import org.json.JSONArray
import org.json.JSONObject
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class PlaybackStateStore @Inject constructor(
    @ApplicationContext context: Context
) {
    private val prefs: SharedPreferences = context.getSharedPreferences(
        "playback_state",
        Context.MODE_PRIVATE
    )

    companion object {
        private const val KEY_CURRENT_TRACK = "current_track"
        private const val KEY_POSITION = "position"
        private const val KEY_QUEUE = "queue"
        private const val KEY_CURRENT_INDEX = "current_index"
        private const val KEY_SHUFFLE_ENABLED = "shuffle_enabled"
        private const val KEY_REPEAT_MODE = "repeat_mode"
        private const val KEY_PLAYBACK_SPEED = "playback_speed"
        private const val KEY_TIMESTAMP = "timestamp"
    }

    data class PlaybackState(
        val currentTrack: Track?,
        val position: Long,
        val queue: List<Track>,
        val currentIndex: Int,
        val shuffleEnabled: Boolean,
        val repeatMode: String,
        val playbackSpeed: Float,
        val timestamp: Long
    )

    fun saveState(state: PlaybackState) {
        val editor = prefs.edit()

        state.currentTrack?.let { track ->
            val trackJson = trackToJson(track)
            editor.putString(KEY_CURRENT_TRACK, trackJson.toString())
        }

        editor.putLong(KEY_POSITION, state.position)
        editor.putInt(KEY_CURRENT_INDEX, state.currentIndex)
        editor.putBoolean(KEY_SHUFFLE_ENABLED, state.shuffleEnabled)
        editor.putString(KEY_REPEAT_MODE, state.repeatMode)
        editor.putFloat(KEY_PLAYBACK_SPEED, state.playbackSpeed)
        editor.putLong(KEY_TIMESTAMP, state.timestamp)

        val queueJson = JSONArray()
        state.queue.forEach { track ->
            queueJson.put(trackToJson(track))
        }
        editor.putString(KEY_QUEUE, queueJson.toString())

        editor.apply()
    }

    fun restoreState(): PlaybackState? {
        val trackJson = prefs.getString(KEY_CURRENT_TRACK, null) ?: return null
        val currentTrack = try {
            jsonToTrack(JSONObject(trackJson))
        } catch (e: Exception) {
            null
        } ?: return null

        val position = prefs.getLong(KEY_POSITION, 0)
        val currentIndex = prefs.getInt(KEY_CURRENT_INDEX, 0)
        val shuffleEnabled = prefs.getBoolean(KEY_SHUFFLE_ENABLED, false)
        val repeatMode = prefs.getString(KEY_REPEAT_MODE, "OFF") ?: "OFF"
        val playbackSpeed = prefs.getFloat(KEY_PLAYBACK_SPEED, 1.0f)
        val timestamp = prefs.getLong(KEY_TIMESTAMP, 0)

        val queue = try {
            val queueJsonStr = prefs.getString(KEY_QUEUE, null)
            if (queueJsonStr != null) {
                val queueJson = JSONArray(queueJsonStr)
                (0 until queueJson.length()).mapNotNull { i ->
                    try {
                        jsonToTrack(queueJson.getJSONObject(i))
                    } catch (e: Exception) {
                        null
                    }
                }
            } else {
                emptyList()
            }
        } catch (e: Exception) {
            emptyList()
        }

        return PlaybackState(
            currentTrack = currentTrack,
            position = position,
            queue = queue,
            currentIndex = currentIndex,
            shuffleEnabled = shuffleEnabled,
            repeatMode = repeatMode,
            playbackSpeed = playbackSpeed,
            timestamp = timestamp
        )
    }

    fun clearState() {
        prefs.edit().clear().apply()
    }

    private fun trackToJson(track: Track): JSONObject {
        return JSONObject().apply {
            put("id", track.id)
            put("title", track.title)
            put("artist", track.artist)
            put("album", track.album)
            put("duration", track.duration)
            put("format", track.format)
            put("coverArtUrl", track.coverArtUrl ?: "")
        }
    }

    private fun jsonToTrack(json: JSONObject): Track {
        return Track(
            id = json.getString("id"),
            title = json.getString("title"),
            artist = json.getString("artist"),
            album = json.getString("album"),
            duration = json.getLong("duration"),
            format = json.optString("format", ""),
            coverArtUrl = json.optString("coverArtUrl", null)
        )
    }
}
