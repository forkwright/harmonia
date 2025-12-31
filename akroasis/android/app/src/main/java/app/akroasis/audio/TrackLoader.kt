// Network track loading and FLAC decoding
package app.akroasis.audio

import app.akroasis.data.model.Track
import app.akroasis.data.repository.MusicRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class TrackLoader @Inject constructor(
    private val musicRepository: MusicRepository
) {

    suspend fun loadAndDecodeTrack(trackId: String): Result<DecodedAudio> =
        withContext(Dispatchers.IO) {
            try {
                val trackResult = musicRepository.getTrack(trackId)
                val track = trackResult.getOrNull()
                if (track == null) {
                    return@withContext Result.failure(
                        trackResult.exceptionOrNull() ?: Exception("Failed to fetch track")
                    )
                }

                if (track.format.uppercase() != "FLAC") {
                    return@withContext Result.failure(
                        Exception("Unsupported format: ${track.format}. Only FLAC is currently supported.")
                    )
                }

                val streamResult = musicRepository.streamTrack(trackId)
                val responseBody = streamResult.getOrNull()
                if (responseBody == null) {
                    return@withContext Result.failure(
                        streamResult.exceptionOrNull() ?: Exception("Failed to stream track")
                    )
                }

                val audioData = responseBody.use { it.bytes() }

                val decoder = FlacDecoder()
                val decodedAudio = try {
                    decoder.decode(audioData)
                } catch (e: Exception) {
                    return@withContext Result.failure(
                        Exception("Failed to decode FLAC: ${e.message}", e)
                    )
                } finally {
                    decoder.close()
                }

                if (decodedAudio == null) {
                    return@withContext Result.failure(Exception("Decoder returned null"))
                }

                Result.success(decodedAudio)
            } catch (e: Exception) {
                Result.failure(Exception("Failed to load track: ${e.message}", e))
            }
        }
}
