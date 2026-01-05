// Network track loading and FLAC decoding
package app.akroasis.audio

import app.akroasis.data.repository.MusicRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

sealed class LoadError(message: String, cause: Throwable? = null) : Exception(message, cause) {
    class NetworkError(message: String, cause: Throwable?) : LoadError(message, cause)
    class UnsupportedFormat(format: String) : LoadError("Unsupported format: $format. Only FLAC is currently supported.")
    class DecodeError(message: String, cause: Throwable?) : LoadError(message, cause)
    class FileSizeError(sizeMB: Long) : LoadError("File too large: ${sizeMB}MB (max 500MB)")
}

@Singleton
class TrackLoader @Inject constructor(
    private val musicRepository: MusicRepository
) {

    suspend fun loadAndDecodeTrack(trackId: String): Result<DecodedAudio> =
        withContext(Dispatchers.IO) {
            try {
                Timber.d("Loading track: $trackId")
                val trackResult = musicRepository.getTrack(trackId)
                val track = trackResult.getOrNull()
                if (track == null) {
                    Timber.w("Failed to fetch track: $trackId")
                    val error = trackResult.exceptionOrNull()
                    return@withContext Result.failure(
                        LoadError.NetworkError("Failed to fetch track", error)
                    )
                }

                if (track.format.uppercase() != "FLAC") {
                    Timber.w("Unsupported format for $trackId: ${track.format}")
                    return@withContext Result.failure(
                        LoadError.UnsupportedFormat(track.format)
                    )
                }

                val streamResult = musicRepository.streamTrack(trackId)
                val responseBody = streamResult.getOrNull()
                if (responseBody == null) {
                    val error = streamResult.exceptionOrNull()
                    return@withContext Result.failure(
                        LoadError.NetworkError("Failed to stream track", error)
                    )
                }

                val contentLength = responseBody.contentLength()
                val maxSizeBytes = 500 * 1024 * 1024L // 500MB

                if (contentLength > maxSizeBytes) {
                    return@withContext Result.failure(
                        LoadError.FileSizeError(contentLength / 1024 / 1024)
                    )
                }

                val audioData = try {
                    responseBody.use { it.bytes() }
                } catch (e: OutOfMemoryError) {
                    Timber.e(e, "Out of memory loading ${contentLength / 1024 / 1024}MB file")
                    return@withContext Result.failure(
                        LoadError.FileSizeError(contentLength / 1024 / 1024)
                    )
                }

                Timber.d("Loaded ${audioData.size / 1024}KB, decoding FLAC")

                val decoder = FlacDecoder()
                val decodedAudio = try {
                    decoder.decode(audioData)
                } catch (e: Exception) {
                    Timber.e(e, "Failed to decode FLAC")
                    return@withContext Result.failure(
                        LoadError.DecodeError("Failed to decode FLAC: ${e.message}", e)
                    )
                } finally {
                    decoder.close()
                }

                if (decodedAudio == null) {
                    Timber.w("FLAC decoder returned null")
                    return@withContext Result.failure(
                        LoadError.DecodeError("Decoder returned null", null)
                    )
                }

                Timber.i("Track loaded: $trackId (${decodedAudio.sampleRate}Hz, ${decodedAudio.bitDepth}-bit, ${decodedAudio.channels}ch)")
                Result.success(decodedAudio)
            } catch (e: Exception) {
                Timber.e(e, "Failed to load track: $trackId")
                Result.failure(LoadError.NetworkError("Failed to load track: ${e.message}", e))
            }
        }
}
