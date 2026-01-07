// Parametric equalizer with per-band controls
package app.akroasis.ui.settings

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Headset
import androidx.compose.material.icons.filled.Save
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import app.akroasis.ui.player.PlayerViewModel

private const val PRESET_BASS_BOOST_NAME = "PRESET_BASS_BOOST_NAME"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EqualizerScreen(
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
    viewModel: PlayerViewModel = hiltViewModel()
) {
    val equalizerEnabled by viewModel.equalizerEnabled.collectAsState()
    var showSaveDialog by remember { mutableStateOf(false) }
    var customPresets by remember { mutableStateOf(viewModel.getCustomEqualizerPresets()) }
    var showAutoEQDialog by remember { mutableStateOf(false) }
    var autoEQSearchQuery by remember { mutableStateOf("") }

    val numBands = viewModel.getNumberOfBands().toInt()
    val bandLevelRange = viewModel.getEqualizerBandLevelRange()
    val minLevel = bandLevelRange?.get(0)?.toFloat() ?: -1500f
    val maxLevel = bandLevelRange?.get(1)?.toFloat() ?: 1500f

    val bandLevels = remember { mutableStateMapOf<Int, Short>() }

    LaunchedEffect(Unit) {
        for (i in 0 until numBands) {
            bandLevels[i] = viewModel.getEqualizerBandLevel(i.toShort())
        }
    }

    if (showSaveDialog) {
        SavePresetDialog(
            onDismiss = { showSaveDialog = false },
            onSave = { presetName ->
                val currentBandLevels = mutableListOf<Short>()
                for (i in 0 until numBands) {
                    currentBandLevels.add(bandLevels[i] ?: 0)
                }
                viewModel.saveEqualizerPreset(presetName, currentBandLevels)
                customPresets = viewModel.getCustomEqualizerPresets()
                showSaveDialog = false
            }
        )
    }

    if (showAutoEQDialog) {
        AutoEQDialog(
            searchQuery = autoEQSearchQuery,
            onSearchQueryChange = { autoEQSearchQuery = it },
            profiles = viewModel.searchAutoEQProfiles(autoEQSearchQuery),
            onDismiss = { showAutoEQDialog = false },
            onProfileSelected = { profile ->
                viewModel.applyAutoEQProfile(profile)
                for (i in 0 until numBands) {
                    bandLevels[i] = viewModel.getEqualizerBandLevel(i.toShort())
                }
                showAutoEQDialog = false
            }
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Equalizer") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.ArrowBack, "Navigate back")
                    }
                },
                actions = {
                    if (equalizerEnabled) {
                        IconButton(onClick = { showAutoEQDialog = true }) {
                            Icon(Icons.Default.Headset, "AutoEQ")
                        }
                        IconButton(onClick = { showSaveDialog = true }) {
                            Icon(Icons.Default.Save, "Save preset")
                        }
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
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text("Enable Equalizer", style = MaterialTheme.typography.titleMedium)
                Switch(
                    checked = equalizerEnabled,
                    onCheckedChange = {
                        if (it) viewModel.enableEqualizer() else viewModel.disableEqualizer()
                    }
                )
            }

            if (equalizerEnabled) {
                Text(
                    text = "Adjust frequency bands to customize your sound",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )

                Divider(modifier = Modifier.padding(vertical = 8.dp))

                Text(
                    text = "Presets",
                    style = MaterialTheme.typography.titleSmall
                )

                val presets = listOf("Flat", "Rock", "Jazz", "Classical", "Pop", "PRESET_BASS_BOOST_NAME")

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    presets.take(3).forEach { preset ->
                        FilterChip(
                            selected = false,
                            onClick = {
                                val equalizerPreset = when (preset) {
                                    "Rock" -> app.akroasis.audio.EqualizerEngine.PRESET_ROCK
                                    "Jazz" -> app.akroasis.audio.EqualizerEngine.PRESET_JAZZ
                                    "Classical" -> app.akroasis.audio.EqualizerEngine.PRESET_CLASSICAL
                                    "Pop" -> app.akroasis.audio.EqualizerEngine.PRESET_POP
                                    "PRESET_BASS_BOOST_NAME" -> app.akroasis.audio.EqualizerEngine.PRESET_BASS_BOOST
                                    else -> app.akroasis.audio.EqualizerEngine.PRESET_FLAT
                                }
                                viewModel.applyEqualizerPreset(equalizerPreset)
                                for (i in 0 until numBands) {
                                    bandLevels[i] = viewModel.getEqualizerBandLevel(i.toShort())
                                }
                            },
                            label = { Text(preset) }
                        )
                    }
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    presets.drop(3).forEach { preset ->
                        FilterChip(
                            selected = false,
                            onClick = {
                                val equalizerPreset = when (preset) {
                                    "Rock" -> app.akroasis.audio.EqualizerEngine.PRESET_ROCK
                                    "Jazz" -> app.akroasis.audio.EqualizerEngine.PRESET_JAZZ
                                    "Classical" -> app.akroasis.audio.EqualizerEngine.PRESET_CLASSICAL
                                    "Pop" -> app.akroasis.audio.EqualizerEngine.PRESET_POP
                                    "PRESET_BASS_BOOST_NAME" -> app.akroasis.audio.EqualizerEngine.PRESET_BASS_BOOST
                                    else -> app.akroasis.audio.EqualizerEngine.PRESET_FLAT
                                }
                                viewModel.applyEqualizerPreset(equalizerPreset)
                                for (i in 0 until numBands) {
                                    bandLevels[i] = viewModel.getEqualizerBandLevel(i.toShort())
                                }
                            },
                            label = { Text(preset) }
                        )
                    }
                }

                if (customPresets.isNotEmpty()) {
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = "Custom Presets",
                        style = MaterialTheme.typography.titleSmall
                    )

                    customPresets.forEach { customPreset ->
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            FilterChip(
                                selected = false,
                                onClick = {
                                    viewModel.loadEqualizerPreset(customPreset)
                                    for (i in 0 until numBands) {
                                        bandLevels[i] = viewModel.getEqualizerBandLevel(i.toShort())
                                    }
                                },
                                label = { Text(customPreset.name) },
                                modifier = Modifier.weight(1f)
                            )
                            IconButton(
                                onClick = {
                                    viewModel.deleteEqualizerPreset(customPreset.name)
                                    customPresets = viewModel.getCustomEqualizerPresets()
                                }
                            ) {
                                Icon(
                                    androidx.compose.material.icons.Icons.Default.Delete,
                                    contentDescription = "Delete preset"
                                )
                            }
                        }
                    }
                }

                Divider(modifier = Modifier.padding(vertical = 8.dp))

                Text(
                    text = "Band Controls",
                    style = MaterialTheme.typography.titleSmall
                )

                for (band in 0 until numBands) {
                    val centerFreq = viewModel.getEqualizerCenterFreq(band.toShort()) ?: 0
                    val freqText = when {
                        centerFreq >= 1000000 -> "${centerFreq / 1000000}MHz"
                        centerFreq >= 1000 -> "${centerFreq / 1000}kHz"
                        else -> "${centerFreq}Hz"
                    }

                    EqualizerBandControl(
                        frequency = freqText,
                        level = bandLevels[band] ?: 0,
                        minLevel = minLevel,
                        maxLevel = maxLevel,
                        onLevelChange = { newLevel ->
                            bandLevels[band] = newLevel
                            viewModel.setEqualizerBandLevel(band.toShort(), newLevel)
                        }
                    )
                }

                Card(
                    modifier = Modifier.fillMaxWidth(),
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.secondaryContainer
                    )
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp)
                    ) {
                        Text(
                            text = "Tip",
                            style = MaterialTheme.typography.titleSmall,
                            color = MaterialTheme.colorScheme.onSecondaryContainer
                        )
                        Text(
                            text = "Adjusting EQ will disable bit-perfect playback. For critical listening, use Flat preset.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSecondaryContainer
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun EqualizerBandControl(
    frequency: String,
    level: Short,
    minLevel: Float,
    maxLevel: Float,
    onLevelChange: (Short) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier.fillMaxWidth()
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text(
                text = frequency,
                style = MaterialTheme.typography.bodyMedium
            )
            Text(
                text = "${level / 100f} dB",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.primary
            )
        }

        Slider(
            value = level.toFloat(),
            onValueChange = { onLevelChange(it.toInt().toShort()) },
            valueRange = minLevel..maxLevel,
            modifier = Modifier.fillMaxWidth()
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SavePresetDialog(
    onDismiss: () -> Unit,
    onSave: (String) -> Unit
) {
    var presetName by remember { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Save Preset") },
        text = {
            Column {
                Text("Enter a name for your custom preset:")
                Spacer(modifier = Modifier.height(8.dp))
                OutlinedTextField(
                    value = presetName,
                    onValueChange = { presetName = it },
                    label = { Text("Preset name") },
                    singleLine = true
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = { if (presetName.isNotBlank()) onSave(presetName) },
                enabled = presetName.isNotBlank()
            ) {
                Text("Save")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AutoEQDialog(
    searchQuery: String,
    onSearchQueryChange: (String) -> Unit,
    profiles: List<app.akroasis.data.model.AutoEQProfile>,
    onDismiss: () -> Unit,
    onProfileSelected: (app.akroasis.data.model.AutoEQProfile) -> Unit
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("AutoEQ Headphone Profiles") },
        text = {
            Column {
                Text(
                    "Select your headphone model for automatic frequency response correction:",
                    style = MaterialTheme.typography.bodySmall
                )
                Spacer(modifier = Modifier.height(8.dp))

                OutlinedTextField(
                    value = searchQuery,
                    onValueChange = onSearchQueryChange,
                    label = { Text("Search headphones") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )

                Spacer(modifier = Modifier.height(8.dp))

                if (profiles.isEmpty()) {
                    Text(
                        "No profiles found",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(16.dp)
                    )
                } else {
                    Column(
                        modifier = Modifier
                            .heightIn(max = 300.dp)
                            .verticalScroll(rememberScrollState())
                    ) {
                        profiles.forEach { profile ->
                            ListItem(
                                headlineContent = { Text(profile.fullName) },
                                supportingContent = {
                                    Text(
                                        "${profile.parametricEq.size} filter bands",
                                        style = MaterialTheme.typography.bodySmall
                                    )
                                },
                                modifier = Modifier.clickable {
                                    onProfileSelected(profile)
                                }
                            )
                            Divider()
                        }
                    }
                }
            }
        },
        confirmButton = {},
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}
