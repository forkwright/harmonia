// Background music playback service with MediaSession
package app.akroasis.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.os.Binder
import android.os.Build
import android.os.IBinder
import android.support.v4.media.MediaMetadataCompat
import android.support.v4.media.session.MediaSessionCompat
import android.support.v4.media.session.PlaybackStateCompat
import androidx.core.app.NotificationCompat
import app.akroasis.MainActivity
import app.akroasis.R
import android.widget.Toast
import app.akroasis.audio.AudioPlayer
import app.akroasis.audio.PlaybackQueue
import app.akroasis.audio.PlaybackState
import app.akroasis.audio.TrackLoader
import app.akroasis.audio.VoiceSearchHandler
import app.akroasis.audio.VoiceSearchResult
import app.akroasis.data.model.MediaType
import app.akroasis.data.repository.MediaRepository
import dagger.hilt.android.AndroidEntryPoint
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import timber.log.Timber
import javax.inject.Inject

@AndroidEntryPoint
class PlaybackService : Service() {

    @Inject
    lateinit var audioPlayer: AudioPlayer

    @Inject
    lateinit var playbackQueue: PlaybackQueue

    @Inject
    lateinit var trackLoader: TrackLoader

    @Inject
    lateinit var voiceSearchHandler: VoiceSearchHandler

    @Inject
    lateinit var mediaRepository: MediaRepository

    private val binder = PlaybackBinder()
    private val serviceScope = CoroutineScope(Dispatchers.Main + Job())

    private lateinit var mediaSession: MediaSessionCompat
    private lateinit var notificationManager: NotificationManager

    private var isForegroundService = false

    private var currentSessionId: String? = null
    private var sessionStartTime: Long = 0L
    private var progressUpdateJob: Job? = null
    private var lastProgressUpdateTime: Long = 0L

    override fun onCreate() {
        super.onCreate()

        notificationManager = getSystemService(NotificationManager::class.java)
        createNotificationChannel()

        mediaSession = MediaSessionCompat(this, "AkroasisMediaSession").apply {
            setCallback(mediaSessionCallback)
            isActive = true
        }

        observePlaybackState()
        observeQueue()
    }

    override fun onBind(intent: Intent?): IBinder {
        return binder
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        handleIntent(intent)
        return START_STICKY
    }

    override fun onDestroy() {
        super.onDestroy()
        stopProgressTracking()
        endCurrentSession()
        serviceScope.cancel()
        mediaSession.isActive = false
        mediaSession.release()
        audioPlayer.stop()
        stopForeground(STOP_FOREGROUND_REMOVE)
        isForegroundService = false
    }

    private fun handleIntent(intent: Intent?) {
        when (intent?.action) {
            ACTION_PLAY -> {
                playbackQueue.currentTrack?.let {
                    // Track loading handled by PlayerViewModel
                }
            }
            ACTION_PAUSE -> audioPlayer.pause()
            ACTION_SKIP_TO_NEXT -> {
                // Handled by PlayerViewModel via MediaSession
            }
            ACTION_SKIP_TO_PREVIOUS -> {
                // Handled by PlayerViewModel via MediaSession
            }
            ACTION_STOP -> {
                playbackQueue.currentTrack?.let { track ->
                    val positionMs = audioPlayer.position.value
                    updateProgressForTrack(track, positionMs)
                }
                stopProgressTracking()
                endCurrentSession()
                audioPlayer.stop()
                stopSelf()
            }
        }
    }

    private fun observePlaybackState() {
        serviceScope.launch {
            audioPlayer.playbackState.collect { state ->
                updateMediaSessionPlaybackState(state)
                updateNotification(state)
            }
        }
    }

    private fun observeQueue() {
        serviceScope.launch {
            playbackQueue.currentIndex.collect { _ ->
                playbackQueue.currentTrack?.let { track ->
                    updateMediaSessionMetadata(track.title, track.artist, track.album)
                }
            }
        }
    }

    private fun updateMediaSessionPlaybackState(state: PlaybackState) {
        val playbackState = when (state) {
            is PlaybackState.Playing -> PlaybackStateCompat.STATE_PLAYING
            is PlaybackState.Paused -> PlaybackStateCompat.STATE_PAUSED
            is PlaybackState.Stopped -> PlaybackStateCompat.STATE_STOPPED
            is PlaybackState.Buffering -> PlaybackStateCompat.STATE_BUFFERING
        }

        val stateBuilder = PlaybackStateCompat.Builder()
            .setState(playbackState, audioPlayer.position.value, 1.0f)
            .setActions(
                PlaybackStateCompat.ACTION_PLAY or
                PlaybackStateCompat.ACTION_PAUSE or
                PlaybackStateCompat.ACTION_SKIP_TO_NEXT or
                PlaybackStateCompat.ACTION_SKIP_TO_PREVIOUS or
                PlaybackStateCompat.ACTION_STOP
            )

        mediaSession.setPlaybackState(stateBuilder.build())
    }

