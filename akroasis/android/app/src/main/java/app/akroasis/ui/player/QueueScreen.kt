// Queue management with drag-to-reorder and swipe-to-remove
package app.akroasis.ui.player

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import org.burnoutcrew.reorderable.ReorderableItem
import org.burnoutcrew.reorderable.detectReorderAfterLongPress
import org.burnoutcrew.reorderable.rememberReorderableLazyListState
import org.burnoutcrew.reorderable.reorderable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.DragHandle
import androidx.compose.material.icons.filled.Undo
import androidx.compose.material.icons.filled.Redo
import androidx.compose.material.icons.filled.FileDownload
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import app.akroasis.data.model.Track
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QueueScreen(
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
    viewModel: PlayerViewModel = hiltViewModel()
) {
    val queue by viewModel.queue.collectAsState()
    val currentIndex by viewModel.currentIndex.collectAsState()
    val scope = rememberCoroutineScope()

    var showExportDialog by remember { mutableStateOf(false) }
    var selectedExportFormat by remember { mutableStateOf<app.akroasis.ui.queue.ExportFormat?>(null) }

    val exportLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.CreateDocument("audio/*")
    ) { uri ->
        uri?.let { outputUri ->
            selectedExportFormat?.let { format ->
                scope.launch {
                    viewModel.exportQueue(format, outputUri)
                }
            }
        }
    }

    if (showExportDialog) {
        ExportFormatDialog(
            onDismiss = { showExportDialog = false },
            onFormatSelected = { format ->
                showExportDialog = false
                selectedExportFormat = format
                exportLauncher.launch("queue.${format.extension}")
            }
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Queue (${queue.size} tracks)") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.ArrowBack, "Navigate back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { showExportDialog = true },
                        enabled = queue.isNotEmpty()
                    ) {
                        Icon(Icons.Default.FileDownload, "Export queue")
                    }
                    IconButton(
                        onClick = { viewModel.undoQueueChange() },
                        enabled = viewModel.canUndoQueue
                    ) {
                        Icon(Icons.Default.Undo, "Undo")
                    }
                    IconButton(
                        onClick = { viewModel.redoQueueChange() },
                        enabled = viewModel.canRedoQueue
                    ) {
                        Icon(Icons.Default.Redo, "Redo")
                    }
                }
            )
        }
    ) { padding ->
        if (queue.isEmpty()) {
            Box(
                modifier = modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = "Queue is empty",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        } else {
            val reorderableState = rememberReorderableLazyListState(
                onMove = { from, to ->
                    scope.launch {
                        viewModel.moveTrackInQueue(from.index, to.index)
                    }
                }
            )

            LazyColumn(
                state = reorderableState.listState,
                modifier = modifier
                    .fillMaxSize()
                    .padding(padding)
                    .reorderable(reorderableState)
                    .detectReorderAfterLongPress(reorderableState),
                contentPadding = PaddingValues(vertical = 8.dp)
            ) {
                itemsIndexed(
                    items = queue,
                    key = { _, track -> track.id }
                ) { index, track ->
                    ReorderableItem(reorderableState, key = track.id) { isDragging ->
                        QueueItem(
                            track = track,
                            isCurrentTrack = index == currentIndex,
                            isDragging = isDragging,
                            onRemove = {
                                scope.launch {
                                    viewModel.removeFromQueue(index)
                                }
                            },
                            onPlayNow = {
                                viewModel.skipToIndex(index)
                            }
                        )
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QueueItem(
    track: Track,
    isCurrentTrack: Boolean,
    isDragging: Boolean = false,
    onRemove: () -> Unit,
    onPlayNow: () -> Unit,
    modifier: Modifier = Modifier
) {
    val dismissState = rememberDismissState(
        confirmValueChange = { dismissValue ->
            if (dismissValue == DismissValue.DismissedToStart) {
                onRemove()
                true
            } else {
                false
            }
        }
    )

    SwipeToDismissBox(
        state = dismissState,
        backgroundContent = {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(horizontal = 16.dp),
                contentAlignment = Alignment.CenterEnd
            ) {
                Icon(
                    imageVector = Icons.Default.Delete,
                    contentDescription = "Delete",
                    tint = MaterialTheme.colorScheme.error
                )
            }
        },
        enableDismissFromStartToEnd = false,
        enableDismissFromEndToStart = true
    ) {
        Surface(
            onClick = onPlayNow,
            modifier = modifier.fillMaxWidth(),
            color = if (isCurrentTrack) {
                MaterialTheme.colorScheme.primaryContainer
            } else {
                MaterialTheme.colorScheme.surface
            },
            tonalElevation = if (isDragging) 8.dp else 0.dp,
            shadowElevation = if (isDragging) 8.dp else 0.dp
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(
                    modifier = Modifier.weight(1f)
                ) {
                    Text(
                        text = track.title,
                        style = MaterialTheme.typography.bodyLarge,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        color = if (isCurrentTrack) {
                            MaterialTheme.colorScheme.onPrimaryContainer
                        } else {
                            MaterialTheme.colorScheme.onSurface
                        }
                    )
                    if (track.artist.isNotEmpty()) {
                        Text(
                            text = track.artist,
                            style = MaterialTheme.typography.bodyMedium,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            color = if (isCurrentTrack) {
                                MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.7f)
                            } else {
                                MaterialTheme.colorScheme.onSurfaceVariant
                            }
                        )
                    }
                    Text(
                        text = formatDuration(track.duration),
                        style = MaterialTheme.typography.bodySmall,
                        color = if (isCurrentTrack) {
                            MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.6f)
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        }
                    )
                }

                Icon(
                    imageVector = Icons.Default.DragHandle,
                    contentDescription = "Drag to reorder",
                    tint = if (isCurrentTrack) {
                        MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.5f)
                    } else {
                        MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f)
                    }
                )
            }
        }
    }
}

@Composable
fun ExportFormatDialog(
    onDismiss: () -> Unit,
    onFormatSelected: (app.akroasis.ui.queue.ExportFormat) -> Unit
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Export Queue") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(
                    text = "Choose format:",
                    style = MaterialTheme.typography.bodyMedium
                )

                app.akroasis.ui.queue.ExportFormat.entries.forEach { format ->
                    FilledTonalButton(
                        onClick = { onFormatSelected(format) },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text(
                            text = when (format) {
                                app.akroasis.ui.queue.ExportFormat.M3U -> "M3U (Standard)"
                                app.akroasis.ui.queue.ExportFormat.M3U8 -> "M3U8 (Extended)"
                                app.akroasis.ui.queue.ExportFormat.PLS -> "PLS (Shoutcast)"
                            }
                        )
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

private fun formatDuration(ms: Long): String {
    val totalSeconds = ms / 1000
    val minutes = totalSeconds / 60
    val seconds = totalSeconds % 60
    return "%d:%02d".format(minutes, seconds)
}
