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
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import app.akroasis.audio.EqualizerEngine
import app.akroasis.data.model.AutoEQProfile
import app.akroasis.data.model.EqualizerPreset
import app.akroasis.ui.player.PlayerViewModel

private const val PRESET_BASS_BOOST = "Bass Boost"

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
                val currentBandLevels = (0 until numBands).map { bandLevels[it] ?: 0 }
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
                refreshBandLevels(bandLevels, numBands, viewModel)
                showAutoEQDialog = false
            }
        )
    }

    Scaffold(
        topBar = {
            EqualizerTopBar(
                equalizerEnabled = equalizerEnabled,
                onNavigateBack = onNavigateBack,
                onShowAutoEQ = { showAutoEQDialog = true },
                onShowSave = { showSaveDialog = true }
            )
        }
    ) { padding ->
        EqualizerContent(
            equalizerEnabled = equalizerEnabled,
            numBands = numBands,
            minLevel = minLevel,
            maxLevel = maxLevel,
            bandLevels = bandLevels,
            customPresets = customPresets,
            onEnableChange = { if (it) viewModel.enableEqualizer() else viewModel.disableEqualizer() },
            onPresetSelected = { preset ->
                viewModel.applyEqualizerPreset(preset)
                refreshBandLevels(bandLevels, numBands, viewModel)
            },
            onCustomPresetSelected = { preset ->
                viewModel.loadEqualizerPreset(preset)
                refreshBandLevels(bandLevels, numBands, viewModel)
            },
            onCustomPresetDeleted = { name ->
                viewModel.deleteEqualizerPreset(name)
                customPresets = viewModel.getCustomEqualizerPresets()
            },
            onBandLevelChange = { band, level ->
                bandLevels[band] = level
                viewModel.setEqualizerBandLevel(band.toShort(), level)
            },
            getCenterFreq = { viewModel.getEqualizerCenterFreq(it.toShort()) ?: 0 },
            modifier = modifier.padding(padding)
        )
    }
}

private fun refreshBandLevels(
    bandLevels: MutableMap<Int, Short>,
    numBands: Int,
    viewModel: PlayerViewModel
) {
    for (i in 0 until numBands) {
        bandLevels[i] = viewModel.getEqualizerBandLevel(i.toShort())
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun EqualizerTopBar(
    equalizerEnabled: Boolean,
    onNavigateBack: () -> Unit,
    onShowAutoEQ: () -> Unit,
    onShowSave: () -> Unit
) {
    TopAppBar(
        title = { Text("Equalizer") },
        navigationIcon = {
            IconButton(onClick = onNavigateBack) {
                Icon(Icons.Default.ArrowBack, "Navigate back")
            }
        },
        actions = {
            if (equalizerEnabled) {
                IconButton(onClick = onShowAutoEQ) {
                    Icon(Icons.Default.Headset, "AutoEQ")
                }
                IconButton(onClick = onShowSave) {
                    Icon(Icons.Default.Save, "Save preset")
                }
            }
        }
    )
}

@Composable
private fun EqualizerContent(
    equalizerEnabled: Boolean,
    numBands: Int,
    minLevel: Float,
    maxLevel: Float,
    bandLevels: Map<Int, Short>,
    customPresets: List<EqualizerPreset>,
    onEnableChange: (Boolean) -> Unit,
    onPresetSelected: (EqualizerEngine.EqualizerPreset) -> Unit,
    onCustomPresetSelected: (EqualizerPreset) -> Unit,
    onCustomPresetDeleted: (String) -> Unit,
    onBandLevelChange: (Int, Short) -> Unit,
    getCenterFreq: (Int) -> Int,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        EnableEqualizerRow(
            enabled = equalizerEnabled,
            onEnabledChange = onEnableChange
        )

        if (equalizerEnabled) {
            EqualizerControls(
                numBands = numBands,
                minLevel = minLevel,
                maxLevel = maxLevel,
                bandLevels = bandLevels,
                customPresets = customPresets,
                onPresetSelected = onPresetSelected,
                onCustomPresetSelected = onCustomPresetSelected,
                onCustomPresetDeleted = onCustomPresetDeleted,
                onBandLevelChange = onBandLevelChange,
                getCenterFreq = getCenterFreq
            )
        }
    }
}

@Composable
private fun EnableEqualizerRow(
    enabled: Boolean,
    onEnabledChange: (Boolean) -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text("Enable Equalizer", style = MaterialTheme.typography.titleMedium)
        Switch(checked = enabled, onCheckedChange = onEnabledChange)
    }
}

@Composable
private fun EqualizerControls(
    numBands: Int,
    minLevel: Float,
    maxLevel: Float,
    bandLevels: Map<Int, Short>,
    customPresets: List<EqualizerPreset>,
    onPresetSelected: (EqualizerEngine.EqualizerPreset) -> Unit,
    onCustomPresetSelected: (EqualizerPreset) -> Unit,
    onCustomPresetDeleted: (String) -> Unit,
    onBandLevelChange: (Int, Short) -> Unit,
    getCenterFreq: (Int) -> Int
) {
    Text(
        text = "Adjust frequency bands to customize your sound",
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant
    )

    HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

    PresetSection(onPresetSelected = onPresetSelected)

    CustomPresetSection(
        presets = customPresets,
        onPresetSelected = onCustomPresetSelected,
        onPresetDeleted = onCustomPresetDeleted
    )

    HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

    BandControlsSection(
        numBands = numBands,
        minLevel = minLevel,
        maxLevel = maxLevel,
        bandLevels = bandLevels,
        onBandLevelChange = onBandLevelChange,
        getCenterFreq = getCenterFreq
    )

    EqualizerTipCard()
}

