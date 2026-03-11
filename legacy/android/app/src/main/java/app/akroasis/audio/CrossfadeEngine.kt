// Crossfade between tracks with volume ramping
package app.akroasis.audio

import android.media.AudioTrack
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class CrossfadeEngine @Inject constructor() {

    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private var crossfadeJob: Job? = null

    fun crossfade(
        outgoingTrack: AudioTrack,
        incomingTrack: AudioTrack,
        durationMs: Int,
        onComplete: () -> Unit = {}
    ) {
        crossfadeJob?.cancel()

        crossfadeJob = scope.launch {
            val steps = 50
            val stepDuration = durationMs / steps

            incomingTrack.play()

            for (i in 0..steps) {
                if (!isActive) break

                val progress = i.toFloat() / steps

                // Equal power crossfade curve for constant perceived volume
                val outgoingVolume = calculateEqualPowerFadeOut(progress)
                val incomingVolume = calculateEqualPowerFadeIn(progress)

                outgoingTrack.setVolume(outgoingVolume)
                incomingTrack.setVolume(incomingVolume)

                delay(stepDuration.toLong())
            }

            outgoingTrack.stop()
            onComplete()
        }
    }

    fun fadeOut(track: AudioTrack, durationMs: Int, onComplete: () -> Unit = {}) {
        crossfadeJob?.cancel()

        crossfadeJob = scope.launch {
            val steps = 50
            val stepDuration = durationMs / steps

            for (i in 0..steps) {
                if (!isActive) break

                val progress = i.toFloat() / steps
                val volume = 1.0f - progress

                track.setVolume(volume)
                delay(stepDuration.toLong())
            }

            track.stop()
            onComplete()
        }
    }

    fun fadeIn(track: AudioTrack, durationMs: Int) {
        crossfadeJob?.cancel()

        crossfadeJob = scope.launch {
            val steps = 50
            val stepDuration = durationMs / steps

            track.setVolume(0.0f)
            track.play()

            for (i in 0..steps) {
                if (!isActive) break

                val progress = i.toFloat() / steps
                track.setVolume(progress)

                delay(stepDuration.toLong())
            }

            track.setVolume(1.0f)
        }
    }

    fun cancel() {
        crossfadeJob?.cancel()
        crossfadeJob = null
    }

    fun release() {
        cancel()
        scope.cancel()
    }

    private fun calculateEqualPowerFadeOut(progress: Float): Float {
        // cos(π/2 * progress) for smooth fade out
        return kotlin.math.cos(Math.PI.toFloat() / 2.0f * progress)
    }

    private fun calculateEqualPowerFadeIn(progress: Float): Float {
        // sin(π/2 * progress) for smooth fade in
        return kotlin.math.sin(Math.PI.toFloat() / 2.0f * progress)
    }
}
