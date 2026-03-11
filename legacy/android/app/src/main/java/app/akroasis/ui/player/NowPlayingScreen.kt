// Now playing screen with playback controls
package app.akroasis.ui.player

import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Battery1Bar
import androidx.compose.material.icons.filled.MusicNote
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.QueueMusic
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Square
import androidx.compose.material.icons.filled.Timer
import androidx.compose.material.icons.filled.TimerOff
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import app.akroasis.audio.AudioFormatInfo
import app.akroasis.audio.AudioPipelineState
import app.akroasis.audio.PlaybackState
import coil.compose.SubcomposeAsyncImage

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NowPlayingScreen(
    onNavigateToSettings: () -> Unit = {},
    onNavigateToQueue: () -> Unit = {},
    modifier: Modifier = Modifier,
    viewModel: PlayerViewModel = hiltViewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    val audioFormat by viewModel.audioFormat.collectAsState()
    val pipelineState by viewModel.pipelineState.collectAsState()
    val playbackSpeed by viewModel.playbackSpeed.collectAsState()
    val sleepTimerActive by viewModel.sleepTimerActive.collectAsState()
    val sleepTimerRemaining by viewModel.sleepTimerRemaining.collectAsState()
    val batteryLevel by viewModel.batteryLevel.collectAsState()
    val isLowBattery by viewModel.isLowBattery.collectAsState()
    val isCharging by viewModel.isCharging.collectAsState()
    val abTestingMode by viewModel.abTestingMode.collectAsState()
    val abTestingVersion by viewModel.abTestingCurrentVersion.collectAsState()
    val abLevelA by viewModel.abLevelA.collectAsState()
    val abLevelB by viewModel.abLevelB.collectAsState()
    val abGainCompensation by viewModel.abGainCompensation.collectAsState()
    val abMatchingEnabled by viewModel.abMatchingEnabled.collectAsState()

    var showSleepTimerSheet by remember { mutableStateOf(false) }
    var showSpeedControlSheet by remember { mutableStateOf(false) }
    var showSignalPath by remember { mutableStateOf(false) }

    if (showSleepTimerSheet) {
        SleepTimerBottomSheet(
            onDismiss = { showSleepTimerSheet = false },
            onStartTimer = { duration -> viewModel.startSleepTimer(duration) },
            onStartEndOfTrack = { viewModel.startSleepTimerEndOfTrack() }
        )
    }

    if (showSpeedControlSheet) {
        SpeedControlBottomSheet(
            currentSpeed = playbackSpeed,
            onDismiss = { showSpeedControlSheet = false },
            onSetSpeed = { speed, saveForTrack ->
                viewModel.setPlaybackSpeedForTrack(speed, saveForTrack)
            },
            onSetSpeedForAlbum = { speed ->
                viewModel.setPlaybackSpeedForAlbum(speed)
            }
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Now Playing") },
                actions = {
                    IconButton(onClick = onNavigateToQueue) {
                        Icon(Icons.Default.QueueMusic, "Queue")
                    }
                    IconButton(onClick = onNavigateToSettings) {
                        Icon(Icons.Default.Settings, "Settings")
                    }
                }
            )
        }
    ) { padding ->
        NowPlayingContent(
            uiState = uiState,
            audioFormat = audioFormat,
            pipelineState = pipelineState,
            playbackSpeed = playbackSpeed,
            sleepTimerActive = sleepTimerActive,
            sleepTimerRemaining = sleepTimerRemaining,
            batteryLevel = batteryLevel,
            isLowBattery = isLowBattery,
            isCharging = isCharging,
            abTestingMode = abTestingMode,
            abTestingVersion = abTestingVersion,
            abLevelA = abLevelA,
            abLevelB = abLevelB,
            abGainCompensation = abGainCompensation,
            abMatchingEnabled = abMatchingEnabled,
            showSignalPath = showSignalPath,
            onToggleSignalPath = { showSignalPath = !showSignalPath },
            onShowSpeedControl = { showSpeedControlSheet = true },
            onShowSleepTimer = { showSleepTimerSheet = true },
            onCancelSleepTimer = { viewModel.cancelSleepTimer() },
            onSeek = { viewModel.seekTo(it) },
            onRetry = { viewModel.retryLoad() },
            onPlayPause = { viewModel.playPause() },
            onStop = { viewModel.stop() },
            onSwitchABVersion = { viewModel.switchABVersion() },
            onExitABTest = { viewModel.exitABTest() },
            onToggleAbLevelMatching = { viewModel.toggleAbLevelMatching() },
            onSetAbManualGain = { viewModel.setAbManualGain(it) },
            getBatteryImpactEstimate = { viewModel.getBatteryImpactEstimate() },
            modifier = modifier.padding(padding)
        )
    }
}

