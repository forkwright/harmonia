// Audio quality badge components for search/filter results
package app.akroasis.ui.components

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

enum class BadgeLayout {
    COMPACT,  // Single row of badges
    EXPANDED  // Stacked badges with labels
}

@Composable
fun AudioQualityBadges(
    format: String,
    bitDepth: Int?,
    sampleRate: Int,
    dynamicRange: Int?,
    lossless: Boolean,
    bitPerfectCapable: Boolean = false,
    modifier: Modifier = Modifier,
    layout: BadgeLayout = BadgeLayout.COMPACT
) {
    when (layout) {
        BadgeLayout.COMPACT -> CompactBadges(
            format = format,
            bitDepth = bitDepth,
            sampleRate = sampleRate,
            dynamicRange = dynamicRange,
            lossless = lossless,
            bitPerfectCapable = bitPerfectCapable,
            modifier = modifier
        )
        BadgeLayout.EXPANDED -> ExpandedBadges(
            format = format,
            bitDepth = bitDepth,
            sampleRate = sampleRate,
            dynamicRange = dynamicRange,
            lossless = lossless,
            bitPerfectCapable = bitPerfectCapable,
            modifier = modifier
        )
    }
}

@Composable
private fun CompactBadges(
    format: String,
    bitDepth: Int?,
    sampleRate: Int,
    dynamicRange: Int?,
    lossless: Boolean,
    bitPerfectCapable: Boolean,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier,
        horizontalArrangement = Arrangement.spacedBy(4.dp)
    ) {
        // Format badge (always show)
        FormatBadge(format = format, lossless = lossless)

        // Hi-Res badge (only if >16/44.1)
        if (isHiRes(bitDepth, sampleRate)) {
            HiResBadge(bitDepth = bitDepth, sampleRate = sampleRate)
        }

        // DR badge (if available)
        dynamicRange?.let { dr ->
            DynamicRangeBadge(dr = dr)
        }

        // Bit-Perfect badge (if capable)
        if (bitPerfectCapable) {
            BitPerfectBadge()
        }
    }
}

@Composable
private fun ExpandedBadges(
    format: String,
    bitDepth: Int?,
    sampleRate: Int,
    dynamicRange: Int?,
    lossless: Boolean,
    bitPerfectCapable: Boolean,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(4.dp)
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            FormatBadge(format = format, lossless = lossless)
            if (isHiRes(bitDepth, sampleRate)) {
                HiResBadge(bitDepth = bitDepth, sampleRate = sampleRate)
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            dynamicRange?.let { dr ->
                DynamicRangeBadge(dr = dr)
            }
            if (bitPerfectCapable) {
                BitPerfectBadge()
            }
        }
    }
}

@Composable
private fun FormatBadge(format: String, lossless: Boolean) {
    Surface(
        color = if (lossless) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceVariant,
        shape = MaterialTheme.shapes.small
    ) {
        Text(
            text = format.uppercase(),
            style = MaterialTheme.typography.labelSmall,
            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
            color = if (lossless) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurfaceVariant,
            fontWeight = if (lossless) FontWeight.Bold else FontWeight.Normal
        )
    }
}

@Composable
private fun HiResBadge(bitDepth: Int?, sampleRate: Int) {
    val text = buildString {
        bitDepth?.let { append("${it}/") }
        append("${sampleRate / 1000}")
    }

    Surface(
        color = MaterialTheme.colorScheme.secondaryContainer,
        shape = MaterialTheme.shapes.small
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelSmall,
            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
            color = MaterialTheme.colorScheme.onSecondaryContainer,
            fontWeight = FontWeight.Medium
        )
    }
}

@Composable
private fun DynamicRangeBadge(dr: Int) {
    val color = getDynamicRangeColor(dr)
    val label = getDynamicRangeLabel(dr)

    Surface(
        color = color.copy(alpha = 0.15f),
        shape = MaterialTheme.shapes.small
    ) {
        Text(
            text = "DR$dr",
            style = MaterialTheme.typography.labelSmall,
            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
            color = color,
            fontWeight = FontWeight.Bold
        )
    }
}

@Composable
private fun BitPerfectBadge() {
    Surface(
        color = MaterialTheme.colorScheme.tertiaryContainer,
        shape = MaterialTheme.shapes.small
    ) {
        Text(
            text = "✓BP",
            style = MaterialTheme.typography.labelSmall,
            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
            color = MaterialTheme.colorScheme.onTertiaryContainer,
            fontWeight = FontWeight.Bold
        )
    }
}

@Composable
private fun getDynamicRangeColor(dr: Int): Color {
    return when {
        dr >= 14 -> Color(0xFF4CAF50)  // Green - Excellent
        dr >= 10 -> Color(0xFFFFA726)  // Orange - Good
        dr >= 7 -> Color(0xFFFF9800)   // Deep Orange - Fair
        else -> MaterialTheme.colorScheme.error  // Red - Compressed
    }
}

private fun getDynamicRangeLabel(dr: Int): String {
    return when {
        dr >= 14 -> "Excellent"
        dr >= 10 -> "Good"
        dr >= 7 -> "Fair"
        else -> "Compressed"
    }
}

private fun isHiRes(bitDepth: Int?, sampleRate: Int): Boolean {
    return (bitDepth != null && bitDepth > 16) || sampleRate > 44100
}
