// Offline content and download management
package app.akroasis.ui.settings

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Cancel
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import app.akroasis.data.download.OfflineDownloadManager
import androidx.hilt.navigation.compose.hiltViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun OfflineSettingsScreen(
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
    downloadManager: OfflineDownloadManager = hiltViewModel()
) {
    val downloadQueue by downloadManager.downloadQueue.collectAsState()
    val storageUsed by downloadManager.storageUsed.collectAsState()
    val storageLimit by downloadManager.storageLimit.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Offline Content") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.ArrowBack, "Navigate back")
                    }
                }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            item {
                StorageCard(
                    storageUsed = storageUsed,
                    storageLimit = storageLimit,
                    onClearAll = { downloadManager.clearAllOfflineContent() }
                )
            }

            item {
                Text(
                    "Download Queue",
                    style = MaterialTheme.typography.titleMedium
                )
            }

            if (downloadQueue.isEmpty()) {
                item {
                    Text(
                        "No downloads in queue",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(16.dp)
                    )
                }
            } else {
                items(downloadQueue) { item ->
                    DownloadItemCard(
                        item = item,
                        onPause = { downloadManager.pauseDownload(item.track.id) },
                        onResume = { downloadManager.resumeDownload(item.track.id) },
                        onCancel = { downloadManager.cancelDownload(item.track.id) }
                    )
                }
            }
        }
    }
}

@Composable
fun StorageCard(
    storageUsed: Long,
    storageLimit: Long,
    onClearAll: () -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.primaryContainer
        )
    ) {
        Column(
            modifier = Modifier.padding(16.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    "Storage",
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onPrimaryContainer
                )
                TextButton(onClick = onClearAll) {
                    Text("Clear All")
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            val usedGB = storageUsed / (1024f * 1024f * 1024f)
            val limitGB = storageLimit / (1024f * 1024f * 1024f)

            LinearProgressIndicator(
                progress = (storageUsed.toFloat() / storageLimit.toFloat()).coerceIn(0f, 1f),
                modifier = Modifier.fillMaxWidth()
            )

            Spacer(modifier = Modifier.height(4.dp))

            Text(
                "${String.format("%.2f", usedGB)} GB / ${String.format("%.1f", limitGB)} GB",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onPrimaryContainer
            )
        }
    }
}

@Composable
fun DownloadItemCard(
    item: OfflineDownloadManager.DownloadItem,
    onPause: () -> Unit,
    onResume: () -> Unit,
    onCancel: () -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth()
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
                    item.track.title,
                    style = MaterialTheme.typography.bodyLarge
                )
                Text(
                    item.track.artist,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )

                when (item.status) {
                    OfflineDownloadManager.DownloadStatus.DOWNLOADING -> {
                        Spacer(modifier = Modifier.height(8.dp))
                        LinearProgressIndicator(
                            progress = item.progress / 100f,
                            modifier = Modifier.fillMaxWidth()
                        )
                        Text(
                            "${item.progress}%",
                            style = MaterialTheme.typography.bodySmall
                        )
                    }
                    OfflineDownloadManager.DownloadStatus.FAILED -> {
                        Text(
                            "Failed: ${item.error ?: "Unknown error"}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error
                        )
                    }
                    OfflineDownloadManager.DownloadStatus.PAUSED -> {
                        Text(
                            "Paused",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                    OfflineDownloadManager.DownloadStatus.QUEUED,
                    OfflineDownloadManager.DownloadStatus.DOWNLOADING,
                    OfflineDownloadManager.DownloadStatus.COMPLETED -> Unit
                }
            }

            Row {
                when (item.status) {
                    OfflineDownloadManager.DownloadStatus.DOWNLOADING -> {
                        IconButton(onClick = onPause) {
                            Icon(Icons.Default.Pause, "Pause")
                        }
                    }
                    OfflineDownloadManager.DownloadStatus.PAUSED -> {
                        IconButton(onClick = onResume) {
                            Icon(Icons.Default.PlayArrow, "Resume")
                        }
                    }
                    OfflineDownloadManager.DownloadStatus.QUEUED,
                    OfflineDownloadManager.DownloadStatus.COMPLETED,
                    OfflineDownloadManager.DownloadStatus.FAILED -> Unit
                }

                IconButton(onClick = onCancel) {
                    Icon(Icons.Default.Cancel, "Cancel")
                }
            }
        }
    }
}
