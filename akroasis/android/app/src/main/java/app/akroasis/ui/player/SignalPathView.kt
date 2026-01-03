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
            Text(
                text = "Signal Path",
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceEvenly,
                verticalAlignment = Alignment.CenterVertically
            ) {
                SignalPathStage(
                    icon = { Icon(Icons.Default.AudioFile, null) },
                    label = "Source",
                    detail = sourceFormat ?: "Unknown",
                    modifier = Modifier.weight(1f)
                )

                Icon(
                    imageVector = Icons.Default.ArrowForward,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(16.dp)
                )

                SignalPathStage(
                    icon = { Icon(Icons.Default.Memory, null) },
                    label = "Decode",
                    detail = audioFormat?.let {
                        "${it.sampleRate / 1000}kHz/${it.bitDepth}bit"
                    } ?: "—",
                    modifier = Modifier.weight(1f)
                )

                Icon(
                    imageVector = Icons.Default.ArrowForward,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(16.dp)
                )

                SignalPathStage(
                    icon = {
                        Icon(
                            imageVector = if (equalizerEnabled || playbackSpeed != 1.0f) {
                                Icons.Default.GraphicEq
                            } else {
                                Icons.Default.Speed
                            },
                            contentDescription = null
                        )
                    },
                    label = "Process",
                    detail = buildString {
                        if (equalizerEnabled) append("EQ ")
                        if (playbackSpeed != 1.0f) append("${playbackSpeed}x")
                        if (isEmpty()) append("Bypass")
                    },
                    modifier = Modifier.weight(1f),
                    isActive = equalizerEnabled || playbackSpeed != 1.0f
                )

                Icon(
                    imageVector = Icons.Default.ArrowForward,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(16.dp)
                )

                SignalPathStage(
                    icon = {
                        Icon(
                            imageVector = if (usbDac != null) {
                                Icons.Default.Headset
                            } else {
                                Icons.Default.Output
                            },
                            contentDescription = null
                        )
                    },
                    label = "Output",
                    detail = usbDac?.productName ?: "System",
                    modifier = Modifier.weight(1f),
                    isActive = usbDac != null
                )
            }

            if (audioFormat != null) {
                Divider(modifier = Modifier.padding(vertical = 4.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceAround
                ) {
                    Text(
                        text = "Bit-perfect: ${if (playbackSpeed == 1.0f && !equalizerEnabled) "Yes" else "No"}",
                        style = MaterialTheme.typography.bodySmall,
                        color = if (playbackSpeed == 1.0f && !equalizerEnabled) {
                            MaterialTheme.colorScheme.primary
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        }
                    )
                    Text(
                        text = "Gapless: ${if (gaplessEnabled) "Enabled" else "Disabled"}",
                        style = MaterialTheme.typography.bodySmall,
                        color = if (gaplessEnabled) {
                            MaterialTheme.colorScheme.primary
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        }
                    )
                    Text(
                        text = "${audioFormat.channels}ch",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }
    }
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

        Text(
            text = label,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center
        )

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
}