@Composable
private fun NowPlayingContent(
    uiState: PlayerUiState,
    audioFormat: AudioFormatInfo?,
    pipelineState: AudioPipelineState?,
    playbackSpeed: Float,
    sleepTimerActive: Boolean,
    sleepTimerRemaining: Long,
    batteryLevel: Int,
    isLowBattery: Boolean,
    isCharging: Boolean,
    abTestingMode: Boolean,
    abTestingVersion: String,
    abLevelA: Float,
    abLevelB: Float,
    abGainCompensation: Float,
    abMatchingEnabled: Boolean,
    showSignalPath: Boolean,
    onToggleSignalPath: () -> Unit,
    onShowSpeedControl: () -> Unit,
    onShowSleepTimer: () -> Unit,
    onCancelSleepTimer: () -> Unit,
    onSeek: (Long) -> Unit,
    onRetry: () -> Unit,
    onPlayPause: () -> Unit,
    onStop: () -> Unit,
    onSwitchABVersion: () -> Unit,
    onExitABTest: () -> Unit,
    onToggleAbLevelMatching: () -> Unit,
    onSetAbManualGain: (Float) -> Unit,
    getBatteryImpactEstimate: () -> String,
    modifier: Modifier = Modifier
) {
    Surface(
        modifier = modifier.fillMaxSize(),
        color = MaterialTheme.colorScheme.background
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.SpaceBetween
        ) {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.Center
            ) {
                AlbumArtSection(
                    playbackState = uiState.playbackState,
                    coverArtUrl = uiState.coverArtUrl,
                    trackTitle = uiState.trackTitle
                )

                TrackInfoSection(
                    trackTitle = uiState.trackTitle,
                    trackArtist = uiState.trackArtist,
                    trackAlbum = uiState.trackAlbum
                )

                AudioFormatChips(
                    audioFormat = audioFormat,
                    playbackSpeed = playbackSpeed,
                    onShowSpeedControl = onShowSpeedControl
                )

                SignalPathCard(
                    pipelineState = pipelineState,
                    modifier = Modifier.padding(top = 16.dp)
                )

                TextButton(
                    onClick = onToggleSignalPath,
                    modifier = Modifier.padding(top = 8.dp)
                ) {
                    Text(if (showSignalPath) "Hide Signal Path" else "Show Signal Path")
                }

                ErrorSection(
                    errorMessage = uiState.errorMessage,
                    onRetry = onRetry
                )
            }

            PlaybackControlsColumn(
                uiState = uiState,
                sleepTimerActive = sleepTimerActive,
                sleepTimerRemaining = sleepTimerRemaining,
                batteryLevel = batteryLevel,
                isLowBattery = isLowBattery,
                isCharging = isCharging,
                abTestingMode = abTestingMode,
                abTestingVersion = abTestingVersion,
                abLevelA = abLevelA,
                abLevelB = abLevelB,
                abGainCompensation = abGainCompensation,
                abMatchingEnabled = abMatchingEnabled,
                onSeek = onSeek,
                onShowSleepTimer = onShowSleepTimer,
                onCancelSleepTimer = onCancelSleepTimer,
                onPlayPause = onPlayPause,
                onStop = onStop,
                onSwitchABVersion = onSwitchABVersion,
                onExitABTest = onExitABTest,
                onToggleAbLevelMatching = onToggleAbLevelMatching,
                onSetAbManualGain = onSetAbManualGain,
                getBatteryImpactEstimate = getBatteryImpactEstimate
            )
        }
    }
}

