// Android MediaSession integration for notification and lock screen controls
package app.akroasis.audio

import android.content.Context
import android.support.v4.media.MediaMetadataCompat
import android.support.v4.media.session.MediaSessionCompat
import android.support.v4.media.session.PlaybackStateCompat
import app.akroasis.data.model.Track
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class MediaSessionManager @Inject constructor(
    @ApplicationContext private val context: Context
) {
    private var mediaSession: MediaSessionCompat? = null
    private var onPlayPause: (() -> Unit)? = null
    private var onSkipToNext: (() -> Unit)? = null
    private var onSkipToPrevious: (() -> Unit)? = null
    private var onSeekTo: ((Long) -> Unit)? = null
    private var onStop: (() -> Unit)? = null

    fun initialize(
        onPlayPause: () -> Unit,
        onSkipToNext: () -> Unit,
        onSkipToPrevious: () -> Unit,
        onSeekTo: (Long) -> Unit,
        onStop: () -> Unit
    ) {
        this.onPlayPause = onPlayPause
        this.onSkipToNext = onSkipToNext
        this.onSkipToPrevious = onSkipToPrevious
        this.onSeekTo = onSeekTo
        this.onStop = onStop

        mediaSession = MediaSessionCompat(context, "AkroasisMediaSession").apply {
            setCallback(object : MediaSessionCompat.Callback() {
                override fun onPlay() {
                    this@MediaSessionManager.onPlayPause?.invoke()
                }

                override fun onPause() {
                    this@MediaSessionManager.onPlayPause?.invoke()
                }

                override fun onSkipToNext() {
                    this@MediaSessionManager.onSkipToNext?.invoke()
                }

                override fun onSkipToPrevious() {
                    this@MediaSessionManager.onSkipToPrevious?.invoke()
                }

                override fun onSeekTo(pos: Long) {
                    this@MediaSessionManager.onSeekTo?.invoke(pos)
                }

                override fun onStop() {
                    this@MediaSessionManager.onStop?.invoke()
                }
            })

            setFlags(
                MediaSessionCompat.FLAG_HANDLES_MEDIA_BUTTONS or
                MediaSessionCompat.FLAG_HANDLES_TRANSPORT_CONTROLS
            )

            isActive = true
        }
    }

    fun updatePlaybackState(
        state: PlaybackState,
        position: Long,
        speed: Float
    ) {
        val playbackState = when (state) {
            is PlaybackState.Playing -> PlaybackStateCompat.STATE_PLAYING
            is PlaybackState.Paused -> PlaybackStateCompat.STATE_PAUSED
            is PlaybackState.Buffering -> PlaybackStateCompat.STATE_BUFFERING
            is PlaybackState.Stopped -> PlaybackStateCompat.STATE_STOPPED
        }

        val stateBuilder = PlaybackStateCompat.Builder()
            .setState(playbackState, position, speed)
            .setActions(
                PlaybackStateCompat.ACTION_PLAY or
                PlaybackStateCompat.ACTION_PAUSE or
                PlaybackStateCompat.ACTION_PLAY_PAUSE or
                PlaybackStateCompat.ACTION_SKIP_TO_NEXT or
                PlaybackStateCompat.ACTION_SKIP_TO_PREVIOUS or
                PlaybackStateCompat.ACTION_SEEK_TO or
                PlaybackStateCompat.ACTION_STOP
            )

        mediaSession?.setPlaybackState(stateBuilder.build())
    }

    fun updateMetadata(track: Track, artworkUrl: String?) {
        val metadata = MediaMetadataCompat.Builder()
            .putString(MediaMetadataCompat.METADATA_KEY_TITLE, track.title)
            .putString(MediaMetadataCompat.METADATA_KEY_ARTIST, track.artist)
            .putString(MediaMetadataCompat.METADATA_KEY_ALBUM, track.album)
            .putLong(MediaMetadataCompat.METADATA_KEY_DURATION, track.duration)
            .putString(MediaMetadataCompat.METADATA_KEY_ALBUM_ART_URI, artworkUrl ?: "")
            .build()

        mediaSession?.setMetadata(metadata)
    }

    fun getSessionToken(): MediaSessionCompat.Token? {
        return mediaSession?.sessionToken
    }

    fun release() {
        mediaSession?.isActive = false
        mediaSession?.release()
        mediaSession = null
    }
}
