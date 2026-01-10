// Audio signal path visualization
package app.akroasis.ui.player

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowForward
import androidx.compose.material.icons.filled.AudioFile
import androidx.compose.material.icons.filled.GraphicEq
import androidx.compose.material.icons.filled.Headset
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.Output
import androidx.compose.material.icons.filled.Speed
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

@Composable
fun SignalPathView(
    sourceFormat: String?,
    audioFormat: app.akroasis.audio.AudioFormatInfo?,
    playbackSpeed: Float,
    equalizerEnabled: Boolean,
    gaplessEnabled: Boolean,
    usbDac: app.akroasis.audio.UsbDacInfo?,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            SignalPathHeader()
            SignalPathStages(
                sourceFormat = sourceFormat,
                audioFormat = audioFormat,
                playbackSpeed = playbackSpeed,
                equalizerEnabled = equalizerEnabled,
                usbDac = usbDac
            )
            if (audioFormat != null) {
                SignalPathStatusRow(
                    playbackSpeed = playbackSpeed,
                    equalizerEnabled = equalizerEnabled,
                    gaplessEnabled = gaplessEnabled,
                    channels = audioFormat.channels
                )
            }
        }
    }
}

@Composable
private fun SignalPathHeader() {
    Text(
        text = "Signal Path",
        style = MaterialTheme.typography.titleMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant
    )
}

@Composable
private fun SignalPathStages(
    sourceFormat: String?,
    audioFormat: app.akroasis.audio.AudioFormatInfo?,
    playbackSpeed: Float,
    equalizerEnabled: Boolean,
    usbDac: app.akroasis.audio.UsbDacInfo?
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceEvenly,
        verticalAlignment = Alignment.CenterVertically
    ) {
        SourceStage(sourceFormat, Modifier.weight(1f))
        StageArrow()
        DecodeStage(audioFormat, Modifier.weight(1f))
        StageArrow()
        ProcessStage(playbackSpeed, equalizerEnabled, Modifier.weight(1f))
        StageArrow()
        OutputStage(usbDac, Modifier.weight(1f))
    }
}

@Composable
private fun SourceStage(sourceFormat: String?, modifier: Modifier) {
    SignalPathStage(
        icon = { Icon(Icons.Default.AudioFile, null) },
        label = "Source",
        detail = sourceFormat ?: "Unknown",
        modifier = modifier
    )
}

@Composable
private fun DecodeStage(audioFormat: app.akroasis.audio.AudioFormatInfo?, modifier: Modifier) {
    SignalPathStage(
        icon = { Icon(Icons.Default.Memory, null) },
        label = "Decode",
        detail = audioFormat?.let { "${it.sampleRate / 1000}kHz/${it.bitDepth}bit" } ?: "—",
        modifier = modifier
    )
}

@Composable
private fun ProcessStage(playbackSpeed: Float, equalizerEnabled: Boolean, modifier: Modifier) {
    val isProcessing = equalizerEnabled || playbackSpeed != 1.0f
    SignalPathStage(
        icon = {
            Icon(
                imageVector = if (isProcessing) Icons.Default.GraphicEq else Icons.Default.Speed,
                contentDescription = null
            )
        },
        label = "Process",
        detail = buildProcessDetail(playbackSpeed, equalizerEnabled),
        modifier = modifier,
        isActive = isProcessing
    )
}

private fun buildProcessDetail(playbackSpeed: Float, equalizerEnabled: Boolean): String {
    return buildString {
        if (equalizerEnabled) append("EQ ")
        if (playbackSpeed != 1.0f) append("${playbackSpeed}x")
        if (isEmpty()) append("Bypass")
    }
}

@Composable
private fun OutputStage(usbDac: app.akroasis.audio.UsbDacInfo?, modifier: Modifier) {
    SignalPathStage(
        icon = {
            Icon(
                imageVector = if (usbDac != null) Icons.Default.Headset else Icons.Default.Output,
                contentDescription = null
            )
        },
        label = "Output",
        detail = usbDac?.productName ?: "System",
        modifier = modifier,
        isActive = usbDac != null
    )
}

@Composable
private fun StageArrow() {
    Icon(
        imageVector = Icons.Default.ArrowForward,
        contentDescription = null,
        tint = MaterialTheme.colorScheme.primary,
        modifier = Modifier.size(16.dp)
    )
}

@Composable
private fun SignalPathStatusRow(
    playbackSpeed: Float,
    equalizerEnabled: Boolean,
    gaplessEnabled: Boolean,
    channels: Int
) {
    val isBitPerfect = playbackSpeed == 1.0f && !equalizerEnabled

    HorizontalDivider(modifier = Modifier.padding(vertical = 4.dp))
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceAround
    ) {
        StatusText(
            text = "Bit-perfect: ${if (isBitPerfect) "Yes" else "No"}",
            isHighlighted = isBitPerfect
        )
        StatusText(
            text = "Gapless: ${if (gaplessEnabled) "Enabled" else "Disabled"}",
            isHighlighted = gaplessEnabled
        )
        StatusText(
            text = "${channels}ch",
            isHighlighted = false
        )
    }
}

@Composable
private fun StatusText(text: String, isHighlighted: Boolean) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodySmall,
        color = if (isHighlighted) {
            MaterialTheme.colorScheme.primary
        } else {
            MaterialTheme.colorScheme.onSurfaceVariant
        }
    )
}

@Composable
fun SignalPathStage(
    icon: @Composable () -> Unit,
    label: String,
    detail: String,
    modifier: Modifier = Modifier,
    isActive: Boolean = true
) {
    Column(
        modifier = modifier,
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp)
    ) {
        StageIconBox(icon = icon, isActive = isActive)
        StageLabel(label = label)
        StageDetail(detail = detail, isActive = isActive)
    }
}

@Composable
private fun StageIconBox(icon: @Composable () -> Unit, isActive: Boolean) {
    Box(
        modifier = Modifier
            .size(40.dp)
            .background(
                color = if (isActive) {
                    MaterialTheme.colorScheme.primaryContainer
                } else {
                    MaterialTheme.colorScheme.surface
                },
                shape = MaterialTheme.shapes.small
            ),
        contentAlignment = Alignment.Center
    ) {
        CompositionLocalProvider(
            LocalContentColor provides if (isActive) {
                MaterialTheme.colorScheme.onPrimaryContainer
            } else {
                MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f)
            }
        ) {
            icon()
        }
    }
}

@Composable
private fun StageLabel(label: String) {
    Text(
        text = label,
        style = MaterialTheme.typography.labelSmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        textAlign = TextAlign.Center
    )
}

@Composable
private fun StageDetail(detail: String, isActive: Boolean) {
    Text(
        text = detail,
        style = MaterialTheme.typography.bodySmall,
        color = if (isActive) {
            MaterialTheme.colorScheme.primary
        } else {
            MaterialTheme.colorScheme.onSurfaceVariant
        },
        textAlign = TextAlign.Center,
        maxLines = 2
    )
}