    private fun updateMediaSessionMetadata(title: String, artist: String, album: String) {
        val metadata = MediaMetadataCompat.Builder()
            .putString(MediaMetadataCompat.METADATA_KEY_TITLE, title)
            .putString(MediaMetadataCompat.METADATA_KEY_ARTIST, artist)
            .putString(MediaMetadataCompat.METADATA_KEY_ALBUM, album)
            .build()

        mediaSession.setMetadata(metadata)
    }

    private fun updateNotification(state: PlaybackState) {
        val notification = buildNotification(state)

        when (state) {
            is PlaybackState.Playing, is PlaybackState.Buffering -> {
                if (!isForegroundService) {
                    startForeground(NOTIFICATION_ID, notification)
                    isForegroundService = true
                } else {
                    notificationManager.notify(NOTIFICATION_ID, notification)
                }
            }
            is PlaybackState.Paused -> {
                if (isForegroundService) {
                    stopForeground(STOP_FOREGROUND_DETACH)
                    isForegroundService = false
                }
                notificationManager.notify(NOTIFICATION_ID, notification)
            }
            is PlaybackState.Stopped -> {
                if (isForegroundService) {
                    stopForeground(STOP_FOREGROUND_REMOVE)
                    isForegroundService = false
                }
            }
        }
    }

    private fun buildNotification(state: PlaybackState): Notification {
        val track = playbackQueue.currentTrack

        val contentIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val playPauseAction = if (state is PlaybackState.Playing) {
            NotificationCompat.Action(
                android.R.drawable.ic_media_pause,
                "Pause",
                createPendingIntent(ACTION_PAUSE)
            )
        } else {
            NotificationCompat.Action(
                android.R.drawable.ic_media_play,
                "Play",
                createPendingIntent(ACTION_PLAY)
            )
        }

        val style = androidx.media.app.NotificationCompat.MediaStyle()
            .setMediaSession(mediaSession.sessionToken)
            .setShowActionsInCompactView(0, 1, 2)

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle(track?.title ?: "No track")
            .setContentText(track?.artist ?: "")
            .setSubText(track?.album ?: "")
            .setSmallIcon(android.R.drawable.ic_media_play)
            .setContentIntent(contentIntent)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setOnlyAlertOnce(true)
            .addAction(
                android.R.drawable.ic_media_previous,
                "Previous",
                createPendingIntent(ACTION_SKIP_TO_PREVIOUS)
            )
            .addAction(playPauseAction)
            .addAction(
                android.R.drawable.ic_media_next,
                "Next",
                createPendingIntent(ACTION_SKIP_TO_NEXT)
            )
            .setStyle(style)
            .build()
    }

