// Gapless verification screen with album scanner
package app.akroasis.ui.gapless

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Error
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GaplessVerificationScreen(
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
    viewModel: GaplessVerificationViewModel = hiltViewModel()
) {
    val scanState by viewModel.scanState.collectAsState()
    val gapMeasurements by viewModel.gapMeasurements.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Gapless Verification") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.ArrowBack, "Back")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp)
        ) {
            when (val state = scanState) {
                is GaplessVerificationViewModel.ScanState.Idle -> {
                    IdleView(measurementCount = gapMeasurements.size)
                }
                is GaplessVerificationViewModel.ScanState.Scanning -> {
                    ScanningView(state.currentTrack, state.totalTracks)
                }
                is GaplessVerificationViewModel.ScanState.Complete -> {
                    GaplessReportView(state.report)
                }
                is GaplessVerificationViewModel.ScanState.Error -> {
                    ErrorView(state.message, onReset = { viewModel.reset() })
                }
            }
        }
    }
}

@Composable
private fun IdleView(measurementCount: Int) {
    Column(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            text = "Gapless Measurements",
            style = MaterialTheme.typography.headlineMedium
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = if (measurementCount > 0) {
                "$measurementCount transitions recorded"
            } else {
                "Play an album with gapless enabled to record measurements"
            },
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun ScanningView(current: Int, total: Int) {
    Column(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        CircularProgressIndicator()
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = "Scanning track $current of $total",
            style = MaterialTheme.typography.bodyLarge
        )
    }
}

@Composable
private fun ErrorView(message: String, onReset: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Icon(
            imageVector = Icons.Default.Error,
            contentDescription = null,
            modifier = Modifier.size(48.dp),
            tint = MaterialTheme.colorScheme.error
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = message,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.error
        )
        Spacer(modifier = Modifier.height(16.dp))
        Button(onClick = onReset) {
            Text("Reset")
        }
    }
}

@Composable
private fun GaplessReportView(report: GaplessReport) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        item {
            SummaryCard(report)
        }
        items(report.trackPairs) { pair ->
            TrackPairRow(pair)
        }
    }
}

@Composable
private fun SummaryCard(report: GaplessReport) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = if (report.passesThreshold) {
                Color(0xFF4CAF50).copy(alpha = 0.1f)
            } else {
                Color(0xFFEF5350).copy(alpha = 0.1f)
            }
        )
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Icon(
                    imageVector = if (report.passesThreshold) Icons.Default.CheckCircle else Icons.Default.Error,
                    contentDescription = null,
                    tint = if (report.passesThreshold) Color(0xFF4CAF50) else Color(0xFFEF5350)
                )
                Text(
                    text = report.albumTitle,
                    style = MaterialTheme.typography.titleLarge
                )
            }

            Spacer(modifier = Modifier.height(12.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceEvenly
            ) {
                MetricChip("Avg", "${String.format("%.2f", report.averageGap)}ms")
                MetricChip("Max", "${String.format("%.2f", report.maxGap)}ms")
                MetricChip("Tracks", report.trackPairs.size.toString())
            }

            Spacer(modifier = Modifier.height(8.dp))

            Text(
                text = if (report.passesThreshold) {
                    "✓ Passes (<50ms threshold)"
                } else {
                    "✗ Exceeds threshold (≥50ms)"
                },
                style = MaterialTheme.typography.bodyMedium,
                color = if (report.passesThreshold) Color(0xFF4CAF50) else Color(0xFFEF5350),
                fontWeight = FontWeight.SemiBold
            )
        }
    }
}

@Composable
private fun MetricChip(label: String, value: String) {
    Card(
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Column(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                text = label,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Text(
                text = value,
                style = MaterialTheme.typography.bodyLarge,
                fontWeight = FontWeight.SemiBold
            )
        }
    }
}

@Composable
private fun TrackPairRow(pair: TrackPairResult) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = "${pair.fromTrack} → ${pair.toTrack}",
                    style = MaterialTheme.typography.bodyMedium,
                    maxLines = 2
                )
            }
            Text(
                text = "${String.format("%.2f", pair.gapMs)}ms",
                style = MaterialTheme.typography.bodyLarge,
                fontFamily = FontFamily.Monospace,
                fontWeight = FontWeight.SemiBold,
                color = if (pair.gapMs < 50f) Color(0xFF4CAF50) else Color(0xFFEF5350)
            )
        }
    }
}
