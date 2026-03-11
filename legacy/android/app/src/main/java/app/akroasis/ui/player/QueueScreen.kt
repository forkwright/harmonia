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
import androidx.compose.material.DismissValue
import androidx.compose.material.ExperimentalMaterialApi
import androidx.compose.material.SwipeToDismiss
import androidx.compose.material.rememberDismissState
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import app.akroasis.data.model.Track
import kotlinx.coroutines.CoroutineScope
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
        handleExportResult(uri, selectedExportFormat, scope, viewModel)
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
            QueueTopBar(
                queueSize = queue.size,
                canUndo = viewModel.canUndoQueue,
                canRedo = viewModel.canRedoQueue,
                onNavigateBack = onNavigateBack,
                onExport = { showExportDialog = true },
                onUndo = { viewModel.undoQueueChange() },
                onRedo = { viewModel.redoQueueChange() }
            )
        }
    ) { padding ->
        QueueContent(
            queue = queue,
            currentIndex = currentIndex,
            onMoveTrack = { from, to -> scope.launch { viewModel.moveTrackInQueue(from, to) } },
            onRemoveTrack = { index -> scope.launch { viewModel.removeFromQueue(index) } },
            onPlayTrack = { index -> viewModel.skipToIndex(index) },
            modifier = modifier.padding(padding)
        )
    }
}

