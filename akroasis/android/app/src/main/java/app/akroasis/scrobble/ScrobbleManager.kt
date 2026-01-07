// Coordinates scrobbling across Last.fm and ListenBrainz
package app.akroasis.scrobble

import android.content.Context
import app.akroasis.data.model.Track
import app.akroasis.data.preferences.ScrobblePreferences
import app.akroasis.scrobble.lastfm.LastFmClient
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import timber.log.Timber
import kotlin.math.pow
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ScrobbleManager @Inject constructor(
    @ApplicationContext private val context: Context,
    private val scrobblePrefs: ScrobblePreferences,
    private val lastFmClient: LastFmClient,
    private val listenBrainzClient: app.akroasis.scrobble.listenbrainz.ListenBrainzClient
) {
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private val scrobbleMutex = Mutex()

    private val _scrobbleState = MutableStateFlow<ScrobbleState>(ScrobbleState.Idle)
    val scrobbleState: StateFlow<ScrobbleState> = _scrobbleState.asStateFlow()

    private var currentTrack: Track? = null
    private var trackStartTime: Long = 0
    private var trackStartSpeed: Float = 1.0f
    private var hasScrobbled = false
    private var hasUpdatedNowPlaying = false

    sealed class ScrobbleState {
        object Idle : ScrobbleState()
        data class NowPlaying(val track: Track) : ScrobbleState()
        data class Scrobbled(val track: Track) : ScrobbleState()
        data class Error(val message: String) : ScrobbleState()
    }

    companion object {
        private const val ERROR_UNKNOWN = "Unknown error"
    }

    init {
        // Restore Last.fm session if available
        scrobblePrefs.lastFmSessionKey?.let { sessionKey ->
            lastFmClient.setSessionKey(sessionKey)
        }

        // Restore ListenBrainz token if available
        scrobblePrefs.listenBrainzToken?.let { token ->
            listenBrainzClient.setToken(token)
        }
    }

    fun onTrackStarted(track: Track, playbackSpeed: Float = 1.0f) {
        Timber.d("Track started: ${track.title} - ${track.artist} (speed: ${playbackSpeed}x)")
        currentTrack = track
        trackStartTime = System.currentTimeMillis() / 1000
        trackStartSpeed = playbackSpeed
        hasScrobbled = false
        hasUpdatedNowPlaying = false

        updateNowPlaying(track)
    }

    fun onPlaybackProgress(track: Track, position: Long, duration: Long) {
        if (currentTrack?.id != track.id) {
            onTrackStarted(track)
            return
        }

        if (hasScrobbled) return

        val scrobbleThreshold = calculateScrobbleThreshold(duration)
        if (position >= scrobbleThreshold && duration >= scrobblePrefs.scrobbleMinDuration * 1000) {
            submitScrobble(track, duration)
        }
    }

    fun onTrackStopped() {
        Timber.d("Track stopped")
        currentTrack = null
        hasScrobbled = false
        hasUpdatedNowPlaying = false
        _scrobbleState.value = ScrobbleState.Idle
    }

    private fun calculateScrobbleThreshold(duration: Long): Long {
        val percentageThreshold = (duration * scrobblePrefs.scrobblePercentage) / 100
        val fixedThreshold = 4 * 60 * 1000L // 4 minutes
        return minOf(percentageThreshold, fixedThreshold)
    }

    private fun updateNowPlaying(track: Track) {
        if (hasUpdatedNowPlaying) return

        scope.launch {
            var nowPlayingUpdated = false

            if (scrobblePrefs.lastFmEnabled && lastFmClient.isAuthenticated()) {
                val result = lastFmClient.updateNowPlaying(
                    track = track.title,
                    artist = track.artist,
                    album = track.album,
                    duration = (track.duration / 1000).toInt()
                )

                if (result.success) {
                    nowPlayingUpdated = true
                } else {
                    _scrobbleState.value = ScrobbleState.Error(result.error ?: ERROR_UNKNOWN)
                }
            }

            if (scrobblePrefs.listenBrainzEnabled && listenBrainzClient.isAuthenticated()) {
                val result = listenBrainzClient.submitPlayingNow(
                    track = track.title,
                    artist = track.artist,
                    album = track.album
                )

                if (result.success) {
                    nowPlayingUpdated = true
                } else if (!nowPlayingUpdated) {
                    _scrobbleState.value = ScrobbleState.Error(result.error ?: ERROR_UNKNOWN)
                }
            }

            if (nowPlayingUpdated) {
                Timber.i("Now Playing updated: ${track.title}")
                hasUpdatedNowPlaying = true
                _scrobbleState.value = ScrobbleState.NowPlaying(track)
            }
        }
    }

    private suspend fun <T> retryWithBackoff(
        maxRetries: Int = 3,
        initialDelayMs: Long = 1000,
        operation: suspend () -> T
    ): T? {
        repeat(maxRetries) { attempt ->
            try {
                return operation()
            } catch (e: Exception) {
                if (attempt < maxRetries - 1) {
                    val delayMs = (initialDelayMs * 2.0.pow(attempt)).toLong()
                    Timber.w("Retry attempt ${attempt + 1}/$maxRetries in ${delayMs}ms")
                    delay(delayMs)
                } else {
                    Timber.e(e, "Failed after $maxRetries attempts")
                    throw e
                }
            }
        }
        return null
    }

    private fun submitScrobble(track: Track, duration: Long) {
        scope.launch {
            scrobbleMutex.withLock {
                if (hasScrobbled) return@withLock

                var scrobbled = false

                if (scrobblePrefs.lastFmEnabled && lastFmClient.isAuthenticated()) {
                    val result = try {
                        retryWithBackoff {
                            lastFmClient.scrobble(
                                track = track.title,
                                artist = track.artist,
                                timestamp = trackStartTime,
                                album = track.album,
                                duration = (duration / 1000).toInt()
                            )
                        }
                    } catch (e: Exception) {
                        null
                    }

                    if (result?.success == true) {
                        scrobbled = true
                    } else {
                        _scrobbleState.value = ScrobbleState.Error(result?.error ?: "Unknown error")
                    }
                }

                if (scrobblePrefs.listenBrainzEnabled && listenBrainzClient.isAuthenticated()) {
                    val result = try {
                        retryWithBackoff {
                            listenBrainzClient.submitListen(
                                track = track.title,
                                artist = track.artist,
                                timestamp = trackStartTime,
                                album = track.album
                            )
                        }
                    } catch (e: Exception) {
                        null
                    }

                    if (result?.success == true) {
                        scrobbled = true
                    } else if (!scrobbled) {
                        _scrobbleState.value = ScrobbleState.Error(result?.error ?: "Unknown error")
                    }
                }

                if (scrobbled) {
                    Timber.i("Scrobbled: ${track.title} - ${track.artist} (started at speed ${trackStartSpeed}x, timestamp: $trackStartTime)")
                    hasScrobbled = true
                    _scrobbleState.value = ScrobbleState.Scrobbled(track)
                } else {
                    Timber.w("Scrobble failed for: ${track.title}")
                }
            }
        }
    }

    suspend fun authenticateLastFm(username: String, password: String): LastFmClient.AuthResult {
        val result = lastFmClient.authenticate(username, password)

        if (result.success && result.sessionKey != null) {
            scrobblePrefs.lastFmSessionKey = result.sessionKey
            scrobblePrefs.lastFmUsername = username
            scrobblePrefs.lastFmEnabled = true
        }

        return result
    }

    fun disconnectLastFm() {
        lastFmClient.clearSession()
        scrobblePrefs.clearLastFmSession()
    }

    fun isLastFmConnected(): Boolean {
        return scrobblePrefs.lastFmEnabled && lastFmClient.isAuthenticated()
    }

    fun getLastFmUsername(): String? = scrobblePrefs.lastFmUsername

    suspend fun authenticateListenBrainz(token: String): app.akroasis.scrobble.listenbrainz.ListenBrainzClient.SubmitResult {
        val result = listenBrainzClient.validateToken(token)

        if (result.success) {
            scrobblePrefs.listenBrainzToken = token
            scrobblePrefs.listenBrainzEnabled = true
        }

        return result
    }

    fun disconnectListenBrainz() {
        listenBrainzClient.clearToken()
        scrobblePrefs.clearListenBrainzSession()
    }

    fun isListenBrainzConnected(): Boolean {
        return scrobblePrefs.listenBrainzEnabled && listenBrainzClient.isAuthenticated()
    }
}
