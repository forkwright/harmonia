// Mock audio analysis repository for Phase 2 UI testing
package app.akroasis.data.repository

import app.akroasis.data.model.AudioAnalysis
import app.akroasis.data.model.ReplayGainInfo
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class MockAudioAnalysisRepository @Inject constructor() {

    private val mockData = mapOf(
        "track_1" to AudioAnalysis(
            trackId = "track_1",
            format = "FLAC",
            sampleRate = 96000,
            bitDepth = 24,
            channels = 2,
            dynamicRange = 14,
            replayGain = ReplayGainInfo(
                trackGain = -6.2f,
                trackPeak = 0.95f,
                albumGain = -5.8f,
                albumPeak = 0.98f
            ),
            lossless = true,
            transcoded = false
        ),
        "track_2" to AudioAnalysis(
            trackId = "track_2",
            format = "MP3",
            sampleRate = 44100,
            bitDepth = null,
            channels = 2,
            dynamicRange = 8,
            replayGain = ReplayGainInfo(
                trackGain = -3.5f,
                trackPeak = 0.88f,
                albumGain = null,
                albumPeak = null
            ),
            lossless = false,
            transcoded = false
        ),
        "track_3" to AudioAnalysis(
            trackId = "track_3",
            format = "FLAC",
            sampleRate = 192000,
            bitDepth = 24,
            channels = 2,
            dynamicRange = 6,
            replayGain = null,
            lossless = true,
            transcoded = true  // Fake hi-res detected
        )
    )

    suspend fun getAudioAnalysis(trackId: String): Result<AudioAnalysis> {
        // Simulate network delay
        kotlinx.coroutines.delay(200)

        return mockData[trackId]?.let { Result.success(it) }
            ?: Result.failure(Exception("Audio analysis not available for track $trackId"))
    }

    fun getAudioAnalysisSync(trackId: String): AudioAnalysis? {
        return mockData[trackId]
    }
}