@Composable
private fun AlbumArtSection(
    playbackState: PlaybackState,
    coverArtUrl: String?,
    trackTitle: String
) {
    Surface(
        modifier = Modifier
            .size(280.dp)
            .padding(bottom = 32.dp)
            .clip(MaterialTheme.shapes.medium),
        shape = MaterialTheme.shapes.medium,
        color = MaterialTheme.colorScheme.surfaceVariant
    ) {
        Box(contentAlignment = Alignment.Center) {
            when {
                playbackState is PlaybackState.Buffering -> CircularProgressIndicator()
                coverArtUrl != null -> AlbumArtImage(coverArtUrl, trackTitle)
                else -> PlaceholderMusicIcon()
            }
        }
    }
}

@Composable
private fun AlbumArtImage(coverArtUrl: String, trackTitle: String) {
    SubcomposeAsyncImage(
        model = coverArtUrl,
        contentDescription = "Album art for $trackTitle",
        modifier = Modifier.fillMaxSize(),
        contentScale = ContentScale.Crop,
        error = { PlaceholderMusicIcon() },
        loading = { CircularProgressIndicator() }
    )
}

@Composable
private fun PlaceholderMusicIcon() {
    Icon(
        imageVector = Icons.Default.MusicNote,
        contentDescription = null,
        modifier = Modifier.size(80.dp),
        tint = MaterialTheme.colorScheme.onSurfaceVariant
    )
}

@Composable
private fun TrackInfoSection(
    trackTitle: String,
    trackArtist: String,
    trackAlbum: String
) {
    Text(
        text = trackTitle,
        style = MaterialTheme.typography.headlineMedium,
        textAlign = TextAlign.Center,
        maxLines = 2,
        overflow = TextOverflow.Ellipsis
    )

    if (trackArtist.isNotEmpty()) {
        Text(
            text = trackArtist,
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = 8.dp)
        )
    }

    if (trackAlbum.isNotEmpty()) {
        Text(
            text = trackAlbum,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = 4.dp)
        )
    }
}

@Composable
private fun AudioFormatChips(
    audioFormat: AudioFormatInfo?,
    playbackSpeed: Float,
    onShowSpeedControl: () -> Unit
) {
    Row(
        modifier = Modifier.padding(top = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        audioFormat?.let { format ->
            Text(
                text = "${format.sampleRate / 1000}kHz / ${format.bitDepth}bit",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.primary,
                textAlign = TextAlign.Center
            )
        }

        FilterChip(
            selected = playbackSpeed != 1.0f,
            onClick = onShowSpeedControl,
            label = {
                Text(
                    text = "${String.format("%.1f", playbackSpeed)}x",
                    style = MaterialTheme.typography.labelSmall
                )
            }
        )
    }
}

@Composable
private fun ErrorSection(
    errorMessage: String?,
    onRetry: () -> Unit
) {
    if (errorMessage == null) return

    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.padding(top = 16.dp)
    ) {
        Text(
            text = errorMessage,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.error,
            textAlign = TextAlign.Center
        )
        Button(
            onClick = onRetry,
            modifier = Modifier.padding(top = 8.dp)
        ) {
            Text("Retry")
        }
    }
}

