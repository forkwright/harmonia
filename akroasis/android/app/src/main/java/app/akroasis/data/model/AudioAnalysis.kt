// Audio quality analysis data from /api/v3/tracks/{id}/audio-analysis
package app.akroasis.data.model

data class AudioAnalysis(
    val trackId: String,
    val format: String,           // "FLAC", "MP3", "ALAC", etc.
    val sampleRate: Int,          // Hz (e.g., 44100, 48000, 96000)
    val bitDepth: Int?,           // 16, 24, 32 (null for lossy)
    val channels: Int,            // 1 (mono), 2 (stereo), 6 (5.1), etc.
    val dynamicRange: Int?,       // DR value (1-20+, null if unavailable)
    val replayGain: ReplayGainInfo?,
    val lossless: Boolean,        // true for FLAC/ALAC/WAV, false for MP3/AAC
    val transcoded: Boolean       // true if spectral analysis detected fake hi-res
)

data class ReplayGainInfo(
    val trackGain: Float,   // dB adjustment for track-level normalization
    val trackPeak: Float,   // Peak sample value (0.0-1.0)
    val albumGain: Float?,  // dB adjustment for album-level normalization
    val albumPeak: Float?   // Album peak sample value (0.0-1.0)
)
