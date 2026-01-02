// Manages offline downloads for tracks, albums, and playlists
package app.akroasis.data.download

import android.content.Context
import app.akroasis.data.model.Track
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.io.File
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class OfflineDownloadManager @Inject constructor(
    @ApplicationContext private val context: Context
) {
    private val _downloadQueue = MutableStateFlow<List<DownloadItem>>(emptyList())
    val downloadQueue: StateFlow<List<DownloadItem>> = _downloadQueue.asStateFlow()

    private val _currentDownload = MutableStateFlow<DownloadItem?>(null)
    val currentDownload: StateFlow<DownloadItem?> = _currentDownload.asStateFlow()

    private val _storageUsed = MutableStateFlow(0L)
    val storageUsed: StateFlow<Long> = _storageUsed.asStateFlow()

    private val _storageLimit = MutableStateFlow(5L * 1024 * 1024 * 1024)
    val storageLimit: StateFlow<Long> = _storageLimit.asStateFlow()

    private val offlineDir: File by lazy {
        File(context.filesDir, "offline").apply {
            if (!exists()) mkdirs()
        }
    }

    data class DownloadItem(
        val track: Track,
        val status: DownloadStatus,
        val progress: Int = 0,
        val bytesDownloaded: Long = 0,
        val totalBytes: Long = 0,
        val error: String? = null
    )

    enum class DownloadStatus {
        QUEUED,
        DOWNLOADING,
        COMPLETED,
        FAILED,
        PAUSED
    }

    init {
        calculateStorageUsed()
    }

    fun queueDownload(track: Track) {
        if (isTrackDownloaded(track.id)) return

        val existingItem = _downloadQueue.value.find { it.track.id == track.id }
        if (existingItem != null) return

        val downloadItem = DownloadItem(
            track = track,
            status = DownloadStatus.QUEUED
        )

        _downloadQueue.value = _downloadQueue.value + downloadItem
    }

    fun queueDownloads(tracks: List<Track>) {
        tracks.forEach { queueDownload(it) }
    }

    fun cancelDownload(trackId: String) {
        _downloadQueue.value = _downloadQueue.value.filter { it.track.id != trackId }

        if (_currentDownload.value?.track?.id == trackId) {
            _currentDownload.value = null
        }
    }

    fun pauseDownload(trackId: String) {
        _downloadQueue.value = _downloadQueue.value.map { item ->
            if (item.track.id == trackId) {
                item.copy(status = DownloadStatus.PAUSED)
            } else {
                item
            }
        }
    }

    fun resumeDownload(trackId: String) {
        _downloadQueue.value = _downloadQueue.value.map { item ->
            if (item.track.id == trackId && item.status == DownloadStatus.PAUSED) {
                item.copy(status = DownloadStatus.QUEUED)
            } else {
                item
            }
        }
    }

    fun deleteOfflineTrack(trackId: String) {
        val file = File(offlineDir, "$trackId.audio")
        if (file.exists()) {
            file.delete()
            calculateStorageUsed()
        }
    }

    fun isTrackDownloaded(trackId: String): Boolean {
        val file = File(offlineDir, "$trackId.audio")
        return file.exists()
    }

    fun getOfflineFilePath(trackId: String): String? {
        val file = File(offlineDir, "$trackId.audio")
        return if (file.exists()) file.absolutePath else null
    }

    fun getAllOfflineTracks(): List<String> {
        return offlineDir.listFiles()
            ?.filter { it.name.endsWith(".audio") }
            ?.map { it.nameWithoutExtension }
            ?: emptyList()
    }

    fun setStorageLimit(limitBytes: Long) {
        _storageLimit.value = limitBytes
    }

    fun getAvailableStorage(): Long {
        return _storageLimit.value - _storageUsed.value
    }

    fun canFitDownload(estimatedBytes: Long): Boolean {
        return getAvailableStorage() >= estimatedBytes
    }

    private fun calculateStorageUsed() {
        val totalSize = offlineDir.listFiles()
            ?.sumOf { it.length() }
            ?: 0L

        _storageUsed.value = totalSize
    }

    fun clearAllOfflineContent() {
        offlineDir.listFiles()?.forEach { it.delete() }
        calculateStorageUsed()
        _downloadQueue.value = emptyList()
        _currentDownload.value = null
    }
}
