// Queue export to playlist formats
package app.akroasis.ui.queue

import android.content.Context
import android.net.Uri
import app.akroasis.data.model.Track
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import javax.inject.Inject
import javax.inject.Singleton

enum class ExportFormat(val extension: String, val mimeType: String) {
    M3U("m3u", "audio/x-mpegurl"),
    M3U8("m3u8", "application/vnd.apple.mpegurl"),
    PLS("pls", "audio/x-scpls")
}

@Singleton
class QueueExporter @Inject constructor(
    private val context: Context
) {
    suspend fun exportQueue(
        tracks: List<Track>,
        format: ExportFormat,
        outputUri: Uri
    ): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            val content = when (format) {
                ExportFormat.M3U -> generateM3U(tracks)
                ExportFormat.M3U8 -> generateM3U8(tracks)
                ExportFormat.PLS -> generatePLS(tracks)
            }

            context.contentResolver.openOutputStream(outputUri)?.use { outputStream ->
                outputStream.write(content.toByteArray(Charsets.UTF_8))
            } ?: return@withContext Result.failure(Exception("Could not open output stream"))

            Result.success(Unit)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    private fun generateM3U(tracks: List<Track>): String {
        val builder = StringBuilder()
        builder.appendLine("#EXTM3U")

        tracks.forEach { track ->
            val durationSeconds = track.duration / 1000
            builder.appendLine("#EXTINF:$durationSeconds,${track.artist} - ${track.title}")
            builder.appendLine(track.filePath)
        }

        return builder.toString()
    }

    private fun generateM3U8(tracks: List<Track>): String {
        val builder = StringBuilder()
        builder.appendLine("#EXTM3U")

        tracks.forEach { track ->
            val durationSeconds = track.duration / 1000
            builder.appendLine("#EXTINF:$durationSeconds,${track.artist} - ${track.title}")
            builder.appendLine("#EXTALB:${track.album}")
            track.coverArtUrl?.let { builder.appendLine("#EXTIMG:$it") }
            builder.appendLine(track.filePath)
        }

        return builder.toString()
    }

    private fun generatePLS(tracks: List<Track>): String {
        val builder = StringBuilder()
        builder.appendLine("[playlist]")
        builder.appendLine("NumberOfEntries=${tracks.size}")
        builder.appendLine()

        tracks.forEachIndexed { index, track ->
            val entryNum = index + 1
            val durationSeconds = track.duration / 1000

            builder.appendLine("File$entryNum=${track.filePath}")
            builder.appendLine("Title$entryNum=${track.artist} - ${track.title}")
            builder.appendLine("Length$entryNum=$durationSeconds")
            builder.appendLine()
        }

        builder.appendLine("Version=2")
        return builder.toString()
    }
}
