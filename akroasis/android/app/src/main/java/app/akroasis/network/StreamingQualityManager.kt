// Manages adaptive streaming quality based on network conditions
package app.akroasis.network

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class StreamingQualityManager @Inject constructor(
    private val networkMonitor: NetworkMonitor
) {
    private val _currentQuality = MutableStateFlow(StreamingQuality.LOSSLESS)
    val currentQuality: StateFlow<StreamingQuality> = _currentQuality.asStateFlow()

    private val _adaptiveStreamingEnabled = MutableStateFlow(true)
    val adaptiveStreamingEnabled: StateFlow<Boolean> = _adaptiveStreamingEnabled.asStateFlow()

    enum class StreamingQuality(
        val displayName: String,
        val maxBitrate: Int,
        val format: String
    ) {
        LOSSLESS("Lossless (Original)", Int.MAX_VALUE, "original"),
        HIGH("High (320 kbps)", 320, "opus_320"),
        MEDIUM("Medium (128 kbps)", 128, "opus_128"),
        LOW("Low (64 kbps)", 64, "opus_64")
    }

    data class QualityProfile(
        val wifi: StreamingQuality,
        val cellular: StreamingQuality,
        val metered: StreamingQuality
    )

    private var customProfile = QualityProfile(
        wifi = StreamingQuality.LOSSLESS,
        cellular = StreamingQuality.MEDIUM,
        metered = StreamingQuality.LOW
    )

    fun setAdaptiveStreamingEnabled(enabled: Boolean) {
        _adaptiveStreamingEnabled.value = enabled
        if (enabled) {
            updateQualityBasedOnNetwork()
        }
    }

    fun setCustomProfile(profile: QualityProfile) {
        customProfile = profile
        if (_adaptiveStreamingEnabled.value) {
            updateQualityBasedOnNetwork()
        }
    }

    fun setManualQuality(quality: StreamingQuality) {
        _adaptiveStreamingEnabled.value = false
        _currentQuality.value = quality
    }

    fun updateQualityBasedOnNetwork() {
        if (!_adaptiveStreamingEnabled.value) return

        val networkState = networkMonitor.getCurrentNetworkState()

        val quality = when {
            !networkState.isConnected -> StreamingQuality.LOW
            networkState.type is NetworkMonitor.NetworkType.WiFi -> customProfile.wifi
            networkState.type is NetworkMonitor.NetworkType.Ethernet -> StreamingQuality.LOSSLESS
            networkState.type is NetworkMonitor.NetworkType.Cellular -> {
                if (networkState.isMetered) {
                    customProfile.metered
                } else {
                    customProfile.cellular
                }
            }
            else -> StreamingQuality.MEDIUM
        }

        _currentQuality.value = quality
    }

    fun getQualityForUrl(baseUrl: String): String {
        val quality = _currentQuality.value
        return if (quality == StreamingQuality.LOSSLESS) {
            baseUrl
        } else {
            "$baseUrl?quality=${quality.format}&maxBitrate=${quality.maxBitrate}"
        }
    }

    fun estimateBandwidthUsage(durationSeconds: Int): Long {
        val bitrateKbps = _currentQuality.value.maxBitrate
        return (bitrateKbps * durationSeconds * 1024L) / 8
    }

    fun canStreamAtQuality(quality: StreamingQuality): Boolean {
        val networkState = networkMonitor.getCurrentNetworkState()

        if (!networkState.isConnected) return false

        return when (quality) {
            StreamingQuality.LOSSLESS -> {
                networkState.type is NetworkMonitor.NetworkType.WiFi ||
                networkState.type is NetworkMonitor.NetworkType.Ethernet
            }
            StreamingQuality.HIGH -> {
                networkState.linkDownstreamBandwidthKbps >= 512
            }
            StreamingQuality.MEDIUM -> {
                networkState.linkDownstreamBandwidthKbps >= 256
            }
            StreamingQuality.LOW -> true
        }
    }

    fun getRecommendedQuality(): StreamingQuality {
        val networkState = networkMonitor.getCurrentNetworkState()

        return when {
            !networkState.isConnected -> StreamingQuality.LOW
            networkState.type is NetworkMonitor.NetworkType.WiFi ||
            networkState.type is NetworkMonitor.NetworkType.Ethernet -> StreamingQuality.LOSSLESS
            networkState.linkDownstreamBandwidthKbps >= 512 -> StreamingQuality.HIGH
            networkState.linkDownstreamBandwidthKbps >= 256 -> StreamingQuality.MEDIUM
            else -> StreamingQuality.LOW
        }
    }
}
