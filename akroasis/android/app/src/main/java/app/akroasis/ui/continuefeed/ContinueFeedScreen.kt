// Continue feed screen for unified media discovery
package app.akroasis.ui.continuefeed

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.*
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import app.akroasis.data.model.MediaItem
import app.akroasis.data.repository.ContinueItem

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContinueFeedScreen(
    onNavigateToPlayer: (MediaItem.Music) -> Unit,
    onNavigateToAudiobook: (MediaItem.Audiobook) -> Unit,
    onNavigateToEbook: (MediaItem.Ebook) -> Unit,
    modifier: Modifier = Modifier,
    viewModel: ContinueFeedViewModel = hiltViewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    val isRefreshing by viewModel.isRefreshing.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Continue") },
                actions = {
                    IconButton(onClick = { viewModel.refresh() }) {
                        Icon(
                            imageVector = Icons.Default.Refresh,
                            contentDescription = "Refresh"
                        )
                    }
                }
            )
        },
        modifier = modifier
    ) { paddingValues ->
        PullToRefreshBox(
            isRefreshing = isRefreshing,
            onRefresh = { viewModel.refresh() },
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            when (val state = uiState) {
                is ContinueFeedUiState.Loading -> LoadingView()
                is ContinueFeedUiState.Empty -> EmptyView()
                is ContinueFeedUiState.Error -> ErrorView(
                    message = state.message,
                    onRetry = { viewModel.retry() }
                )
                is ContinueFeedUiState.Success -> ContinueFeedContent(
                    items = state.items,
                    onItemClick = { continueItem ->
                        handleItemClick(
                            continueItem = continueItem,
                            onNavigateToPlayer = onNavigateToPlayer,
                            onNavigateToAudiobook = onNavigateToAudiobook,
                            onNavigateToEbook = onNavigateToEbook
                        )
                    }
                )
            }
        }
    }
}

@Composable
private fun ContinueFeedContent(
    items: List<ContinueItem>,
    onItemClick: (ContinueItem) -> Unit,
    modifier: Modifier = Modifier
) {
    LazyColumn(
        modifier = modifier.fillMaxSize(),
        contentPadding = PaddingValues(vertical = 8.dp)
    ) {
        items(items, key = { it.mediaItem.id }) { continueItem ->
            ContinueCard(
                continueItem = continueItem,
                onPlayClick = onItemClick
            )
        }
    }
}

@Composable
private fun LoadingView(modifier: Modifier = Modifier) {
    Box(
        modifier = modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            CircularProgressIndicator()
            Text(
                text = "Loading your continue feed...",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun EmptyView(modifier: Modifier = Modifier) {
    Box(
        modifier = modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.padding(32.dp)
        ) {
            Text(
                text = "Nothing to continue",
                style = MaterialTheme.typography.headlineSmall,
                textAlign = TextAlign.Center
            )
            Text(
                text = "Start listening to music, audiobooks, or reading ebooks to see them here",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center
            )
        }
    }
}

@Composable
private fun ErrorView(
    message: String,
    onRetry: () -> Unit,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp),
            modifier = Modifier.padding(32.dp)
        ) {
            Text(
                text = "Error loading feed",
                style = MaterialTheme.typography.headlineSmall,
                color = MaterialTheme.colorScheme.error,
                textAlign = TextAlign.Center
            )
            Text(
                text = message,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center
            )
            Button(onClick = onRetry) {
                Text("Retry")
            }
        }
    }
}

private fun handleItemClick(
    continueItem: ContinueItem,
    onNavigateToPlayer: (MediaItem.Music) -> Unit,
    onNavigateToAudiobook: (MediaItem.Audiobook) -> Unit,
    onNavigateToEbook: (MediaItem.Ebook) -> Unit
) {
    when (val mediaItem = continueItem.mediaItem) {
        is MediaItem.Music -> onNavigateToPlayer(mediaItem)
        is MediaItem.Audiobook -> onNavigateToAudiobook(mediaItem)
        is MediaItem.Ebook -> onNavigateToEbook(mediaItem)
    }
}