@Composable
private fun PlaybackControlsColumn(
    uiState: PlayerUiState,
    sleepTimerActive: Boolean,
    sleepTimerRemaining: Long,
    batteryLevel: Int,
    isLowBattery: Boolean,
    isCharging: Boolean,
    abTestingMode: Boolean,
    abTestingVersion: String,
    abLevelA: Float,
    abLevelB: Float,
    abGainCompensation: Float,
    abMatchingEnabled: Boolean,
    onSeek: (Long) -> Unit,
    onShowSleepTimer: () -> Unit,
    onCancelSleepTimer: () -> Unit,
    onPlayPause: () -> Unit,
    onStop: () -> Unit,
    onSwitchABVersion: () -> Unit,
    onExitABTest: () -> Unit,
    onToggleAbLevelMatching: () -> Unit,
    onSetAbManualGain: (Float) -> Unit,
    getBatteryImpactEstimate: () -> String
) {
    Column {
        SeekBarSection(
            position = uiState.position,
            duration = uiState.duration,
            onSeek = onSeek
        )

        SleepTimerIndicator(
            isActive = sleepTimerActive,
            remainingTime = sleepTimerRemaining,
            onCancel = onCancelSleepTimer
        )

        BatteryWarningSection(
            isLowBattery = isLowBattery,
            isCharging = isCharging,
            batteryLevel = batteryLevel,
            isPlaying = uiState.playbackState is PlaybackState.Playing,
            getBatteryImpactEstimate = getBatteryImpactEstimate
        )

        ABTestingSection(
            isActive = abTestingMode,
            currentVersion = abTestingVersion,
            levelA = abLevelA,
            levelB = abLevelB,
            gainCompensation = abGainCompensation,
            matchingEnabled = abMatchingEnabled,
            onSwitchVersion = onSwitchABVersion,
            onExit = onExitABTest,
            onToggleMatching = onToggleAbLevelMatching,
            onManualGainChange = onSetAbManualGain
        )

        PlaybackButtonsRow(
            playbackState = uiState.playbackState,
            sleepTimerActive = sleepTimerActive,
            onShowSleepTimer = onShowSleepTimer,
            onCancelSleepTimer = onCancelSleepTimer,
            onStop = onStop,
            onPlayPause = onPlayPause
        )
    }
}

@Composable
private fun SeekBarSection(
    position: Long,
    duration: Long,
    onSeek: (Long) -> Unit
) {
    Slider(
        value = if (duration > 0) position.toFloat() / duration else 0f,
        onValueChange = { progress -> onSeek((progress * duration).toLong()) },
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp)
    )

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(
            text = formatTime(position),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Text(
            text = formatTime(duration),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun SleepTimerIndicator(
    isActive: Boolean,
    remainingTime: Long,
    onCancel: () -> Unit
) {
    if (!isActive) return

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 8.dp),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            imageVector = Icons.Default.Timer,
            contentDescription = null,
            modifier = Modifier.size(16.dp),
            tint = MaterialTheme.colorScheme.primary
        )
        Spacer(modifier = Modifier.width(8.dp))
        Text(
            text = "Sleep timer: ${formatTime(remainingTime)}",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.primary
        )
        Spacer(modifier = Modifier.width(8.dp))
        IconButton(
            onClick = onCancel,
            modifier = Modifier.size(24.dp)
        ) {
            Icon(
                imageVector = Icons.Default.TimerOff,
                contentDescription = "Cancel sleep timer",
                modifier = Modifier.size(16.dp)
            )
        }
    }
}

@Composable
private fun BatteryWarningSection(
    isLowBattery: Boolean,
    isCharging: Boolean,
    batteryLevel: Int,
    isPlaying: Boolean,
    getBatteryImpactEstimate: () -> String
) {
    when {
        isLowBattery && !isCharging -> {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 8.dp),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    imageVector = Icons.Default.Battery1Bar,
                    contentDescription = null,
                    modifier = Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.error
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = "Low battery ($batteryLevel%) - Effects disabled",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error
                )
            }
        }
        isPlaying -> {
            Text(
                text = getBatteryImpactEstimate(),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(vertical = 4.dp)
            )
        }
    }
}

