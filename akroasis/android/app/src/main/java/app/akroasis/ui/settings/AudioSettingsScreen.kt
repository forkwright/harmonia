// Audio quality and playback settings
package app.akroasis.ui.settings

import kotlin.math.absoluteValue
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import app.akroasis.audio.EqualizerEngine
import app.akroasis.ui.player.PlayerViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AudioSettingsScreen(
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
    viewModel: PlayerViewModel = hiltViewModel()
) {
    val connectedDacs by viewModel.connectedDacs.collectAsState()
    val preferredDac by viewModel.preferredDac.collectAsState()
    val playbackSpeed by viewModel.playbackSpeed.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Audio Quality") },
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
            EqualizerSection(viewModel = viewModel)
            PlaybackSpeedSection(speed = playbackSpeed, onSpeedChange = { viewModel.setPlaybackSpeed(it) })
            CrossfeedSection(viewModel = viewModel)
            HeadroomSection(viewModel = viewModel)
            GaplessPlaybackSection(viewModel = viewModel)
            CrossfadeSection(viewModel = viewModel)
            UsbDacSection(
                connectedDacs = connectedDacs,
                preferredDac = preferredDac,
                onDacSelected = { viewModel.setPreferredDac(it) },
                onRefresh = { viewModel.scanForUsbDacs() }
            )
        }
    }
}

@Composable
fun EqualizerSection(viewModel: PlayerViewModel) {
    var eqEnabled by remember { mutableStateOf(false) }
    var selectedPreset by remember { mutableStateOf("Flat") }

    Column {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text("Equalizer", style = MaterialTheme.typography.titleMedium)
            Switch(
                checked = eqEnabled,
                onCheckedChange = {
                    eqEnabled = it
                    if (it) viewModel.enableEqualizer() else viewModel.disableEqualizer()
                }
            )
        }

        if (eqEnabled) {
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                "Preset",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.height(4.dp))

            val presets = listOf("Flat", "Rock", "Jazz", "Classical", "Pop", "Bass Boost")
            presets.forEach { preset ->
                FilterChip(
                    selected = selectedPreset == preset,
                    onClick = {
                        selectedPreset = preset
                        val equalizerPreset = when (preset) {
                            "Rock" -> EqualizerEngine.PRESET_ROCK
                            "Jazz" -> EqualizerEngine.PRESET_JAZZ
                            "Classical" -> EqualizerEngine.PRESET_CLASSICAL
                            "Pop" -> EqualizerEngine.PRESET_POP
                            "Bass Boost" -> EqualizerEngine.PRESET_BASS_BOOST
                            else -> EqualizerEngine.PRESET_FLAT
                        }
                        viewModel.applyEqualizerPreset(equalizerPreset)
                    },
                    label = { Text(preset) },
                    modifier = Modifier.padding(end = 8.dp)
                )
            }
        }
    }
}

@Composable
fun PlaybackSpeedSection(speed: Float, onSpeedChange: (Float) -> Unit) {
    val presets = listOf(0.5f, 0.75f, 1.0f, 1.25f, 1.5f, 2.0f, 2.5f, 3.0f)

    Column {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text("Playback Speed", style = MaterialTheme.typography.titleMedium)
            Text("${String.format("%.2f", speed)}x", style = MaterialTheme.typography.bodyMedium)
        }

        Spacer(modifier = Modifier.height(8.dp))

        // Quick preset buttons
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            presets.forEach { preset ->
                FilterChip(
                    selected = (speed - preset).absoluteValue < 0.01f,
                    onClick = { onSpeedChange(preset) },
                    label = { Text("${String.format("%.2f", preset)}x") }
                )
            }
        }

        Spacer(modifier = Modifier.height(8.dp))

        Slider(
            value = speed,
            onValueChange = onSpeedChange,
            valueRange = 0.5f..3.0f,
            steps = 24
        )
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text("0.5x", style = MaterialTheme.typography.bodySmall)
            Text("3.0x", style = MaterialTheme.typography.bodySmall)
        }
    }
}

@Composable
fun GaplessPlaybackSection(viewModel: PlayerViewModel) {
    var gaplessEnabled by remember { mutableStateOf(true) }

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text("Gapless Playback", style = MaterialTheme.typography.titleMedium)
            Text(
                "Eliminates silence between tracks",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
        Switch(
            checked = gaplessEnabled,
            onCheckedChange = {
                gaplessEnabled = it
                if (it) viewModel.enableGapless() else viewModel.disableGapless()
            }
        )
    }
}

@Composable
fun CrossfadeSection(viewModel: PlayerViewModel) {
    var crossfadeDuration by remember { mutableStateOf(0) }

    Column {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text("Crossfade", style = MaterialTheme.typography.titleMedium)
            Text("${crossfadeDuration}s", style = MaterialTheme.typography.bodyMedium)
        }
        Slider(
            value = crossfadeDuration.toFloat(),
            onValueChange = {
                crossfadeDuration = it.toInt()
                viewModel.setCrossfadeDuration(it.toInt() * 1000)
            },
            valueRange = 0f..10f,
            steps = 9
        )
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text("Off", style = MaterialTheme.typography.bodySmall)
            Text("10s", style = MaterialTheme.typography.bodySmall)
        }
    }
}

