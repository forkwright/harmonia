// Search results display with audio quality badges
package app.akroasis.ui.search

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.akroasis.data.model.SearchResult
import app.akroasis.ui.components.AudioQualityBadges
import app.akroasis.ui.components.BadgeLayout

@Composable
fun SearchResultsScreen(
    results: List<SearchResult>,
    onTrackClick: (SearchResult) -> Unit,
    bitPerfectCalculator: app.akroasis.audio.BitPerfectCalculator,
    modifier: Modifier = Modifier
) {
    if (results.isEmpty()) {
        Box(
            modifier = modifier.fillMaxSize(),
            contentAlignment = Alignment.Center
        ) {
            Text(
                text = "No results found",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    } else {
        LazyColumn(
            modifier = modifier.fillMaxSize(),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            item {
                Text(
                    text = "${results.size} results",
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(bottom = 8.dp)
                )
            }

            items(results) { result ->
                SearchResultItem(
                    result = result,
                    bitPerfectCalculator = bitPerfectCalculator,
                    onClick = { onTrackClick(result) }
                )
            }
        }
    }
}

@Composable
private fun SearchResultItem(
    result: SearchResult,
    bitPerfectCalculator: app.akroasis.audio.BitPerfectCalculator,
    onClick: () -> Unit
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Column(
            modifier = Modifier.padding(12.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            // Track title
            Text(
                text = result.title,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )

            // Artist + Album
            Row(
                horizontalArrangement = Arrangement.spacedBy(4.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                result.artist?.let { artist ->
                    Text(
                        text = artist,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f, fill = false)
                    )
                }

                result.album?.let { album ->
                    if (result.artist != null) {
                        Text(
                            text = "•",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                    Text(
                        text = album,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f, fill = false)
                    )
                }
            }

            // Audio quality badges
            AudioQualityBadges(
                format = formatOrDefault(result),
                bitDepth = result.bitDepth,
                sampleRate = sampleRateOrDefault(result),
                dynamicRange = result.dynamicRange,
                lossless = result.lossless,
                bitPerfectCapable = bitPerfectCalculator.isBitPerfect(
                    sampleRate = sampleRateOrDefault(result),
                    bitDepth = result.bitDepth,
                    format = formatOrDefault(result)
                ),
                layout = BadgeLayout.COMPACT
            )

            // Duration + Track/Disc number
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                result.durationSeconds?.let { duration ->
                    Text(
                        text = formatDuration(duration),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                if (result.discNumber > 1 || result.trackNumber > 0) {
                    Text(
                        text = buildString {
                            if (result.discNumber > 1) {
                                append("Disc ${result.discNumber}")
                            }
                            if (result.trackNumber > 0) {
                                if (result.discNumber > 1) append(" • ")
                                append("Track ${result.trackNumber}")
                            }
                        },
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }
    }
}

private fun formatDuration(seconds: Int): String {
    val mins = seconds / 60
    val secs = seconds % 60
    return "$mins:${secs.toString().padStart(2, '0')}"
}

private fun formatOrDefault(result: SearchResult): String {
    return result.format ?: "Unknown"
}

private fun sampleRateOrDefault(result: SearchResult): Int {
    return result.sampleRate ?: 44100  // Fallback for legacy data
}