@Composable
private fun ABTestingSection(
    isActive: Boolean,
    currentVersion: String,
    levelA: Float,
    levelB: Float,
    gainCompensation: Float,
    matchingEnabled: Boolean,
    onSwitchVersion: () -> Unit,
    onExit: () -> Unit,
    onToggleMatching: () -> Unit,
    onManualGainChange: (Float) -> Unit
) {
    if (!isActive) return

    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 8.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.primaryContainer
        )
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column {
                Text(
                    text = "A/B Comparison Mode",
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onPrimaryContainer
                )
                Text(
                    text = "Playing version $currentVersion",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.7f)
                )
            }
            Row {
                FilledTonalButton(onClick = onSwitchVersion) {
                    Text("Switch to ${if (currentVersion == "A") "B" else "A"}")
                }
                Spacer(modifier = Modifier.width(8.dp))
                TextButton(onClick = onExit) {
                    Text("Exit")
                }
            }
        }
    }

    ABLevelMeterCard(
        levelA = levelA,
        levelB = levelB,
        gainCompensation = gainCompensation,
        matchingEnabled = matchingEnabled,
        onToggleMatching = onToggleMatching,
        onManualGainChange = onManualGainChange,
        modifier = Modifier.padding(vertical = 8.dp)
    )
}

@Composable
private fun PlaybackButtonsRow(
    playbackState: PlaybackState,
    sleepTimerActive: Boolean,
    onShowSleepTimer: () -> Unit,
    onCancelSleepTimer: () -> Unit,
    onStop: () -> Unit,
    onPlayPause: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(top = 24.dp),
        horizontalArrangement = Arrangement.SpaceEvenly,
        verticalAlignment = Alignment.CenterVertically
    ) {
        IconButton(
            onClick = { if (sleepTimerActive) onCancelSleepTimer() else onShowSleepTimer() },
            modifier = Modifier.size(64.dp)
        ) {
            Icon(
                imageVector = if (sleepTimerActive) Icons.Default.TimerOff else Icons.Default.Timer,
                contentDescription = if (sleepTimerActive) "Cancel sleep timer" else "Sleep timer",
                modifier = Modifier.size(32.dp),
                tint = if (sleepTimerActive) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface
            )
        }

        IconButton(
            onClick = onStop,
            modifier = Modifier.size(64.dp)
        ) {
            Icon(
                imageVector = Icons.Default.Square,
                contentDescription = "Stop",
                modifier = Modifier.size(32.dp)
            )
        }

        FilledIconButton(
            onClick = onPlayPause,
            modifier = Modifier.size(80.dp)
        ) {
            Icon(
                imageVector = if (playbackState is PlaybackState.Playing) Icons.Default.Pause else Icons.Default.PlayArrow,
                contentDescription = if (playbackState is PlaybackState.Playing) "Pause" else "Play",
                modifier = Modifier.size(40.dp)
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SleepTimerBottomSheet(
    onDismiss: () -> Unit,
    onStartTimer: (Long) -> Unit,
    onStartEndOfTrack: () -> Unit
) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text(
                text = "Sleep Timer",
                style = MaterialTheme.typography.titleLarge,
                modifier = Modifier.padding(bottom = 8.dp)
            )

            val presets = listOf(
                "15 minutes" to app.akroasis.audio.SleepTimer.FIFTEEN_MINUTES,
                "30 minutes" to app.akroasis.audio.SleepTimer.THIRTY_MINUTES,
                "45 minutes" to app.akroasis.audio.SleepTimer.FORTYFIVE_MINUTES,
                "60 minutes" to app.akroasis.audio.SleepTimer.SIXTY_MINUTES
            )

            presets.forEach { (label, duration) ->
                FilledTonalButton(
                    onClick = {
                        onStartTimer(duration)
                        onDismiss()
                    },
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text(label)
                }
            }

            FilledTonalButton(
                onClick = {
                    onStartEndOfTrack()
                    onDismiss()
                },
                modifier = Modifier.fillMaxWidth()
            ) {
                Text("End of track")
            }

            Spacer(modifier = Modifier.height(16.dp))
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SpeedControlBottomSheet(
    currentSpeed: Float,
    onDismiss: () -> Unit,
    onSetSpeed: (Float, Boolean) -> Unit,
    onSetSpeedForAlbum: (Float) -> Unit
) {
    var selectedSpeed by remember { mutableStateOf(currentSpeed) }
    var savePreference by remember { mutableStateOf("session") }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Text(
                text = "Playback Speed",
                style = MaterialTheme.typography.titleLarge
            )

            Text(
                text = "${String.format("%.2f", selectedSpeed)}x",
                style = MaterialTheme.typography.displaySmall,
                textAlign = TextAlign.Center,
                modifier = Modifier.fillMaxWidth()
            )

            SpeedAdjustmentButtons(
                selectedSpeed = selectedSpeed,
                onSpeedChange = { selectedSpeed = it }
            )

            SpeedPresetChips(
                selectedSpeed = selectedSpeed,
                onPresetSelected = { selectedSpeed = it }
            )

            SavePreferenceOptions(
                selectedOption = savePreference,
                onOptionSelected = { savePreference = it }
            )

            SpeedControlActions(
                selectedSpeed = selectedSpeed,
                savePreference = savePreference,
                onDismiss = onDismiss,
                onSetSpeed = onSetSpeed,
                onSetSpeedForAlbum = onSetSpeedForAlbum
            )

            Spacer(modifier = Modifier.height(16.dp))
        }
    }
}

@Composable
private fun SpeedAdjustmentButtons(
    selectedSpeed: Float,
    onSpeedChange: (Float) -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceEvenly,
        verticalAlignment = Alignment.CenterVertically
    ) {
        FilledTonalButton(
            onClick = { onSpeedChange((selectedSpeed - 0.05f).coerceAtLeast(0.25f)) }
        ) {
            Text("-")
        }

        FilledTonalButton(onClick = { onSpeedChange(1.0f) }) {
            Text("Reset")
        }

        FilledTonalButton(
            onClick = { onSpeedChange((selectedSpeed + 0.05f).coerceAtMost(3.0f)) }
        ) {
            Text("+")
        }
    }
}

