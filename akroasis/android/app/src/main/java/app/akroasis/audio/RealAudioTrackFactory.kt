// Real AudioTrack factory implementation
package app.akroasis.audio

import android.app.ActivityManager
import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.os.Build
import dagger.hilt.android.qualifiers.ApplicationContext
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class RealAudioTrackFactory @Inject constructor(
    @ApplicationContext private val context: Context
) : AudioTrackFactory {

    override fun createAudioTrack(decodedAudio: DecodedAudio, playbackSpeed: Float): AudioTrack? {
        val channelConfig = if (decodedAudio.channels == 1) {
            AudioFormat.CHANNEL_OUT_MONO
        } else {
            AudioFormat.CHANNEL_OUT_STEREO
        }

        val audioFormat = AudioFormat.Builder()
            .setSampleRate(decodedAudio.sampleRate)
            .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
            .setChannelMask(channelConfig)
            .build()

        val audioAttributes = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_MEDIA)
            .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
            .apply {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                    setAllowedCapturePolicy(AudioAttributes.ALLOW_CAPTURE_BY_NONE)
                }
            }
            .build()

        val bufferSize = decodedAudio.samples.size * 2
        val memoryThresholdBytes = calculateMemoryThreshold()

        return try {
            val track = if (bufferSize < memoryThresholdBytes) {
                AudioTrack.Builder()
                    .setAudioAttributes(audioAttributes)
                    .setAudioFormat(audioFormat)
                    .setBufferSizeInBytes(bufferSize)
                    .setTransferMode(AudioTrack.MODE_STATIC)
                    .apply {
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                            setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
                        }
                    }
                    .build()
                    .also { it.write(decodedAudio.samples, 0, decodedAudio.samples.size) }
            } else {
                val minBufferSize = AudioTrack.getMinBufferSize(
                    decodedAudio.sampleRate,
                    channelConfig,
                    AudioFormat.ENCODING_PCM_16BIT
                )

                AudioTrack.Builder()
                    .setAudioAttributes(audioAttributes)
                    .setAudioFormat(audioFormat)
                    .setBufferSizeInBytes(minBufferSize)
                    .setTransferMode(AudioTrack.MODE_STREAM)
                    .build()
            }

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                val params = track.playbackParams
                params.speed = playbackSpeed
                track.playbackParams = params
            }

            track
        } catch (e: Exception) {
            Timber.e(e, "Failed to create AudioTrack")
            null
        }
    }

    private fun calculateMemoryThreshold(): Int {
        val activityManager = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
        val memoryInfo = ActivityManager.MemoryInfo()
        activityManager.getMemoryInfo(memoryInfo)
        val availableMemoryMB = memoryInfo.availMem / (1024 * 1024)
        return if (availableMemoryMB > 512) 64 * 1024 * 1024 else 32 * 1024 * 1024
    }
}
