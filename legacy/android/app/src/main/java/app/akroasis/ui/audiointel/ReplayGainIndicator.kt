// ReplayGain mode indicator chip
package app.akroasis.ui.audiointel

import androidx.compose.foundation.layout.padding
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import app.akroasis.data.model.ReplayGainInfo

enum class ReplayGainMode {
    OFF,
    TRACK,
    ALBUM
}

@Composable
fun ReplayGainIndicator(
    mode: ReplayGainMode,
    replayGainInfo: ReplayGainInfo?,
    modifier: Modifier = Modifier
) {
    if (mode == ReplayGainMode.OFF || replayGainInfo == null) {
        return
    }

    val gainValue = when (mode) {
        ReplayGainMode.TRACK -> replayGainInfo.trackGain
        ReplayGainMode.ALBUM -> replayGainInfo.albumGain ?: replayGainInfo.trackGain
        ReplayGainMode.OFF -> return
    }

    val peakValue = when (mode) {
        ReplayGainMode.TRACK -> replayGainInfo.trackPeak
        ReplayGainMode.ALBUM -> replayGainInfo.albumPeak ?: replayGainInfo.trackPeak
        ReplayGainMode.OFF -> return
    }

    Surface(
        modifier = modifier,
        color = MaterialTheme.colorScheme.secondaryContainer,
        shape = MaterialTheme.shapes.small
    ) {
        Text(
            text = "RG ${mode.name} ${formatGain(gainValue)}",
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
            color = MaterialTheme.colorScheme.onSecondaryContainer,
            fontWeight = FontWeight.Medium
        )
    }
}

private fun formatGain(gain: Float): String {
    return when {
        gain > 0 -> "+%.1f dB".format(gain)
        else -> "%.1f dB".format(gain)
    }
}
