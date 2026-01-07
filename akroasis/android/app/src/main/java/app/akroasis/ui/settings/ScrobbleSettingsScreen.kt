// Scrobbling settings and authentication
package app.akroasis.ui.settings

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Info
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import app.akroasis.data.preferences.ScrobblePreferences
import app.akroasis.scrobble.ScrobbleManager
import app.akroasis.ui.player.PlayerViewModel
import androidx.hilt.navigation.compose.hiltViewModel
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ScrobbleSettingsScreen(
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
    viewModel: PlayerViewModel = hiltViewModel(),
    scrobblePrefs: ScrobblePreferences = hiltViewModel()
) {
    val scrobbleState by viewModel.scrobbleState.collectAsState()
    val scope = rememberCoroutineScope()

    var showLastFmLogin by remember { mutableStateOf(false) }
    var showListenBrainzToken by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Scrobbling") },
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
            ScrobbleStatusCard(scrobbleState)

            LastFmSection(
                isConnected = viewModel.isLastFmConnected(),
                username = viewModel.getLastFmUsername(),
                onConnect = { showLastFmLogin = true },
                onDisconnect = { viewModel.disconnectLastFm() }
            )

            ListenBrainzSection(
                isConnected = viewModel.isListenBrainzConnected(),
                onConnect = { showListenBrainzToken = true },
                onDisconnect = { viewModel.disconnectListenBrainz() }
            )

            ScrobbleBehaviorSection(
                scrobblePercentage = scrobblePrefs.scrobblePercentage,
                scrobbleMinDuration = scrobblePrefs.scrobbleMinDuration,
                onPercentageChange = { scrobblePrefs.scrobblePercentage = it },
                onMinDurationChange = { scrobblePrefs.scrobbleMinDuration = it }
            )

            if (showLastFmLogin) {
                LastFmLoginDialog(
                    onDismiss = { showLastFmLogin = false },
                    onLogin = { username, password ->
                        scope.launch {
                            val result = viewModel.authenticateLastFm(username, password)
                            if (result.success) {
                                showLastFmLogin = false
                            }
                        }
                    }
                )
            }

            if (showListenBrainzToken) {
                ListenBrainzTokenDialog(
                    onDismiss = { showListenBrainzToken = false },
                    onSubmit = { token ->
                        scope.launch {
                            val result = viewModel.authenticateListenBrainz(token)
                            if (result.success) {
                                showListenBrainzToken = false
                            }
                        }
                    }
                )
            }
        }
    }
}

@Composable
fun ScrobbleStatusCard(state: ScrobbleManager.ScrobbleState) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = when (state) {
                is ScrobbleManager.ScrobbleState.Scrobbled -> MaterialTheme.colorScheme.primaryContainer
                is ScrobbleManager.ScrobbleState.NowPlaying -> MaterialTheme.colorScheme.secondaryContainer
                is ScrobbleManager.ScrobbleState.Error -> MaterialTheme.colorScheme.errorContainer
                else -> MaterialTheme.colorScheme.surfaceVariant
            }
        )
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    when (state) {
                        is ScrobbleManager.ScrobbleState.NowPlaying -> "Now Playing"
                        is ScrobbleManager.ScrobbleState.Scrobbled -> "Scrobbled"
                        is ScrobbleManager.ScrobbleState.Error -> "Error"
                        else -> "Idle"
                    },
                    style = MaterialTheme.typography.titleMedium
                )
                when (state) {
                    is ScrobbleManager.ScrobbleState.NowPlaying,
                    is ScrobbleManager.ScrobbleState.Scrobbled -> {
                        val track = when (state) {
                            is ScrobbleManager.ScrobbleState.NowPlaying -> state.track
                            is ScrobbleManager.ScrobbleState.Scrobbled -> state.track
                            else -> null
                        }
                        track?.let {
                            Text(
                                "${it.title} - ${it.artist}",
                                style = MaterialTheme.typography.bodySmall
                            )
                        }
                    }
                    is ScrobbleManager.ScrobbleState.Error -> {
                        Text(
                            state.message,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error
                        )
                    }
                    else -> {
                        Text(
                            "No scrobbling activity",
                            style = MaterialTheme.typography.bodySmall
                        )
                    }
                }
            }

            if (state is ScrobbleManager.ScrobbleState.Scrobbled) {
                Icon(
                    Icons.Default.Check,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onPrimaryContainer
                )
            }
        }
    }
}

