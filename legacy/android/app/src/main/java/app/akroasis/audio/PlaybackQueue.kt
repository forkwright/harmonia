// Playback queue management with shuffle and repeat
package app.akroasis.audio

import app.akroasis.data.model.MediaItem
import app.akroasis.data.model.Track
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

class PlaybackQueue {
    private val queueMutex = Mutex()
    private val _tracks = MutableStateFlow<List<Track>>(emptyList())
    val tracks: StateFlow<List<Track>> = _tracks.asStateFlow()

    private val _currentIndex = MutableStateFlow(-1)
    val currentIndex: StateFlow<Int> = _currentIndex.asStateFlow()

    private val _shuffleEnabled = MutableStateFlow(false)
    val shuffleEnabled: StateFlow<Boolean> = _shuffleEnabled.asStateFlow()

    private val _repeatMode = MutableStateFlow(RepeatMode.OFF)
    val repeatMode: StateFlow<RepeatMode> = _repeatMode.asStateFlow()

    private var originalOrder: List<Track> = emptyList()
    private var shuffledIndices: List<Int> = emptyList()

    private val historyDeque = ArrayDeque<QueueSnapshot>(50)
    private var historyIndex = -1

    init {
        saveSnapshot()
    }

    val canUndo: Boolean
        get() = historyIndex > 0

    val canRedo: Boolean
        get() = historyIndex < historyDeque.size - 1

    val currentTrack: Track?
        get() = _tracks.value.getOrNull(_currentIndex.value)

    val hasNext: Boolean
        get() = when (_repeatMode.value) {
            RepeatMode.ALL -> _tracks.value.isNotEmpty()
            RepeatMode.ONE -> true
            RepeatMode.OFF -> _currentIndex.value < _tracks.value.size - 1
        }

    val hasPrevious: Boolean
        get() = when (_repeatMode.value) {
            RepeatMode.ALL -> _tracks.value.isNotEmpty()
            RepeatMode.ONE -> true
            RepeatMode.OFF -> _currentIndex.value > 0
        }

    suspend fun setQueue(tracks: List<Track>, startIndex: Int = 0) = queueMutex.withLock {
        originalOrder = tracks
        _tracks.value = tracks
        _currentIndex.value = if (tracks.isEmpty()) -1 else startIndex.coerceIn(0, tracks.size - 1)

        if (_shuffleEnabled.value) {
            reshuffleUnsafe()
        }

        saveSnapshot()
    }

    suspend fun addToQueue(track: Track) = queueMutex.withLock {
        val currentTracks = _tracks.value.toMutableList()
        currentTracks.add(track)
        _tracks.value = currentTracks

        if (originalOrder.isNotEmpty()) {
            originalOrder = originalOrder + track
        }

        saveSnapshot()
    }

    suspend fun addNextInQueue(track: Track) = queueMutex.withLock {
        val currentTracks = _tracks.value.toMutableList()
        val insertIndex = (_currentIndex.value + 1).coerceIn(0, currentTracks.size)
        currentTracks.add(insertIndex, track)
        _tracks.value = currentTracks

        if (originalOrder.isNotEmpty()) {
            originalOrder = originalOrder + track
        }

        saveSnapshot()
    }

    suspend fun removeFromQueue(index: Int) = queueMutex.withLock {
        if (index !in _tracks.value.indices) return@withLock

        val currentTracks = _tracks.value.toMutableList()
        currentTracks.removeAt(index)
        _tracks.value = currentTracks

        if (index < _currentIndex.value) {
            _currentIndex.value = _currentIndex.value - 1
        } else if (index == _currentIndex.value && _currentIndex.value >= currentTracks.size) {
            _currentIndex.value = currentTracks.size - 1
        }

        saveSnapshot()
    }

    suspend fun moveTrack(fromIndex: Int, toIndex: Int) = queueMutex.withLock {
        if (fromIndex !in _tracks.value.indices || toIndex !in _tracks.value.indices) return@withLock

        val currentTracks = _tracks.value.toMutableList()
        val track = currentTracks.removeAt(fromIndex)
        currentTracks.add(toIndex, track)
        _tracks.value = currentTracks

        when {
            fromIndex == _currentIndex.value -> _currentIndex.value = toIndex
            fromIndex < _currentIndex.value && toIndex >= _currentIndex.value ->
                _currentIndex.value = _currentIndex.value - 1
            fromIndex > _currentIndex.value && toIndex <= _currentIndex.value ->
                _currentIndex.value = _currentIndex.value + 1
        }

        saveSnapshot()
    }

    suspend fun skipToNext(): Track? = queueMutex.withLock {
        if (!hasNext) return@withLock null

        _currentIndex.value = when (_repeatMode.value) {
            RepeatMode.ONE -> _currentIndex.value
            RepeatMode.ALL -> (_currentIndex.value + 1) % _tracks.value.size
            RepeatMode.OFF -> (_currentIndex.value + 1).coerceAtMost(_tracks.value.size - 1)
        }

        return@withLock currentTrack
    }

