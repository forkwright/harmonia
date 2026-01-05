// Smart playlist management screen
package app.akroasis.ui.playlist

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import app.akroasis.data.local.SmartPlaylistEntity
import app.akroasis.data.model.FilterRequest
import app.akroasis.ui.focus.FocusFilterScreen
import java.text.SimpleDateFormat
import java.util.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SmartPlaylistScreen(
    onNavigateBack: () -> Unit,
    viewModel: SmartPlaylistViewModel = hiltViewModel()
) {
    val playlists by viewModel.playlists.collectAsState()
    val uiState by viewModel.uiState.collectAsState()

    var showCreateDialog by remember { mutableStateOf(false) }
    var showEditDialog by remember { mutableStateOf<SmartPlaylistEntity?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Smart Playlists") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.Add, contentDescription = "Back")
                    }
                }
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = { showCreateDialog = true }
            ) {
                Icon(Icons.Default.Add, contentDescription = "Create playlist")
            }
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            when (uiState) {
                is SmartPlaylistUiState.Loading -> {
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center
                    ) {
                        CircularProgressIndicator()
                    }
                }
                is SmartPlaylistUiState.Error -> {
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center
                    ) {
                        Text(
                            text = (uiState as SmartPlaylistUiState.Error).message,
                            color = MaterialTheme.colorScheme.error
                        )
                    }
                }
                else -> {
                    if (playlists.isEmpty()) {
                        Box(
                            modifier = Modifier.fillMaxSize(),
                            contentAlignment = Alignment.Center
                        ) {
                            Column(
                                horizontalAlignment = Alignment.CenterHorizontally,
                                verticalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                Text(
                                    text = "No smart playlists yet",
                                    style = MaterialTheme.typography.bodyLarge
                                )
                                Text(
                                    text = "Tap + to create one",
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }
                    } else {
                        LazyColumn(
                            modifier = Modifier
                                .fillMaxSize()
                                .padding(16.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            items(playlists) { playlist ->
                                SmartPlaylistCard(
                                    playlist = playlist,
                                    onEdit = { showEditDialog = playlist },
                                    onDelete = { viewModel.deletePlaylist(playlist.id) },
                                    onRefresh = { viewModel.refreshPlaylist(playlist.id) }
                                )
                            }
                        }
                    }
                }
            }
        }
    }

    if (showCreateDialog) {
        CreateSmartPlaylistDialog(
            onDismiss = { showCreateDialog = false },
            onCreate = { name, filterRequest ->
                viewModel.createPlaylist(name, filterRequest)
                showCreateDialog = false
            }
        )
    }

    showEditDialog?.let { playlist ->
        EditSmartPlaylistDialog(
            playlist = playlist,
            onDismiss = { showEditDialog = null },
            onUpdate = { name, filterRequest ->
                viewModel.updatePlaylist(playlist.id, name, filterRequest)
                showEditDialog = null
            }
        )
    }
}

@Composable
private fun SmartPlaylistCard(
    playlist: SmartPlaylistEntity,
    onEdit: () -> Unit,
    onDelete: () -> Unit,
    onRefresh: () -> Unit
) {
    val dateFormat = remember { SimpleDateFormat("MMM d, yyyy", Locale.getDefault()) }

    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onEdit() }
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = playlist.name,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.weight(1f)
                )
                Row(
                    horizontalArrangement = Arrangement.spacedBy(4.dp)
                ) {
                    IconButton(onClick = onRefresh) {
                        Icon(
                            Icons.Default.Refresh,
                            contentDescription = "Refresh",
                            tint = MaterialTheme.colorScheme.primary
                        )
                    }
                    IconButton(onClick = onEdit) {
                        Icon(
                            Icons.Default.Edit,
                            contentDescription = "Edit",
                            tint = MaterialTheme.colorScheme.secondary
                        )
                    }
                    IconButton(onClick = onDelete) {
                        Icon(
                            Icons.Default.Delete,
                            contentDescription = "Delete",
                            tint = MaterialTheme.colorScheme.error
                        )
                    }
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text(
                    text = "${playlist.trackCount} tracks",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Text(
                    text = "Updated ${dateFormat.format(Date(playlist.lastRefreshed))}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            Text(
                text = "${playlist.filterRequest.conditions.size} filter rules (${playlist.filterRequest.logic})",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.primary
            )
        }
    }
}

@Composable
private fun CreateSmartPlaylistDialog(
    onDismiss: () -> Unit,
    onCreate: (String, FilterRequest) -> Unit
) {
    var name by remember { mutableStateOf("") }
    var filterRequest by remember {
        mutableStateOf(FilterRequest(conditions = emptyList()))
    }
    var showFilterScreen by remember { mutableStateOf(false) }

    if (showFilterScreen) {
        FocusFilterScreen(
            onNavigateBack = { showFilterScreen = false },
            onApplyFilter = { filter ->
                filterRequest = filter
                showFilterScreen = false
            }
        )
    } else {
        AlertDialog(
            onDismissRequest = onDismiss,
            title = { Text("Create Smart Playlist") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        value = name,
                        onValueChange = { name = it },
                        label = { Text("Playlist Name") },
                        placeholder = { Text("e.g. Hi-Res FLAC") },
                        modifier = Modifier.fillMaxWidth()
                    )

                    TextButton(
                        onClick = { showFilterScreen = true },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text(
                            if (filterRequest.conditions.isEmpty()) {
                                "Configure Filter Rules"
                            } else {
                                "${filterRequest.conditions.size} rules configured"
                            }
                        )
                    }
                }
            },
            confirmButton = {
                TextButton(
                    onClick = { onCreate(name, filterRequest) },
                    enabled = name.isNotBlank() && filterRequest.conditions.isNotEmpty()
                ) {
                    Text("CREATE")
                }
            },
            dismissButton = {
                TextButton(onClick = onDismiss) {
                    Text("CANCEL")
                }
            }
        )
    }
}

@Composable
private fun EditSmartPlaylistDialog(
    playlist: SmartPlaylistEntity,
    onDismiss: () -> Unit,
    onUpdate: (String, FilterRequest) -> Unit
) {
    var name by remember { mutableStateOf(playlist.name) }
    var filterRequest by remember { mutableStateOf(playlist.filterRequest) }
    var showFilterScreen by remember { mutableStateOf(false) }

    if (showFilterScreen) {
        FocusFilterScreen(
            onNavigateBack = { showFilterScreen = false },
            onApplyFilter = { filter ->
                filterRequest = filter
                showFilterScreen = false
            }
        )
    } else {
        AlertDialog(
            onDismissRequest = onDismiss,
            title = { Text("Edit Smart Playlist") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        value = name,
                        onValueChange = { name = it },
                        label = { Text("Playlist Name") },
                        modifier = Modifier.fillMaxWidth()
                    )

                    TextButton(
                        onClick = { showFilterScreen = true },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("${filterRequest.conditions.size} filter rules")
                    }
                }
            },
            confirmButton = {
                TextButton(
                    onClick = { onUpdate(name, filterRequest) },
                    enabled = name.isNotBlank()
                ) {
                    Text("UPDATE")
                }
            },
            dismissButton = {
                TextButton(onClick = onDismiss) {
                    Text("CANCEL")
                }
            }
        )
    }
}
