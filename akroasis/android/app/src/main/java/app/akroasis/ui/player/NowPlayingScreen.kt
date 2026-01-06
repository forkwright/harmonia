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
    val preferredDac by viewModel.preferredDac.collectAsState()
    val equalizerEnabled by viewModel.equalizerEnabled.collectAsState()
    val gaplessEnabled by viewModel.gaplessEnabled.collectAsState()
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
    Surface(
        modifier = modifier.fillMaxSize().padding(padding),
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
                Surface(
                    modifier = Modifier
                        .size(280.dp)
                        .padding(bottom = 32.dp)
                        .clip(MaterialTheme.shapes.medium),
                    shape = MaterialTheme.shapes.medium,
                    color = MaterialTheme.colorScheme.surfaceVariant
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        if (uiState.playbackState is PlaybackState.Buffering) {
                            CircularProgressIndicator()
                        } else if (uiState.coverArtUrl != null) {
                            SubcomposeAsyncImage(
                                model = uiState.coverArtUrl,
                                contentDescription = "Album art for ${uiState.trackTitle}",
                                modifier = Modifier.fillMaxSize(),
                                contentScale = ContentScale.Crop,
                                error = {
                                    Icon(
                                        imageVector = Icons.Default.MusicNote,
                                        contentDescription = null,
                                        modifier = Modifier.size(80.dp),
                                        tint = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                },
                                loading = {
                                    CircularProgressIndicator()
                                }
                            )
                        } else {
                            Icon(
                                imageVector = Icons.Default.MusicNote,
                                contentDescription = null,
                                modifier = Modifier.size(80.dp),
                                tint = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }

                Text(
                    text = uiState.trackTitle,
                    style = MaterialTheme.typography.headlineMedium,
                    textAlign = TextAlign.Center,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis
                )

                if (uiState.trackArtist.isNotEmpty()) {
                    Text(
                        text = uiState.trackArtist,
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        textAlign = TextAlign.Center,
                        modifier = Modifier.padding(top = 8.dp)
                    )
                }

                if (uiState.trackAlbum.isNotEmpty()) {
                    Text(
                        text = uiState.trackAlbum,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        textAlign = TextAlign.Center,
                        modifier = Modifier.padding(top = 4.dp)
                    )
                }

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
                        onClick = { showSpeedControlSheet = true },
                        label = {
                            Text(
                                text = "${String.format("%.1f", playbackSpeed)}x",
                                style = MaterialTheme.typography.labelSmall
                            )
                        }
                    )
                }

                SignalPathCard(
                    pipelineState = pipelineState,
                    modifier = Modifier.padding(top = 16.dp)
                )

                TextButton(
                    onClick = { showSignalPath = !showSignalPath },
                    modifier = Modifier.padding(top = 8.dp)
                ) {
                    Text(if (showSignalPath) "Hide Signal Path" else "Show Signal Path")
                }

                if (uiState.errorMessage != null) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        modifier = Modifier.padding(top = 16.dp)
                    ) {
                        uiState.errorMessage?.let { error ->
                            Text(
                                text = error,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.error,
                                textAlign = TextAlign.Center
                            )
                        }
                        Button(
                            onClick = { viewModel.retryLoad() },
                            modifier = Modifier.padding(top = 8.dp)
                        ) {
                            Text("Retry")
                        }
                    }
                }
            }

            Column {
                Slider(
                    value = if (uiState.duration > 0) {
                        uiState.position.toFloat() / uiState.duration
                    } else 0f,
                    onValueChange = { progress ->
                        viewModel.seekTo((progress * uiState.duration).toLong())
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp)
                )

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text(
                        text = formatTime(uiState.position),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Text(
                        text = formatTime(uiState.duration),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                if (sleepTimerActive) {
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
                            text = "Sleep timer: ${formatTime(sleepTimerRemaining)}",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.primary
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        IconButton(
                            onClick = { viewModel.cancelSleepTimer() },
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

                if (isLowBattery && !isCharging) {
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
                } else if (uiState.playbackState is PlaybackState.Playing) {
                    Text(
                        text = viewModel.getBatteryImpactEstimate(),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(vertical = 4.dp)
                    )
                }

                if (abTestingMode) {
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
                                    text = "Playing version $abTestingVersion",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.7f)
                                )
                            }
                            Row {
                                FilledTonalButton(
                                    onClick = { viewModel.switchABVersion() }
                                ) {
                                    Text("Switch to ${if (abTestingVersion == "A") "B" else "A"}")
                                }
                                Spacer(modifier = Modifier.width(8.dp))
                                TextButton(
                                    onClick = { viewModel.exitABTest() }
                                ) {
                                    Text("Exit")
                                }
                            }
                        }
                    }

                    ABLevelMeterCard(
                        levelA = abLevelA,
                        levelB = abLevelB,
                        gainCompensation = abGainCompensation,
                        matchingEnabled = abMatchingEnabled,
                        onToggleMatching = { viewModel.toggleAbLevelMatching() },
                        onManualGainChange = { viewModel.setAbManualGain(it) },
                        modifier = Modifier.padding(vertical = 8.dp)
                    )
                }

                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 24.dp),
                    horizontalArrangement = Arrangement.SpaceEvenly,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    IconButton(
                        onClick = {
                            if (sleepTimerActive) {
                                viewModel.cancelSleepTimer()
                            } else {
                                showSleepTimerSheet = true
                            }
                        },
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
                        onClick = { viewModel.stop() },
                        modifier = Modifier.size(64.dp)
                    ) {
                        Icon(
                            imageVector = Icons.Default.Square,
                            contentDescription = "Stop",
                            modifier = Modifier.size(32.dp)
                        )
                    }

                    FilledIconButton(
                        onClick = { viewModel.playPause() },
                        modifier = Modifier.size(80.dp)
                    ) {
                        Icon(
                            imageVector = when (uiState.playbackState) {
                                is PlaybackState.Playing -> Icons.Default.Pause
                                else -> Icons.Default.PlayArrow
                            },
                            contentDescription = when (uiState.playbackState) {
                                is PlaybackState.Playing -> "Pause"
                                else -> "Play"
                            },
                            modifier = Modifier.size(40.dp)
                        )
                    }
                }
            }
        }
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
    ModalBottomSheet(
        onDismissRequest = onDismiss
    ) {
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

    ModalBottomSheet(
        onDismissRequest = onDismiss
    ) {
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

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceEvenly,
                verticalAlignment = Alignment.CenterVertically
            ) {
                FilledTonalButton(
                    onClick = { selectedSpeed = (selectedSpeed - 0.05f).coerceAtLeast(0.25f) }
                ) {
                    Text("-")
                }

                FilledTonalButton(
                    onClick = { selectedSpeed = 1.0f }
                ) {
                    Text("Reset")
                }

                FilledTonalButton(
                    onClick = { selectedSpeed = (selectedSpeed + 0.05f).coerceAtMost(3.0f) }
                ) {
                    Text("+")
                }
            }

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
                        onClick = { selectedSpeed = preset },
                        label = { Text("${String.format("%.2f", preset)}x") },
                        modifier = Modifier.weight(1f)
                    )
                }
            }

            Text(
                text = "Remember for",
                style = MaterialTheme.typography.titleSmall
            )

            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    RadioButton(
                        selected = savePreference == "session",
                        onClick = { savePreference = "session" }
                    )
                    Text(
                        text = "This session only",
                        modifier = Modifier.padding(start = 8.dp)
                    )
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    RadioButton(
                        selected = savePreference == "track",
                        onClick = { savePreference = "track" }
                    )
                    Text(
                        text = "This track",
                        modifier = Modifier.padding(start = 8.dp)
                    )
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    RadioButton(
                        selected = savePreference == "album",
                        onClick = { savePreference = "album" }
                    )
                    Text(
                        text = "This album",
                        modifier = Modifier.padding(start = 8.dp)
                    )
                }
            }

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

            Spacer(modifier = Modifier.height(16.dp))
        }
    }
}

private fun formatTime(ms: Long): String {
    val totalSeconds = ms / 1000
    val minutes = totalSeconds / 60
    val seconds = totalSeconds % 60
    return "%d:%02d".format(minutes, seconds)
}