private fun handleExportResult(
    uri: android.net.Uri?,
    format: app.akroasis.ui.queue.ExportFormat?,
    scope: CoroutineScope,
    viewModel: PlayerViewModel
) {
    uri?.let { outputUri ->
        format?.let { exportFormat ->
            scope.launch { viewModel.exportQueue(exportFormat, outputUri) }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun QueueTopBar(
    queueSize: Int,
    canUndo: Boolean,
    canRedo: Boolean,
    onNavigateBack: () -> Unit,
    onExport: () -> Unit,
    onUndo: () -> Unit,
    onRedo: () -> Unit
) {
    TopAppBar(
        title = { Text("Queue ($queueSize tracks)") },
        navigationIcon = {
            IconButton(onClick = onNavigateBack) {
                Icon(Icons.Default.ArrowBack, "Navigate back")
            }
        },
        actions = {
            IconButton(onClick = onExport, enabled = queueSize > 0) {
                Icon(Icons.Default.FileDownload, "Export queue")
            }
            IconButton(onClick = onUndo, enabled = canUndo) {
                Icon(Icons.Default.Undo, "Undo")
            }
            IconButton(onClick = onRedo, enabled = canRedo) {
                Icon(Icons.Default.Redo, "Redo")
            }
        }
    )
}

@Composable
private fun QueueContent(
    queue: List<Track>,
    currentIndex: Int,
    onMoveTrack: (Int, Int) -> Unit,
    onRemoveTrack: (Int) -> Unit,
    onPlayTrack: (Int) -> Unit,
    modifier: Modifier = Modifier
) {
    if (queue.isEmpty()) {
        EmptyQueueMessage(modifier = modifier)
    } else {
        QueueList(
            queue = queue,
            currentIndex = currentIndex,
            onMoveTrack = onMoveTrack,
            onRemoveTrack = onRemoveTrack,
            onPlayTrack = onPlayTrack,
            modifier = modifier
        )
    }
}

@Composable
private fun EmptyQueueMessage(modifier: Modifier = Modifier) {
    Box(
        modifier = modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = "Queue is empty",
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun QueueList(
    queue: List<Track>,
    currentIndex: Int,
    onMoveTrack: (Int, Int) -> Unit,
    onRemoveTrack: (Int) -> Unit,
    onPlayTrack: (Int) -> Unit,
    modifier: Modifier = Modifier
) {
    val reorderableState = rememberReorderableLazyListState(
        onMove = { from, to -> onMoveTrack(from.index, to.index) }
    )

    LazyColumn(
        state = reorderableState.listState,
        modifier = modifier
            .fillMaxSize()
            .reorderable(reorderableState)
            .detectReorderAfterLongPress(reorderableState),
        contentPadding = PaddingValues(vertical = 8.dp)
    ) {
        itemsIndexed(items = queue, key = { _, track -> track.id }) { index, track ->
            ReorderableItem(reorderableState, key = track.id) { isDragging ->
                QueueItem(
                    track = track,
                    isCurrentTrack = index == currentIndex,
                    isDragging = isDragging,
                    onRemove = { onRemoveTrack(index) },
                    onPlayNow = { onPlayTrack(index) }
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterialApi::class)
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
        confirmStateChange = { dismissValue ->
            if (dismissValue == DismissValue.DismissedToStart) {
                onRemove()
                true
            } else {
                false
            }
        }
    )

    SwipeToDismiss(
        state = dismissState,
        background = { SwipeToDismissBackground() },
        directions = setOf(androidx.compose.material.DismissDirection.EndToStart)
    ) {
        QueueItemContent(
            track = track,
            isCurrentTrack = isCurrentTrack,
            isDragging = isDragging,
            onPlayNow = onPlayNow,
            modifier = modifier
        )
    }
}

@Composable
private fun SwipeToDismissBackground() {
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
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun QueueItemContent(
    track: Track,
    isCurrentTrack: Boolean,
    isDragging: Boolean,
    onPlayNow: () -> Unit,
    modifier: Modifier = Modifier
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
            TrackDetails(
                track = track,
                isCurrentTrack = isCurrentTrack,
                modifier = Modifier.weight(1f)
            )
            DragHandleIcon(isCurrentTrack = isCurrentTrack)
        }
    }
}

@Composable
private fun TrackDetails(
    track: Track,
    isCurrentTrack: Boolean,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier) {
        TrackTitle(title = track.title, isCurrentTrack = isCurrentTrack)
        if (track.artist.isNotEmpty()) {
            TrackArtist(artist = track.artist, isCurrentTrack = isCurrentTrack)
        }
        TrackDuration(duration = track.duration, isCurrentTrack = isCurrentTrack)
    }
}

@Composable
private fun TrackTitle(title: String, isCurrentTrack: Boolean) {
    Text(
        text = title,
        style = MaterialTheme.typography.bodyLarge,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        color = if (isCurrentTrack) {
            MaterialTheme.colorScheme.onPrimaryContainer
        } else {
            MaterialTheme.colorScheme.onSurface
        }
    )
}

@Composable
private fun TrackArtist(artist: String, isCurrentTrack: Boolean) {
    Text(
        text = artist,
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

@Composable
private fun TrackDuration(duration: Long, isCurrentTrack: Boolean) {
    Text(
        text = formatDuration(duration),
        style = MaterialTheme.typography.bodySmall,
        color = if (isCurrentTrack) {
            MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.6f)
        } else {
            MaterialTheme.colorScheme.onSurfaceVariant
        }
    )
}

@Composable
private fun DragHandleIcon(isCurrentTrack: Boolean) {
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

@Composable
fun ExportFormatDialog(
    onDismiss: () -> Unit,
    onFormatSelected: (app.akroasis.ui.queue.ExportFormat) -> Unit
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Export Queue") },
        text = { ExportFormatOptions(onFormatSelected = onFormatSelected) },
        confirmButton = {},
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}

@Composable
private fun ExportFormatOptions(onFormatSelected: (app.akroasis.ui.queue.ExportFormat) -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(text = "Choose format:", style = MaterialTheme.typography.bodyMedium)
        app.akroasis.ui.queue.ExportFormat.entries.forEach { format ->
            ExportFormatButton(format = format, onSelected = { onFormatSelected(format) })
        }
    }
}

@Composable
private fun ExportFormatButton(
    format: app.akroasis.ui.queue.ExportFormat,
    onSelected: () -> Unit
) {
    FilledTonalButton(onClick = onSelected, modifier = Modifier.fillMaxWidth()) {
        Text(text = getFormatDisplayName(format))
    }
}

private fun getFormatDisplayName(format: app.akroasis.ui.queue.ExportFormat): String {
    return when (format) {
        app.akroasis.ui.queue.ExportFormat.M3U -> "M3U (Standard)"
        app.akroasis.ui.queue.ExportFormat.M3U8 -> "M3U8 (Extended)"
        app.akroasis.ui.queue.ExportFormat.PLS -> "PLS (Shoutcast)"
    }
}

private fun formatDuration(ms: Long): String {
    val totalSeconds = ms / 1000
    val minutes = totalSeconds / 60
    val seconds = totalSeconds % 60
    return "%d:%02d".format(minutes, seconds)
}
