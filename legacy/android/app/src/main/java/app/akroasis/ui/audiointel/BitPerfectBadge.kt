// Bit-perfect playback indicator badge
package app.akroasis.ui.audiointel

import androidx.compose.foundation.layout.padding
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

@Composable
fun BitPerfectBadge(
    isBitPerfect: Boolean,
    modifier: Modifier = Modifier
) {
    if (!isBitPerfect) return

    Surface(
        modifier = modifier,
        color = MaterialTheme.colorScheme.primaryContainer,
        shape = MaterialTheme.shapes.small
    ) {
        Text(
            text = "✓ BIT-PERFECT",
            style = MaterialTheme.typography.labelSmall,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
            color = MaterialTheme.colorScheme.onPrimaryContainer,
            fontWeight = FontWeight.Bold
        )
    }
}

fun calculateBitPerfect(
    trackSampleRate: Int,
    dacSampleRate: Int,
    dspActive: Boolean,
    resampling: Boolean
): Boolean {
    return trackSampleRate == dacSampleRate && !dspActive && !resampling
}
