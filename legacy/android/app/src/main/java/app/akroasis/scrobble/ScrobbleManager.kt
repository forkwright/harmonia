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

private const val ERROR_UNKNOWN = "Unknown error"
private const val MAX_RETRIES = 3
private const val INITIAL_DELAY_MS = 1000L
private const val FOUR_MINUTES_MS = 4 * 60 * 1000L

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

    init {
        restoreSessions()
    }

    private fun restoreSessions() {
        scrobblePrefs.lastFmSessionKey?.let { lastFmClient.setSessionKey(it) }
        scrobblePrefs.listenBrainzToken?.let { listenBrainzClient.setToken(it) }
    }

    fun onTrackStarted(track: Track, playbackSpeed: Float = 1.0f) {
        Timber.d("Track started: ${track.title} - ${track.artist} (speed: ${playbackSpeed}x)")
        resetTrackState(track, playbackSpeed)
        updateNowPlaying(track)
    }

    private fun resetTrackState(track: Track, playbackSpeed: Float) {
        currentTrack = track
        trackStartTime = System.currentTimeMillis() / 1000
        trackStartSpeed = playbackSpeed
        hasScrobbled = false
        hasUpdatedNowPlaying = false
    }

    fun onPlaybackProgress(track: Track, position: Long, duration: Long) {
        if (currentTrack?.id != track.id) {
            onTrackStarted(track)
            return
        }
        if (hasScrobbled) return
        if (!shouldScrobble(position, duration)) return

        submitScrobble(track, duration)
    }

    private fun shouldScrobble(position: Long, duration: Long): Boolean {
        val scrobbleThreshold = calculateScrobbleThreshold(duration)
        val minDuration = scrobblePrefs.scrobbleMinDuration * 1000
        return position >= scrobbleThreshold && duration >= minDuration
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
        return minOf(percentageThreshold, FOUR_MINUTES_MS)
    }

    private fun isLastFmReady(): Boolean =
        scrobblePrefs.lastFmEnabled && lastFmClient.isAuthenticated()

    private fun isListenBrainzReady(): Boolean =
        scrobblePrefs.listenBrainzEnabled && listenBrainzClient.isAuthenticated()

    private suspend fun updateLastFmNowPlaying(track: Track): Boolean {
        if (!isLastFmReady()) return false

        val result = lastFmClient.updateNowPlaying(
            track = track.title,
            artist = track.artist,
            album = track.album,
            duration = (track.duration / 1000).toInt()
        )
        handleResultError(result.success, result.error, false)
        return result.success
    }

    private suspend fun updateListenBrainzNowPlaying(track: Track, previousSuccess: Boolean): Boolean {
        if (!isListenBrainzReady()) return false

        val result = listenBrainzClient.submitPlayingNow(
            track = track.title,
            artist = track.artist,
            album = track.album
        )
        handleResultError(result.success, result.error, previousSuccess)
        return result.success
    }

    private fun handleResultError(success: Boolean, error: String?, previousSuccess: Boolean) {
        if (!success && !previousSuccess) {
            _scrobbleState.value = ScrobbleState.Error(error ?: ERROR_UNKNOWN)
        }
    }

    private fun updateNowPlaying(track: Track) {
        if (hasUpdatedNowPlaying) return

        scope.launch {
            val results = submitNowPlayingToServices(track)
            if (results.anySuccess) {
                Timber.i("Now Playing updated: ${track.title}")
                hasUpdatedNowPlaying = true
                _scrobbleState.value = ScrobbleState.NowPlaying(track)
            }
        }
    }

    private suspend fun submitNowPlayingToServices(track: Track): ScrobbleResults {
        val lastFmSuccess = updateLastFmNowPlaying(track)
        val listenBrainzSuccess = updateListenBrainzNowPlaying(track, lastFmSuccess)
        return ScrobbleResults(lastFmSuccess, listenBrainzSuccess)
    }

    private suspend fun <T> retryWithBackoff(operation: suspend () -> T): T? {
        repeat(MAX_RETRIES) { attempt ->
            try {
                return operation()
            } catch (e: Exception) {
                handleRetryAttempt(attempt, e)
            }
        }
        return null
    }

    private suspend fun handleRetryAttempt(attempt: Int, e: Exception) {
        if (attempt < MAX_RETRIES - 1) {
            val delayMs = calculateBackoffDelay(attempt)
            Timber.w("Retry attempt ${attempt + 1}/$MAX_RETRIES in ${delayMs}ms")
            delay(delayMs)
        } else {
            Timber.e(e, "Failed after $MAX_RETRIES attempts")
            throw e
        }
    }

    private fun calculateBackoffDelay(attempt: Int): Long =
        (INITIAL_DELAY_MS * 2.0.pow(attempt)).toLong()

    private suspend fun submitLastFmScrobble(track: Track, duration: Long): Boolean {
        if (!isLastFmReady()) return false

        val result = runScrobbleWithRetry {
            lastFmClient.scrobble(
                track = track.title,
                artist = track.artist,
                timestamp = trackStartTime,
                album = track.album,
                duration = (duration / 1000).toInt()
            )
        }
        handleScrobbleResult(result?.success, result?.error, false)
        return result?.success == true
    }

    private suspend fun submitListenBrainzScrobble(track: Track, previousSuccess: Boolean): Boolean {
        if (!isListenBrainzReady()) return false

        val result = runScrobbleWithRetry {
            listenBrainzClient.submitListen(
                track = track.title,
                artist = track.artist,
                timestamp = trackStartTime,
                album = track.album
            )
        }
        handleScrobbleResult(result?.success, result?.error, previousSuccess)
        return result?.success == true
    }

    private suspend fun <T> runScrobbleWithRetry(operation: suspend () -> T): T? {
        return try {
            retryWithBackoff(operation)
        } catch (e: Exception) {
            null
        }
    }

    private fun handleScrobbleResult(success: Boolean?, error: String?, previousSuccess: Boolean) {
        if (success != true && !previousSuccess) {
            _scrobbleState.value = ScrobbleState.Error(error ?: ERROR_UNKNOWN)
        }
    }

    private fun submitScrobble(track: Track, duration: Long) {
        scope.launch {
            scrobbleMutex.withLock {
                if (hasScrobbled) return@withLock
                performScrobble(track, duration)
            }
        }
    }

    private suspend fun performScrobble(track: Track, duration: Long) {
        val results = submitScrobbleToServices(track, duration)
        updateScrobbleState(track, results)
    }

    private suspend fun submitScrobbleToServices(track: Track, duration: Long): ScrobbleResults {
        val lastFmSuccess = submitLastFmScrobble(track, duration)
        val listenBrainzSuccess = submitListenBrainzScrobble(track, lastFmSuccess)
        return ScrobbleResults(lastFmSuccess, listenBrainzSuccess)
    }

    private fun updateScrobbleState(track: Track, results: ScrobbleResults) {
        if (results.anySuccess) {
            logScrobbleSuccess(track)
            hasScrobbled = true
            _scrobbleState.value = ScrobbleState.Scrobbled(track)
        } else {
            Timber.w("Scrobble failed for: ${track.title}")
        }
    }

    private fun logScrobbleSuccess(track: Track) {
        Timber.i("Scrobbled: ${track.title} - ${track.artist} (started at speed ${trackStartSpeed}x, timestamp: $trackStartTime)")
    }

    suspend fun authenticateLastFm(username: String, password: String): LastFmClient.AuthResult {
        val result = lastFmClient.authenticate(username, password)
        if (result.success && result.sessionKey != null) {
            saveLastFmSession(username, result.sessionKey)
        }
        return result
    }

    private fun saveLastFmSession(username: String, sessionKey: String) {
        scrobblePrefs.lastFmSessionKey = sessionKey
        scrobblePrefs.lastFmUsername = username
        scrobblePrefs.lastFmEnabled = true
    }

    fun disconnectLastFm() {
        lastFmClient.clearSession()
        scrobblePrefs.clearLastFmSession()
    }

    fun isLastFmConnected(): Boolean = isLastFmReady()

    fun getLastFmUsername(): String? = scrobblePrefs.lastFmUsername

    suspend fun authenticateListenBrainz(token: String): app.akroasis.scrobble.listenbrainz.ListenBrainzClient.SubmitResult {
        val result = listenBrainzClient.validateToken(token)
        if (result.success) {
            saveListenBrainzSession(token)
        }
        return result
    }

    private fun saveListenBrainzSession(token: String) {
        scrobblePrefs.listenBrainzToken = token
        scrobblePrefs.listenBrainzEnabled = true
    }

    fun disconnectListenBrainz() {
        listenBrainzClient.clearToken()
        scrobblePrefs.clearListenBrainzSession()
    }

    fun isListenBrainzConnected(): Boolean = isListenBrainzReady()

    private data class ScrobbleResults(
        val lastFmSuccess: Boolean,
        val listenBrainzSuccess: Boolean
    ) {
        val anySuccess: Boolean get() = lastFmSuccess || listenBrainzSuccess
    }
}
