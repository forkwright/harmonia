// Signal path visualization card
package app.akroasis.ui.player

import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import app.akroasis.audio.AudioPath
import app.akroasis.audio.AudioPipelineState

@Composable
fun SignalPathCard(
    pipelineState: AudioPipelineState?,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = "Signal Path",
                style = MaterialTheme.typography.titleMedium,
                fontFamily = FontFamily.Serif
            )
            Spacer(modifier = Modifier.height(12.dp))

            if (pipelineState == null) {
                Text(
                    "No active playback",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            } else {
                SignalPathFlow(pipelineState)
            }
        }
    }
}

@Composable
private fun SignalPathFlow(state: AudioPipelineState) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        SignalPathNode(
            "Input",
            "${state.inputFormat.sampleRate/1000}kHz/${state.inputFormat.bitDepth}bit"
        )
        SignalPathArrow()

        if (state.dspChain.isNotEmpty()) {
            state.dspChain.forEach { dsp ->
                SignalPathNode(dsp.label(), dsp.value())
                SignalPathArrow()
            }
        }

        SignalPathNode(
            "Output",
            state.audioPath.describe(),
            highlight = state.audioPath is AudioPath.BitPerfect || state.audioPath is AudioPath.UsbDac
        )

        if (state.gaplessActive) {
            Spacer(modifier = Modifier.height(4.dp))
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(4.dp)
            ) {
                Text(
                    "✓",
                    style = MaterialTheme.typography.bodySmall,
                    color = Color(0xFF4CAF50)
                )
                Text(
                    "Gapless Active",
                    style = MaterialTheme.typography.bodySmall,
                    color = Color(0xFF4CAF50)
                )
            }
        }
    }
}

@Composable
private fun SignalPathNode(label: String, value: String, highlight: Boolean = false) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            label,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Text(
            value,
            style = MaterialTheme.typography.bodySmall,
            fontFamily = FontFamily.Monospace,
            color = if (highlight) Color(0xFF9966CC) else MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun SignalPathArrow() {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.Center
    ) {
        Icon(
            imageVector = Icons.Default.ArrowDownward,
            contentDescription = null,
            modifier = Modifier.size(16.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f)
        )
    }
}
