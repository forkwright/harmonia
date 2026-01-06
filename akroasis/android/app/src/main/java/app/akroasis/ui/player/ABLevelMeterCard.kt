// A/B level meter for scientific audio comparison
package app.akroasis.ui.player

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlin.math.abs

@Composable
fun ABLevelMeterCard(
    levelA: Float,
    levelB: Float,
    gainCompensation: Float,
    matchingEnabled: Boolean,
    onToggleMatching: () -> Unit,
    onManualGainChange: (Float) -> Unit,
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
                text = "A/B Level Matching",
                style = MaterialTheme.typography.titleMedium,
                fontFamily = FontFamily.Serif
            )
            Spacer(modifier = Modifier.height(12.dp))

            // Level meters
            LevelMeter(label = "A", levelDb = levelA)
            Spacer(modifier = Modifier.height(8.dp))
            LevelMeter(label = "B", levelDb = levelB)

            Spacer(modifier = Modifier.height(16.dp))

            // Gain compensation display
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    "Gain Compensation",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Text(
                    "${if (gainCompensation >= 0) "+" else ""}${String.format("%.1f", gainCompensation)} dB",
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    color = when {
                        abs(gainCompensation) < 0.5f -> Color(0xFF4CAF50)
                        abs(gainCompensation) < 3.0f -> Color(0xFFFFA726)
                        else -> Color(0xFFEF5350)
                    }
                )
            }

            Spacer(modifier = Modifier.height(12.dp))

            // Auto matching toggle
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    "Automatic Matching",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Switch(
                    checked = matchingEnabled,
                    onCheckedChange = { onToggleMatching() }
                )
            }

            // Manual gain slider (only when auto is disabled)
            if (!matchingEnabled) {
                Spacer(modifier = Modifier.height(12.dp))
                Text(
                    "Manual Gain",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Slider(
                    value = gainCompensation,
                    onValueChange = onManualGainChange,
                    valueRange = -12f..12f,
                    steps = 47,
                    modifier = Modifier.fillMaxWidth()
                )
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text(
                        "-12 dB",
                        style = MaterialTheme.typography.bodySmall,
                        fontSize = 10.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f)
                    )
                    Text(
                        "+12 dB",
                        style = MaterialTheme.typography.bodySmall,
                        fontSize = 10.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f)
                    )
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            // Helper text
            Text(
                "Level matching eliminates 'louder sounds better' bias for scientific A/B comparison",
                style = MaterialTheme.typography.bodySmall,
                fontSize = 11.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f)
            )
        }
    }
}

@Composable
private fun LevelMeter(label: String, levelDb: Float) {
    Column {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                label,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Text(
                "${String.format("%.1f", levelDb)} dB",
                style = MaterialTheme.typography.bodySmall,
                fontFamily = FontFamily.Monospace,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }

        Spacer(modifier = Modifier.height(4.dp))

        // Level bar
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(12.dp)
                .background(
                    MaterialTheme.colorScheme.surfaceVariant,
                    RoundedCornerShape(6.dp)
                )
        ) {
            val normalizedLevel = ((levelDb + 96f) / 96f).coerceIn(0f, 1f)
            Box(
                modifier = Modifier
                    .fillMaxHeight()
                    .fillMaxWidth(normalizedLevel)
                    .background(
                        getLevelColor(levelDb),
                        RoundedCornerShape(6.dp)
                    )
            )
        }
    }
}

@Composable
private fun getLevelColor(levelDb: Float): Color {
    return when {
        levelDb < -18f -> Color(0xFF4CAF50)
        levelDb < -6f -> Color(0xFF8BC34A)
        levelDb < -3f -> Color(0xFFFFA726)
        else -> Color(0xFFEF5350)
    }
}