@Composable
private fun PresetSection(onPresetSelected: (EqualizerEngine.EqualizerPreset) -> Unit) {
    Text(text = "Presets", style = MaterialTheme.typography.titleSmall)

    val presetNames = listOf("Flat", "Rock", "Jazz", "Classical", "Pop", PRESET_BASS_BOOST)

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        presetNames.take(3).forEach { name ->
            PresetChip(name = name, onSelected = { onPresetSelected(getPresetValues(name)) })
        }
    }

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        presetNames.drop(3).forEach { name ->
            PresetChip(name = name, onSelected = { onPresetSelected(getPresetValues(name)) })
        }
    }
}

@Composable
private fun PresetChip(name: String, onSelected: () -> Unit) {
    FilterChip(
        selected = false,
        onClick = onSelected,
        label = { Text(name) }
    )
}

private fun getPresetValues(presetName: String): EqualizerEngine.EqualizerPreset = when (presetName) {
    "Rock" -> EqualizerEngine.PRESET_ROCK
    "Jazz" -> EqualizerEngine.PRESET_JAZZ
    "Classical" -> EqualizerEngine.PRESET_CLASSICAL
    "Pop" -> EqualizerEngine.PRESET_POP
    PRESET_BASS_BOOST -> EqualizerEngine.PRESET_BASS_BOOST
    else -> EqualizerEngine.PRESET_FLAT
}

@Composable
private fun CustomPresetSection(
    presets: List<EqualizerPreset>,
    onPresetSelected: (EqualizerPreset) -> Unit,
    onPresetDeleted: (String) -> Unit
) {
    if (presets.isEmpty()) return

    Spacer(modifier = Modifier.height(8.dp))
    Text(text = "Custom Presets", style = MaterialTheme.typography.titleSmall)

    presets.forEach { preset ->
        CustomPresetRow(
            preset = preset,
            onSelected = { onPresetSelected(preset) },
            onDeleted = { onPresetDeleted(preset.name) }
        )
    }
}

@Composable
private fun CustomPresetRow(
    preset: EqualizerPreset,
    onSelected: () -> Unit,
    onDeleted: () -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        FilterChip(
            selected = false,
            onClick = onSelected,
            label = { Text(preset.name) },
            modifier = Modifier.weight(1f)
        )
        IconButton(onClick = onDeleted) {
            Icon(Icons.Default.Delete, contentDescription = "Delete preset")
        }
    }
}

@Composable
private fun BandControlsSection(
    numBands: Int,
    minLevel: Float,
    maxLevel: Float,
    bandLevels: Map<Int, Short>,
    onBandLevelChange: (Int, Short) -> Unit,
    getCenterFreq: (Int) -> Int
) {
    Text(text = "Band Controls", style = MaterialTheme.typography.titleSmall)

    for (band in 0 until numBands) {
        val centerFreq = getCenterFreq(band)
        val freqText = formatFrequency(centerFreq)

        EqualizerBandControl(
            frequency = freqText,
            level = bandLevels[band] ?: 0,
            minLevel = minLevel,
            maxLevel = maxLevel,
            onLevelChange = { onBandLevelChange(band, it) }
        )
    }
}

private fun formatFrequency(freq: Int): String = when {
    freq >= 1000000 -> "${freq / 1000000}MHz"
    freq >= 1000 -> "${freq / 1000}kHz"
    else -> "${freq}Hz"
}

@Composable
private fun EqualizerTipCard() {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.secondaryContainer
        )
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
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

@Composable
fun EqualizerBandControl(
    frequency: String,
    level: Short,
    minLevel: Float,
    maxLevel: Float,
    onLevelChange: (Short) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text(text = frequency, style = MaterialTheme.typography.bodyMedium)
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
            TextButton(onClick = onDismiss) { Text("Cancel") }
        }
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AutoEQDialog(
    searchQuery: String,
    onSearchQueryChange: (String) -> Unit,
    profiles: List<AutoEQProfile>,
    onDismiss: () -> Unit,
    onProfileSelected: (AutoEQProfile) -> Unit
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

                AutoEQProfileList(
                    profiles = profiles,
                    onProfileSelected = onProfileSelected
                )
            }
        },
        confirmButton = {},
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel") }
        }
    )
}

@Composable
private fun AutoEQProfileList(
    profiles: List<AutoEQProfile>,
    onProfileSelected: (AutoEQProfile) -> Unit
) {
    if (profiles.isEmpty()) {
        Text(
            "No profiles found",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(16.dp)
        )
        return
    }

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
                modifier = Modifier.clickable { onProfileSelected(profile) }
            )
            HorizontalDivider()
        }
    }
}