    private fun createPendingIntent(action: String): PendingIntent {
        val intent = Intent(this, PlaybackService::class.java).apply {
            this.action = action
        }
        return PendingIntent.getService(
            this,
            action.hashCode(),
            intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Playback",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Music playback notifications"
                setShowBadge(false)
                lockscreenVisibility = Notification.VISIBILITY_PUBLIC
            }
            notificationManager.createNotificationChannel(channel)
        }
    }

    private val mediaSessionCallback = object : MediaSessionCompat.Callback() {
        override fun onPlay() {
            playbackQueue.currentTrack?.let { track ->
                loadAndPlayTrack(track)
            }
        }

        override fun onPause() {
            audioPlayer.pause()
        }

        override fun onSkipToNext() {
            serviceScope.launch {
                playbackQueue.skipToNext()?.let { track ->
                    loadAndPlayTrack(track)
                }
            }
        }

        override fun onSkipToPrevious() {
            serviceScope.launch {
                val currentPosition = audioPlayer.getCurrentPositionMs()
                if (currentPosition > 3000) {
                    audioPlayer.seekTo(0)
                } else {
                    playbackQueue.skipToPrevious()?.let { track ->
                        loadAndPlayTrack(track)
                    }
                }
            }
        }

        override fun onSeekTo(pos: Long) {
            audioPlayer.seekTo(pos)
        }

        override fun onStop() {
            // Save final progress before stopping
            playbackQueue.currentTrack?.let { track ->
                val positionMs = audioPlayer.position.value
                updateProgressForTrack(track, positionMs)
            }
            stopProgressTracking()
            endCurrentSession()
            audioPlayer.stop()
            stopSelf()
        }

        override fun onPlayFromSearch(query: String?, extras: android.os.Bundle?) {
            timber.log.Timber.d("Voice search: query='$query'")

            serviceScope.launch {
                when (val result = voiceSearchHandler.handleVoiceSearch(query, extras)) {
                    is VoiceSearchResult.Success -> {
                        timber.log.Timber.d("Voice search found ${result.tracks.size} tracks")

                        // Replace queue with search results
                        playbackQueue.setQueue(result.tracks, result.startIndex)

                        // Start playing from specified index
                        result.tracks.getOrNull(result.startIndex)?.let { track ->
                            loadAndPlayTrack(track)
                        }
                    }
                    is VoiceSearchResult.NoResults -> {
                        timber.log.Timber.w("Voice search: no results for '${result.query}'")
                        Toast.makeText(
                            this@PlaybackService,
                            "No results found for '${result.query}'",
                            Toast.LENGTH_SHORT
                        ).show()
                    }
                    is VoiceSearchResult.Error -> {
                        timber.log.Timber.e("Voice search error: ${result.message}")
                        Toast.makeText(
                            this@PlaybackService,
                            "Search failed: ${result.message}",
                            Toast.LENGTH_SHORT
                        ).show()
                    }
                }
            }
        }
    }

    private fun loadAndPlayTrack(track: app.akroasis.data.model.Track) {
        serviceScope.launch {
            // End any existing session before starting new track
            endCurrentSession()

            val decodedResult = trackLoader.loadAndDecodeTrack(track.id)
            val decodedAudio = decodedResult.getOrNull()
            if (decodedAudio != null) {
                audioPlayer.play(decodedAudio)
                // Start new session and progress tracking for this track
                startSessionForTrack(track)
                startProgressTracking(track)
            }
        }
    }

    private fun startSessionForTrack(track: app.akroasis.data.model.Track) {
        serviceScope.launch {
            sessionStartTime = System.currentTimeMillis()
            mediaRepository.startSession(
                mediaId = track.id,
                mediaType = MediaType.MUSIC
            ).onSuccess { session ->
                currentSessionId = session.id
                Timber.d("Started session ${session.id} for track ${track.title}")
            }.onFailure { e ->
                Timber.e(e, "Failed to start session for track ${track.title}")
            }
        }
    }

    private fun endCurrentSession() {
        val sessionId = currentSessionId ?: return
        val durationMs = System.currentTimeMillis() - sessionStartTime

        serviceScope.launch {
            mediaRepository.endSession(sessionId, durationMs)
                .onSuccess {
                    Timber.d("Ended session $sessionId (duration: ${durationMs}ms)")
                }
                .onFailure { e ->
                    Timber.e(e, "Failed to end session $sessionId")
                }
        }
        currentSessionId = null
        sessionStartTime = 0L
    }

    private fun startProgressTracking(track: app.akroasis.data.model.Track) {
        progressUpdateJob?.cancel()
        progressUpdateJob = serviceScope.launch {
            while (isActive) {
                delay(PROGRESS_UPDATE_INTERVAL_MS)

                val currentState = audioPlayer.playbackState.value
                if (currentState is PlaybackState.Playing) {
                    val positionMs = audioPlayer.position.value
                    val now = System.currentTimeMillis()

                    // Only update if enough time has passed since last update
                    if (now - lastProgressUpdateTime >= PROGRESS_UPDATE_INTERVAL_MS) {
                        updateProgressForTrack(track, positionMs)
                        lastProgressUpdateTime = now
                    }
                }
            }
        }
    }

    private fun updateProgressForTrack(track: app.akroasis.data.model.Track, positionMs: Long) {
        serviceScope.launch {
            mediaRepository.updateProgress(
                mediaId = track.id,
                mediaType = MediaType.MUSIC,
                positionMs = positionMs,
                durationMs = track.duration?.toLong()
            ).onFailure { e ->
                Timber.w(e, "Failed to update progress for track ${track.id}")
            }
        }
    }

    private fun stopProgressTracking() {
        progressUpdateJob?.cancel()
        progressUpdateJob = null
    }

    inner class PlaybackBinder : Binder() {
        fun getService(): PlaybackService = this@PlaybackService
    }

    companion object {
        private const val CHANNEL_ID = "playback_channel"
        private const val NOTIFICATION_ID = 1
        private const val PROGRESS_UPDATE_INTERVAL_MS = 15_000L // Update every 15 seconds

        const val ACTION_PLAY = "app.akroasis.PLAY"
        const val ACTION_PAUSE = "app.akroasis.PAUSE"
        const val ACTION_SKIP_TO_NEXT = "app.akroasis.SKIP_TO_NEXT"
        const val ACTION_SKIP_TO_PREVIOUS = "app.akroasis.SKIP_TO_PREVIOUS"
        const val ACTION_STOP = "app.akroasis.STOP"
    }
}
