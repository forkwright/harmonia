// Battery monitoring for power-aware playback
package app.akroasis.audio

import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.BatteryManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
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
class BatteryMonitor @Inject constructor(
    private val context: Context
) {
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())

    private val _batteryLevel = MutableStateFlow(100)
    val batteryLevel: StateFlow<Int> = _batteryLevel.asStateFlow()

    private val _isCharging = MutableStateFlow(false)
    val isCharging: StateFlow<Boolean> = _isCharging.asStateFlow()

    private val _isLowBattery = MutableStateFlow(false)
    val isLowBattery: StateFlow<Boolean> = _isLowBattery.asStateFlow()

    var lowBatteryThreshold: Int = 20

    init {
        startMonitoring()
    }

    private fun startMonitoring() {
        scope.launch {
            while (isActive) {
                updateBatteryStatus()
                delay(30000) // Check every 30 seconds
            }
        }
    }

    private fun updateBatteryStatus() {
        val batteryStatus: Intent? = IntentFilter(Intent.ACTION_BATTERY_CHANGED).let { filter ->
            context.registerReceiver(null, filter)
        }

        batteryStatus?.let { intent ->
            val level = intent.getIntExtra(BatteryManager.EXTRA_LEVEL, -1)
            val scale = intent.getIntExtra(BatteryManager.EXTRA_SCALE, -1)
            val batteryPct = level * 100 / scale

            _batteryLevel.value = batteryPct
            _isLowBattery.value = batteryPct <= lowBatteryThreshold

            val status = intent.getIntExtra(BatteryManager.EXTRA_STATUS, -1)
            _isCharging.value = status == BatteryManager.BATTERY_STATUS_CHARGING ||
                    status == BatteryManager.BATTERY_STATUS_FULL
        }
    }

    fun estimateBatteryImpact(
        equalizerEnabled: Boolean,
        playbackSpeed: Float,
        usbDacConnected: Boolean
    ): String {
        var drainRate = 5.0 // Base playback drain %/hour

        if (equalizerEnabled) drainRate += 2.0
        if (playbackSpeed != 1.0f) drainRate += 1.5
        if (usbDacConnected) drainRate += 3.0

        val currentLevel = _batteryLevel.value
        val hoursRemaining = if (drainRate > 0) currentLevel / drainRate else 0.0

        return "~${hoursRemaining.toInt()}h at current quality"
    }

    fun release() {
        scope.cancel()
    }

    companion object {
        const val LOW_BATTERY_THRESHOLD_DEFAULT = 20
        const val CRITICAL_BATTERY_THRESHOLD = 10
    }
}
