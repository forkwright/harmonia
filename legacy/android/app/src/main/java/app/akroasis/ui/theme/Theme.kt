package app.akroasis.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val DarkColorScheme = darkColorScheme(
    primary = Color(0xFFB87333),  // Bronze
    secondary = Color(0xFFCD7F32),  // Copper
    tertiary = Color(0xFF9966CC)  // Amethyst
)

private val LightColorScheme = lightColorScheme(
    primary = Color(0xFFB87333),
    secondary = Color(0xFFCD7F32),
    tertiary = Color(0xFF9966CC)
)

@Composable
fun AkroasisTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit
) {
    val colorScheme = when {
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}
