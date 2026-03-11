// Continue feed card component for media items
package app.akroasis.ui.continuefeed

import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Book
import androidx.compose.material.icons.filled.Headphones
import androidx.compose.material.icons.filled.MusicNote
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.akroasis.data.model.MediaItem
import app.akroasis.data.model.MediaProgress
import app.akroasis.data.repository.ContinueItem
import coil.compose.AsyncImage

@Composable
fun ContinueCard(
    continueItem: ContinueItem,
    onPlayClick: (ContinueItem) -> Unit,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp),
        onClick = { onPlayClick(continueItem) }
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            // Cover art
            CoverArt(
                coverArtUrl = continueItem.mediaItem.coverArtUrl,
                modifier = Modifier.size(80.dp)
            )

            // Metadata column
            Column(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxHeight(),
                verticalArrangement = Arrangement.SpaceBetween
            ) {
                // Title and metadata
                Column {
                    Text(
                        text = continueItem.mediaItem.title,
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.SemiBold,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis
                    )

                    Spacer(modifier = Modifier.height(4.dp))

                    Row(
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        MediaTypeBadge(mediaItem = continueItem.mediaItem)

                        Text(
                            text = getMediaCreator(continueItem.mediaItem),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis
                        )
                    }
                }

                // Progress section
                Column {
                    LinearProgressIndicator(
                        progress = { continueItem.progress.percentComplete },
                        modifier = Modifier.fillMaxWidth(),
                    )

                    Spacer(modifier = Modifier.height(4.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = formatProgress(continueItem.progress),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )

                        if (continueItem.progress.isComplete) {
                            AssistChip(
                                onClick = { /* Mark as unfinished */ },
                                label = { Text("Completed") },
                                colors = AssistChipDefaults.assistChipColors(
                                    containerColor = MaterialTheme.colorScheme.primaryContainer
                                )
                            )
                        }
                    }
                }
            }

            // Play button
            IconButton(
                onClick = { onPlayClick(continueItem) },
                modifier = Modifier.align(Alignment.CenterVertically)
            ) {
                Icon(
                    imageVector = Icons.Default.PlayArrow,
                    contentDescription = "Resume ${continueItem.mediaItem.title}",
                    tint = MaterialTheme.colorScheme.primary
                )
            }
        }
    }
}

@Composable
private fun CoverArt(
    coverArtUrl: String?,
    modifier: Modifier = Modifier
) {
    AsyncImage(
        model = coverArtUrl,
        contentDescription = "Cover art",
        modifier = modifier.clip(MaterialTheme.shapes.small),
        contentScale = ContentScale.Crop
    )
}

@Composable
private fun MediaTypeBadge(mediaItem: MediaItem) {
    val (icon, label, color) = when (mediaItem) {
        is MediaItem.Music -> Triple(
            Icons.Default.MusicNote,
            "Music",
            MaterialTheme.colorScheme.tertiary
        )
        is MediaItem.Audiobook -> Triple(
            Icons.Default.Headphones,
            "Audiobook",
            MaterialTheme.colorScheme.secondary
        )
        is MediaItem.Ebook -> Triple(
            Icons.Default.Book,
            "Ebook",
            MaterialTheme.colorScheme.primary
        )
    }

    AssistChip(
        onClick = { /* Filter by type */ },
        label = { Text(label, style = MaterialTheme.typography.labelSmall) },
        leadingIcon = {
            Icon(
                imageVector = icon,
                contentDescription = null,
                modifier = Modifier.size(16.dp)
            )
        },
        colors = AssistChipDefaults.assistChipColors(
            containerColor = color.copy(alpha = 0.1f),
            labelColor = color,
            leadingIconContentColor = color
        )
    )
}

private fun getMediaCreator(mediaItem: MediaItem): String {
    return when (mediaItem) {
        is MediaItem.Music -> mediaItem.artist
        is MediaItem.Audiobook -> mediaItem.author
        is MediaItem.Ebook -> mediaItem.author
    }
}

private fun formatProgress(progress: MediaProgress): String {
    val percent = (progress.percentComplete * 100).toInt()
    val timeAgo = formatTimeAgo(System.currentTimeMillis() - progress.lastPlayedAt)
    return "$percent% · $timeAgo"
}

private fun formatTimeAgo(millisAgo: Long): String {
    val seconds = millisAgo / 1000
    val minutes = seconds / 60
    val hours = minutes / 60
    val days = hours / 24

    return when {
        days > 0 -> "${days}d ago"
        hours > 0 -> "${hours}h ago"
        minutes > 0 -> "${minutes}m ago"
        else -> "Just now"
    }
}