    suspend fun skipToPrevious(): Track? = queueMutex.withLock {
        if (!hasPrevious) return@withLock null

        _currentIndex.value = when (_repeatMode.value) {
            RepeatMode.ONE -> _currentIndex.value
            RepeatMode.ALL -> {
                if (_currentIndex.value == 0) _tracks.value.size - 1
                else _currentIndex.value - 1
            }
            RepeatMode.OFF -> (_currentIndex.value - 1).coerceAtLeast(0)
        }

        return@withLock currentTrack
    }

    suspend fun skipToIndex(index: Int): Track? = queueMutex.withLock {
        if (index !in _tracks.value.indices) return@withLock null
        _currentIndex.value = index
        return@withLock currentTrack
    }

    suspend fun toggleShuffle() = queueMutex.withLock {
        _shuffleEnabled.value = !_shuffleEnabled.value

        if (_shuffleEnabled.value) {
            reshuffleUnsafe()
        } else {
            val currentTrack = this.currentTrack
            _tracks.value = originalOrder
            _currentIndex.value = originalOrder.indexOf(currentTrack).coerceAtLeast(0)
        }

        saveSnapshot()
    }

    fun cycleRepeatMode() {
        _repeatMode.value = when (_repeatMode.value) {
            RepeatMode.OFF -> RepeatMode.ALL
            RepeatMode.ALL -> RepeatMode.ONE
            RepeatMode.ONE -> RepeatMode.OFF
        }
    }

    suspend fun clear() = queueMutex.withLock {
        _tracks.value = emptyList()
        _currentIndex.value = -1
        originalOrder = emptyList()
        shuffledIndices = emptyList()
    }

    private fun reshuffleUnsafe() {
        if (_tracks.value.isEmpty()) return

        val currentTrack = this.currentTrack
        val indices = _tracks.value.indices.toMutableList()

        currentTrack?.let { track ->
            val currentTrackIndex = _tracks.value.indexOf(track)
            if (currentTrackIndex >= 0) {
                indices.remove(currentTrackIndex)
            }
        }

        shuffledIndices = indices.shuffled()

        val shuffled = mutableListOf<Track>()
        currentTrack?.let { shuffled.add(it) }
        shuffledIndices.forEach { index ->
            shuffled.add(_tracks.value[index])
        }

        _tracks.value = shuffled
        _currentIndex.value = 0
    }

    private fun saveSnapshot() {
        val snapshot = QueueSnapshot(
            tracks = _tracks.value.toList(),
            currentIndex = _currentIndex.value,
            timestamp = System.currentTimeMillis()
        )

        if (historyIndex < historyDeque.size - 1) {
            while (historyDeque.size > historyIndex + 1) {
                historyDeque.removeLast()
            }
        }

        historyDeque.addLast(snapshot)
        if (historyDeque.size > 50) {
            historyDeque.removeFirst()
        } else {
            historyIndex++
        }
    }

    suspend fun undo(): Boolean = queueMutex.withLock {
        if (!canUndo) return@withLock false

        historyIndex--
        val snapshot = historyDeque[historyIndex]

        _tracks.value = snapshot.tracks.toList()
        _currentIndex.value = snapshot.currentIndex
        originalOrder = snapshot.tracks.toList()

        return@withLock true
    }

    suspend fun redo(): Boolean = queueMutex.withLock {
        if (!canRedo) return@withLock false

        historyIndex++
        val snapshot = historyDeque[historyIndex]

        _tracks.value = snapshot.tracks.toList()
        _currentIndex.value = snapshot.currentIndex
        originalOrder = snapshot.tracks.toList()

        return@withLock true
    }

    suspend fun setAudiobookChapters(
        audiobook: MediaItem.Audiobook,
        startChapter: Int = 0
    ) = queueMutex.withLock {
        val chapterTracks = audiobook.chapters.map { chapter ->
            Track(
                id = "${audiobook.id}_ch${chapter.index}",
                title = chapter.title,
                artist = audiobook.narrator ?: audiobook.author,
                album = audiobook.title,
                albumArtist = audiobook.author,
                trackNumber = chapter.index + 1,
                discNumber = 1,
                year = null,
                duration = chapter.endTimeMs - chapter.startTimeMs,
                bitrate = null,
                sampleRate = null,
                bitDepth = null,
                format = audiobook.format,
                fileSize = audiobook.fileSize / audiobook.totalChapters,
                filePath = audiobook.filePath,
                coverArtUrl = audiobook.coverArtUrl,
                replayGainTrackGain = null,
                replayGainAlbumGain = null,
                createdAt = audiobook.createdAt,
                updatedAt = audiobook.updatedAt
            )
        }

        setQueue(chapterTracks, startChapter)
    }

    fun getCurrentChapter(): Int? {
        val trackId = currentTrack?.id ?: return null
        return if (trackId.contains("_ch")) {
            trackId.substringAfterLast("_ch").toIntOrNull()
        } else null
    }

    suspend fun skipToChapter(chapterIndex: Int): Track? {
        return skipToIndex(chapterIndex)
    }
}

data class QueueSnapshot(
    val tracks: List<Track>,
    val currentIndex: Int,
    val timestamp: Long
)

enum class RepeatMode {
    OFF,
    ALL,
    ONE
}
