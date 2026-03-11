// Streaming quality and network settings
package app.akroasis.ui.settings

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.NetworkCheck
import androidx.compose.material.icons.filled.SignalCellularAlt
import androidx.compose.material.icons.filled.Wifi
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import app.akroasis.network.NetworkMonitor
import app.akroasis.network.StreamingQualityManager
import androidx.hilt.navigation.compose.hiltViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StreamingQualityScreen(
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
    qualityManager: StreamingQualityManager = hiltViewModel(),
    networkMonitor: NetworkMonitor = hiltViewModel()
) {
    val adaptiveEnabled by qualityManager.adaptiveStreamingEnabled.collectAsState()
    val currentQuality by qualityManager.currentQuality.collectAsState()
    val networkState by networkMonitor.observeNetworkState().collectAsState(
        initial = networkMonitor.getCurrentNetworkState()
    )

    var wifiQuality by remember { mutableStateOf(StreamingQualityManager.StreamingQuality.LOSSLESS) }
    var cellularQuality by remember { mutableStateOf(StreamingQualityManager.StreamingQuality.MEDIUM) }
    var meteredQuality by remember { mutableStateOf(StreamingQualityManager.StreamingQuality.LOW) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Streaming Quality") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.ArrowBack, "Navigate back")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(24.dp)
        ) {
            NetworkStatusCard(networkState = networkState, currentQuality = currentQuality)

            AdaptiveStreamingSection(
                enabled = adaptiveEnabled,
                onToggle = { qualityManager.setAdaptiveStreamingEnabled(it) }
            )

            if (adaptiveEnabled) {
                QualityProfileSection(
                    wifiQuality = wifiQuality,
                    cellularQuality = cellularQuality,
                    meteredQuality = meteredQuality,
                    onWifiQualityChange = {
                        wifiQuality = it
                        updateProfile(qualityManager, wifiQuality, cellularQuality, meteredQuality)
                    },
                    onCellularQualityChange = {
                        cellularQuality = it
                        updateProfile(qualityManager, wifiQuality, cellularQuality, meteredQuality)
                    },
                    onMeteredQualityChange = {
                        meteredQuality = it
                        updateProfile(qualityManager, wifiQuality, cellularQuality, meteredQuality)
                    }
                )
            } else {
                ManualQualitySection(
                    selectedQuality = currentQuality,
                    onQualitySelected = { qualityManager.setManualQuality(it) }
                )
            }

            BandwidthEstimateCard(qualityManager = qualityManager)
        }
    }
}

private fun updateProfile(
    manager: StreamingQualityManager,
    wifi: StreamingQualityManager.StreamingQuality,
    cellular: StreamingQualityManager.StreamingQuality,
    metered: StreamingQualityManager.StreamingQuality
) {
    manager.setCustomProfile(
        StreamingQualityManager.QualityProfile(
            wifi = wifi,
            cellular = cellular,
            metered = metered
        )
    )
}

@Composable
fun NetworkStatusCard(
    networkState: NetworkMonitor.NetworkState,
    currentQuality: StreamingQualityManager.StreamingQuality
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.primaryContainer
        )
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column {
                    Text(
                        "Current Network",
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onPrimaryContainer
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        when (networkState.type) {
                            is NetworkMonitor.NetworkType.WiFi -> "WiFi"
                            is NetworkMonitor.NetworkType.Cellular -> "Cellular"
                            is NetworkMonitor.NetworkType.Ethernet -> "Ethernet"
                            is NetworkMonitor.NetworkType.None -> "No connection"
                            else -> "Unknown"
                        },
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onPrimaryContainer
                    )
                }

                Icon(
                    when (networkState.type) {
                        is NetworkMonitor.NetworkType.WiFi -> Icons.Default.Wifi
                        is NetworkMonitor.NetworkType.Cellular -> Icons.Default.SignalCellularAlt
                        else -> Icons.Default.NetworkCheck
                    },
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onPrimaryContainer
                )
            }

            Spacer(modifier = Modifier.height(12.dp))

            Text(
                "Streaming at: ${currentQuality.displayName}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onPrimaryContainer
            )

            if (networkState.isMetered) {
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    "⚠️ Metered connection",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error
                )
            }
        }
    }
}

@Composable
fun AdaptiveStreamingSection(
    enabled: Boolean,
    onToggle: (Boolean) -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text("Adaptive Quality", style = MaterialTheme.typography.titleMedium)
            Text(
                "Automatically adjust quality based on network",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
        Switch(checked = enabled, onCheckedChange = onToggle)
    }
}

@Composable
fun QualityProfileSection(
    wifiQuality: StreamingQualityManager.StreamingQuality,
    cellularQuality: StreamingQualityManager.StreamingQuality,
    meteredQuality: StreamingQualityManager.StreamingQuality,
    onWifiQualityChange: (StreamingQualityManager.StreamingQuality) -> Unit,
    onCellularQualityChange: (StreamingQualityManager.StreamingQuality) -> Unit,
    onMeteredQualityChange: (StreamingQualityManager.StreamingQuality) -> Unit
) {
    Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text("Quality Profiles", style = MaterialTheme.typography.titleMedium)

        QualitySelector(
            label = "WiFi",
            selectedQuality = wifiQuality,
            onQualitySelected = onWifiQualityChange
        )

        QualitySelector(
            label = "Cellular",
            selectedQuality = cellularQuality,
            onQualitySelected = onCellularQualityChange
        )

        QualitySelector(
            label = "Metered Connection",
            selectedQuality = meteredQuality,
            onQualitySelected = onMeteredQualityChange
        )
    }
}

@Composable
fun ManualQualitySection(
    selectedQuality: StreamingQualityManager.StreamingQuality,
    onQualitySelected: (StreamingQualityManager.StreamingQuality) -> Unit
) {
    Column {
        Text("Manual Quality", style = MaterialTheme.typography.titleMedium)
        Spacer(modifier = Modifier.height(8.dp))
        QualitySelector(
            label = "Quality",
            selectedQuality = selectedQuality,
            onQualitySelected = onQualitySelected
        )
    }
}

@Composable
fun QualitySelector(
    label: String,
    selectedQuality: StreamingQualityManager.StreamingQuality,
    onQualitySelected: (StreamingQualityManager.StreamingQuality) -> Unit
) {
    Column {
        Text(label, style = MaterialTheme.typography.bodyMedium)
        Spacer(modifier = Modifier.height(8.dp))

        StreamingQualityManager.StreamingQuality.values().forEach { quality ->
            FilterChip(
                selected = selectedQuality == quality,
                onClick = { onQualitySelected(quality) },
                label = { Text(quality.displayName) },
                modifier = Modifier.padding(end = 8.dp)
            )
        }
    }
}

@Composable
fun BandwidthEstimateCard(qualityManager: StreamingQualityManager) {
    val currentQuality by qualityManager.currentQuality.collectAsState()

    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.secondaryContainer
        )
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                "Estimated Usage",
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.onSecondaryContainer
            )
            Spacer(modifier = Modifier.height(8.dp))

            val oneHourUsage = qualityManager.estimateBandwidthUsage(3600)
            val oneHourMB = oneHourUsage / (1024 * 1024)

            Text(
                "~${oneHourMB} MB per hour at ${currentQuality.displayName}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSecondaryContainer
            )
        }
    }
}
