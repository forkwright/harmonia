// Sleep timer with auto-stop and fade out
package app.akroasis.audio

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class SleepTimer @Inject constructor() {

    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private var timerJob: Job? = null

    private val _remainingTimeMs = MutableStateFlow(0L)
    val remainingTimeMs: StateFlow<Long> = _remainingTimeMs.asStateFlow()

    private val _isActive = MutableStateFlow(false)
    val isActive: StateFlow<Boolean> = _isActive.asStateFlow()

    var onTimerExpired: (() -> Unit)? = null

    fun start(durationMs: Long) {
        cancel()

        _remainingTimeMs.value = durationMs
        _isActive.value = true

        timerJob = scope.launch {
            while (isActive && _remainingTimeMs.value > 0) {
                delay(1000)
                _remainingTimeMs.value = (_remainingTimeMs.value - 1000).coerceAtLeast(0)
            }

            if (_remainingTimeMs.value == 0L) {
                onTimerExpired?.invoke()
            }

            _isActive.value = false
        }
    }

    fun cancel() {
        timerJob?.cancel()
        timerJob = null
        _remainingTimeMs.value = 0
        _isActive.value = false
    }

    fun release() {
        cancel()
        scope.cancel()
    }

    companion object {
        const val FIFTEEN_MINUTES = 15 * 60 * 1000L
        const val THIRTY_MINUTES = 30 * 60 * 1000L
        const val FORTYFIVE_MINUTES = 45 * 60 * 1000L
        const val SIXTY_MINUTES = 60 * 60 * 1000L
    }
}