@Composable
private fun SpeedPresetChips(
    selectedSpeed: Float,
    onPresetSelected: (Float) -> Unit
) {
    Text(
        text = "Presets",
        style = MaterialTheme.typography.titleSmall
    )

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        listOf(0.5f, 0.75f, 1.0f, 1.25f, 1.5f, 2.0f).forEach { preset ->
            FilterChip(
                selected = selectedSpeed == preset,
                onClick = { onPresetSelected(preset) },
                label = { Text("${String.format("%.2f", preset)}x") },
                modifier = Modifier.weight(1f)
            )
        }
    }
}

@Composable
private fun SavePreferenceOptions(
    selectedOption: String,
    onOptionSelected: (String) -> Unit
) {
    Text(
        text = "Remember for",
        style = MaterialTheme.typography.titleSmall
    )

    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        listOf(
            "session" to "This session only",
            "track" to "This track",
            "album" to "This album"
        ).forEach { (value, label) ->
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                RadioButton(
                    selected = selectedOption == value,
                    onClick = { onOptionSelected(value) }
                )
                Text(
                    text = label,
                    modifier = Modifier.padding(start = 8.dp)
                )
            }
        }
    }
}

@Composable
private fun SpeedControlActions(
    selectedSpeed: Float,
    savePreference: String,
    onDismiss: () -> Unit,
    onSetSpeed: (Float, Boolean) -> Unit,
    onSetSpeedForAlbum: (Float) -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        TextButton(
            onClick = onDismiss,
            modifier = Modifier.weight(1f)
        ) {
            Text("Cancel")
        }

        Button(
            onClick = {
                when (savePreference) {
                    "session" -> onSetSpeed(selectedSpeed, false)
                    "track" -> onSetSpeed(selectedSpeed, true)
                    "album" -> onSetSpeedForAlbum(selectedSpeed)
                }
                onDismiss()
            },
            modifier = Modifier.weight(1f)
        ) {
            Text("Apply")
        }
    }
}

private fun formatTime(ms: Long): String {
    val totalSeconds = ms / 1000
    val minutes = totalSeconds / 60
    val seconds = totalSeconds % 60
    return "%d:%02d".format(minutes, seconds)
}
