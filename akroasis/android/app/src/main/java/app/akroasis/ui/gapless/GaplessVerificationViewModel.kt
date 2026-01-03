// Gapless verification and album scanning
package app.akroasis.ui.gapless

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.akroasis.audio.GaplessPlaybackEngine
import app.akroasis.data.model.Album
import app.akroasis.data.model.Track
import app.akroasis.data.repository.MusicRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

@HiltViewModel
class GaplessVerificationViewModel @Inject constructor(
    private val musicRepository: MusicRepository,
    private val gaplessEngine: GaplessPlaybackEngine
) : ViewModel() {

    sealed class ScanState {
        data object Idle : ScanState()
        data class Scanning(val currentTrack: Int, val totalTracks: Int) : ScanState()
        data class Complete(val report: GaplessReport) : ScanState()
        data class Error(val message: String) : ScanState()
    }

    private val _scanState = MutableStateFlow<ScanState>(ScanState.Idle)
    val scanState: StateFlow<ScanState> = _scanState.asStateFlow()

    val gapMeasurements = gaplessEngine.gapMeasurements

    fun scanAlbum(album: Album) {
        viewModelScope.launch {
            _scanState.value = ScanState.Idle
            gaplessEngine.clearGapMeasurements()

            val tracks = musicRepository.getAlbumTracks(album.id).sortedBy { it.trackNumber }
            if (tracks.size < 2) {
                _scanState.value = ScanState.Error("Album must have at least 2 tracks")
                return@launch
            }

            val trackPairs = mutableListOf<TrackPairResult>()

            for (i in 0 until tracks.size - 1) {
                _scanState.value = ScanState.Scanning(i + 1, tracks.size - 1)

                val currentMeasurements = gaplessEngine.gapMeasurements.value
                val measurementsBefore = currentMeasurements.size

                // Simulate track transition (in real app, this would trigger actual playback)
                // For now, we'll use the existing measurements from gaplessEngine

                // Wait for new measurement
                kotlinx.coroutines.delay(100)
                val measurementsAfter = gaplessEngine.gapMeasurements.value.size

                if (measurementsAfter > measurementsBefore) {
                    val latestMeasurement = gaplessEngine.gapMeasurements.value.last()
                    trackPairs.add(
                        TrackPairResult(
                            fromTrack = tracks[i].title,
                            toTrack = tracks[i + 1].title,
                            gapMs = latestMeasurement.gapMs
                        )
                    )
                }
            }

            if (trackPairs.isEmpty()) {
                _scanState.value = ScanState.Error("No gap measurements recorded. Enable gapless and play the album first.")
                return@launch
            }

            val report = GaplessReport(
                albumTitle = album.title,
                trackPairs = trackPairs,
                averageGap = trackPairs.map { it.gapMs }.average().toFloat(),
                maxGap = trackPairs.maxOf { it.gapMs },
                passesThreshold = trackPairs.all { it.gapMs < 50f }
            )

            _scanState.value = ScanState.Complete(report)
        }
    }

    fun reset() {
        _scanState.value = ScanState.Idle
        gaplessEngine.clearGapMeasurements()
    }
}

data class GaplessReport(
    val albumTitle: String,
    val trackPairs: List<TrackPairResult>,
    val averageGap: Float,
    val maxGap: Float,
    val passesThreshold: Boolean
)

data class TrackPairResult(
    val fromTrack: String,
    val toTrack: String,
    val gapMs: Float
)
