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

private const val MAX_FILE_SIZE_MB = 500L
private const val MAX_FILE_SIZE_BYTES = MAX_FILE_SIZE_MB * 1024 * 1024

@Singleton
class TrackLoader @Inject constructor(
    private val musicRepository: MusicRepository
) {

    suspend fun loadAndDecodeTrack(trackId: String): Result<DecodedAudio> =
        withContext(Dispatchers.IO) {
            try {
                Timber.d("Loading track: $trackId")
                fetchAndDecode(trackId)
            } catch (e: Exception) {
                Timber.e(e, "Failed to load track: $trackId")
                Result.failure(LoadError.NetworkError("Failed to load track: ${e.message}", e))
            }
        }

    private suspend fun fetchAndDecode(trackId: String): Result<DecodedAudio> {
        val track = fetchTrackMetadata(trackId).getOrElse { return Result.failure(it) }
        validateFormat(track.format).getOrElse { return Result.failure(it) }
        val audioData = streamAudioData(trackId).getOrElse { return Result.failure(it) }
        return decodeFlac(trackId, audioData)
    }

    private suspend fun fetchTrackMetadata(trackId: String): Result<app.akroasis.data.model.Track> {
        val trackResult = musicRepository.getTrack(trackId)
        val track = trackResult.getOrNull()
        if (track == null) {
            Timber.w("Failed to fetch track: $trackId")
            return Result.failure(LoadError.NetworkError("Failed to fetch track", trackResult.exceptionOrNull()))
        }
        return Result.success(track)
    }

    private fun validateFormat(format: String): Result<Unit> {
        if (format.uppercase() != "FLAC") {
            Timber.w("Unsupported format: $format")
            return Result.failure(LoadError.UnsupportedFormat(format))
        }
        return Result.success(Unit)
    }

    private suspend fun streamAudioData(trackId: String): Result<ByteArray> {
        val streamResult = musicRepository.streamTrack(trackId)
        val responseBody = streamResult.getOrNull()
            ?: return Result.failure(LoadError.NetworkError("Failed to stream track", streamResult.exceptionOrNull()))

        val contentLength = responseBody.contentLength()
        if (contentLength > MAX_FILE_SIZE_BYTES) {
            return Result.failure(LoadError.FileSizeError(contentLength / 1024 / 1024))
        }

        return try {
            val data = responseBody.use { it.bytes() }
            Timber.d("Loaded ${data.size / 1024}KB, decoding FLAC")
            Result.success(data)
        } catch (e: OutOfMemoryError) {
            Timber.e(e, "Out of memory loading ${contentLength / 1024 / 1024}MB file")
            Result.failure(LoadError.FileSizeError(contentLength / 1024 / 1024))
        }
    }

    private fun decodeFlac(trackId: String, audioData: ByteArray): Result<DecodedAudio> {
        val decoder = FlacDecoder()
        return try {
            val decodedAudio = decoder.decode(audioData)
            if (decodedAudio == null) {
                Timber.w("FLAC decoder returned null")
                return Result.failure(LoadError.DecodeError("Decoder returned null", null))
            }
            Timber.i("Track loaded: $trackId (${decodedAudio.sampleRate}Hz, ${decodedAudio.bitDepth}-bit, ${decodedAudio.channels}ch)")
            Result.success(decodedAudio)
        } catch (e: Exception) {
            Timber.e(e, "Failed to decode FLAC")
            Result.failure(LoadError.DecodeError("Failed to decode FLAC: ${e.message}", e))
        } finally {
            decoder.close()
        }
    }
}
