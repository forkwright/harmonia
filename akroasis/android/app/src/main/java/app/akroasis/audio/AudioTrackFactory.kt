// Factory for creating AudioTrack instances (enables testing)
package app.akroasis.audio

import android.media.AudioTrack

interface AudioTrackFactory {
    fun createAudioTrack(decodedAudio: DecodedAudio, playbackSpeed: Float): AudioTrack?
}