@Composable
fun UsbDacSection(
    connectedDacs: List<app.akroasis.audio.UsbDacInfo>,
    preferredDac: app.akroasis.audio.UsbDacInfo?,
    onDacSelected: (app.akroasis.audio.UsbDacInfo?) -> Unit,
    onRefresh: () -> Unit
) {
    Column {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text("USB DAC", style = MaterialTheme.typography.titleMedium)
            TextButton(onClick = onRefresh) {
                Text("Scan")
            }
        }

        if (connectedDacs.isEmpty()) {
            Text(
                "No USB audio devices detected",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        } else {
            connectedDacs.forEach { dac ->
                FilterChip(
                    selected = preferredDac?.id == dac.id,
                    onClick = { onDacSelected(dac) },
                    label = {
                        Column {
                            Text(dac.productName)
                            Text(
                                dac.formatCapabilities(),
                                style = MaterialTheme.typography.bodySmall
                            )
                        }
                    },
                    modifier = Modifier.fillMaxWidth()
                )
                Spacer(modifier = Modifier.height(8.dp))
            }
        }
    }
}

@Composable
fun CrossfeedSection(viewModel: PlayerViewModel) {
    val crossfeedEngine = viewModel.crossfeedEngine
    val isEnabled by crossfeedEngine.isEnabled.collectAsState()
    val strength by crossfeedEngine.crossfeedStrength.collectAsState()

    Column {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text("Crossfeed", style = MaterialTheme.typography.titleMedium)
                Text(
                    "Reduces stereo fatigue on headphones",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            Switch(
                checked = isEnabled,
                onCheckedChange = {
                    if (it) viewModel.enableCrossfeed() else viewModel.disableCrossfeed()
                }
            )
        }

        if (isEnabled) {
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                "Strength",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.height(4.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                val strengths = listOf(
                    "Low" to app.akroasis.audio.CrossfeedEngine.STRENGTH_LOW,
                    "Medium" to app.akroasis.audio.CrossfeedEngine.STRENGTH_MEDIUM,
                    "High" to app.akroasis.audio.CrossfeedEngine.STRENGTH_HIGH
                )

                strengths.forEach { (label, value) ->
                    FilterChip(
                        selected = (strength - value).absoluteValue < 0.01f,
                        onClick = { viewModel.setCrossfeedStrength(value) },
                        label = { Text(label) }
                    )
                }
            }
        }
    }
}

@Composable
fun HeadroomSection(viewModel: PlayerViewModel) {
    val headroomManager = viewModel.headroomManager
    val isEnabled by headroomManager.isEnabled.collectAsState()
    val headroomDb by headroomManager.headroomDb.collectAsState()
    val clippingDetected by headroomManager.clippingDetected.collectAsState()
    val peakLevel by headroomManager.peakLevel.collectAsState()

    Column {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text("Headroom", style = MaterialTheme.typography.titleMedium)
                Text(
                    "Prevents clipping when using DSP effects",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            Switch(
                checked = isEnabled,
                onCheckedChange = {
                    if (it) viewModel.enableHeadroom() else viewModel.disableHeadroom()
                }
            )
        }

        if (isEnabled) {
            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Headroom", style = MaterialTheme.typography.bodySmall)
                Text(
                    "${String.format("%.1f", headroomDb)} dB",
                    style = MaterialTheme.typography.bodyMedium
                )
            }

            Slider(
                value = headroomDb,
                onValueChange = { viewModel.setHeadroom(it) },
                valueRange = -12f..0f,
                steps = 23
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("-12 dB", style = MaterialTheme.typography.bodySmall)
                Text("0 dB", style = MaterialTheme.typography.bodySmall)
            }

            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("Peak Level", style = MaterialTheme.typography.bodySmall)
                Text(
                    "${(peakLevel * 100).toInt()}%",
                    style = MaterialTheme.typography.bodyMedium,
                    color = if (peakLevel > 0.95f) MaterialTheme.colorScheme.error
                    else MaterialTheme.colorScheme.onSurface
                )
            }

            if (clippingDetected) {
                Spacer(modifier = Modifier.height(8.dp))
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.errorContainer
                    )
                ) {
                    Row(
                        modifier = Modifier.padding(12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(
                            Icons.Default.Warning,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.onErrorContainer
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Column {
                            Text(
                                "Clipping Detected",
                                style = MaterialTheme.typography.titleSmall,
                                color = MaterialTheme.colorScheme.onErrorContainer
                            )
                            Text(
                                "Increase headroom to prevent distortion",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onErrorContainer
                            )
                        }
                    }
                }
            }
        }
    }
}
