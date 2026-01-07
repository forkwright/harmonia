// Ebook reader state and progress management with Readium
package app.akroasis.ui.ebook

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.akroasis.data.model.MediaItem
import app.akroasis.data.model.MediaProgress
import app.akroasis.data.model.MediaType
import app.akroasis.data.readium.ReadiumManager
import app.akroasis.data.repository.EbookRepository
import app.akroasis.data.repository.MediaRepository
import app.akroasis.data.repository.Session
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import org.readium.r2.navigator.Navigator
import org.readium.r2.shared.publication.Locator
import org.readium.r2.shared.publication.Publication
import org.readium.r2.shared.publication.services.positions
import org.readium.r2.shared.util.AbsoluteUrl
import org.readium.r2.shared.util.getOrElse
import timber.log.Timber
import java.io.File
import javax.inject.Inject

@HiltViewModel
class EbookViewModel @Inject constructor(
    private val ebookRepository: EbookRepository,
    private val mediaRepository: MediaRepository,
    private val readiumManager: ReadiumManager
) : ViewModel() {

    private val _currentEbook = MutableStateFlow<MediaItem.Ebook?>(null)
    val currentEbook: StateFlow<MediaItem.Ebook?> = _currentEbook.asStateFlow()

    private val _publication = MutableStateFlow<Publication?>(null)
    val publication: StateFlow<Publication?> = _publication.asStateFlow()

    private val _totalPositions = MutableStateFlow(0)
    val totalPositions: StateFlow<Int> = _totalPositions.asStateFlow()

    private val _savedLocator = MutableStateFlow<Locator?>(null)
    val savedLocator: StateFlow<Locator?> = _savedLocator.asStateFlow()

    private val _progress = MutableStateFlow<MediaProgress?>(null)
    val progress: StateFlow<MediaProgress?> = _progress.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _currentSession = MutableStateFlow<Session?>(null)
    val currentSession: StateFlow<Session?> = _currentSession.asStateFlow()

    private var navigator: Navigator? = null
    private var progressSaveJob: Job? = null

    fun loadEbook(ebookId: String, userId: String = "default") {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null

            try {
                // Load ebook metadata
                val ebookResult = ebookRepository.getEbook(ebookId)
                if (ebookResult.isSuccess) {
                    _currentEbook.value = ebookResult.getOrNull()
                    Timber.d("Loaded ebook: ${_currentEbook.value?.title}")

                    // Load reading progress
                    val progressResult = mediaRepository.getProgress(ebookId, userId)
                    if (progressResult.isSuccess) {
                        _progress.value = progressResult.getOrNull()
                        Timber.d("Loaded progress: ${_progress.value?.percentComplete}% complete")
                    }

                    // Start reading session
                    startSession(ebookId, userId)

                    // Start auto-save
                    startProgressAutoSave(ebookId)
                } else {
                    _error.value = "Failed to load ebook: ${ebookResult.exceptionOrNull()?.message}"
                    Timber.e("Failed to load ebook: ${ebookResult.exceptionOrNull()}")
                }
            } catch (e: Exception) {
                _error.value = "Error loading ebook: ${e.message}"
                Timber.e(e, "Error loading ebook")
            } finally {
                _isLoading.value = false
            }
        }
    }

    fun updateProgress(
        positionMs: Long,
        totalDurationMs: Long?,
        userId: String = "default"
    ) {
        viewModelScope.launch {
            val ebook = _currentEbook.value ?: return@launch

            try {
                val percentComplete = if (totalDurationMs != null && totalDurationMs > 0) {
                    (positionMs.toFloat() / totalDurationMs.toFloat()).coerceIn(0f, 1f)
                } else {
                    0f
                }

                // Update local state
                _progress.value = MediaProgress(
                    mediaItemId = ebook.id,
                    mediaType = MediaType.EBOOK,
                    positionMs = positionMs,
                    totalDurationMs = totalDurationMs,
                    percentComplete = percentComplete,
                    lastPlayedAt = System.currentTimeMillis(),
                    isComplete = percentComplete >= 0.95f
                )

                // Save to backend
                mediaRepository.updateProgress(
                    mediaId = ebook.id,
                    mediaType = MediaType.EBOOK,
                    positionMs = positionMs,
                    durationMs = totalDurationMs,
                    userId = userId
                )

                Timber.d("Updated reading progress: $percentComplete% (${positionMs}ms / ${totalDurationMs}ms)")
            } catch (e: Exception) {
                Timber.e(e, "Failed to update progress")
            }
        }
    }

    fun updateProgressFromPage(
        currentPage: Int,
        totalPages: Int,
        userId: String = "default"
    ) {
        // Convert page number to position estimate
        // Assume average reading time of 2 minutes per page
        val avgReadTimePerPageMs = 120_000L
        val positionMs = currentPage * avgReadTimePerPageMs
        val totalDurationMs = totalPages * avgReadTimePerPageMs

        updateProgress(positionMs, totalDurationMs, userId)
    }

    private fun startSession(ebookId: String, userId: String = "default") {
        viewModelScope.launch {
            try {
                val sessionResult = mediaRepository.startSession(
                    mediaId = ebookId,
                    mediaType = MediaType.EBOOK,
                    userId = userId
                )
                if (sessionResult.isSuccess) {
                    _currentSession.value = sessionResult.getOrNull()
                    Timber.d("Started reading session: ${_currentSession.value?.id}")
                }
            } catch (e: Exception) {
                Timber.e(e, "Failed to start session")
            }
        }
    }

    fun endSession() {
        viewModelScope.launch {
            val session = _currentSession.value ?: return@launch

            try {
                val durationMs = System.currentTimeMillis() - session.startedAt
                mediaRepository.endSession(session.id, durationMs)
                _currentSession.value = null
                Timber.d("Ended reading session: ${session.id}, duration: ${durationMs}ms")
            } catch (e: Exception) {
                Timber.e(e, "Failed to end session")
            }

            stopProgressAutoSave()
        }
    }

    private fun startProgressAutoSave(ebookId: String) {
        progressSaveJob?.cancel()
        progressSaveJob = viewModelScope.launch {
            while (isActive) {
                delay(30000) // Save every 30 seconds (reading is slower than playback)

                val currentProgress = _progress.value
                if (currentProgress != null && currentProgress.positionMs > 0) {
                    try {
                        mediaRepository.updateProgress(
                            mediaId = ebookId,
                            mediaType = MediaType.EBOOK,
                            positionMs = currentProgress.positionMs,
                            durationMs = currentProgress.totalDurationMs
                        )
                        Timber.d("Auto-saved reading progress")
                    } catch (e: Exception) {
                        Timber.e(e, "Failed to auto-save progress")
                    }
                }
            }
        }
    }

    private fun stopProgressAutoSave() {
        progressSaveJob?.cancel()
        progressSaveJob = null
    }

    fun clearError() {
        _error.value = null
    }

    suspend fun openEpubFile(filePath: String): Result<Publication> = try {
        val file = File(filePath)
        val url = AbsoluteUrl(file.toURI().toString())!!

        val asset = readiumManager.assetRetriever.retrieve(url)
            .getOrElse { error ->
                return Result.failure(Exception("Failed to retrieve EPUB: $error"))
            }

        val publication = readiumManager.publicationOpener.open(
            asset = asset,
            allowUserInteraction = true
        ).getOrElse { error ->
            return Result.failure(Exception("Failed to open EPUB: $error"))
        }

        _publication.value = publication

        // Calculate and cache total positions (positions() is a suspend function)
        val positions = publication.positions()
        _totalPositions.value = positions.size

        Timber.d("Opened EPUB: ${publication.metadata.title}, positions: ${positions.size}")
        Result.success(publication)
    } catch (e: Exception) {
        Timber.e(e, "Error opening EPUB file")
        Result.failure(e)
    }

    fun setNavigator(nav: Navigator) {
        navigator = nav

        viewModelScope.launch {
            nav.currentLocator.collect { locator ->
                locator?.let { updateProgressFromLocator(it) }
            }
        }
    }

    private fun updateProgressFromLocator(locator: Locator) {
        if (_currentEbook.value == null || _publication.value == null) return

        val currentPosition = locator.locations.position ?: 0
        val totalPositions = _totalPositions.value

        val positionMs = currentPosition * 120_000L
        val totalDurationMs = totalPositions * 120_000L

        updateProgress(positionMs, totalDurationMs)

        _savedLocator.value = locator
    }

    fun getSavedLocatorJson(): String? {
        return _savedLocator.value?.toJSON()?.toString()
    }

    fun restoreLocatorFromJson(json: String?): Locator? {
        if (json == null) return null
        return try {
            Locator.fromJSON(org.json.JSONObject(json))
        } catch (e: Exception) {
            Timber.e(e, "Failed to restore locator from JSON")
            null
        }
    }

    override fun onCleared() {
        super.onCleared()
        endSession()
        stopProgressAutoSave()
        navigator = null
        _publication.value = null
    }
}