@Composable
fun LastFmSection(
    isConnected: Boolean,
    username: String?,
    onConnect: () -> Unit,
    onDisconnect: () -> Unit
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("Last.fm", style = MaterialTheme.typography.titleMedium)

        if (isConnected && username != null) {
            Card(modifier = Modifier.fillMaxWidth()) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column {
                        Text("Connected", style = MaterialTheme.typography.bodyLarge)
                        Text(
                            username,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                    OutlinedButton(onClick = onDisconnect) {
                        Text("Disconnect")
                    }
                }
            }
        } else {
            Button(
                onClick = onConnect,
                modifier = Modifier.fillMaxWidth()
            ) {
                Text("Connect to Last.fm")
            }
            Text(
                "Scrobble your listening history to Last.fm",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
fun ScrobbleBehaviorSection(
    scrobblePercentage: Int,
    scrobbleMinDuration: Int,
    onPercentageChange: (Int) -> Unit,
    onMinDurationChange: (Int) -> Unit
) {
    Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text("Scrobble Behavior", style = MaterialTheme.typography.titleMedium)

        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(
                containerColor = MaterialTheme.colorScheme.surfaceVariant
            )
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    Icons.Default.Info,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(end = 8.dp)
                )
                Text(
                    "Tracks are scrobbled after ${scrobblePercentage}% played or 4 minutes, whichever comes first. Minimum track length: ${scrobbleMinDuration}s",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }

        Column {
            Text("Scrobble at", style = MaterialTheme.typography.bodyMedium)
            Spacer(modifier = Modifier.height(8.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Slider(
                    value = scrobblePercentage.toFloat(),
                    onValueChange = { onPercentageChange(it.toInt()) },
                    valueRange = 10f..100f,
                    steps = 17,
                    modifier = Modifier.weight(1f)
                )
                Text(
                    "$scrobblePercentage%",
                    modifier = Modifier.padding(start = 8.dp),
                    style = MaterialTheme.typography.bodyMedium
                )
            }
        }

        Column {
            Text("Minimum duration (seconds)", style = MaterialTheme.typography.bodyMedium)
            Spacer(modifier = Modifier.height(8.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Slider(
                    value = scrobbleMinDuration.toFloat(),
                    onValueChange = { onMinDurationChange(it.toInt()) },
                    valueRange = 0f..120f,
                    steps = 23,
                    modifier = Modifier.weight(1f)
                )
                Text(
                    "${scrobbleMinDuration}s",
                    modifier = Modifier.padding(start = 8.dp),
                    style = MaterialTheme.typography.bodyMedium
                )
            }
        }
    }
}

@Composable
fun ListenBrainzSection(
    isConnected: Boolean,
    onConnect: () -> Unit,
    onDisconnect: () -> Unit
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("ListenBrainz", style = MaterialTheme.typography.titleMedium)

        if (isConnected) {
            Card(modifier = Modifier.fillMaxWidth()) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text("Connected", style = MaterialTheme.typography.bodyLarge)
                    OutlinedButton(onClick = onDisconnect) {
                        Text("Disconnect")
                    }
                }
            }
        } else {
            Button(
                onClick = onConnect,
                modifier = Modifier.fillMaxWidth()
            ) {
                Text("Connect to ListenBrainz")
            }
            Text(
                "Scrobble to ListenBrainz (open-source Last.fm alternative)",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
fun LastFmLoginDialog(
    onDismiss: () -> Unit,
    onLogin: (username: String, password: String) -> Unit
) {
    var username by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Connect to Last.fm") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = username,
                    onValueChange = { username = it },
                    label = { Text("Username") },
                    modifier = Modifier.fillMaxWidth(),
                    enabled = !isLoading
                )
                OutlinedTextField(
                    value = password,
                    onValueChange = { password = it },
                    label = { Text("Password") },
                    visualTransformation = PasswordVisualTransformation(),
                    modifier = Modifier.fillMaxWidth(),
                    enabled = !isLoading
                )
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    isLoading = true
                    onLogin(username, password)
                },
                enabled = username.isNotBlank() && password.isNotBlank() && !isLoading
            ) {
                if (isLoading) {
                    CircularProgressIndicator(modifier = Modifier.size(16.dp))
                } else {
                    Text("Connect")
                }
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, enabled = !isLoading) {
                Text("Cancel")
            }
        }
    )
}

@Composable
fun ListenBrainzTokenDialog(
    onDismiss: () -> Unit,
    onSubmit: (token: String) -> Unit
) {
    var token by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Connect to ListenBrainz") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(
                    "Get your user token from ListenBrainz settings:",
                    style = MaterialTheme.typography.bodySmall
                )
                Text(
                    "https://listenbrainz.org/settings/",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.primary
                )
                Spacer(modifier = Modifier.height(8.dp))
                OutlinedTextField(
                    value = token,
                    onValueChange = { token = it },
                    label = { Text("User Token") },
                    modifier = Modifier.fillMaxWidth(),
                    enabled = !isLoading
                )
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    isLoading = true
                    onSubmit(token)
                },
                enabled = token.isNotBlank() && !isLoading
            ) {
                if (isLoading) {
                    CircularProgressIndicator(modifier = Modifier.size(16.dp))
                } else {
                    Text("Connect")
                }
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, enabled = !isLoading) {
                Text("Cancel")
            }
        }
    )
}
