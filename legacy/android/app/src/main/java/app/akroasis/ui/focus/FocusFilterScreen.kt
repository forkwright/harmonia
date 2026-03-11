// Roon-style Focus filtering screen for advanced library queries
package app.akroasis.ui.focus

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import app.akroasis.data.model.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FocusFilterScreen(
    onNavigateBack: () -> Unit,
    onApplyFilter: (FilterRequest) -> Unit
) {
    var filterRules by remember { mutableStateOf(listOf<FilterRule>()) }
    var filterLogic by remember { mutableStateOf(FilterLogic.AND) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Focus Filter") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.Close, contentDescription = "Close")
                    }
                },
                actions = {
                    TextButton(
                        onClick = {
                            onApplyFilter(
                                FilterRequest(
                                    conditions = filterRules,
                                    logic = filterLogic
                                )
                            )
                        },
                        enabled = filterRules.isNotEmpty()
                    ) {
                        Text("APPLY")
                    }
                }
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = {
                    filterRules = filterRules + FilterRule(
                        field = FilterField.FORMAT,
                        operator = FilterOperator.EQUALS,
                        value = "FLAC"
                    )
                }
            ) {
                Icon(Icons.Default.Add, contentDescription = "Add rule")
            }
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Logic selector (AND/OR)
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "Match",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold
                )
                FilterChip(
                    selected = filterLogic == FilterLogic.AND,
                    onClick = { filterLogic = FilterLogic.AND },
                    label = { Text("ALL") }
                )
                FilterChip(
                    selected = filterLogic == FilterLogic.OR,
                    onClick = { filterLogic = FilterLogic.OR },
                    label = { Text("ANY") }
                )
                Text(
                    text = "conditions",
                    style = MaterialTheme.typography.bodyMedium
                )
            }

            Divider()

            // Filter rules list
            if (filterRules.isEmpty()) {
                Box(
                    modifier = Modifier.fillMaxWidth(),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = "Tap + to add a filter rule",
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            } else {
                LazyColumn(
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    items(filterRules) { rule ->
                        FilterRuleCard(
                            rule = rule,
                            onRemove = {
                                filterRules = filterRules.filter { it != rule }
                            },
                            onUpdate = { updated ->
                                filterRules = filterRules.map {
                                    if (it == rule) updated else it
                                }
                            }
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun FilterRuleCard(
    rule: FilterRule,
    onRemove: () -> Unit,
    onUpdate: (FilterRule) -> Unit
) {
    var expanded by remember { mutableStateOf(false) }

    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Column(
            modifier = Modifier.padding(12.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                if (!expanded) {
                    Text(
                        text = formatFilterRule(rule),
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.weight(1f)
                    )
                } else {
                    Text(
                        text = "Edit Filter Rule",
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.SemiBold,
                        modifier = Modifier.weight(1f)
                    )
                }
                IconButton(onClick = onRemove) {
                    Icon(
                        Icons.Default.Close,
                        contentDescription = "Remove",
                        tint = MaterialTheme.colorScheme.error
                    )
                }
            }

            if (expanded) {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    // Field dropdown
                    FieldDropdown(
                        selectedField = rule.field,
                        onFieldSelected = { field ->
                            onUpdate(rule.copy(field = field, operator = getDefaultOperator(field)))
                        }
                    )

                    // Operator dropdown
                    OperatorDropdown(
                        selectedOperator = rule.operator,
                        availableOperators = getValidOperators(rule.field),
                        onOperatorSelected = { operator ->
                            onUpdate(rule.copy(operator = operator))
                        }
                    )

                    // Value input
                    ValueInput(
                        field = rule.field,
                        operator = rule.operator,
                        value = rule.value,
                        onValueChanged = { value ->
                            onUpdate(rule.copy(value = value))
                        }
                    )

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.End
                    ) {
                        TextButton(onClick = { expanded = false }) {
                            Text("DONE")
                        }
                    }
                }
            } else {
                TextButton(
                    onClick = { expanded = true },
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text("Tap to edit")
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun FieldDropdown(
    selectedField: FilterField,
    onFieldSelected: (FilterField) -> Unit
) {
    var expanded by remember { mutableStateOf(false) }

    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = it }
    ) {
        OutlinedTextField(
            value = formatFieldName(selectedField),
            onValueChange = {},
            readOnly = true,
            label = { Text("Field") },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            modifier = Modifier
                .menuAnchor()
                .fillMaxWidth()
        )
        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false }
        ) {
            FilterField.values().forEach { field ->
                DropdownMenuItem(
                    text = { Text(formatFieldName(field)) },
                    onClick = {
                        onFieldSelected(field)
                        expanded = false
                    }
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun OperatorDropdown(
    selectedOperator: FilterOperator,
    availableOperators: List<FilterOperator>,
    onOperatorSelected: (FilterOperator) -> Unit
) {
    var expanded by remember { mutableStateOf(false) }

    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = it }
    ) {
        OutlinedTextField(
            value = formatOperatorName(selectedOperator),
            onValueChange = {},
            readOnly = true,
            label = { Text("Operator") },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            modifier = Modifier
                .menuAnchor()
                .fillMaxWidth()
        )
        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false }
        ) {
            availableOperators.forEach { operator ->
                DropdownMenuItem(
                    text = { Text(formatOperatorName(operator)) },
                    onClick = {
                        onOperatorSelected(operator)
                        expanded = false
                    }
                )
            }
        }
    }
}

@Composable
private fun ValueInput(
    field: FilterField,
    operator: FilterOperator,
    value: Any,
    onValueChanged: (Any) -> Unit
) {
    when (field) {
        FilterField.SAMPLE_RATE -> {
            OutlinedTextField(
                value = value.toString(),
                onValueChange = { newValue ->
                    newValue.toIntOrNull()?.let { onValueChanged(it) }
                },
                label = { Text("Sample Rate (Hz)") },
                placeholder = { Text("e.g. 44100, 96000") },
                modifier = Modifier.fillMaxWidth()
            )
        }
        FilterField.BIT_DEPTH -> {
            OutlinedTextField(
                value = value.toString(),
                onValueChange = { newValue ->
                    newValue.toIntOrNull()?.let { onValueChanged(it) }
                },
                label = { Text("Bit Depth") },
                placeholder = { Text("e.g. 16, 24, 32") },
                modifier = Modifier.fillMaxWidth()
            )
        }
        FilterField.BITRATE -> {
            OutlinedTextField(
                value = value.toString(),
                onValueChange = { newValue ->
                    newValue.toIntOrNull()?.let { onValueChanged(it) }
                },
                label = { Text("Bitrate (kbps)") },
                placeholder = { Text("e.g. 320, 256") },
                modifier = Modifier.fillMaxWidth()
            )
        }
        FilterField.DYNAMIC_RANGE -> {
            OutlinedTextField(
                value = value.toString(),
                onValueChange = { newValue ->
                    newValue.toIntOrNull()?.let { onValueChanged(it) }
                },
                label = { Text("Dynamic Range (dB)") },
                placeholder = { Text("e.g. 12, 14") },
                modifier = Modifier.fillMaxWidth()
            )
        }
        FilterField.YEAR -> {
            OutlinedTextField(
                value = value.toString(),
                onValueChange = { newValue ->
                    newValue.toIntOrNull()?.let { onValueChanged(it) }
                },
                label = { Text("Year") },
                placeholder = { Text("e.g. 2020") },
                modifier = Modifier.fillMaxWidth()
            )
        }
        FilterField.LOSSLESS -> {
            var boolValue by remember { mutableStateOf(value as? Boolean ?: true) }
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text("Lossless", modifier = Modifier.weight(1f))
                Switch(
                    checked = boolValue,
                    onCheckedChange = {
                        boolValue = it
                        onValueChanged(it)
                    }
                )
            }
        }
        else -> {
            // Text fields for format, codec, artist, album, genre
            OutlinedTextField(
                value = value.toString(),
                onValueChange = { onValueChanged(it) },
                label = { Text("Value") },
                placeholder = { Text("e.g. FLAC, MP3, Rock") },
                modifier = Modifier.fillMaxWidth()
            )
        }
    }
}

private fun formatFieldName(field: FilterField): String {
    return when (field) {
        FilterField.FORMAT -> "Format"
        FilterField.SAMPLE_RATE -> "Sample Rate"
        FilterField.BIT_DEPTH -> "Bit Depth"
        FilterField.CODEC -> "Codec"
        FilterField.BITRATE -> "Bitrate"
        FilterField.DYNAMIC_RANGE -> "Dynamic Range"
        FilterField.LOSSLESS -> "Lossless"
        FilterField.ARTIST -> "Artist"
        FilterField.ALBUM -> "Album"
        FilterField.GENRE -> "Genre"
        FilterField.YEAR -> "Year"
    }
}

private fun formatOperatorName(operator: FilterOperator): String {
    return when (operator) {
        FilterOperator.EQUALS -> "is"
        FilterOperator.NOT_EQUALS -> "is not"
        FilterOperator.GREATER_THAN -> "greater than"
        FilterOperator.LESS_THAN -> "less than"
        FilterOperator.GREATER_THAN_OR_EQUAL -> "greater than or equal to"
        FilterOperator.LESS_THAN_OR_EQUAL -> "less than or equal to"
        FilterOperator.CONTAINS -> "contains"
        FilterOperator.NOT_CONTAINS -> "does not contain"
        FilterOperator.IN -> "in list"
        FilterOperator.NOT_IN -> "not in list"
    }
}

private fun getDefaultOperator(field: FilterField): FilterOperator {
    return when (field) {
        FilterField.SAMPLE_RATE, FilterField.BIT_DEPTH, FilterField.BITRATE,
        FilterField.DYNAMIC_RANGE, FilterField.YEAR -> FilterOperator.GREATER_THAN_OR_EQUAL
        FilterField.LOSSLESS -> FilterOperator.EQUALS
        else -> FilterOperator.EQUALS
    }
}

private fun getValidOperators(field: FilterField): List<FilterOperator> {
    return when (field) {
        FilterField.SAMPLE_RATE, FilterField.BIT_DEPTH, FilterField.BITRATE,
        FilterField.DYNAMIC_RANGE, FilterField.YEAR -> listOf(
            FilterOperator.EQUALS,
            FilterOperator.NOT_EQUALS,
            FilterOperator.GREATER_THAN,
            FilterOperator.LESS_THAN,
            FilterOperator.GREATER_THAN_OR_EQUAL,
            FilterOperator.LESS_THAN_OR_EQUAL
        )
        FilterField.LOSSLESS -> listOf(
            FilterOperator.EQUALS
        )
        FilterField.FORMAT, FilterField.GENRE, FilterField.CODEC -> listOf(
            FilterOperator.EQUALS,
            FilterOperator.NOT_EQUALS,
            FilterOperator.IN,
            FilterOperator.NOT_IN
        )
        else -> listOf(
            FilterOperator.EQUALS,
            FilterOperator.NOT_EQUALS,
            FilterOperator.CONTAINS,
            FilterOperator.NOT_CONTAINS
        )
    }
}

private fun formatFilterRule(rule: FilterRule): String {
    val fieldName = formatFieldName(rule.field)
    val operatorName = when (rule.operator) {
        FilterOperator.EQUALS -> "is"
        FilterOperator.NOT_EQUALS -> "is not"
        FilterOperator.GREATER_THAN -> ">"
        FilterOperator.LESS_THAN -> "<"
        FilterOperator.GREATER_THAN_OR_EQUAL -> ">="
        FilterOperator.LESS_THAN_OR_EQUAL -> "<="
        FilterOperator.CONTAINS -> "contains"
        FilterOperator.NOT_CONTAINS -> "does not contain"
        FilterOperator.IN -> "in"
        FilterOperator.NOT_IN -> "not in"
    }
    return "$fieldName $operatorName ${rule.value}"
}
